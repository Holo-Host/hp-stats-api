use mongodb::bson;
use rocket::serde::json::Json;
use rocket::*;
use rocket::{self, get, post, State};

mod db;
mod types;
use types::{ApiError, Capacity, HostStats, Result, Uptime};

#[cfg(test)]
mod test;

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

#[get("/capacity")]
async fn capacity(pool: &State<db::AppDbPool>) -> Result<Json<Capacity>> {
    Ok(Json(db::network_capacity(&pool.mongo).await?))
}

#[post("/stats", format = "application/json", data = "<stats>")]
async fn add_host_stats(stats: HostStats, pool: &State<db::AppDbPool>) -> Result<(), ApiError> {
    Ok(db::add_host_stats(stats, &pool).await?)
}

#[launch]
async fn rocket() -> _ {
    rocket::build()
        .manage(db::init_db_pool().await)
        .mount("/", rocket::routes![index])
        .mount(
            "/hosts/",
            rocket::routes![uptime, list_available, add_host_stats],
        )
        .mount("/network/", rocket::routes![capacity])
}
