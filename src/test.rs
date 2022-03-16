use super::rocket;
use anyhow::{Context, Result};
use base64::encode_config;
use ed25519_dalek::*;
use holochain_conductor_api::{AppStatusFilter, InstalledAppInfo, InstalledAppInfoStatus};

#[allow(deprecated)]
use holochain_types::prelude::{CellId, HoloHash, InstalledCell};

use mongodb::bson::{doc, oid::ObjectId, Document};
use mongodb::Collection;
use rocket::http::ContentType;
use rocket::http::Header;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::response::Debug;
use std::collections::HashMap;
use std::env::var;
use test_case::test_case;

use crate::types;
use types::{
    AgentPubKeys, ApiError, DateCreated, HostRegistration, HostStats, NumberInt, NumberLong,
    OldHoloportIds, RegistrationCode,
};

async fn sign_payload(payload: &HostStats) -> Result<String> {
    let keypair =
    hpos_config_seed_bundle_explorer::unlock(&"k6VoY3NiMJGWonB3xBAAQN7Er6j8qToC9DmZLEuyzSAAAcQYio6xDNOQk3OUasqcEakpoYHPrJRGTduhxDFlOTHqQ5KPP4kYlSbq7xFRswXhPAfnfzI-JcAgkHwosIgqf0tYSCCzI8U0JOhNRLxJxCyCrWRldmljZV9udW1iZXIAq2dlbmVyYXRlX2J5r3F1aWNrc3RhcnQtdjIuMA".to_string(), Some("pass".to_string()))
        .await
        .context(format!("Unable to unlock the device bundle"))?;
    let payload = serde_json::to_vec(&payload).context("Failed to convert payload to bytes")?;
    let signature = keypair
        .try_sign(&payload)
        .context("Failed to sign payload")?;
    Ok(encode_config(
        &signature.to_bytes()[..],
        base64::STANDARD_NO_PAD,
    ))
}

// Add values to the collection `registrations`
async fn add_host_registration(
    hr: HostRegistration,
    local_db: &mongodb::Client,
) -> Result<(), ApiError> {
    let hp_status: Collection<Document> = local_db
        .database("opsconsoledb")
        .collection("registrations");
    let agent_pub_keys_doc = doc! {
        "pub_key": &hr.registration_code[0].agent_pub_keys[0].pub_key,
        "role": &hr.registration_code[0].agent_pub_keys[0].role,
    };

    let registration_code_doc = doc! {
        "code": &hr.registration_code[0].code,
        "role": &hr.registration_code[0].role,
        "agent_pub_keys": agent_pub_keys_doc
    };

    let number_long_doc = doc! {
        "number_long": hr.created.date.number_long.to_string()
    };

    let date_created_doc = doc! {
        "date": number_long_doc
    };

    let number_int_doc = doc! {
        "number_int": hr.__v.number_int.to_string()
    };

    let old_holoport_ids_doc = doc! {
        "_id": &hr.old_holoport_ids[0]._id,
        "processed": &hr.old_holoport_ids[0].processed,
        "newId": &hr.old_holoport_ids[0].new_id
    };

    let val = doc! {
        "_id" : hr._id,
        "givenNames" : hr.given_names,
        "lastName" : hr.last_name,
        "email" : hr.email,
        "isJurisdictionNotInList" : hr.is_jurisdiction_not_in_list,
        "legalJurisdiction" : hr.legal_jurisdiction,
        "created" : date_created_doc,
        "oldHoloportIds" : old_holoport_ids_doc,
        "registrationCode" : registration_code_doc,
        "__v" : number_int_doc
    };
    match hp_status.insert_one(val.clone(), None).await {
        Ok(_) => Ok(()),
        Err(e) => Err(ApiError::Database(Debug(e))),
    }
}

fn gen_mock_apps(count: i32) -> Vec<InstalledAppInfo> {
    let mut hpos_apps = Vec::new();
    for i in 0..count {
        hpos_apps.push(InstalledAppInfo {
            installed_app_id: format!("uhCkk...appId{:?}", i),
            cell_data: vec![InstalledCell {
                cell_id: CellId(
                    HoloHash::from_raw_39(HoloHash::get_raw_39(
                        "uhCkkcF0X1dpwHFeIPI6-7rzM6ma9IgyiqD-othxgENSkL1S",
                    )),
                    HoloHash::from_raw_39(HoloHash::get_raw_39(
                        "uhCAkOyRlY09kreaeLDd9-0bp-17DW2N4Vqx1kFodKTXFkrgFiA09",
                    )),
                ),
                role_id: format!("app_role_id_{:?}", i),
            }],
            status: InstalledAppInfoStatus::Running,
        })
    }
    hpos_apps
}

