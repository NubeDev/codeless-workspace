/// OpenAI Codex CLI runner.
///
/// Spawns `codex --quiet --full-auto [--model <m>] <prompt>` and streams
/// stdout as plain-text lines. The Codex CLI reads `OPENAI_API_KEY` from
/// the environment — this crate does not set it.
///
/// Install: `npm install -g @openai/codex`
use std::time::Instant;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::sync::CancellationToken;

use crate::runner::{OnEvent, Runner};
use crate::types::{
    CliCfg, Event, EventKind, Provider, RunResult, RunnerError, RunnerInput, SessionId,
};

pub struct CodexRunner;

#[async_trait]
impl Runner for CodexRunner {
    fn provider(&self) -> &Provider {
        &Provider::Codex
    }

    async fn ready(&self) -> bool {
        // Probe the binary on PATH. `--version` is fast and side-effect
        // free; a non-zero exit or missing binary means we cannot run.
        tokio::process::Command::new("codex")
            .arg("--version")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn run(
        &self,
        input: RunnerInput,
        session_id: SessionId,
        on_event: OnEvent,
        cancel: CancellationToken,
    ) -> Result<RunResult, RunnerError> {
        let cfg: CliCfg = match input {
            RunnerInput::Cli(c) => c,
            other => {
                return Err(RunnerError::WrongInputKind {
                    provider: self.provider().to_string(),
                    expected: "cli",
                    got: other.kind_tag(),
                });
            }
        };
        let mut result = RunResult {
            provider: self.provider().to_string(),
            ..Default::default()
        };

        let model = cfg.model.clone().unwrap_or_else(|| "codex".to_string());

        let mut args = vec!["--quiet".to_string(), "--full-auto".to_string()];
        if let Some(m) = &cfg.model {
            args.extend(["--model".to_string(), m.clone()]);
        }
        args.push(cfg.prompt.clone());

        let mut cmd = tokio::process::Command::new("codex");
        cmd.args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            // Drop the `Child` -> SIGKILL on Unix. Belt-and-braces for
            // cancellation: even if the cancel branch below misses the
            // wait window, dropping `child` at function exit reaps the
            // subprocess instead of leaking it.
            .kill_on_drop(true);
        if let Some(dir) = &cfg.work_dir {
            cmd.current_dir(dir);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("spawn codex: {e}");
                let _ = on_event
                    .send(Event {
                        session_id: session_id.clone(),
                        provider: self.provider().to_string(),
                        kind: EventKind::Error {
                            message: msg.clone(),
                        },
                    })
                    .await;
                result.error = Some(msg);
                return Ok(result);
            }
        };

        let stdout = child.stdout.take().expect("stdout was piped");
        let mut lines = BufReader::new(stdout).lines();
        let start = Instant::now();

        // Emit "connected" before the first line.
        let _ = on_event
            .send(Event {
                session_id: session_id.clone(),
                provider: self.provider().to_string(),
                kind: EventKind::Connected {
                    model: Some(model.clone()),
                },
            })
            .await;

        let mut text_buf = String::new();
        let mut cancelled = false;
        loop {
            tokio::select! {
                line_result = lines.next_line() => {
                    match line_result {
                        Ok(Some(line)) => {
                            let content = format!("{line}\n");
                            text_buf.push_str(&content);
                            let _ = on_event
                                .send(Event {
                                    session_id: session_id.clone(),
                                    provider: self.provider().to_string(),
                                    kind: EventKind::Text { content },
                                })
                                .await;
                        }
                        Ok(None) | Err(_) => break,
                    }
                }
                _ = cancel.cancelled() => {
                    cancelled = true;
                    break;
                }
            }
        }

        if cancelled {
            // Drop the child to trigger kill_on_drop; reap so it does
            // not leave a zombie.
            let _ = child.start_kill();
            let _ = child.wait().await;
            let duration_ms = start.elapsed().as_millis() as u64;
            result.text = text_buf;
            result.model = Some(model);
            result.duration_ms = duration_ms;
            result.error = Some("cancelled".into());
            return Ok(result);
        }

        let wait_result = child.wait().await;
        let duration_ms = start.elapsed().as_millis() as u64;

        let error = match wait_result {
            Ok(status) if !status.success() => {
                let msg = format!("codex exited with code {}", status.code().unwrap_or(-1));
                let _ = on_event
                    .send(Event {
                        session_id: session_id.clone(),
                        provider: self.provider().to_string(),
                        kind: EventKind::Error {
                            message: msg.clone(),
                        },
                    })
                    .await;
                Some(msg)
            }
            Err(e) => {
                let msg = format!("codex wait: {e}");
                let _ = on_event
                    .send(Event {
                        session_id: session_id.clone(),
                        provider: self.provider().to_string(),
                        kind: EventKind::Error {
                            message: msg.clone(),
                        },
                    })
                    .await;
                Some(msg)
            }
            _ => {
                let _ = on_event
                    .send(Event {
                        session_id: session_id.clone(),
                        provider: self.provider().to_string(),
                        kind: EventKind::Done {
                            duration_ms,
                            cost_usd: 0.0,
                            input_tokens: 0,
                            output_tokens: 0,
                        },
                    })
                    .await;
                None
            }
        };

        result.text = text_buf;
        result.model = Some(model);
        result.duration_ms = duration_ms;
        result.error = error;
        Ok(result)
    }
}
