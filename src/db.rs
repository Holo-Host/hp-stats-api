use mongodb::bson::doc;
use mongodb::{error::Error, Client, Database};

use rocket::response::Debug;

// [rocket::response::Debug](https://api.rocket.rs/v0.5-rc/rocket/response/struct.Debug.html) implements Responder to Error
pub type Result<T, E = Debug<Error>> = std::result::Result<T, E>;

// AppDbPool is managed by Rocket as a State, which means it is available across threads.
// Type mongodb::Database (starting v2.0.0 of mongodb driver) represents a connection pool to db,
// therefore it is thread-safe and can be passed around between threads via State
pub struct AppDbPool {
    pub db: Database,
}

// Initialize database and return in form of a AppDbPool
pub async fn init_db_pool() -> AppDbPool {
    let client_uri = "mongodb+srv://peeech:KHqu4aHZtlnvioQ4@cluster0.xfjzk.mongodb.net/";
    let client = Client::with_uri_str(client_uri).await.unwrap();

    AppDbPool {
        db: client.database("pjs-test"),
    }
}

// Ping database and return a string if successful
pub async fn ping_database(db: &Database) -> Result<String> {
    db.run_command(doc! {"ping": 1}, None).await?;
    Ok(format!("Connected to db."))
}
