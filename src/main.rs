use base64;
use ed25519_dalek::{PublicKey, Signature};
use hpos_config_core::public_key::to_holochain_encoded_agent_key;
use mongodb::bson;
use rocket::data::{self, Data, FromData};
use rocket::http::{Method, Status};
use rocket::outcome::Outcome::*;
use rocket::request::Request;
use rocket::serde::json::Json;
use rocket::{self, get, post, State};
use std::time::SystemTime;

mod db;
mod types;
use types::{
    ApiError, Capacity, ErrorMessage, ErrorMessageInfo, HoloportStatus, HostStats, HostSummary,
    Result, Uptime,
};

#[get("/")]
async fn index(pool: &State<db::AppDbPool>) -> Result<String> {
    db::ping_database(&pool.mongo).await
}

#[get("/<name>/uptime")]
async fn uptime(name: String, pool: &State<db::AppDbPool>) -> Result<Option<Json<Uptime>>> {
    if let Some(uptime) = db::host_uptime(name, &pool.mongo).await {
        return Ok(Some(Json(uptime)));
    }
    Ok(None)
}

#[get("/list-available?<days>")]
async fn list_available(
    days: u64,
    pool: &State<db::AppDbPool>,
) -> Result<Json<Vec<HostSummary>>, ApiError> {
    Ok(Json(db::list_available_hosts(&pool.mongo, days).await?))
}

#[get("/registered?<days>")]
async fn list_registered(
    days: u64,
    pool: &State<db::AppDbPool>,
) -> Result<Json<Vec<bson::Bson>>, ApiError> {
    Ok(Json(db::list_registered_hosts(&pool.mongo, days).await?))
}

#[get("/capacity")]
async fn capacity(pool: &State<db::AppDbPool>) -> Result<Json<Capacity>> {
    Ok(Json(db::network_capacity(&pool.mongo).await?))
}

#[post("/stats", format = "application/json", data = "<stats>")]
async fn update_stats(stats: HostStats, pool: &State<db::AppDbPool>) -> Result<(), ApiError> {
    // todo: move this into `decode_pubkey` common fn
    let decoded_pubkey = base36::decode(&stats.holoport_id).unwrap();
    let public_key = PublicKey::from_bytes(&decoded_pubkey).unwrap();

    // Confirm host exists in registration records
    let _ = db::verify_host(
        stats.email,
        to_holochain_encoded_agent_key(&public_key),
        &pool.mongo,
    )
    .await
    .or_else(|e| {
        return Err(ApiError::MissingRecord(ErrorMessageInfo(format!(
            "Provided host's holoport_id is not registered among valid hosts.  Error: {:?}",
            e
        ))));
    });

    // Add utc timestamp to stats payload and insert into db
    let holoport_status = HoloportStatus {
        timestamp: format!("{:?}", SystemTime::now()),
        holo_network: stats.holo_network,
        channel: stats.channel,
        holoport_model: stats.holoport_model,
        ssh_success: stats.ssh_status,
        ip: stats.wan_ip,
        holoport_id: stats.holoport_id,
        hosting_info: None,
        error: None,
    };
    db::add_holoport_status(holoport_status, &pool.mongo).await?;
    Ok(())
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    rocket::build()
        .manage(db::init_db_pool().await)
        .mount("/", rocket::routes![index])
        .mount(
            "/hosts/",
            rocket::routes![uptime, list_available, list_registered, update_stats],
        )
        .mount("/network/", rocket::routes![capacity])
        .launch()
        .await
}

#[rocket::async_trait]
impl<'r> FromData<'r> for HostStats {
    type Error = ApiError;

    async fn from_data(request: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        // Use data guard on `/stats` POST to verify host's signature in headers
        if request.method() == Method::Post && request.uri().path() == "/hosts/stats" {
            let signature = match request.headers().get_one("x-hpos-signature") {
                Some(signature) => signature.to_string(),
                None => {
                    return Failure((
                        Status::Unauthorized,
                        ApiError::MissingSignature(ErrorMessage(
                            "Host's hpos signature was not located in the request headers.",
                        )),
                    ))
                }
            };

            let byte_unit_data = data.open(data::ByteUnit::max_value());
            let decoded_data = byte_unit_data.into_bytes().await.unwrap();
            let host_stats: HostStats = match serde_json::from_slice(&decoded_data.value) {
                Ok(a) => a,
                Err(e) => {
                    return Failure((
                        Status::UnprocessableEntity,
                        ApiError::InvalidPayload(ErrorMessageInfo(
                            format!("Provided payload to `hosts/stats` does not match expected payload. Error: {:?}", e),
                        )),
                    ));
                }
            };

            // TEMP NOTE: comment out in manual test - sig not verifiable
            let decoded_sig = base64::decode(signature).unwrap();
            let ed25519_sig = Signature::from_bytes(&decoded_sig).unwrap();

            // todo: move this into `decode_pubkey` common fn
            let decoded_pubkey = base36::decode(&host_stats.holoport_id).unwrap();
            let public_key = PublicKey::from_bytes(&decoded_pubkey).unwrap();

            // TEMP NOTE: comment out in manual test - sig not verifiable
            return match public_key.verify_strict(&decoded_data.value, &ed25519_sig) {
                Ok(_) => Success(host_stats),
                Err(_) => Failure((
                    Status::Unauthorized,
                    ApiError::InvalidSignature(ErrorMessage(
                        "Provided host signature does not match signature of signed payload.",
                    )),
                )),
            };

            // NOTE: comment in manual for test - sig not verifiable
            // return Success(host_stats);
        }
        Failure((
            Status::BadRequest,
            ApiError::BadRequest(ErrorMessage(
                "Made an unrecognized call with HostStats parameters.",
            )),
        ))
    }
}
