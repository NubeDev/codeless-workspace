//! Specta TypeScript codegen snapshot. The generated `.ts` is the
//! wire contract every TypeScript hackline client compiles against
//! (SCOPE.md Phase 5 — `@hackline/client` npm package). A diff
//! against the checked-in `wire.ts.snap` is a wire-format change; if
//! intentional, regenerate with
//! `SPECTA_UPDATE=1 cargo test -p hackline-proto --features specta \
//!   --test specta_snapshot`.
//!
//! Scope: every wire type whose fields are statically typed. Envelope
//! types that carry a `serde_json::Value` payload (`MsgEnvelope`,
//! `CmdEnvelope`, `ApiRequest`, `ApiReply`) are intentionally out of
//! scope here — specta's recursive `Value` definition stack-overflows
//! the TS exporter, and a clean fix needs a `Value -> unknown` shim
//! that's tracked as its own goal alongside the npm client wiring.
//! The snapshot still covers the full connection-lifecycle and SSE
//! event surfaces, which is what the npm client's first slice needs.

use std::path::PathBuf;

use hackline_proto::{
    agent_info::AgentInfo,
    connect::{ConnectAck, ConnectRequest},
    event::Event,
    msg::{CmdAck, CmdResult, LogLevel},
    zid::Zid,
};
use specta::TypeCollection;
use specta_typescript::{BigIntExportBehavior, Typescript};

fn collect() -> TypeCollection {
    let mut types = TypeCollection::default();
    types
        .register_mut::<Zid>()
        .register_mut::<AgentInfo>()
        .register_mut::<ConnectRequest>()
        .register_mut::<ConnectAck>()
        .register_mut::<Event>()
        .register_mut::<LogLevel>()
        .register_mut::<CmdAck>()
        .register_mut::<CmdResult>();
    types
}

fn snapshot_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/wire.ts.snap")
}

#[test]
fn wire_types_match_snapshot() {
    let types = collect();
    // `Number` because every i64 we put on the wire (envelope `ts`,
    // `enqueued_at`, `expires_at`) is a unix-millis value well inside
    // JS's `Number.MAX_SAFE_INTEGER` window. If a future column ever
    // crosses 2^53-1 we revisit; until then the JSON wire form already
    // ships these as numbers, so the TS contract should match.
    let ts = Typescript::default().bigint(BigIntExportBehavior::Number);
    let rendered = ts.export(&types).expect("export typescript");

    let path = snapshot_path();
    if std::env::var("SPECTA_UPDATE").is_ok() {
        std::fs::write(&path, &rendered).expect("write snapshot");
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "snapshot {} missing ({e}). Run with SPECTA_UPDATE=1 to create it.",
            path.display()
        )
    });

    if expected != rendered {
        let diff_path = path.with_extension("snap.actual");
        std::fs::write(&diff_path, &rendered).expect("write actual");
        panic!(
            "specta TS snapshot drift. Compare {} vs {}; rerun with \
             SPECTA_UPDATE=1 if the change is intended.",
            path.display(),
            diff_path.display(),
        );
    }
}
