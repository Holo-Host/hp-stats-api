use base64;
use ed25519_dalek::Signature;
use mongodb::bson;
use rocket::data::{self, Data, FromData};
use rocket::http::{Method, Status};
use rocket::outcome::Outcome::*;
use rocket::request::Request;
use rocket::serde::json::Json;
use rocket::{self, get, post, State};

mod db;
mod types;
use types::{ApiError, Capacity, Error400, Error401, HostStats, Result, Uptime};

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
) -> Result<Json<Vec<HostStats>>, ApiError> {
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
async fn add_host_stats(stats: HostStats, pool: &State<db::AppDbPool>) -> Result<(), ApiError> {
    Ok(db::add_host_stats(stats, &pool).await?)
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    rocket::build()
        .manage(db::init_db_pool().await)
        .mount("/", rocket::routes![index])
        .mount(
            "/hosts/",
            rocket::routes![uptime, list_available, list_registered, add_host_stats],
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
                        ApiError::MissingSignature(Error401::Message(
                            "Host's hpos signature was not located in the request headers.",
                        )),
                    ))
                }
            };

            let byte_unit_data = data.open(data::ByteUnit::max_value());
            let decoded_data = byte_unit_data.into_bytes().await.unwrap();
            let host_stats: HostStats = match serde_json::from_slice(&decoded_data.value) {
                Ok(hs) => hs,
                Err(e) => {
                    return Failure((
                        Status::UnprocessableEntity,
                        ApiError::InvalidPayload(Error400::Info(
                            format!("Provided payload to `hosts/stats` does not match expected payload. Error: {:?}", e),
                        )),
                    ));
                }
            };

            // TEMP NOTE: comment out for manual postman test - sig not verifiable
            let decoded_sig = match base64::decode(signature) {
                Ok(ds) => ds,
                Err(e) => {
                    return Failure((
                        Status::UnprocessableEntity,
                        ApiError::InvalidSignature(Error401::Info(
                            format!("Provided signature to `hosts/stats` does not have the expected encoding. Error: {:?}", e),
                        )),
                    ));
                }
            };
            let ed25519_sig = Signature::from_bytes(&decoded_sig).unwrap();

            let ed25519_pubkey = db::decode_pubkey(&host_stats.holoport_id_base36);

            // TEMP NOTE: comment out for manual postman test - sig not verifiable
            return match ed25519_pubkey.verify_strict(&decoded_data.value, &ed25519_sig) {
                Ok(_) => Success(host_stats),
                Err(_) => Failure((
                    Status::Unauthorized,
                    ApiError::InvalidSignature(Error401::Message(
                        "Provided host signature does not match signature of signed payload.",
                    )),
                )),
            };

            // TEMP NOTE: comment in for manual postman test - sig not verifiable
            // return Success(host_stats);
        }
        Failure((
            Status::BadRequest,
            ApiError::BadRequest(Error400::Message(
                "Made an unrecognized api call with the `HostStats` struct as parameters.",
            )),
        ))
    }
}
