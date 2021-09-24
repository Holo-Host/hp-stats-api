use mongodb::bson::{doc, Document};
use mongodb::{Client, Database, Collection};
use std::env::var;

use crate::types::Result;

// AppDbPool is managed by Rocket as a State, which means it is available across threads.
// Type mongodb::Database (starting v2.0.0 of mongodb driver) represents a connection pool to db,
// therefore it is thread-safe and can be passed around between threads via State
pub struct AppDbPool {
    pub db: Database,
}

// Initialize database and return in form of a AppDbPool
pub async fn init_db_pool() -> AppDbPool {
    let mongo_uri: String = var("MONGO_URI").expect("MONGO_URI must be set in the env");
    let client = Client::with_uri_str(mongo_uri).await.unwrap();

    AppDbPool {
        db: client.database("pjs-test"),
    }
}

// Ping database and return a string if successful
// Timeouts to 500 error response
pub async fn ping_database(db: &Database) -> Result<String> {
    db.run_command(doc! {"ping": 1}, None).await?;
    Ok(format!("Connected to db."))
}

// Find a value of uptime for host identified by its name in a collection `performance_summary`
// Returns 404 if not found
pub async fn host_statistics(name: String, db: &Database) -> Result<Option<String>> {
    let records: Collection<Document> = db.collection("performance_summary");

    if let Some(record) = records.find_one(Some(doc! {"name": name}), None).await?{
        if let Ok(val) = record.get_str("uptime") {
            return Ok(Some(val.to_string()))
        }
    }
    Ok(None)
}
