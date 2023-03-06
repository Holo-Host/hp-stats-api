# hp-stats-api
Public API for collecting and reading statistics on holoports and holo network

## Ownership Info
Codeowner: @jettech
Consulted: @peeech
Informed: None


## Endpoints:

### GET `/`

Gets status of connection to db.

#### `200 OK`

### DELETE `/cleanup`

Deletes from host_statistics.holoport_status documents with timestamp field older than 30 days.

#### `200 OK`

### GET `/hosts/list-available?hours=7`

hours = Cut off time. Records older than this will be ignored.

This endpoint returns all the holoports on the holo network as seen by both zerotier network controller and Holoport's netstatsd. Data from both sources is merged and analyzed for possible errors. All the errors are reported in form of an array under field `errors`.

#### `200 OK`

```json
[
  {
    "zerotier_ip": "172.26.215.31",                                     # IP address on Zerotier network
    "wan_ip": "77.12.0.3",                                              # IPv4 address on internet
    "last_zerotier_online": 123456678810,                               # timestamp of the last contact of the host with Zerotier network controller
    "last_netstatsd_reported": 123456678834,                            # timestamp of the last update from netstatsd
    "holoport_id": "5zvezgwyz5robqc9s20n9655be0ot9vxmgqwm8g4iy5ite9a4", # base36 encoded public key of the host
    "registered_email": "alex@email.qq",                                # email address used at registration
    "holo_network": "devNet",                                           # can be one of devNet, alphaNet, flexNet...
    "channel": "develop",                                               # nix-channel that HPOS is following
    "holoport_model": "holoport",                                       # HP or HP+
    "ssh_status": true,                                                 # is SSH enabled?
    "hpos_app_list": [],                                                # list of hosted happs as reported by netstatsd
    "channel_version": "89ec8aaef697b4741e6f0cefc4a9f8e7cc1e18dd",      # the git revision that HPOS is currently running
    "hpos_version": "89ec8aaef697b4741e6f0cefc4a9f8e7cc1e18dd",         # the git revision channel that HPOS has downloaded
    "errors": []
  }
]
```

### GET `/hosts/<name>/uptime`

#### `200 OK`

```json
{
  "uptime": 0.95
}
```

### GET `/network/capacity`

#### `200 OK`

```json
{
  "total_hosts": 2100, // All hosts in database
  "read_only": 1341, // Hosts that have at least 50% uptime in last 7 days
  "source_chain": 300 // Hosts that have at least 90% uptime in last 7 days
}
```

### POST `/hosts/stats`

payload:
```json
{
  "holoNetwork":    <string>  # can be one of devNet, alphaNet, flexNet...
  "channel" :       <string>  # nix-channel that HPOS is following
  "holoportModel":  <string>  # HP or HP+
  "sshStatus":      <bool>    # is SSH enabled?
  "ztIp":           <string>  # IP address on Zerotier network
  "wanIp":          <string>  # IPv4 address on internet
  "holoportId":     <string>  # base36 encoded public key of the host
  "timestamp":      <string>  # updated on API server
  "hposVersion":    <string>  # the git revision channel that HPOS has downloaded
  "channelVersion": <string>  # the git revision that HPOS is currently running
}
```

#### `200 OK`

# Prerequisites

For connecting to database binary requires `MONGO_URI` environmental variable which is representing full mongo db uri in a format: `mongodb+srv://<user>:<pass>@cluster0.<cluster>.mongodb.net/`.
