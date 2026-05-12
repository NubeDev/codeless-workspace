use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::types::{Event, Provider, RunResult, RunnerError, RunnerInput, SessionId};

/// Channel sender used to stream [`Event`]s out of a [`Runner::run`] call.
///
/// **Backpressure semantics.** The channel is bounded: capacity is the
/// caller's choice when constructing the underlying `mpsc::channel`.
///
/// - REST runners drive the run from `async` context and `await` each
///   `send`. A slow consumer naturally slows the producer (HTTP frames
///   stop being pulled, the producer parks until capacity frees up).
/// - CLI runners emit events from the sync callback that
///   `claude-wrapper::stream_query` (and the codex stdout reader) hand
///   us. Those callbacks cannot `.await`, so CLI runners use
///   [`try_send`](mpsc::Sender::try_send) and drop events when the
///   channel is full. Drops are best-effort: a `tracing::warn!` records
///   the overflow so an under-sized channel is visible in logs.
///
/// Receivers that no longer care should drop their `mpsc::Receiver`;
/// the runner's `send` calls will return `SendError` and are ignored.
pub type OnEvent = mpsc::Sender<Event>;

/// Every AI backend implements this trait.
///
/// `run` takes a typed [`RunnerInput`] (CLI or REST). A runner that is
/// handed the wrong variant returns [`RunnerError::WrongInputKind`]
/// instead of silently dropping the fields it cannot use; the typed
/// error makes integration mistakes loud and unmistakable. Upstream
/// failures (network, parse, model-side errors) continue to flow
/// through [`RunResult::error`].
///
/// Streaming events go out through [`OnEvent`] — see its docs for the
/// per-transport backpressure contract.
#[async_trait]
pub trait Runner: Send + Sync {
    /// Returns the [`Provider`] this runner serves.
    ///
    /// Borrowed because every impl returns a `'static` enum constant —
    /// callers that needed an owned value cloned anyway, but the hot
    /// callsites (registry lookup, error formatting, tracing fields)
    /// only need to read. Cheaper, no information lost.
    fn provider(&self) -> &Provider;

    /// `true` if the backend is installed / configured well enough to be
    /// worth dispatching to. Async because some implementations may want
    /// to perform an inexpensive I/O probe (binary discovery touches the
    /// filesystem).
    ///
    /// - CLI runners locate their binary using the same discovery the
    ///   real run uses; a `false` here means the binary genuinely isn't
    ///   anywhere we know to look.
    /// - REST runners check that an API key is reachable (env var or
    ///   process-level configuration). They do **not** make network
    ///   calls — a healthy `ready()` does not prove the upstream is up,
    ///   only that the agent can attempt the call.
    async fn ready(&self) -> bool;

    /// Run a prompt, streaming events to `on_event`.
    ///
    /// Returns `Err(RunnerError::WrongInputKind)` when the input variant
    /// does not match the runner's transport. On any other path the
    /// run produces a [`RunResult`]; whether that result represents a
    /// success or an upstream failure is communicated via
    /// [`RunResult::error`].
    ///
    /// **Cancellation / kill semantics.** When `cancel` fires, the
    /// runner stops promptly and returns `Ok(RunResult)` with
    /// `error = Some("cancelled")`.
    ///
    /// - CLI runners spawn their child with
    ///   [`tokio::process::Command::kill_on_drop(true)`] and drop the
    ///   process handle on cancel; the OS-level child receives `SIGKILL`
    ///   on Unix when its `Child` is dropped, so the subprocess does not
    ///   leak after the run returns.
    /// - REST runners select against `cancel` inside their streaming
    ///   loop. Cancelling closes the HTTP body (the SDK's Drop tears
    ///   down the in-flight request) and the in-progress chunk is
    ///   discarded.
    ///
    /// Cancellation is best-effort prompt: callers typically observe
    /// the `RunResult` within a few hundred milliseconds. Pass
    /// `CancellationToken::new()` if you do not need cancellation.
    async fn run(
        &self,
        input: RunnerInput,
        session_id: SessionId,
        on_event: OnEvent,
        cancel: CancellationToken,
    ) -> Result<RunResult, RunnerError>;
}
