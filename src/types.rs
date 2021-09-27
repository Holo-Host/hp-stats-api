use rocket::serde::{Deserialize, Serialize};

use bson::oid::ObjectId;
use mongodb::{bson, error::Error};
use rocket::response::Debug;

// [rocket::response::Debug](https://api.rocket.rs/v0.5-rc/rocket/response/struct.Debug.html) implements Responder to Error
pub type Result<T, E = Debug<Error>> = std::result::Result<T, E>;

// Return type for /network/capacity endpoint
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Capacity {
    pub total_hosts: u16,
    pub read_only: u16,
    pub source_chain: u16,
}

impl Capacity {
    pub fn add_host(&mut self, uptime: f32) {
        self.total_hosts += 1;
        if uptime >= 0.5 {
            self.read_only += 1
        };
        if uptime >= 0.9 {
            self.source_chain += 1
        };
    }
}

// Data schema in `performance_summary` collection
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Performance {
    _id: ObjectId,
    name: String,
    description: String,
    physicalAddress: String,
    zt_ipaddress: String,
    created_at: i64,
    pub uptime: f32,
}

// Return type for /host/statistics endpoint
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Uptime {
    pub uptime: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Host {
    _id: ObjectId,
    name: String,
    IP: String,
    pub timestamp: i64,
    sshSuccess: bool,
    holoNetwork: Option<String>,
    channel: Option<String>,
    holoportModel: Option<String>,
    hostingInfo: Option<String>,
    error: Option<String>,
    // alphaTest: bool,
    // assignedTo: Option<String>
}
