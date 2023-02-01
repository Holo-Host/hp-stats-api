use crate::types::{ApiError, HostInfo, HostStats, Result, ZerotierMember};

/// Returns all available hosts listed in `host_statistics.holoport_status`
/// and `host_statistics.latest_raw_snap` collections. Function merges two data sets
/// with zerotierIp as a primary key and in case of merge conflicts resolves them and
/// reports to user in `errors` field
/// Hosts losted in `host_statistics.holoport_status` but not listed in `host_statistics.latest_raw_snap`
/// are also marked as flawed
pub async fn list_available_hosts(
    hosts: Vec<HostStats>,
    members: Vec<ZerotierMember>,
) -> Result<Vec<HostInfo>, ApiError> {
    Ok(merge_host_info(hosts, members))
}

/// Takes each `member` and finds corresponding `host` by matching on `zerotier_ip`.
/// Builds HostInfo based on this data and reports errors in case of inconsistency.
/// Remaining hosts are added as HostInfo with error field set
fn merge_host_info(mut hosts: Vec<HostStats>, mut members: Vec<ZerotierMember>) -> Vec<HostInfo> {
    let mut host_info_vec: Vec<HostInfo> = vec![];

    // this loop iretates over all members and tries to find a matching host
    // If it finds a matching host it removes it from hosts
    while let Some(member) = members.pop() {
        let mut errors: Vec<String> = vec![];
        let host = find_in_hosts(&mut hosts, &member.zerotier_ip, &mut errors);
        let holoport_id = resolve_holoport_id(host.holoport_id, member.name, &mut errors);

        host_info_vec.push(HostInfo {
            zerotier_ip: member.zerotier_ip,
            wan_ip: member.physical_address,
            last_zerotier_online: zero_to_none(member.last_online), // 0 means never seen which will print as null
            last_netstatsd_reported: host.timestamp,
            holoport_id,
            registered_email: member.description,
            holo_network: host.holo_network,
            channel: host.channel,
            holoport_model: host.holoport_model,
            ssh_status: host.ssh_status,
            hpos_app_list: host.hpos_app_list,
            channel_version: host.channel_version,
            hpos_version: host.hpos_version,
            errors,
        });
    }

    // If a host remainded in hosts then it was not present in members
    // which means that either this zt_ip was reported also by some other holoport
    // or that this zt_ip was not present in members at all
    // In both cases we need to set an error
    while let Some(host) = hosts.pop() {
        host_info_vec.push(HostInfo {
            zerotier_ip: host.zt_ip.clone(),
            wan_ip: None,
            last_zerotier_online: None,
            last_netstatsd_reported: host.timestamp,
            holoport_id: Some(host.holoport_id),
            registered_email: None,
            holo_network: host.holo_network,
            channel: host.channel,
            holoport_model: host.holoport_model,
            ssh_status: host.ssh_status,
            hpos_app_list: host.hpos_app_list,
            channel_version: host.channel_version,
            hpos_version: host.hpos_version,
            errors: vec![format!(
                "Netstatsd reported zerotier IP as {} but Zerotier Central has no knowledge of it",
                host.zt_ip.unwrap_or("None".into())
            )],
        });
    }

    host_info_vec
}

fn zero_to_none(num: i64) -> Option<i64> {
    if num == 0 {
        return None;
    }
    Some(num)
}

/// Finds in `hosts` a host with `zerotier_ip` and once found removes it from `hosts`. If none found returns empty `HostStats`.
/// Sets an `errors` if `zerotier_ip` is `None`
fn find_in_hosts(
    hosts: &mut Vec<HostStats>,
    zerotier_ip: &Option<String>,
    errors: &mut Vec<String>,
) -> HostStats {
    if zerotier_ip.is_none() {
        errors.push("This holoport does not have IP assigned in ZT network".to_string());
        // Return empty HostStats
        return HostStats::default();
    }
    if let Some(index) = hosts.iter().position(|host| host.zt_ip == *zerotier_ip) {
        hosts.swap_remove(index)
    } else {
        errors.push(format!(
        "IP {} is listed in Zerotier Central as active, but no holoport reported this IP via netstatsd within queried timeframe",
        zerotier_ip.as_ref().unwrap_or(&"???".to_string())
    ));
        HostStats::default()
    }
}

/// Compare holoport ids reported by zerotier and netstatsd,
/// report an error in case of mismatch and return the one reported by netstatsd
fn resolve_holoport_id(
    holoport_id: String,
    zerotier_name: Option<String>,
    errors: &mut Vec<String>,
) -> Option<String> {
    if holoport_id.is_empty() {
        return None;
    }
    if zerotier_name.is_none() || Some(holoport_id.clone()) != zerotier_name {
        errors.push(format!(
            "Mismatched holoport ID between data from zerotier ({}) and netstatsd ({})",
            zerotier_name.unwrap_or("???".into()),
            &holoport_id
        ));
    }

    Some(holoport_id)
}
