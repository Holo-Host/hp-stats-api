use mongodb::bson::{self, doc, Document};
use mongodb::options::AggregateOptions;
use mongodb::{Client, Collection};
use rocket::futures::TryStreamExt;
use rocket::response::Debug;
use std::collections::HashMap;
use std::env::var;
use std::time::{Duration, SystemTime};

use crate::types::{
    ApiError, Assignment, Capacity, ErrorMessage, ErrorMessageInfo, Host, HostRegistration,
    HostStats, HostSummary, Performance, Result, Uptime,
};

const DAYS_TOO_LARGE: ErrorMessage =
    ErrorMessage("Days specified is too large. Cutoff is earlier than start of unix epoch");

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

// Return the most recent record for hosts stored in `holoports_status` collection that have a successful SSH record
// Ignores records older than <cutoff> days
pub async fn list_available_hosts(db: &Client, cutoff: u64) -> Result<Vec<HostSummary>, ApiError> {
    // Retrieve and store in memory all holoport assignments
    let hp_assignment: Collection<Assignment> = db
        .database("host_statistics")
        .collection("alpha_program_holoports");

    let mut cursor = hp_assignment
        .find(None, None)
        .await
        .map_err(Debug)
        .map_err(ApiError::Database)?;

    let mut assignment_map = HashMap::new();

    while let Some(a) = cursor
        .try_next()
        .await
        .map_err(Debug)
        .map_err(ApiError::Database)?
    {
        assignment_map.insert(a.name, "");
    }

    // Retrieve all holoport statuses and format for an API response
    let hp_status: Collection<Host> = db
        .database("host_statistics")
        .collection("holoports_status");

    let cutoff_ms = match get_cutoff_timestamp(cutoff) {
        Some(x) => x,
        None => return Err(ApiError::BadRequest(DAYS_TOO_LARGE)),
    };
    let pipeline = vec![
        doc! {
            // only successful ssh results in last <cutoff> days
            "$match": {
                "sshSuccess": true,
                "timestamp": {"$gte": cutoff_ms}
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

    let cursor = hp_status
        .aggregate(pipeline, Some(options))
        .await
        .map_err(Debug)
        .map_err(ApiError::Database)?;

    // Update fields alpha_test and assigned_to based on the content of assignment_map
    let cursor_extended = cursor.try_filter_map(|host| async {
        let mut host: HostSummary = bson::from_document(host)?;
        if let Some(assigned_to) = assignment_map.get(&host._id) {
            host.alpha_program = Some(true);
            host.assigned_to = Some(assigned_to.to_string());
        }

        Ok(Some(host))
    });

    cursor_extended
        .try_collect()
        .await
        .map_err(Debug)
        .map_err(ApiError::Database)
}

// This gets a list of all HPs including those not SSH'd
pub async fn list_registered_hosts(db: &Client, cutoff: u64) -> Result<Vec<bson::Bson>, ApiError> {
    // Retrieve all holoport statuses and format for an API response
    let hp_status: Collection<Host> = db
        .database("host_statistics")
        .collection("holoports_status");

    let cutoff_ms = match get_cutoff_timestamp(cutoff) {
        Some(x) => x,
        None => return Err(ApiError::BadRequest(DAYS_TOO_LARGE)),
    };

    let filter = doc!{"timestamp": {"$gte": cutoff_ms}};

    Ok(hp_status
        .distinct("name", filter, None)
        .await
        .map_err(Debug)
        .map_err(ApiError::Database)?)
}

// Helper function to get cutoff timestamp for filter
// We use u64 for days because otherwise we have to recast as u64 in the function, and 4 bytes isn't a big deal here
// Returns None if days is too large and causes negative timestamp (propagates .checked_sub() which does the same)
fn get_cutoff_timestamp(days: u64) -> Option<i64> {
    let current_timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime should be after unix epoch");
    let valid_duration = Duration::from_secs(60 * 60 * 24 * days);

    let cutoff_timestamp = current_timestamp.checked_sub(valid_duration)?;
    use std::convert::TryInto;
    Some(
        cutoff_timestamp
            .as_millis()
            .try_into()
            .expect("We should be fewer than 2^63 milliseconds since start of unix epoch"),
    )
}

// Add values to the collection `holoports_status`
pub async fn add_holoport_status(hs: HostStats, db: &Client) -> Result<(), ApiError> {
    let hp_status: Collection<Document> = db
        .database("host_statistics")
        .collection("holoports_status");
    let val = doc! {
        "holo_network": hs.holo_network,
        "channel": hs.channel,
        "holoport_model": hs.holoport_model,
        "ssh_status": hs.ssh_status,
        "zt_ip": hs.zt_ip,
        "wan_ip": hs.wan_ip,
        "holoport_id_base36": hs.holoport_id_base36,
        "timestamp": hs.timestamp,
        "email": hs.email
    };
    match hp_status.insert_one(val.clone(), None).await {
        Ok(_) => Ok(()),
        Err(e) => Err(ApiError::Database(Debug(e))),
    }
}

/// Ops Console DB:
// Find registration values for host identified by email in the `opsconsoledb` collection `registration`
// and determine whether the provided host pub key exists within record
pub async fn verify_host(email: String, pub_key: String, db: &Client) -> Result<(), ApiError> {
    let records: Collection<HostRegistration> = db
        .database("opsconsoledb")
        .collection("performance_summary");

    let host_registration = match records
        .find_one(Some(doc! {"email": email.clone()}), None)
        .await
    {
        Ok(r) => r.unwrap(),
        Err(e) => return Err(ApiError::Database(Debug(e))),
    };

    if host_registration
        .registration_code
        .iter()
        .any(|r| r.agent_pub_keys.iter().any(|keys| keys.pub_key == pub_key))
    {
        return Ok(());
    }

    Err(ApiError::MissingRecord(ErrorMessageInfo(format!(
        "No host found with provided email. {:?}",
        email
    ))))
}
