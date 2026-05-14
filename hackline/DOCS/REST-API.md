# REST API

Authoritative source for endpoint shapes. The router lives in
[`crates/hackline-gateway/src/api/router.rs`](../crates/hackline-gateway/src/api/router.rs);
each handler file is named `<verb>.rs` under its resource folder so
this doc and the source map 1:1.

`Authorization: Bearer <token>` on everything except `GET /v1/health`
and the claim endpoints. JSON in / JSON out.

## Health

| Method | Path | Handler |
|---|---|---|
| GET | `/v1/health` | `api/health.rs` |

## Claim (first-boot)

| Method | Path | Handler |
|---|---|---|
| GET | `/v1/claim/status` | `api/claim/status.rs` |
| POST | `/v1/claim` | `api/claim/post.rs` |

`POST /v1/claim` is **atomic** — the row delete and the owner-row
insert happen in a single SQL transaction. See [`AUTH.md`](./AUTH.md).

## Devices

| Method | Path | Handler |
|---|---|---|
| GET | `/v1/devices` | `api/devices/list.rs` |
| POST | `/v1/devices` | `api/devices/create.rs` |
| GET | `/v1/devices/:id` | `api/devices/get.rs` |
| PATCH | `/v1/devices/:id` | `api/devices/patch.rs` |
| DELETE | `/v1/devices/:id` | `api/devices/delete.rs` |
| GET | `/v1/devices/:id/info` | `api/devices/info.rs` |
| GET | `/v1/devices/:id/health` | `api/devices/health.rs` |

`PATCH` mutable fields: `label`, `customer_id`. Anything else is
rejected.

## Tunnels

| Method | Path | Handler |
|---|---|---|
| GET | `/v1/tunnels` | `api/tunnels/list.rs` |
| POST | `/v1/tunnels` | `api/tunnels/create.rs` |
| DELETE | `/v1/tunnels/:id` | `api/tunnels/delete.rs` |

## Users

| Method | Path | Handler |
|---|---|---|
| GET | `/v1/users` | `api/users/list.rs` |
| POST | `/v1/users` | `api/users/create.rs` |
| DELETE | `/v1/users/:id` | `api/users/delete.rs` |
| POST | `/v1/users/:id/tokens` | `api/users/mint_token.rs` |

## Audit

| Method | Path | Handler |
|---|---|---|
| GET | `/v1/audit?cursor=…&limit=…` | `api/audit/list.rs` |

Cursor-based pagination. Offset pagination on the audit table is a
footgun and is not supported.

## Events (SSE)

| Method | Path | Handler |
|---|---|---|
| GET | `/v1/events` | `api/events/all.rs` |
| GET | `/v1/devices/:id/events` | `api/events/per_device.rs` |

Event variants live in [`hackline-proto::event`](../crates/hackline-proto/src/event.rs).

When deploying behind Caddy, the Caddyfile must include
`flush_interval -1` for `/v1/events` and `/v1/devices/*/events` —
otherwise the proxy buffers and clients see a broken feed.
