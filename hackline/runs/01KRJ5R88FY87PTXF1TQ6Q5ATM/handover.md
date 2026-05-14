## Done

- Expanded /home/user/code/rust/codeless-workspace/hackline/DOCS/REST-API.md from a thin handler-index into a complete API reference: conventions, error mapping, per-endpoint request/response JSON, SSE framing, audit cursor semantics, and a live-vs-stub status table.
- Committed as 1e1d181 on master in the workspace repo (hackline lives inside the same git repo, not a submodule).

## Next

- (none)

## What you need to know

- Source of truth used: handler files under hackline/crates/hackline-gateway/src/api/**, db repositories under .../src/db/*.rs, GatewayError, and hackline_proto::event/agent_info. Many endpoints listed in the doc are stubs in the router today (claim, devices info/health/patch, users/*, audit, events); the status table at the bottom of REST-API.md flags each one.
- Examples (token strings, scopes, audit row shape) are reasoned from neighbouring docs (AUTH.md, DATABASE.md) — not all are observable in code yet because those handlers are unimplemented. If a future handler diverges, update REST-API.md in the same commit.
- The commit is local only; not pushed. Hackline is inside the workspace repo at master; no ./bin/mani run was used because this was an interactive edit, not a JOB-LOOP tick.

## Open questions

- POST /v1/tunnels validation: the doc states `kind=tcp` ⇒ public_port required, `kind=http` ⇒ public_hostname required. Today the CHECK constraint fires inside SQLite and surfaces as 500. Confirm whether the desired behaviour is a 400 mapping in the handler before this contract is depended on.
- Claim 409 status for already-claimed gateway is documented but not implemented; verify against AUTH.md before the handler lands.
