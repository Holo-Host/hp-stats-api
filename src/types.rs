use rocket::{
    response::Responder,
    serde::{Deserialize, Serialize},
};

use bson::oid::ObjectId;
use mongodb::{bson, error::Error};
use rocket::response::Debug;

// [rocket::response::Debug](https://api.rocket.rs/v0.5-rc/rocket/response/struct.Debug.html) implements Responder to Error
pub type Result<T, E = Debug<Error>> = std::result::Result<T, E>;

#[derive(Responder, Debug)]
#[response(status = 400)]
pub struct ErrorMessage(pub &'static str);

#[derive(Responder, Debug)]
#[response(status = 400)]
pub struct ErrorMessageInfo(pub String);

#[derive(Responder, Debug)]
pub enum ApiError {
    BadRequest(ErrorMessage),
    Database(Debug<Error>),
    InvalidPayload(ErrorMessageInfo),
    MissingRecord(ErrorMessageInfo),
    MissingSignature(ErrorMessage),
    InvalidSignature(ErrorMessage),
}

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
    physical_address: Option<String>,
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
    pub timestamp: f64,
    ssh_success: bool,
    holo_network: Option<String>,
    channel: Option<String>,
    holoport_model: Option<String>,
    hosting_info: Option<String>,
    error: Option<String>,
    pub alpha_program: Option<bool>,
    pub assigned_to: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct HostSummary {
    #[serde(rename = "_id")]
    pub _id: String,
    #[serde(rename = "IP")]
    ip: String,
    pub timestamp: f64,
    ssh_success: bool,
    holo_network: Option<String>,
    channel: Option<String>,
    holoport_model: Option<String>,
    hosting_info: Option<String>,
    error: Option<String>,
    pub alpha_program: Option<bool>,
    pub assigned_to: Option<String>,
}

// Data schema in `holoports_assignment` collection
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub name: String,
}

// Input type for /hosts/stats endpoint
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct HostStats {
    pub email: String, // >> discuss adding email to `HostStats` schema to use as filter instead of pubkey/hostid
    pub holo_network: Option<String>,
    pub channel: Option<String>,
    pub holoport_model: Option<String>,
    pub ssh_status: bool, // why is this not passed in as ssh_success to match the schema
    pub zt_ip: String,    // what are we using this value for? > should it be added to the schema?
    pub wan_ip: String, // is this going to be the correct ip of the hp? - or does it still need to be configured with this as input?
    pub holoport_id: String,
}

// Data schema in collection `holoports_status`
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct HoloportStatus {
    #[serde(rename = "name")]
    pub holoport_id: String,
    #[serde(rename = "IP")]
    pub ip: String,
    pub timestamp: String,
    pub ssh_success: bool,
    pub holo_network: Option<String>,
    pub channel: Option<String>,
    pub holoport_model: Option<String>,
    pub hosting_info: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct NumberInt {
    number_int: u16,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct NumberLong {
    number_long: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct DateCreated {
    date: NumberLong,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct AgentPubKeys {
    pub pub_key: String,
    role: String,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct RegistrationCode {
    code: String,
    role: String,
    pub agent_pub_keys: Vec<AgentPubKeys>,
}

// Data schema in database `opsconsoledb`, collection `registration`
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct HostRegistration {
    #[serde(skip)]
    _id: ObjectId,
    #[serde(skip)]
    _v: NumberInt,
    given_names: String,
    last_name: String,
    is_jurisdiction_not_in_list: bool,
    legal_jurisdiction: String,
    created: DateCreated,
    old_holoport_ids: Vec<String>,
    pub registration_code: Vec<RegistrationCode>,
}
