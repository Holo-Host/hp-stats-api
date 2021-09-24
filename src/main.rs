use rocket::{self, get, State};

mod db;
mod types;
use types::Result;

#[get("/")]
async fn index(pool: &State<db::AppDbPool>) -> Result<String> {
    db::ping_database(&pool.db).await
}

#[get("/statistics/<name>")]
async fn statistics(name: String, pool: &State<db::AppDbPool>) -> Result<Option<String>> {
    db::host_statistics(name, &pool.db).await
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    rocket::build()
        .manage(db::init_db_pool().await)
        .mount("/", rocket::routes![index])
        .mount("/host/", rocket::routes![statistics])
        .launch()
        .await
}
