use rocket::serde::json::Json;
use rocket::{self, get, State};

mod db;
mod types;
use types::{Capacity, Host, Result, Uptime};

#[get("/")]
async fn index(pool: &State<db::AppDbPool>) -> Result<String> {
    db::ping_database(&pool.db).await
}

#[get("/statistics/<name>")]
async fn statistics(name: String, pool: &State<db::AppDbPool>) -> Result<Option<Json<Uptime>>> {
    if let Some(uptime) = db::host_uptime(name, &pool.db).await {
        return Ok(Some(Json(uptime)));
    }
    Ok(None)
}

#[get("/list")]
async fn list_all(pool: &State<db::AppDbPool>) -> Result<Json<Vec<Host>>> {
    Ok(Json(db::list_all_hosts(&pool.db).await?))
}

#[get("/capacity")]
async fn capacity(pool: &State<db::AppDbPool>) -> Result<Json<Capacity>> {
    Ok(Json(db::network_capacity(&pool.db).await?))
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    rocket::build()
        .manage(db::init_db_pool().await)
        .mount("/", rocket::routes![index])
        .mount("/host/", rocket::routes![statistics, list_all])
        .mount("/network/", rocket::routes![capacity])
        .launch()
        .await
}
