use base64;
use ed25519_dalek::Signature;
use rocket::data::{self, Data, FromData};
use rocket::http::{Method, Status};
use rocket::outcome::Outcome::*;
use rocket::request::Request;
use rocket::{
    response::Responder,
    serde::{Deserialize, Serialize},
};

use bson::oid::ObjectId;
use mongodb::{bson, error::Error};
use rocket::response::Debug;

use super::db;

// [rocket::response::Debug](https://api.rocket.rs/v0.5-rc/rocket/response/struct.Debug.html) implements Responder to Error
pub type Result<T, E = Debug<Error>> = std::result::Result<T, E>;

// Debug errors default to 500
pub type Error500 = Debug<Error>;

#[derive(Responder, Debug)]
#[response(status = 400)]
pub enum Error400 {
    Info(String),
    Message(&'static str),
}

#[derive(Responder, Debug)]
#[response(status = 401)]

pub enum Error401 {
    Info(String),
    Message(&'static str),
}

#[derive(Responder, Debug)]
#[response(status = 404)]
// Disable warning if any type within 404 enum is unused
#[allow(dead_code)]
pub enum Error404 {
    Info(String),
    Message(&'static str),
}

#[derive(Responder, Debug)]
pub enum ApiError {
    BadRequest(Error400),
    Database(Error500),
    InvalidPayload(Error400),
    MissingRecord(Error404),
    MissingSignature(Error401),
    InvalidSignature(Error401),
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

// Data schema in `holoports_assignment` collection
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub name: String,
}

// Input type for /hosts/stats endpoint
// Data schema in collection `holoport_status`
// Note: We wrap each feild value in Option<T> because if the HPOS `netstatd` fails to collect data, it will send null in failed field.
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct HostStats {
    pub holo_network: Option<String>,
    pub channel: Option<String>,
    pub holoport_model: Option<String>,
    pub ssh_status: Option<bool>,
    pub zt_ip: Option<String>,
    pub wan_ip: Option<String>,
    pub holoport_id: String,
    pub timestamp: Option<i64>,
}

// Type for the list/available endpoint
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct HostLatest {
    #[serde(rename = "_id")]
    pub _id: String,
    pub holo_network: Option<String>,
    pub channel: Option<String>,
    pub holoport_model: Option<String>,
    pub ssh_status: Option<bool>,
    pub zt_ip: Option<String>,
    pub wan_ip: Option<String>,
    pub timestamp: Option<i64>,
}

#[rocket::async_trait]
impl<'r> FromData<'r> for HostStats {
    type Error = ApiError;

    async fn from_data(request: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        // Use data guard on `/stats` POST to verify host's signature in headers
        if request.method() == Method::Post && request.uri().path() == "/hosts/stats" {
            let signature = match request.headers().get_one("x-hpos-signature") {
                Some(signature) => signature.to_string(),
                None => {
                    return Failure((
                        Status::Unauthorized,
                        ApiError::MissingSignature(Error401::Message(
                            "Host's hpos signature was not located in the request headers.",
                        )),
                    ))
                }
            };

            let byte_unit_data = data.open(data::ByteUnit::max_value());
            let decoded_data = byte_unit_data.into_bytes().await.unwrap();
            let host_stats: HostStats = match serde_json::from_slice(&decoded_data.value) {
                Ok(hs) => hs,
                Err(e) => {
                    return Failure((
                        Status::UnprocessableEntity,
                        ApiError::InvalidPayload(Error400::Info(
                            format!("Provided payload to `hosts/stats` does not match expected payload. Error: {:?}", e),
                        )),
                    ));
                }
            };

            let decoded_sig = match base64::decode(signature) {
                Ok(ds) => ds,
                Err(e) => {
                    return Failure((
                        Status::UnprocessableEntity,
                        ApiError::InvalidSignature(Error401::Info(
                            format!("Provided signature to `hosts/stats` does not have the expected encoding. Error: {:?}", e),
                        )),
                    ));
                }
            };
            let ed25519_sig = Signature::from_bytes(&decoded_sig).unwrap();

            let ed25519_pubkey = db::decode_pubkey(&host_stats.holoport_id);

            return match ed25519_pubkey.verify_strict(&decoded_data.value, &ed25519_sig) {
                Ok(_) => Success(host_stats),
                Err(_) => Failure((
                    Status::Unauthorized,
                    ApiError::InvalidSignature(Error401::Message(
                        "Provided host signature does not match signature of signed payload.",
                    )),
                )),
            };
        }
        Failure((
            Status::BadRequest,
            ApiError::BadRequest(Error400::Message(
                "Made an unrecognized api call with the `HostStats` struct as parameters.",
            )),
        ))
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct NumberInt {
    pub number_int: u16,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct NumberLong {
    pub number_long: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct DateCreated {
    pub date: NumberLong,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct AgentPubKeys {
    pub pub_key: String,
    pub role: String,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct RegistrationCode {
    pub code: String,
    pub role: String,
    pub agent_pub_keys: Vec<AgentPubKeys>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct OldHoloportIds {
    pub _id: String,
    pub processed: bool,
    pub new_id: String,
}

// Data schema in database `opsconsoledb`, collection `registration`
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct HostRegistration {
    #[serde(skip)]
    pub _id: ObjectId,
    #[serde(skip)]
    pub __v: NumberInt,
    pub given_names: String,
    pub email: String,
    pub last_name: String,
    pub is_jurisdiction_not_in_list: bool,
    pub legal_jurisdiction: String,
    pub created: DateCreated,
    pub old_holoport_ids: Vec<OldHoloportIds>,
    pub registration_code: Vec<RegistrationCode>,
}
