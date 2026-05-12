#!/usr/bin/env bash
#
# Smoke-test the Claude Code runner end-to-end without a browser. Boots
# a fresh git repo under /tmp, registers it through `demo bootstrap`,
# starts `codeless serve --enable-claude`, hits /server/info to confirm
# the new route reports a `claude` runner with a binary path, then
# submits a job and polls for completion. The job's worktree should
# end up with `hello.txt` containing "hi". Catches regressions in the
# server_info plumbing, the implicit worktree-root default, and the
# claude-runner adapter wiring.
#
# Requires the `claude` binary to be installed AND authenticated
# (`claude auth login`). Without auth, the runner adapter surfaces a
# JobFailed event and the script reports the auth gap explicitly
# rather than timing out blindly.
#
# Usage:
#   ./scripts/smoke-claude-demo.sh                          # build + run
#   CODELESS_BIN=path/to/codeless ./scripts/smoke-claude-demo.sh
#   TIMEOUT_SECS=180 ./scripts/smoke-claude-demo.sh         # longer wait
#   KEEP_TMP=1 ./scripts/smoke-claude-demo.sh               # leave /tmp dir
#
# Cleans up the temp dir and child server on exit unless KEEP_TMP=1.

set -euo pipefail

PORT="${PORT:-7798}"
HOST="127.0.0.1"
TIMEOUT_SECS="${TIMEOUT_SECS:-120}"

if [[ -n "${CODELESS_BIN:-}" ]]; then
    BIN="$CODELESS_BIN"
else
    echo "building codeless..."
    cargo build -p codeless-cli --bin codeless --manifest-path codeless/Cargo.toml >&2
    BIN="codeless/target/debug/codeless"
fi

TMP="$(mktemp -d -t codeless-claude-demo.XXXXXX)"
TARGET="$TMP/demo-target"
DB="$TMP/demo.db"
SECRETS="$TMP/secrets.toml"
SERVE_LOG="$TMP/serve.log"

SERVER_PID=""

cleanup() {
    if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    if [[ "${KEEP_TMP:-0}" != "1" ]]; then
        rm -rf "$TMP"
    else
        echo "smoke: tmp kept at $TMP"
    fi
}
trap cleanup EXIT INT TERM

echo "smoke: initialising demo-target repo at $TARGET..."
mkdir -p "$TARGET"
(
    cd "$TARGET"
    git init -q
    git config user.email "smoke@codeless.test"
    git config user.name "Codeless Smoke"
    echo "# demo" > README.md
    git add -A
    git commit -q -m "init"
)

echo "smoke: seeding demo bootstrap (local-path=$TARGET)..."
"$BIN" --db "$DB" demo bootstrap --local-path "$TARGET"

echo "smoke: starting server on http://$HOST:$PORT (fs-root=$TARGET, --enable-claude)..."
"$BIN" --secrets-file "$SECRETS" --db "$DB" serve \
    --bind "$HOST:$PORT" \
    --fs-root "$TARGET" \
    --enable-claude \
    >"$SERVE_LOG" 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 50); do
    if grep -q "listening on" "$SERVE_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done
if ! grep -q "listening on" "$SERVE_LOG" 2>/dev/null; then
    echo "FAIL: server did not bind within 5s. log:" >&2
    cat "$SERVE_LOG" >&2
    exit 1
fi

echo "smoke: GET /server/info..."
info="$(curl -fsS "http://$HOST:$PORT/server/info")"
echo "$info" | grep -q '"id":"claude"' || {
    echo "FAIL: claude runner not present in /server/info: $info" >&2
    exit 1
}
echo "$info" | grep -q '"worktree_root"' || {
    echo "FAIL: worktree_root field missing: $info" >&2
    exit 1
}
# Implicit default kicks in only when --worktree-root is unset and
# --fs-root is set, which is exactly this run.
expected_wt="$TARGET/.codeless/worktrees"
echo "$info" | grep -q "\"worktree_root\":\"$expected_wt\"" || {
    echo "FAIL: worktree_root did not default to $expected_wt: $info" >&2
    exit 1
}

# `claude` block being null means the binary discovery missed the
# install; the rest of the script will fail in a worse place, so flag
# it loudly here.
if echo "$info" | grep -q '"claude":null'; then
    echo "FAIL: claude binary not discovered. /server/info: $info" >&2
    exit 1
