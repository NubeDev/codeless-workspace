/// OpenAI cloud REST runner — backed by the `async-openai` crate.
///
/// Auth: `OPENAI_API_KEY` env var, or supply via [`RestCfg::api_key`].
/// Also works with any OpenAI-compatible provider via [`RestCfg::base_url`].
use std::time::Instant;

use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::runner::{OnEvent, Runner};
use crate::types::{
    Event, EventKind, Provider, RestCfg, RunResult, RunnerError, RunnerInput, SessionId,
};

const DEFAULT_MODEL: &str = "gpt-4o";
const DEFAULT_MAX_TOKENS: u32 = 4096;

pub struct OpenAiRunner;

#[async_trait]
impl Runner for OpenAiRunner {
    fn provider(&self) -> &Provider {
        &Provider::OpenAi
    }

    async fn ready(&self) -> bool {
        // Key presence only — no network probe. Falls back to
        // `OPENAI_API_KEY` when `RestCfg::api_key` is empty.
        std::env::var("OPENAI_API_KEY")
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
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        {
            Some(k) => k,
            None => {
                let msg = "no API key: set OPENAI_API_KEY or RestCfg::api_key".to_string();
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

        let mut config = OpenAIConfig::new().with_api_key(&api_key);
        if let Some(base) = &cfg.base_url {
            config = config.with_api_base(base);
        }
        let client = Client::with_config(config);

        let model = cfg.model.as_deref().unwrap_or(DEFAULT_MODEL).to_string();
        let max_tokens = cfg.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);

        // Build the messages list.
        let mut messages: Vec<async_openai::types::chat::ChatCompletionRequestMessage> = Vec::new();

        if let Some(sys) = &cfg.system_prompt {
            messages.push(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(sys.as_str())
                    .build()
                    .expect("system message build")
                    .into(),
            );
        }
        for m in &cfg.history {
            let msg: async_openai::types::chat::ChatCompletionRequestMessage = match m.role.as_str()
            {
                "assistant" => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(m.content.as_str())
                    .build()
                    .expect("assistant message build")
                    .into(),
                _ => ChatCompletionRequestUserMessageArgs::default()
                    .content(m.content.as_str())
                    .build()
                    .expect("user message build")
                    .into(),
            };
            messages.push(msg);
        }
        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(cfg.prompt.as_str())
                .build()
                .expect("user message build")
                .into(),
        );

        let request = match CreateChatCompletionRequestArgs::default()
            .model(&model)
            .max_tokens(max_tokens as u16)
            .messages(messages)
            .stream(true)
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("request build: {e}");
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

        let mut stream = match client.chat().create_stream(request).await {
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
        let mut text_buf = String::new();
        let mut error: Option<String> = None;
        let mut connected = false;

        let mut cancelled = false;
        loop {
            let chunk_result = tokio::select! {
                next = stream.next() => match next {
                    Some(c) => c,
                    None => break,
                },
                _ = cancel.cancelled() => {
                    cancelled = true;
                    break;
                }
            };
            match chunk_result {
                Ok(response) => {
                    if !connected {
                        connected = true;
                        let m = Some(response.model.clone())
                            .filter(|s| !s.is_empty())
                            .or(Some(model.clone()));
                        let _ = on_event
                            .send(Event {
                                session_id: session_id.clone(),
                                provider: provider_str.clone(),
                                kind: EventKind::Connected { model: m },
                            })
                            .await;
                    }

                    for choice in response.choices {
                        if let Some(content) = choice.delta.content {
                            if !content.is_empty() {
                                text_buf.push_str(&content);
                                let _ = on_event
                                    .send(Event {
                                        session_id: session_id.clone(),
                                        provider: provider_str.clone(),
                                        kind: EventKind::Text { content },
                                    })
                                    .await;
                            }
                        }
                        // finish_reason signals stream end.
                        if choice.finish_reason.is_some() {
                            let _ = on_event
                                .send(Event {
                                    session_id: session_id.clone(),
                                    provider: provider_str.clone(),
                                    kind: EventKind::Done {
                                        duration_ms: start.elapsed().as_millis() as u64,
                                        cost_usd: 0.0,
                                        input_tokens: 0,
                                        output_tokens: 0,
                                    },
                                })
                                .await;
                        }
                    }
                }
                Err(e) => {
                    let msg = format!("stream recv: {e}");
                    warn!(provider = "openai", "{msg}");
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
        result.error = error;
        Ok(result)
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
