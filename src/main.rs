use rocket::{self, get, State};

mod db;

#[get("/")]
async fn index(pool: &State<db::AppDbPool>) -> db::Result<String> {
    db::ping_database(&pool.db).await
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    rocket::build()
        .manage(db::init_db_pool().await)
        .mount("/", rocket::routes![index])
        .launch()
        .await
}
