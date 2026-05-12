/// Anthropic cloud REST runner — backed by the `anthropic-ai-sdk` crate.
///
/// Auth: `ANTHROPIC_API_KEY` env var, or supply via [`RestCfg::api_key`].
use std::collections::HashMap;
use std::time::Instant;

use anthropic_ai_sdk::client::AnthropicClient;
use anthropic_ai_sdk::types::message::{
    ContentBlock, ContentBlockDelta, CreateMessageParams, Message, MessageClient, MessageError,
    RequiredMessageParams, Role, StreamEvent, Thinking, ThinkingType, Tool as SdkTool,
    ToolChoice as SdkToolChoice,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::runner::{OnEvent, Runner};
use crate::types::{
    Event, EventKind, Provider, RestCfg, RunResult, RunnerError, RunnerInput, SessionId,
    ToolChoice, ToolDef, ToolUse,
};

const DEFAULT_MODEL: &str = "claude-opus-4-5";
const DEFAULT_MAX_TOKENS: u32 = 8096;
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicRunner;

#[async_trait]
impl Runner for AnthropicRunner {
    fn provider(&self) -> &Provider {
        &Provider::Anthropic
    }

    async fn ready(&self) -> bool {
        // Key presence only — no network probe. The runner falls back to
        // `ANTHROPIC_API_KEY` when `RestCfg::api_key` is empty, so a key
        // visible to the process is what we check here.
        std::env::var("ANTHROPIC_API_KEY")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }

    async fn run(
        &self,
        input: RunnerInput,
        session_id: SessionId,
        on_event: OnEvent,
        cancel: CancellationToken,
    ) -> Result<RunResult, RunnerError> {
        let cfg: RestCfg = match input {
            RunnerInput::Rest(c) => c,
            other => {
                return Err(RunnerError::WrongInputKind {
                    provider: self.provider().to_string(),
                    expected: "rest",
                    got: other.kind_tag(),
                });
            }
        };
        let mut result = RunResult {
            provider: self.provider().to_string(),
            ..Default::default()
        };

        let api_key = match cfg
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        {
            Some(k) => k,
            None => {
                let msg = "no API key: set ANTHROPIC_API_KEY or RestCfg::api_key".to_string();
                emit_error(
                    &on_event,
                    &session_id,
                    &self.provider().to_string(),
                    msg.clone(),
                )
                .await;
                result.error = Some(msg);
                return Ok(result);
            }
        };

        let mut builder = AnthropicClient::builder(api_key, ANTHROPIC_VERSION);
        if let Some(url) = cfg.base_url.as_deref() {
            builder = builder.with_api_base_url(url);
        }
        let client = match builder.build::<MessageError>() {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("client init: {e}");
                emit_error(
                    &on_event,
                    &session_id,
                    &self.provider().to_string(),
                    msg.clone(),
                )
                .await;
                result.error = Some(msg);
                return Ok(result);
            }
        };

        let model = cfg.model.as_deref().unwrap_or(DEFAULT_MODEL).to_string();
        let max_tokens = cfg.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);

        let mut messages: Vec<Message> = cfg
            .history
            .iter()
            .map(|m| {
                let role = if m.role == "assistant" {
                    Role::Assistant
                } else {
                    Role::User
                };
                Message::new_text(role, &m.content)
            })
            .collect();
        messages.push(Message::new_text(Role::User, &cfg.prompt));

        let mut params = CreateMessageParams::new(RequiredMessageParams {
            model: model.clone(),
            messages,
            max_tokens,
        })
        .with_stream(true);

        if let Some(sys) = &cfg.system_prompt {
            params = params.with_system(sys);
        }
        if !cfg.tools.is_empty() {
            params = params.with_tools(cfg.tools.iter().map(to_sdk_tool).collect());
        }
        if let Some(choice) = &cfg.tool_choice {
            params = params.with_tool_choice(to_sdk_choice(choice));
        }
        if let Some(budget) = parse_thinking_budget(cfg.thinking_budget.as_deref()) {
            params = params.with_thinking(Thinking {
                budget_tokens: budget,
                type_: ThinkingType::Enabled,
            });
        }

        let mut stream = match client.create_message_streaming(&params).await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("request: {e}");
                emit_error(
                    &on_event,
                    &session_id,
                    &self.provider().to_string(),
                    msg.clone(),
                )
                .await;
                result.error = Some(msg);
                return Ok(result);
            }
        };

        let provider_str = self.provider().to_string();
        let start = Instant::now();
        let mut input_tokens: u32 = 0;
        let mut output_tokens: u32 = 0;
        let mut text_buf = String::new();
        let mut error: Option<String> = None;

        // Per-content-block-index state for in-flight tool_use assembly.
        struct PendingTool {
            id: String,
            name: String,
            input_json: String,
        }
        let mut pending: HashMap<usize, PendingTool> = HashMap::new();
        let mut tool_uses: Vec<ToolUse> = Vec::new();

        let mut cancelled = false;
        loop {
            let ev_result = tokio::select! {
                next = stream.next() => match next {
                    Some(r) => r,
                    None => break,
                },
                _ = cancel.cancelled() => {
                    cancelled = true;
                    break;
                }
            };
            match ev_result {
                Ok(ev) => match ev {
                    StreamEvent::MessageStart { message } => {
                        let _ = on_event
                            .send(Event {
                                session_id: session_id.clone(),
                                provider: provider_str.clone(),
                                kind: EventKind::Connected {
                                    model: Some(model.clone()),
                                },
                            })
                            .await;
                        input_tokens = message.usage.input_tokens;
                    }
                    StreamEvent::ContentBlockStart {
                        index,
                        content_block: ContentBlock::ToolUse { id, name, input },
                    } => {
                        // `input` here is usually `{}` — the real payload streams as
                        // input_json_delta. We seed with the empty-object case so a tool
                        // that takes no input still produces a valid entry on stop.
                        let seed = if input.is_null() || input == serde_json::json!({}) {
                            String::new()
                        } else {
                            input.to_string()
                        };
                        pending.insert(
                            index,
                            PendingTool {
                                id,
                                name,
                                input_json: seed,
                            },
                        );
                    }
                    StreamEvent::ContentBlockStart { .. } => {}
                    StreamEvent::ContentBlockDelta { index, delta } => match delta {
                        ContentBlockDelta::TextDelta { text } => {
                            text_buf.push_str(&text);
                            let _ = on_event
                                .send(Event {
                                    session_id: session_id.clone(),
                                    provider: provider_str.clone(),
                                    kind: EventKind::Text { content: text },
                                })
                                .await;
                        }
                        ContentBlockDelta::InputJsonDelta { partial_json } => {
                            if let Some(p) = pending.get_mut(&index) {
                                p.input_json.push_str(&partial_json);
                            }
                        }
                        _ => {}
                    },
                    StreamEvent::ContentBlockStop { index } => {
                        if let Some(p) = pending.remove(&index) {
                            let input = if p.input_json.trim().is_empty() {
                                serde_json::json!({})
                            } else {
                                match serde_json::from_str::<serde_json::Value>(&p.input_json) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        warn!(provider = "anthropic", tool = %p.name, "tool input parse: {e}");
                                        serde_json::json!({ "__parse_error": e.to_string(), "__raw": p.input_json })
                                    }
                                }
                            };
                            let _ = on_event
                                .send(Event {
                                    session_id: session_id.clone(),
                                    provider: provider_str.clone(),
                                    kind: EventKind::ToolUse {
                                        id: Some(p.id.clone()),
                                        name: p.name.clone(),
                                        input: Some(input.clone()),
                                    },
                                })
                                .await;
                            tool_uses.push(ToolUse {
                                id: p.id,
                                name: p.name,
                                input,
                            });
                        }
                    }
                    StreamEvent::MessageDelta { usage: Some(u), .. } => {
                        output_tokens = u.output_tokens;
                    }
                    StreamEvent::MessageDelta { .. } => {}
                    StreamEvent::MessageStop => {
                        let _ = on_event
                            .send(Event {
                                session_id: session_id.clone(),
                                provider: provider_str.clone(),
                                kind: EventKind::Done {
                                    duration_ms: start.elapsed().as_millis() as u64,
                                    cost_usd: 0.0,
                                    input_tokens,
                                    output_tokens,
                                },
                            })
                            .await;
                        break;
                    }
                    StreamEvent::Error { error: e } => {
                        let msg = format!("stream error: {:?}", e);
                        warn!(provider = "anthropic", "{msg}");
                        let _ = on_event
                            .send(Event {
                                session_id: session_id.clone(),
                                provider: provider_str.clone(),
                                kind: EventKind::Error {
                                    message: msg.clone(),
                                },
                            })
                            .await;
                        error = Some(msg);
                        break;
                    }
                    _ => {}
                },
                Err(e) => {
                    let msg = format!("stream recv: {e}");
                    warn!(provider = "anthropic", "{msg}");
                    let _ = on_event
                        .send(Event {
                            session_id: session_id.clone(),
                            provider: provider_str.clone(),
                            kind: EventKind::Error {
                                message: msg.clone(),
                            },
                        })
                        .await;
                    error = Some(msg);
                    break;
                }
            }
        }

        if cancelled {
            error = Some("cancelled".into());
        }
        result.text = text_buf;
        result.model = Some(model);
        result.duration_ms = start.elapsed().as_millis() as u64;
        result.input_tokens = input_tokens;
        result.output_tokens = output_tokens;
        result.tool_calls = tool_uses.len() as u32;
        result.tool_uses = tool_uses;
        result.error = error;
        Ok(result)
    }
}

