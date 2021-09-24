use rocket::{self, get, State};

mod db;
mod types;
use types::Result;

#[get("/")]
async fn index(pool: &State<db::AppDbPool>) -> Result<String> {
    db::ping_database(&pool.db).await
}

#[get("/statistics/<id>")]
async fn statistics(id: String, pool: &State<db::AppDbPool>) -> Result<String> {
    db::host_statistics(id, &pool.db)
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
