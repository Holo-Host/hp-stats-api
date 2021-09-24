// use rocket::serde::{Serialize};

use rocket::response::Debug;
use mongodb::error::Error;

// [rocket::response::Debug](https://api.rocket.rs/v0.5-rc/rocket/response/struct.Debug.html) implements Responder to Error
pub type Result<T, E = Debug<Error>> = std::result::Result<T, E>;

// #[derive(Serialize)]
// #[serde(crate = "rocket::serde")]
// pub struct Host {
//     name: String,
//     IP: String,
//     timestamp: String,
//     sshSuccess: bool,
//     holoNetwork: Option<String>,
//     channel: Option<String>,
//     holoportModel: Option<String>,
//     hostingInfo: Option<String>,
//     error: Option<String>,
//     alphaTest: bool,
//     assignedTo: Option<String>
// }


