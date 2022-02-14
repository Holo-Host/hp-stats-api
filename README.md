# match-service-api
Public API for Holo's match service

Endpoints:

#### GET `/`

#### `200 OK`

Text: status of connection to db

GET
`/hosts/list-available?days=7`

days = Cut off time. Records older than this will be ignored.

#### `200 OK`

```json
[
  {
    "_id": "string",
    "IP": "string",
    "timestamp": 0,
    "sshSuccess": true,
    "holoNetwork": "string",
    "channel": "string",
    "holoportModel": "string",
    "hostingInfo": "string",
    "error": "string",
    "alphaProgram": true,
    "assignedTo": "string"
  }
]
```

GET
`/hosts/registered?days=7`

days = Cut off time. Records older than this will be ignored.

#### `200 OK`

```json
[
  "holoport_id_1",
  "holoport_id_2",
  "holoport_id_3",
  "holoport_id_4"
]
```

GET
`/hosts/<name>/uptime`

#### `200 OK`

```json
{
  "uptime": 0.95
}
```

GET
`/network/capacity`

#### `200 OK`

```json
{
  "total_hosts": 2100, // All hosts in database
  "read_only": 1341, // Hosts that have at least 50% uptime in last 7 days
  "source_chain": 300 // Hosts that have at least 90% uptime in last 7 days
}
```

# Prerequisites

For connecting to database binary requires `MONGO_URI` environmental variable which is representing full mongo db uri in a format: `mongodb+srv://<user>:<pass>@cluster0.<cluster>.mongodb.net/`.
