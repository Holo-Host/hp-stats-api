use mongodb::bson;
use rocket::serde::json::Json;
use rocket::*;
use rocket::{self, get, post, State};

mod db;
mod types;
use types::{ApiError, Capacity, HostStats, Result, Uptime};

// #[cfg(test)]
// mod tests;

// TODO: REMOVE - TEMPORARY TEST
#[get("/test")]
async fn basic_test() -> &'static str {
    "Success!"
}

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

// #[rocket::main]
// async fn main() -> Result<(), rocket::Error> {
//     rocket::build()
//         .manage(db::init_db_pool().await)
//         .mount("/", rocket::routes![index])
//         .mount(
//             "/hosts/",
//             rocket::routes![uptime, list_available, list_registered, add_host_stats],
//         )
//         .mount("/network/", rocket::routes![capacity])
//         .launch()
//         .await
// }

#[launch]
async fn rocket() -> _ {
    rocket::build()
        .manage(db::init_db_pool().await)
        .mount("/", rocket::routes![index, basic_test])
        .mount(
            "/hosts/",
            rocket::routes![uptime, list_available, list_registered, add_host_stats],
        )
        .mount("/network/", rocket::routes![capacity])
}

#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::http::ContentType;
    use rocket::http::Header;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    use crate::types;
    use types::HostStats;

    // #[rocket::async_test]
    // async fn basic_test() {
    //     let client = Client::tracked(super::rocket().await)
    //         .await
    //         .expect("valid rocket instance");
    //     let response = client.get("/test").dispatch().await;
    //     assert_eq!(response.status(), Status::Ok);
    //     assert_eq!(response.into_string().await.unwrap(), "Success!");
    // }

    #[rocket::async_test]
    async fn add_host_stats() {
        let payload = HostStats {
            holo_network: Some("holo_network".to_string()),
            channel: Some("channel".to_string()),
            holoport_model: Some("holoport_model".to_string()),
            ssh_status: Some(true),
            zt_ip: Some("zt_ip".to_string()),
            wan_ip: Some("wan_ip".to_string()),
            holoport_id: "4sycear14xnlwjiopbqm7k085qntog2oeps47c6lmvee33xjco".to_string(),
            timestamp: Some("timestamp".to_string()),
        };

        let valid_signature = "oAcrxO0Xn2/Rub7BsNLgYRE1Km8Hn/+eWeYf2hpFziQ3qRRzwOEdEm+L9UvZK6FDLJf//BNPQrrTAZW0X6doAw";

        let client = Client::tracked(super::rocket().await)
            .await
            .expect("valid rocket instance");
        let response = client
            .post("/hosts/stats")
            .json(&payload)
            .header(ContentType::JSON)
            .header(Header::new("x-hpos-signature", valid_signature))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
    }
}
