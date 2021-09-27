# match-service-api-rust
Public API for Holo's match service

Endpoints:

GET
`/`

Returns status of connection to db

#### `200 OK`

GET
`/hosts/list`

#### `200 OK`

```json
[
  {
    name:"5j60okm4zt9elo8gu5u4qgh2bv3gusdo7uo48nwdb2d18wk59h",
    IP:"172.26.29.50",
    timestamp:1631089852191,
    sshSuccess:true,
    holoNetwork:"flexNet",
    channel:"923",
    holoportModel:"holoport-plus",
    hostingInfo:"{\"totalSourceChains\":0,\"currentTotalStorage\":0,\"usage\":{\"cpu\":0}}",
    error:null,
    alphaTest: true,
    assignedTo: null
  }
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

Binary requires `MONGO_URI` env var representing full mongo db uri in a format: `mongodb+srv://<user>:<pass>@cluster0.<cluster>.mongodb.net/`.
