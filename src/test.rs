use std::collections::HashMap;

/// Unit tests of handler functions
/// All those tests that require interaction with db are included
/// in integration test suite
use super::handlers::list_available_hosts;
use super::types::{HostInfo, HostStats, ZerotierMember};

#[rocket::async_test]
async fn host_found_with_no_errors() {
    let mut hosts: Vec<HostStats> = vec![];
    hosts.push(HostStats {
        holo_network: Some("mainNet".into()),
        channel: Some("master".into()),
        holoport_model: Some("holoportPlus".into()),
        ssh_status: Some(true),
        zt_ip: Some("172.26.215.30".into()),
        wan_ip: Some("8.12.11.123".into()),
        holoport_id: "5zvezgwyz5robqc9s20n9655be0ot9vxmgqwm8g4iy5ite9a4".into(),
        timestamp: Some(12345678910),
        hpos_app_list: Some(HashMap::new()),
        channel_version: Some("89ec8aaef697b4741e6f0cefc4a9f8e7cc1e18dd".into()),
        hpos_version: Some("89ec8aaef697b4741e6f0cefc4a9f8e7cc1e18dd".into()),
    });

    let mut members: Vec<ZerotierMember> = vec![];
    members.push(ZerotierMember {
        last_online: 123456678899,
        zerotier_ip: Some("172.26.215.30".into()),
        physical_address: Some("8.12.11.123".into()),
        name: Some("5zvezgwyz5robqc9s20n9655be0ot9vxmgqwm8g4iy5ite9a4".into()),
        description: Some("beno@email.qq".into()),
    });

    let expected_result = vec![
        HostInfo {
            zerotier_ip: Some("172.26.215.30".into()),
            wan_ip: Some("8.12.11.123".into()),
            last_zerotier_online: Some(123456678899),
            last_netstatsd_reported: Some(12345678910),
            holoport_id: Some("5zvezgwyz5robqc9s20n9655be0ot9vxmgqwm8g4iy5ite9a4".into()),
            registered_email: Some("beno@email.qq".into()),
            holo_network: Some("mainNet".into()),
            channel: Some("master".into()),
            holoport_model: Some("holoportPlus".into()),
            ssh_status: Some(true),
            hpos_app_list: Some(HashMap::new()),
            channel_version: Some("89ec8aaef697b4741e6f0cefc4a9f8e7cc1e18dd".into()),
            hpos_version: Some("89ec8aaef697b4741e6f0cefc4a9f8e7cc1e18dd".into()),
            errors: vec![],
        },
    ];

    let result = list_available_hosts(hosts, members).await.unwrap();

    assert_eq!(&expected_result[..], &result[..]);
}

#[rocket::async_test]
async fn host_found_mismatched_names() {
    let mut hosts: Vec<HostStats> = vec![];
    hosts.push(HostStats {
        holo_network: Some("devNet".into()),
        channel: Some("develop".into()),
        holoport_model: Some("holoport".into()),
        ssh_status: Some(false),
        zt_ip: Some("172.26.215.31".into()),
        wan_ip: Some("77.12.0.3".into()),
        holoport_id: "6avezgwyz5robqc9s20n9655be0ot9vxmgqwm8g4iy58g4iy5".into(),
        timestamp: Some(12345678000),
        hpos_app_list: Some(HashMap::new()),
        channel_version: Some("89ec8aaef697b4741e6f0cefc4a9f8e7aaaaaaa".into()),
        hpos_version: Some("89ec8aaef697b4741e6f0cefc4a9f8e7aaaaaab".into()),
    });

    let mut members: Vec<ZerotierMember> = vec![];
    members.push(ZerotierMember {
        last_online: 123456678810,
        zerotier_ip: Some("172.26.215.31".into()),
        physical_address: Some("77.12.0.3".into()),
        name: Some("5zvezgwyz5robqc9s20n9655be0ot9vxmgqwm8g4iy5ite9a4".into()),
        description: Some("alex@email.qq".into()),
    });

    let expected_result = vec![
        HostInfo {
            zerotier_ip: Some("172.26.215.31".into()),
            wan_ip: Some("77.12.0.3".into()),
            last_zerotier_online: Some(123456678810),
            last_netstatsd_reported: Some(12345678000),
            holoport_id: Some("6avezgwyz5robqc9s20n9655be0ot9vxmgqwm8g4iy58g4iy5".into()),
            registered_email: Some("alex@email.qq".into()),
            holo_network: Some("devNet".into()),
            channel: Some("develop".into()),
            holoport_model: Some("holoport".into()),
            ssh_status: Some(false),
            hpos_app_list: Some(HashMap::new()),
            channel_version: Some("89ec8aaef697b4741e6f0cefc4a9f8e7aaaaaaa".into()),
            hpos_version: Some("89ec8aaef697b4741e6f0cefc4a9f8e7aaaaaab".into()),
            errors: vec!["Mismatched holoport ID between data from zerotier (Some(\"5zvezgwyz5robqc9s20n9655be0ot9vxmgqwm8g4iy5ite9a4\")) and netstatsd (\"6avezgwyz5robqc9s20n9655be0ot9vxmgqwm8g4iy58g4iy5\")".into()],
        },
    ];

    let result = list_available_hosts(hosts, members).await.unwrap();

    assert_eq!(&expected_result[..], &result[..]);
}
