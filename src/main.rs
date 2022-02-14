use mongodb::bson;
use rocket::serde::json::Json;
use rocket::{self, get, State};

mod db;
mod types;
use types::{Capacity, HostSummary, Result, Uptime, ListAvailableError};

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
async fn list_available(days: u64, pool: &State<db::AppDbPool>) -> Result<Json<Vec<HostSummary>>, ListAvailableError> {
    Ok(Json(db::list_available_hosts(&pool.mongo, days).await?))
}

#[get("/registered?<days>")]
async fn list_registered(days: u64, pool: &State<db::AppDbPool>) -> Result<Json<Vec<bson::Bson>>, ListAvailableError> {
    Ok(Json(db::list_registered_hosts(&pool.mongo, days).await?))
}

#[get("/capacity")]
async fn capacity(pool: &State<db::AppDbPool>) -> Result<Json<Capacity>> {
    Ok(Json(db::network_capacity(&pool.mongo).await?))
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
