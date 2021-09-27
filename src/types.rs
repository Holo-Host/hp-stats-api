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
    #[serde(rename = "physicalAddress")]
    physical_address: String,
    zt_ipaddress: String,
    created_at: i64,
    pub uptime: f32,
}

// Return type for /hosts/uptime endpoint
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Uptime {
    pub uptime: f32,
}

// Data schema in `holoports_status` collection
// and return type for /hosts/list endpoint
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct Host {
    #[serde(skip)]
    _id: ObjectId,
    pub name: String,
    #[serde(rename = "IP")]
    ip: String,
    pub timestamp: i64,
    ssh_success: bool,
    holo_network: Option<String>,
    channel: Option<String>,
    holoport_model: Option<String>,
    hosting_info: Option<String>,
    error: Option<String>,
    pub alpha_test: Option<bool>,
    pub assigned_to: Option<String>,
}

// Data schema in `holoports_assignment` collection
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub name: String,
    pub assigned_to: String,
}
