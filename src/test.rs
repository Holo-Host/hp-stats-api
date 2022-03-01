
use super::rocket;
use rocket::http::ContentType;
use rocket::http::Header;
use rocket::http::Status;
use rocket::local::asynchronous::Client;

use crate::types;
use types::HostStats;

// #[rocket::async_test]
// async fn basic_test() {
//     let client = Client::tracked(super::rocket().await)
//         .await
//         .expect("valid rocket instance");
//     let response = client.get("/test").dispatch().await;
//     assert_eq!(response.status(), Status::Ok);
//     assert_eq!(response.into_string().await.unwrap(), "Success!");
// }

#[rocket::async_test]
async fn add_host_stats() {
    let payload = HostStats {
        holo_network: Some("holo_network".to_string()),
        channel: Some("channel".to_string()),
        holoport_model: Some("holoport_model".to_string()),
        ssh_status: Some(true),
        zt_ip: Some("zt_ip".to_string()),
        wan_ip: Some("wan_ip".to_string()),
        holoport_id: "4sycear14xnlwjiopbqm7k085qntog2oeps47c6lmvee33xjco".to_string(),
        timestamp: Some("timestamp".to_string()),
    };

    let valid_signature =
        "oAcrxO0Xn2/Rub7BsNLgYRE1Km8Hn/+eWeYf2hpFziQ3qRRzwOEdEm+L9UvZK6FDLJf//BNPQrrTAZW0X6doAw";

    let client = Client::tracked(super::rocket().await)
        .await
        .expect("valid rocket instance");
    let response = client
        .post("/hosts/stats")
        .json(&payload)
        .header(ContentType::JSON)
        .header(Header::new("x-hpos-signature", valid_signature))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}
