use mongodb::bson::{self, doc};
use mongodb::options::AggregateOptions;
use mongodb::{Client, Collection};
use rocket::futures::TryStreamExt;
use rocket::response::Debug;
use std::collections::HashMap;
use std::env::var;

use crate::types::{Assignment, Capacity, Host, HostSummary, Performance, Result, Uptime};

// AppDbPool is managed by Rocket as a State, which means it is available across threads.
// Type mongodb::Database (starting v2.0.0 of mongodb driver) represents a connection pool to db,
// therefore it is thread-safe and can be passed around between threads via State
pub struct AppDbPool {
    pub mongo: Client,
}

// Initialize database and return in form of an AppDbPool
pub async fn init_db_pool() -> AppDbPool {
    let mongo_uri: String = var("MONGO_URI").expect("MONGO_URI must be set in the env");
    let client = Client::with_uri_str(mongo_uri).await.unwrap();

    AppDbPool { mongo: client }
}

// Ping database and return a string if successful
// Timeouts to 500 error response
pub async fn ping_database(db: &Client) -> Result<String> {
    db.database("host_statistics")
        .run_command(doc! {"ping": 1}, None)
        .await?;
    Ok(format!("Connected to db. v0.0.2"))
}

// Find a value of uptime for host identified by its name in a collection `performance_summary`
// Returns 404 if not found
pub async fn host_uptime(name: String, db: &Client) -> Option<Uptime> {
    let records: Collection<Performance> = db
        .database("host_statistics")
        .collection("performance_summary");

    if let Some(host) = records
        .find_one(Some(doc! {"name": name}), None)
        .await
        .unwrap()
    {
        return Some(Uptime {
            uptime: host.uptime,
        });
    }
    None
}

// Calculate network capacity from all the records in `performance_summary` collection
pub async fn network_capacity(db: &Client) -> Result<Capacity> {
    let records: Collection<Performance> = db
        .database("host_statistics")
        .collection("performance_summary");
    let cursor = records.find(None, None).await?;

    // cursor is a stream so it requires try_fold() from TryStreamExt
    cursor
        .try_fold(
            Capacity {
                total_hosts: 0,
                read_only: 0,
                source_chain: 0,
            },
            |mut total_capacity, performance| async move {
                total_capacity.add_host(performance.uptime);
                Ok(total_capacity)
            },
        )
        .await
        .map_err(Debug)
}

// Return all the hosts stored in `holoports_status` collection
pub async fn list_all_hosts(db: &Client) -> Result<Vec<HostSummary>> {
    // Retrieve and store in memory all holoport assignments
    let hp_assignment: Collection<Assignment> = db
        .database("host_statistics")
        .collection("alpha_program_holoports");

    let mut cursor = hp_assignment.find(None, None).await?;

    let mut assignment_map = HashMap::new();

    while let Some(a) = cursor.try_next().await? {
        assignment_map.insert(a.name, "");
    }

    // Retrieve all holoport statuses and format for an API response
    let hp_status: Collection<Host> = db
        .database("host_statistics")
        .collection("holoports_status");

    let pipeline = vec![
        doc! {
            // only successful ssh results
            "$match": {
                "sshSuccess": true
            }
        },
        doc! {
            // sort by timestamp, descending:
            "$sort": {
                "timestamp": -1
            }
        },
        doc! {
            "$group": {
                "_id": "$name",
                "IP": {"$first": "$IP"},
                "timestamp": {"$first": "$timestamp"},
                "sshSuccess": {"$first": "$sshSuccess"},
                "holoNetwork": {"$first": "$holoNetwork"},
                "channel": {"$first": "$channel"},
                "holoportModel": {"$first": "$holoportModel"},
                "hostingInfo": {"$first": "$hostingInfo"},
                "error": {"$first": "$error"},
            }
        },
    ];

    let options = AggregateOptions::builder().allow_disk_use(true).build();

    let cursor = hp_status.aggregate(pipeline, Some(options)).await?;

    // Update fields alpha_test and assigned_to based on the content of assignment_map
    let cursor_extended = cursor.try_filter_map(|host| async {
        let mut host: HostSummary = bson::from_document(host)?;
        if let Some(assigned_to) = assignment_map.get(&host._id) {
            host.alpha_program = Some(true);
            host.assigned_to = Some(assigned_to.to_string());
        }

        Ok(Some(host))
    });

    cursor_extended.try_collect().await.map_err(Debug)
}

pub async fn list_registered_hosts(db: &Client) -> Result<Vec<bson::Bson>> {
    // Retrieve all holoport statuses and format for an API response
    let hp_status: Collection<Host> = db
        .database("host_statistics")
        .collection("holoports_status");

    hp_status.distinct("name", None, None).await.map_err(Debug)
}
