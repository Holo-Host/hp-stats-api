use ed25519_dalek::VerifyingKey;
use hpos_config_core::public_key::to_holochain_encoded_agent_key;
use mongodb::bson::{self, doc, DateTime, Document};
use mongodb::options::AggregateOptions;
use mongodb::{Client, Collection};
use rocket::futures::TryStreamExt;
use rocket::response::Debug;
use rocket::State;
use std::convert::{TryFrom, TryInto};
use std::env::var;
use std::time::{Duration, SystemTime};

use crate::types::{
    ApiError, Capacity, Error400, Error404, HostRegistration, HostStats, Performance, Result,
    Uptime, ZerotierMember,
};

const HOURS_TOO_LARGE: Error400 =
    Error400::Message("Hours specified is too large. Cutoff is earlier than start of unix epoch");

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
    Ok("Connected to db. v0.0.2".to_string())
}

// Delete from host_statistics.holoport_status documents with
// timestamp field older than 14 days. Used for database cleanup
pub async fn cleanup_database(db: &Client) -> Result<String, ApiError> {
    let hp_status: Collection<HostStats> =
        db.database("host_statistics").collection("holoport_status");

    let cutoff_ms = match get_cutoff_timestamp(14 * 24) {
        Some(x) => x,
        None => return Err(ApiError::BadRequest(HOURS_TOO_LARGE)),
    };

    let val = doc! {
        "timestamp": {
          "$lt": cutoff_ms
        }
    };

    match hp_status.delete_many(val, None).await {
        Ok(res) => Ok(format!(
            "Deleted {} documents from holoport_status collection",
            res.deleted_count
        )),
        Err(e) => Err(ApiError::Database(Debug(e))),
    }
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

pub async fn get_zerotier_members(db: &Client) -> Result<Vec<ZerotierMember>, ApiError> {
    let zerotier_snapshot: Collection<ZerotierMember> =
        db.database("host_statistics").collection("latest_raw_snap");

    // Use aggregation pipeline to extract only relevant fields from database
    let pipeline = vec![
        doc! {
            "$match": {
              "config.authorized": true
            }
        },
        doc! {
            // Rename key _id to holoportId
            "$project": {
              "lastOnline": 1,
              "zerotierIp": { "$first": "$config.ipAssignments" },
              "physicalAddress": 1,
              "name": 1,
              "description": 1
              }
        },
    ];

    let options = AggregateOptions::builder().allow_disk_use(true).build();

    let cursor = zerotier_snapshot
        .aggregate(pipeline, Some(options))
        .await
        .map_err(Debug)
        .map_err(ApiError::Database)?;

    cursor
        .try_filter_map(|member| async { Ok(Some(bson::from_document(member)?)) })
        .try_collect()
        .await
        .map_err(Debug)
        .map_err(ApiError::Database)
}

// Return the most recent record for hosts stored in `holoport_status` collection
// Ignores records older than <cutoff> hours
pub async fn get_hosts_stats(db: &Client, cutoff: u64) -> Result<Vec<HostStats>, ApiError> {
    let cutoff_ms = match get_cutoff_timestamp(cutoff) {
        Some(x) => x,
        None => return Err(ApiError::BadRequest(HOURS_TOO_LARGE)),
    };

    let hp_status: Collection<HostStats> =
        db.database("host_statistics").collection("holoport_status");

    let pipeline = vec![
        doc! {
            // get entries within last <cutoff> hours
            "$match": {
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
            // Group by holoport ID and return first (the most recent) record
            "$group": {
                "_id": "$holoportId",
                "holoNetwork": {"$first": "$holoNetwork"},
                "channel": {"$first": "$channel"},
                "holoportModel": {"$first": "$holoportModel"},
                "sshStatus": {"$first": "$sshStatus"},
                "ztIp": {"$first": "$ztIp"},
                "wanIp": {"$first": "$wanIp"},
                "timestamp": {"$first": "$timestamp"},
                "hposAppList": {"$first": "$hposAppList"},
                "channelVersion": {"$first": "$channelVersion"},
                "hposVersion": {"$first": "$hposVersion"},
            }
        },
        doc! {
            // Rename key _id to holoportId
            "$project": {
                "_id": 0,
                "holoportId": "$_id",
                "holoNetwork": "$holoNetwork",
                "channel": "$channel",
                "holoportModel": "$holoportModel",
                "sshStatus": "$sshStatus",
                "ztIp": "$ztIp",
                "wanIp": "$wanIp",
                "timestamp":"$timestamp",
                "hposAppList": "$hposAppList",
                "channelVersion": "$channelVersion",
                "hposVersion": "$hposVersion",
              }
        },
    ];

    let options = AggregateOptions::builder().allow_disk_use(true).build();

    let cursor = hp_status
        .aggregate(pipeline, Some(options))
        .await
        .map_err(Debug)
        .map_err(ApiError::Database)?;

    cursor
        .try_filter_map(|host| async { Ok(Some(bson::from_document(host)?)) })
        .try_collect()
        .await
        .map_err(Debug)
        .map_err(ApiError::Database)
}

// Helper function to get cutoff timestamp for filter
// We use u64 for hours because otherwise we have to recast as u64 in the function, and 4 bytes isn't a big deal here
// Returns None if hours is too large and causes negative timestamp (propagates .checked_sub() which does the same)
fn get_cutoff_timestamp(hours: u64) -> Option<i64> {
    let current_timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime should be after unix epoch");
    let valid_duration = Duration::from_secs(60 * 60 * hours);

    let cutoff_timestamp = current_timestamp.checked_sub(valid_duration)?;
    use std::convert::TryInto;
    Some(
        cutoff_timestamp
            .as_secs()
            .try_into()
            .expect("We should be fewer than 2^63 milliseconds since start of unix epoch"),
    )
}

// Add values to the collection `holoport_status`
pub async fn add_holoport_status(hs: HostStats, db: &Client) -> Result<(), ApiError> {
    let hp_status: Collection<Document> =
        db.database("host_statistics").collection("holoport_status");
    let hpos_app_list =
        match bson::to_bson(&hs.hpos_app_list) {
            Ok(bson) => bson,
            Err(e) => return Err(ApiError::InvalidPayload(Error400::Info(format!(
                "The `hposAppList` field within the `hosts/stats` payload does not match expected format. Error: {:?}",
                e
            )))),
        };

    let val = doc! {
        "holoNetwork": hs.holo_network,
        "channel": hs.channel,
        "holoportModel": hs.holoport_model,
        "sshStatus": hs.ssh_status,
        "ztIp": hs.zt_ip,
        "wanIp": hs.wan_ip,
        "holoportId": hs.holoport_id,
        "timestamp": hs.timestamp,
        "hposAppList": hpos_app_list,
        "channelVersion": hs.channel_version,
        "hposVersion": hs.hpos_version,
        "dateCreated": DateTime::now(), // Field of bson type Date required for TTL index
    };
    match hp_status.insert_one(val, None).await {
        Ok(_) => Ok(()),
        Err(e) => Err(ApiError::Database(Debug(e))),
    }
}

/// Ops Console DB:
/// Find registration values for host identified in the `opsconsoledb` collection `registration`
/// and determine whether the provided host pub key exists within record
pub async fn verify_host(pub_key: String, db: &Client) -> Result<(), ApiError> {
    let records: Collection<HostRegistration> =
        db.database("opsconsoledb").collection("registrations");

    let mut host_registrations = match records.find(None, None).await {
        Ok(cursor) => cursor,
        Err(e) => return Err(ApiError::Database(Debug(e))),
    };

    let mut host_collection = Vec::new();

    while let Some(hr) = host_registrations
        .try_next()
        .await
        .map_err(Debug)
        .map_err(ApiError::Database)?
    {
        host_collection.push(hr);
    }

    if let Some(found) = host_collection.iter().find_map(|hr| {
        Some(
            hr.registration_code
                .iter()
                .any(|r| r.agent_pub_keys.iter().any(|keys| keys.pub_key == pub_key)),
        )
    }) {
        if found {
            return Ok(());
        }
    }

    Err(ApiError::MissingRecord(Error404::Info(format!(
        "No host found with provided public key. {:?}",
        pub_key
    ))))
}

pub fn decode_pubkey(holoport_id: &str) -> VerifyingKey {
    let decoded_pubkey = base36::decode(holoport_id).unwrap();
    VerifyingKey::from_bytes(&decoded_pubkey.try_into().unwrap()).unwrap()
}

pub async fn add_host_stats(stats: HostStats, pool: &State<AppDbPool>) -> Result<(), ApiError> {
    let ed25519_pubkey = decode_pubkey(&stats.holoport_id);

    // Confirm host exists in registration records
    let _ = verify_host(to_holochain_encoded_agent_key(&ed25519_pubkey), &pool.mongo)
        .await
        .map_err(|e| {
            ApiError::MissingRecord(Error404::Info(format!(
                "Provided host's holoport_id is not registered among valid hosts.  Error: {:?}",
                e
            )))
        });

    // Add utc timestamp to stats payload and insert into db
    let holoport_status = HostStats {
        timestamp: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()
            .map(|t| i64::try_from(t.as_secs()).ok().unwrap_or(0)),
        ..stats
    };
    add_holoport_status(holoport_status, &pool.mongo).await?;
    Ok(())
}