fi
if echo "$info" | grep -q '"authenticated":false'; then
    echo "WARN: claude reports not-authenticated. Run \`claude auth login\` first." >&2
    echo "WARN: continuing anyway; the runner may still succeed if the wrapper picks up cached creds." >&2
fi

post() {
    curl -fsS -X POST \
        -H "content-type: application/json" \
        -d "$2" \
        "http://$HOST:$PORT$1"
}

echo "smoke: list_repos to find the demo repo id..."
repos="$(post /rpc/list_repos '{}')"
repo_id="$(echo "$repos" | grep -oE '"id":"[A-Z0-9]{26}"' | head -1 | cut -d'"' -f4)"
if [[ -z "$repo_id" ]]; then
    echo "FAIL: could not extract repo id from list_repos: $repos" >&2
    exit 1
fi
echo "smoke: repo_id=$repo_id"

branch="codeless/job-smoke-$$"
echo "smoke: submitting job (runner=claude, branch=$branch)..."
job="$(post /rpc/submit_job "{
    \"repo_id\": \"$repo_id\",
    \"prompt\": \"create a file named hello.txt at the repository root containing exactly the word: hi. Then run: git add hello.txt && git commit -m 'add hello'.\",
    \"template_yaml\": null,
    \"runner\": \"claude\",
    \"branch\": \"$branch\",
    \"cost_cap_cents\": 100,
    \"wall_clock_cap_ms\": 120000
}")"
job_id="$(echo "$job" | grep -oE '"id":"[A-Z0-9]{26}"' | head -1 | cut -d'"' -f4)"
if [[ -z "$job_id" ]]; then
    echo "FAIL: could not extract job id from submit_job: $job" >&2
    exit 1
fi
echo "smoke: job_id=$job_id"

echo "smoke: polling /rpc/get_job($job_id) for terminal state (timeout=${TIMEOUT_SECS}s)..."
DEADLINE=$(( $(date +%s) + TIMEOUT_SECS ))
last_status=""
while true; do
    job="$(post /rpc/get_job "{\"job_id\":\"$job_id\"}")"
    status="$(echo "$job" | grep -oE '"status":"[a-z]+"' | head -1 | cut -d'"' -f4)"
    if [[ "$status" != "$last_status" ]]; then
        echo "smoke: status=$status"
        last_status="$status"
    fi
    jobs="$job"
    case "$status" in
        completed) break ;;
        failed|stopped)
            echo "FAIL: job terminal=$status. jobs=$jobs" >&2
            echo "--- server log tail ---" >&2
            tail -40 "$SERVE_LOG" >&2
            exit 1
            ;;
    esac
    if [[ "$(date +%s)" -ge "$DEADLINE" ]]; then
        echo "FAIL: timeout (${TIMEOUT_SECS}s). last status=$status" >&2
        echo "--- server log tail ---" >&2
        tail -40 "$SERVE_LOG" >&2
        exit 1
    fi
    sleep 1
done

echo "smoke: worktree is reaped after completion; verifying via the branch in the source repo..."
# After the driver releases the worktree, the only durable record is
# the commit on `$branch` in the source repo at $TARGET. If Claude
# created hello.txt but did not commit, the work is lost — that is a
# real failure mode worth surfacing.
if ! ( cd "$TARGET" && git rev-parse --verify "$branch" >/dev/null 2>&1 ); then
    echo "FAIL: branch $branch missing in $TARGET (worktree never created or pruned without commits)" >&2
    exit 1
fi
file_in_branch="$(cd "$TARGET" && git show "$branch:hello.txt" 2>/dev/null || true)"
if [[ -z "$file_in_branch" ]]; then
    echo "FAIL: hello.txt not present in branch $branch. branch tip:" >&2
    ( cd "$TARGET" && git log "$branch" --oneline | head -5 ) >&2
    exit 1
fi
if ! grep -qi "hi" <<<"$file_in_branch"; then
    echo "FAIL: hello.txt did not contain 'hi'. content=$file_in_branch" >&2
    exit 1
fi

echo "smoke: PASS — claude committed hello.txt ('$file_in_branch') on $branch"
