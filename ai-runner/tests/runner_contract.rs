//! Contract tests for the `Runner` trait surface introduced in the
//! v1 foundation work (typed inputs, async streaming, cancellation).
//!
//! These run hermetically — no network, no upstream binary required —
//! so they belong in the default test suite rather than under `#[ignore]`.

use std::time::{Duration, Instant};

use ai_runner::runners::anthropic::AnthropicRunner;
use ai_runner::{CliCfg, Runner, RunnerError, RunnerInput};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn rest_runner_rejects_cli_input_with_typed_error() {
    // Hand a REST-shaped runner a CLI-shaped input. The contract is:
    // typed `WrongInputKind`, not silent field-dropping. The wrong
    // variant must round-trip back to the caller verbatim so a misuse
    // surfaces during integration rather than producing empty runs in
    // production.
    let runner = AnthropicRunner;
    let (tx, _rx) = mpsc::channel(1);
    let cancel = CancellationToken::new();

    let err = runner
        .run(
            RunnerInput::Cli(CliCfg {
                prompt: "anything".into(),
                ..Default::default()
            }),
            "wrong-input-kind".into(),
            tx,
            cancel,
        )
        .await
        .expect_err("REST runner must reject Cli input");

    match err {
        RunnerError::WrongInputKind {
            provider,
            expected,
            got,
        } => {
            assert_eq!(provider, "anthropic");
            assert_eq!(expected, "rest");
            assert_eq!(got, "cli");
        }
    }
}

#[tokio::test]
async fn cancellation_token_kills_a_long_running_cli_run_within_a_second() {
    // Validates the CLI-side cancellation pattern that `CodexRunner`
    // uses: spawn the child with `kill_on_drop(true)`, race the read
    // loop against `cancel.cancelled()`, then `start_kill` + `wait` to
    // reap. Tested here against `/bin/sleep` so the test is hermetic
    // and does not require the real `codex` binary.
    if !std::path::Path::new("/bin/sleep").exists()
        && !std::path::Path::new("/usr/bin/sleep").exists()
    {
        eprintln!("no sleep binary found; skipping"); // NO_PRINTLN_LINT:allow
        return;
    }

    let cancel = CancellationToken::new();
    let cancel_for_task = cancel.clone();
    let task = tokio::spawn(async move {
        let mut child = tokio::process::Command::new("sleep")
            .arg("60")
            .kill_on_drop(true)
            .spawn()
            .expect("spawn sleep");
        let pid = child.id();
        tokio::select! {
            _ = child.wait() => {}
            _ = cancel_for_task.cancelled() => {
                let _ = child.start_kill();
                let _ = child.wait().await;
            }
        }
        pid
    });

    // Let the child start, then cancel.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let started = Instant::now();
    cancel.cancel();
    let pid = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .expect("task should complete within 2s of cancel")
        .expect("task did not panic");
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(1),
        "cancel-to-reap took too long: {elapsed:?}"
    );
    // The child PID must be gone after `wait`. We probed via /proc on
    // Linux; on macOS / other targets, the assertion above (the task
    // returned) is sufficient evidence.
    if let Some(pid) = pid {
        if cfg!(target_os = "linux") {
            let proc_path = format!("/proc/{pid}");
            assert!(
                !std::path::Path::new(&proc_path).exists(),
                "child pid {pid} still present in /proc after reap"
            );
        }
    }
}

#[tokio::test]
async fn mpsc_backpressure_does_not_block_the_runtime() {
    // The `OnEvent` channel is bounded: when full, an awaiting producer
    // parks until the consumer drains. The runtime must keep scheduling
    // other tasks while the producer is parked — otherwise a slow event
    // consumer would freeze the entire process.
    let (tx, mut rx) = mpsc::channel::<u32>(1);

    // Fill the single-slot buffer so the next send must park.
    tx.send(0).await.expect("first send fits the buffer");

    // The producer parks on `send` until we drain.
    let producer = tokio::spawn(async move {
        for i in 1..=3 {
            tx.send(i).await.expect("send while consumer is alive");
        }
    });

    // While the producer is parked, an unrelated task must still run
    // and complete. If the runtime were blocked, this would never tick.
    let unrelated = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(20)).await;
        42u32
    });

    let unrelated_result = tokio::time::timeout(Duration::from_millis(500), unrelated)
        .await
        .expect("runtime is not blocked")
        .expect("unrelated task did not panic");
    assert_eq!(unrelated_result, 42);

    // Drain the channel; the producer task will then complete.
    let mut drained = Vec::new();
    while drained.len() < 4 {
        match tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Some(v)) => drained.push(v),
            Ok(None) => break,
            Err(_) => panic!("recv timed out — backpressure did not release"),
        }
    }
    assert_eq!(drained, vec![0, 1, 2, 3]);
    producer.await.expect("producer joins cleanly");
}
