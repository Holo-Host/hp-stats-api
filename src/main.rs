use rocket::serde::json::Json;
use rocket::*;
use rocket::{self, get, post, State};

mod db;
mod handlers;
mod types;

use handlers::list_available_hosts;
use types::{ApiError, Capacity, HostInfo, HostStats, Result, Uptime};

#[cfg(test)]
mod test;

#[get("/")]
async fn index(pool: &State<db::AppDbPool>) -> Result<String> {
    db::ping_database(&pool.mongo).await
}

#[delete("/cleanup")]
async fn cleanup(pool: &State<db::AppDbPool>) -> Result<String, ApiError> {
    db::cleanup_database(&pool.mongo).await
}

#[get("/<name>/uptime")]
async fn uptime(name: String, pool: &State<db::AppDbPool>) -> Result<Option<Json<Uptime>>> {
    if let Some(uptime) = db::host_uptime(name, &pool.mongo).await {
        return Ok(Some(Json(uptime)));
    }
    Ok(None)
}

#[get("/list-available?<hours>")]
async fn list_available(
    hours: u64,
    pool: &State<db::AppDbPool>,
) -> Result<Json<Vec<HostInfo>>, ApiError> {
    // TODO: return BAD_REQUEST if hours not passed
    let hosts = db::get_hosts_stats(&pool.mongo, hours).await?;
    let members = db::get_zerotier_members(&pool.mongo).await?;

    Ok(Json(list_available_hosts(hosts, members).await?))
}

#[get("/capacity")]
async fn capacity(pool: &State<db::AppDbPool>) -> Result<Json<Capacity>> {
    Ok(Json(db::network_capacity(&pool.mongo).await?))
}

#[post("/stats", format = "application/json", data = "<stats>")]
async fn add_host_stats(stats: HostStats, pool: &State<db::AppDbPool>) -> Result<(), ApiError> {
    db::add_host_stats(stats, pool).await
}

#[launch]
async fn rocket() -> _ {
    rocket::build()
        .manage(db::init_db_pool().await)
        .mount("/", rocket::routes![index, cleanup])
        .mount(
            "/hosts/",
            rocket::routes![uptime, list_available, add_host_stats],
        )
        .mount("/network/", rocket::routes![capacity])
}
