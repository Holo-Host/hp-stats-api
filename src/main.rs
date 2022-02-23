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
// use rocket::figment::value::Value;

mod db;
mod types;
use types::{
    ApiError, Capacity, HoloportStatus, HostError, HostSignature, HostStats, HostSummary, Result,
    Uptime,
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

#[post("/stats", data = "<stats>")]
async fn update_stats(stats: HostStats, pool: &State<db::AppDbPool>) -> Result<(), ApiError> {
    // TODO: Return ApiError instead of unwraps:
    let decoded_pubkey = base36::decode(&stats.holoport_id).unwrap();
    let public_key = PublicKey::from_bytes(&decoded_pubkey).unwrap();

    // Confirm host exists in registration records
    db::verify_host(
        stats.email,
        to_holochain_encoded_agent_key(&public_key),
        &pool.mongo,
    )
    .await
    .or_else(|_| {
        // todo: update error
        return Err("Failure((Status::Unauthorized, HostError::MissingRecord))");
    });

    // Add utc timestamp to stats payload and insert into db
    let timestamp = SystemTime::now();
    let holoport_status = HoloportStatus {
        timestamp,
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
            rocket::routes![uptime, list_available, list_registered],
        )
        .mount("/network/", rocket::routes![capacity])
        .launch()
        .await
}

#[rocket::async_trait]
impl<'r> FromData<'r> for HostStats {
    type Error = HostError;

    async fn from_data(request: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        // todo: Handle Errors where `unwrap` is used
        // Use data guard on `/stats` POST to verify host's signature in headers
        if request.method() == Method::Post && request.uri().path() == "/stats" {
            // TODO-QUESTION: Ask what the header name in which signature will be passed (using generic 'x-custom-header' for now)
            let signature = request.headers().get_one("X-Custom-Header");
            match signature {
                Some(signature) => HostSignature(signature.to_string()),
                None => return Failure((Status::Unauthorized, HostError::MissingSignature)),
            };

            // todo: Move this into a try_from Data into HostStats:
            let body = data
                .open(data::ByteUnit::max_value())
                .stream_to(tokio::io::stdout())
                .await
                .unwrap();
            let host_stats: HostStats = match serde_json::from_str(&format!("{}", body)) {
                Ok(a) => a,
                Err(_) => return Failure((Status::UnprocessableEntity, HostError::InvalidPayload)),
            };

            // TODO-QUESTION:Ask what encoding is being used (assuming BASE64 right now)
            let decoded_sig = base64::decode(signature.unwrap()).unwrap();
            let ed25519_sig = Signature::from_bytes(&decoded_sig).unwrap();
            let decoded_data = data
                .open(data::ByteUnit::max_value())
                .into_bytes()
                .await
                .unwrap();

            let decoded_pubkey = base36::decode(&host_stats.holoport_id).unwrap();
            let public_key = PublicKey::from_bytes(&decoded_pubkey).unwrap();
            public_key
                .verify_strict(&decoded_data.value, &ed25519_sig)
                .or_else(|_| {
                    // todo: fix error reference
                    return Err("Failure((Status::Unauthorized, HostError::InvalidSignature))");
                });

            return Success(host_stats);
        }

        // Otherwise no data guard check needed
        Forward(data)
    }
}
