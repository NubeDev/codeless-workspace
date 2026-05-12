#!/usr/bin/env bash
#
# Smoke-test the end-to-end browser demo path without a browser. Boots
# codeless-server on a loopback port, seeds the demo data via the
# bootstrap CLI verb, polls list_jobs until the mock job reaches a
# terminal state, and exits 0 on success. Catches regressions in the
# bootstrap, server, runner-driver, and fs.* paths that the demo
# instructions in DEMO-UI.md depend on.
#
# Usage:
#   ./scripts/smoke-demo.sh             # build + run
#   CODELESS_BIN=path/to/codeless ./scripts/smoke-demo.sh   # skip build
#
# Cleans up the temp dir and child server on exit, including on Ctrl-C.

set -euo pipefail

PORT="${PORT:-7799}"
HOST="127.0.0.1"
TIMEOUT_SECS="${TIMEOUT_SECS:-15}"

if [[ -n "${CODELESS_BIN:-}" ]]; then
    BIN="$CODELESS_BIN"
else
    echo "building codeless..."
    cargo build -p codeless-cli --bin codeless --manifest-path codeless/Cargo.toml >&2
    BIN="codeless/target/debug/codeless"
fi

TMP="$(mktemp -d)"
DB="$TMP/demo.db"
SECRETS="$TMP/secrets.toml"
SERVE_LOG="$TMP/serve.log"

SERVER_PID=""

cleanup() {
    if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$TMP"
}
trap cleanup EXIT INT TERM

echo "smoke: minting bearer token..."
TOKEN="$("$BIN" --secrets-file "$SECRETS" --db "$DB" serve --init-token)"

echo "smoke: seeding demo bootstrap..."
"$BIN" --db "$DB" demo bootstrap

echo "smoke: starting server on http://$HOST:$PORT (fs-root=$TMP)..."
"$BIN" --secrets-file "$SECRETS" --db "$DB" \
    serve --bind "$HOST:$PORT" --fs-root "$TMP" >"$SERVE_LOG" 2>&1 &
SERVER_PID=$!

# Wait for the listen line so curl never races the bind.
for _ in $(seq 1 50); do
    if grep -q "listening on" "$SERVE_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

post() {
    curl -fsS -X POST \
        -H "Authorization: Bearer $TOKEN" \
        -H "content-type: application/json" \
        -d "$2" \
        "http://$HOST:$PORT$1"
}

echo "smoke: hitting /rpc/list_repos..."
repos="$(post /rpc/list_repos '{}')"
echo "$repos" | grep -q '"name":"demo"' || {
    echo "FAIL: expected demo repo in list_repos: $repos" >&2
    exit 1
}

echo "smoke: hitting /rpc/fs_cwd..."
cwd="$(post /rpc/fs_cwd '{}')"
echo "$cwd" | grep -q "\"path\"" || {
    echo "FAIL: fs_cwd did not return a path: $cwd" >&2
    exit 1
}

echo "smoke: polling /rpc/list_jobs for terminal state..."
DEADLINE=$(( $(date +%s) + TIMEOUT_SECS ))
while true; do
    jobs="$(post /rpc/list_jobs '{}')"
    if echo "$jobs" | grep -q '"status":"completed"'; then
        echo "smoke: mock job completed."
        break
    fi
    if echo "$jobs" | grep -qE '"status":"(failed|stopped)"'; then
        echo "FAIL: job reached a non-completed terminal state: $jobs" >&2
        exit 1
    fi
    if [[ "$(date +%s)" -ge "$DEADLINE" ]]; then
        echo "FAIL: timeout (${TIMEOUT_SECS}s) waiting for job completion. last list_jobs:" >&2
        echo "$jobs" >&2
        echo "--- server log ---" >&2
        tail -20 "$SERVE_LOG" >&2
        exit 1
    fi
    sleep 0.3
done

echo "smoke: PASS"
