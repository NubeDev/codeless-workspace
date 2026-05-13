/// GitHub Copilot CLI runner.
///
/// Spawns `copilot -p <prompt> --allow-all-tools --no-ask-user [-C <cwd>]
/// [--model <m>]` and streams stdout as plain-text lines.
///
/// Auth is handled by the `copilot` binary itself (GitHub device flow,
/// state in `~/.copilot/`). This crate does not set credentials.
///
/// Non-interactive mode requires `--allow-all-tools` (or `--allow-all` /
/// `--yolo`); without it the CLI refuses to run headless. `--no-ask-user`
/// disables the `ask_user` tool so the agent never stalls waiting for
/// input from a non-existent TTY.
///
/// Install: `curl -fsSL https://gh.io/copilot-install | bash`
use std::time::Instant;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::sync::CancellationToken;

use crate::runner::{OnEvent, Runner};
use crate::types::{
    CliCfg, Event, EventKind, Provider, RunResult, RunnerError, RunnerInput, SessionId,
};

pub struct CopilotRunner;

#[async_trait]
impl Runner for CopilotRunner {
    fn provider(&self) -> &Provider {
        &Provider::Copilot
    }

    async fn ready(&self) -> bool {
        tokio::process::Command::new("copilot")
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

        let model = cfg.model.clone().unwrap_or_else(|| "copilot".to_string());

        // `-p` is the headless prompt flag. `--allow-all-tools` is
        // required for non-interactive mode — without it copilot exits
        // with a permission error before running. `--no-ask-user`
        // disables the ask_user tool so the agent never stalls waiting
        // for a TTY response. `-C` sets the working directory inside
        // the binary instead of relying on the parent process cwd,
        // matching how Codeless scopes each run to a git worktree.
        let mut args: Vec<String> = vec![
            "-p".into(),
            cfg.prompt.clone(),
            "--allow-all-tools".into(),
            "--no-ask-user".into(),
            "--no-auto-update".into(),
        ];
        if let Some(m) = &cfg.model {
            args.extend(["--model".into(), m.clone()]);
        }
        if let Some(dir) = &cfg.work_dir {
            args.extend(["-C".into(), dir.clone()]);
        }

        let mut cmd = tokio::process::Command::new("copilot");
        cmd.args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        if let Some(dir) = &cfg.work_dir {
            cmd.current_dir(dir);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("spawn copilot: {e}");
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
                let msg = format!("copilot exited with code {}", status.code().unwrap_or(-1));
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
                let msg = format!("copilot wait: {e}");
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
