# Chat for this job

## user @ 2026-05-14T11:06:43Z

hackline/DOCS
hackline/DOCS/sessions

can you see if you can access these docs, the last stage i did was hackline/DOCS/sessions/2026-05-14-goal5-cmd-api-host-routing.md and now i wanna do stage 6 and 7, so we need to write the scope

## assistant @ 2026-05-14T11:08:20Z

I can see the docs. Let me peek the other recent goal docs to understand the cadence/granularity used.Yes — I can see the docs. Goals 0–5 line up with SCOPE.md Phases 0 through 2 (the "extras" being the sqlite-rest + auth-cli sessions slotted between phase milestones), and goal 5 just landed Phase 2 (cmd + api + HTTP host-routing).

Per SCOPE.md §13 the next two phases on the roadmap are:

- **Phase 3 — Audit completeness + admin UI**: tunnel.session rows with byte counts, cmd/api audit entries, static React admin bundle served from gateway talking to REST + SSE, `/metrics` exposure.
- **Phase 4 — Multi-tenant orgs**: `orgs` + `org_id` FKs on `users` and `devices`, cross-org isolation at REST + Zenoh ACL (per-org keyexpr prefix).

Before I write a strawman SCOPE.md / template.yaml, one quick check — do you want stage-6-7 to be **one job covering both Phase 3 and Phase 4 as two stages** (stage 6 = audit+admin UI, stage 7 = multi-tenant orgs), or did you mean **stages 6 and 7 inside Phase 3** (e.g. split audit-completeness from admin-UI because the UI alone is large)?

Phase 4 piggybacking on Phase 3 is risky — multi-tenancy schema changes touch every table that Phase 3 audit work also touches, so if 6+7 are independent phases you'll want the audit work merged before starting org isolation. If you want them as one job I'd recommend running stage 6 to completion (and probably merging) before unblocking stage 7.

Tell me which framing you want and I'll fill in `template.yaml`, `SCOPE.md`, `WORKFLOW.md`, and the two per-stage docs accordingly.