/// Map a provider-agnostic `thinking_budget` string into a usize
/// `budget_tokens` value for the Anthropic SDK. Accepts the CLI-style
/// aliases `low` / `medium` / `high` and raw integer token counts.
/// Returns `None` when the field is absent or unrecognised (runner
/// treats as "no extended thinking").
fn parse_thinking_budget(raw: Option<&str>) -> Option<usize> {
    let raw = raw?.trim();
    match raw.to_ascii_lowercase().as_str() {
        "" | "off" | "none" | "disabled" => None,
        "low" => Some(1024),
        "medium" => Some(4096),
        "high" => Some(16384),
        other => other.parse::<usize>().ok().filter(|n| *n >= 1024),
    }
}

fn to_sdk_tool(t: &ToolDef) -> SdkTool {
    SdkTool {
        name: t.name.clone(),
        description: t.description.clone(),
        input_schema: t.input_schema.clone(),
    }
}

fn to_sdk_choice(c: &ToolChoice) -> SdkToolChoice {
    match c {
        ToolChoice::Auto => SdkToolChoice::Auto,
        ToolChoice::Any => SdkToolChoice::Any,
        ToolChoice::Tool { name } => SdkToolChoice::Tool { name: name.clone() },
        ToolChoice::None => SdkToolChoice::None,
    }
}

async fn emit_error(on_event: &OnEvent, session_id: &SessionId, provider: &str, message: String) {
    let _ = on_event
        .send(Event {
            session_id: session_id.clone(),
            provider: provider.to_string(),
            kind: EventKind::Error { message },
        })
        .await;
}