#[rocket::async_test]
async fn call_index() {
    let client = Client::tracked(super::rocket().await)
        .await
        .expect("valid rocket instance");
    let response = client.get("/").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.into_string().await.unwrap(),
        "Connected to db. v0.0.2"
    );
}

#[test_case(true ; "when signature is valid")]
#[test_case(false  ; "when signature is not valid")]
#[rocket::async_test]
async fn add_host_stats(pass_valid_signature: bool) {
    let mongo_uri: String = var("MONGO_URI").expect("MONGO_URI must be set in the env");
    let client = mongodb::Client::with_uri_str(mongo_uri).await.unwrap();

    // Pre-populate `opsconsoledb` registrations collection with host that has a `pub_key` matching the `holoport_id` sent in `/host/stats` post payload
    let host_registration = HostRegistration {
        _id: ObjectId::new(),
        given_names: "FirstName".to_string(),
        last_name: "LastName".to_string(),
        email: "first.last1@email.com".to_string(),
        is_jurisdiction_not_in_list: true,
        legal_jurisdiction: "United States".to_string(),
        created: DateCreated {
            date: NumberLong {
                number_long: 1646149410
            }
        },
        old_holoport_ids: vec![OldHoloportIds {
            _id: "0waaoeca1p8hcwmxpfupp6lr0ydy495qj9eoas1tq6qnblzpn".to_string(),
            processed: true,
            new_id: "52khmj02jl1xkl5mo6v0hoa4p2gftv33plgt69ay5i3ebjtu6k".to_string(),
        }],
        registration_code : vec![
            RegistrationCode {
                code : "Cv/aKR0JreZaeY0ioDoEDiSg78GiYwtZJYDmeLq6C7qN9p39kqu9RV8aSB8pzdVGiU/2STXfWZJC8kj3H2G4HA==".to_string(),
                role : "host".to_string(),
                agent_pub_keys : vec![
                    AgentPubKeys {
                        role : "host".to_string(),
                        // Note: The `pub_key` must be the holo_hash encoded version of the host's `holoport_id`
                        pub_key : "uhCAkOyRlY09kreaeLDd9-0bp-17DW2N4Vqx1kFodKTXFkrgFiA09".to_string()
                    }
                ]
            }
        ],
        __v: NumberInt {
            number_int: 16410
        },
    };

    let _ = add_host_registration(host_registration, &client).await;

    let mut hpos_app_list = HashMap::new();
    // hpos_app_list.insert("uhCkk...appId1".to_string(), AppStatusFilter::Running);
    // hpos_app_list.insert("uhCkk...appId2".to_string(), AppStatusFilter::Running);
    // hpos_app_list.insert("uhCkk...appId3".to_string(), AppStatusFilter::Running);
    // hpos_app_list.insert("uhCkk...appId4".to_string(), AppStatusFilter::Paused);
    // hpos_app_list.insert("uhCkk...appId5".to_string(), AppStatusFilter::Paused);
    // hpos_app_list.insert("uhCkk...appId6".to_string(), AppStatusFilter::Disabled);

    let hpos_happs_mock = gen_mock_apps(6);
    hpos_happs_mock.iter().for_each(|happ| {
        let happ_status = match &happ.status {
            InstalledAppInfoStatus::Paused { .. } => AppStatusFilter::Paused,
            InstalledAppInfoStatus::Disabled { .. } => AppStatusFilter::Disabled,
            InstalledAppInfoStatus::Running => AppStatusFilter::Running,
        };
        hpos_app_list.insert(happ.installed_app_id.clone(), happ_status);
    });

    // Create payload, sign payload, and call `/host/stats` endpoint, passing valid signature within call header
    let payload = HostStats {
        holo_network: None,
        channel: None,
        holoport_model: None,
        ssh_status: None,
        zt_ip: None,
        wan_ip: None,
        // Note: The `holoport_id` must be the base_36 encoded version of the `host_registration.registration_code[i].agent_pub_keys[i].pub_key`
        holoport_id: "1h2di6px7otkjwudmycadu5teaywao46jelpegg7jujncbcbzs".to_string(),
        timestamp: None,
        hpos_app_list: Some(hpos_app_list),
    };

    let signature;
    let status;
    if pass_valid_signature {
        signature = sign_payload(&payload).await.unwrap();
        status = Status::Ok;
    } else {
        signature =
            "oAcrxO0Xn2/Rub7BsNLgYRE1Km8Hn/+eWeYf2hpFziQ3qRRzwOEdEm+L9UvZK6FDLJf//BNPQrrTAZW0X6doAw"
                .to_string();
        status = Status::Unauthorized;
    }

    let client = Client::tracked(super::rocket().await)
        .await
        .expect("valid rocket instance");
    let response = client
        .post("/hosts/stats")
        .json(&payload)
        .header(ContentType::JSON)
        .header(Header::new("x-hpos-signature", signature))
        .dispatch()
        .await;

    assert_eq!(response.status(), status);
}
