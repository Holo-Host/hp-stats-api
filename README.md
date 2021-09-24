# match-service-api-rust
Public API for Holo's match service

Endpoints:

GET
`/host/statistics/Hc7179WYizSRLRSb6DWgZf4dhw5b0ACdlvAw3WYH8`

#### `200 OK`

```json
{
  "uptime": 0.95
}
```

GET
`/network/capacity`

#### `200 OK`

`total_hosts`: All hosts in database
`read_only`: Hosts that have at least 50% uptime in last 7 days
`source_chain`: Hosts that have at least 90% uptime in last 7 days

```json
{
  "total_hosts": 2100,
  "read_only": 1341,
  "source_chain": 300
}
```

GET
`/network/hosts`

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