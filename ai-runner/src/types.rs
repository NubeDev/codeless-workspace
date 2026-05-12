use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Caller-supplied identifier grouping every event and final result for one
/// run. A newtype rather than a bare `String` so the agent-domain id (which
/// the orchestrator chooses and uses to scope locks, traces, and approvals)
/// cannot be silently mixed up with the *upstream-CLI* session id that the
/// `claude` binary returns for resume support — that one stays a `String` on
/// [`RunResult::session_id`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Which AI backend to use.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Claude Code CLI — auth managed by the `claude` binary itself.
    Claude,
    /// OpenAI Codex CLI — reads `OPENAI_API_KEY` from environment.
    Codex,
    /// Anthropic cloud REST API — key via `RestCfg::api_key` or `ANTHROPIC_API_KEY`.
    Anthropic,
    /// OpenAI cloud REST API — key via `RestCfg::api_key` or `OPENAI_API_KEY`.
    OpenAi,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Provider::Claude => "claude",
            Provider::Codex => "codex",
            Provider::Anthropic => "anthropic",
            Provider::OpenAi => "openai",
        };
        f.write_str(s)
    }
}

/// A single message in a multi-turn conversation (REST providers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryMessage {
    /// `"system"`, `"user"`, or `"assistant"`.
    pub role: String,
    pub content: String,
}

/// Configuration for a CLI-transport run (claude-wrapper, codex CLI).
///
/// CLI runners spawn an external binary that owns its own auth and tool
/// surface; we only get to set the few flags the binary exposes. Fields
/// that don't apply to a particular CLI are ignored at the spawn site.
#[derive(Debug, Clone, Default)]
pub struct CliCfg {
    /// The user prompt.
    pub prompt: String,
    /// Optional system prompt / context.
    pub system_prompt: Option<String>,
    /// Model override, e.g. `"claude-opus-4-5"`.
    pub model: Option<String>,
    /// Resume a previous CLI session by its session ID.
    pub resume_id: Option<String>,
    /// MCP server URL, e.g. `http://localhost:8090/mcp`.
    pub mcp_url: Option<String>,
    /// Bearer token for MCP server auth.
    pub mcp_token: Option<String>,
    /// Tool filter pattern, e.g. `"mcp__acme__*"`.
    pub allowed_tools: Option<String>,
    /// Thinking budget: `"low"`, `"medium"`, `"high"`, or a token count.
    pub thinking_budget: Option<String>,
    /// Working directory for the spawned subprocess.
    pub work_dir: Option<String>,
}

/// Configuration for a REST-transport run (Anthropic, OpenAI cloud APIs).
///
/// REST runners assemble a JSON request body; the field set mirrors what
/// the underlying SDK accepts. Tool definitions are first-class here —
/// the model returns a structured tool_use block we extract.
#[derive(Debug, Clone, Default)]
pub struct RestCfg {
    /// The user prompt.
    pub prompt: String,
    /// Optional system prompt / context.
    pub system_prompt: Option<String>,
    /// Model override, e.g. `"gpt-4o"`.
    pub model: Option<String>,
    /// API key. Falls back to the standard env var when absent.
    pub api_key: Option<String>,
    /// Base URL override (proxies, local servers).
    pub base_url: Option<String>,
    /// Pre-loaded conversation history for stateless REST providers.
    pub history: Vec<HistoryMessage>,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
    /// Extra HTTP headers forwarded verbatim.
    pub extra_headers: HashMap<String, String>,
    /// Tools exposed to the model for structured output / function calling.
    pub tools: Vec<ToolDef>,
    /// How the model is allowed / required to pick a tool.
    pub tool_choice: Option<ToolChoice>,
    /// Thinking budget: `"low"`, `"medium"`, `"high"`, or a token count.
    pub thinking_budget: Option<String>,
}

/// Typed input for a single run. Replaces the old `RunConfig` god-struct.
///
/// CLI and REST transports don't share enough fields for a flat config to
/// be honest about which knobs work where; the previous design hid the
/// mismatch by silently dropping fields. The variant split surfaces the
/// transport choice in the type system: a runner that gets the wrong
/// variant returns [`RunnerError::WrongInputKind`] rather than ignoring
/// fields the caller went to the trouble of populating.
#[derive(Debug, Clone)]
pub enum RunnerInput {
    /// CLI-shaped run (subprocess, binary-managed auth).
    Cli(CliCfg),
    /// REST-shaped run (HTTP API, key-managed auth).
    Rest(RestCfg),
}

impl RunnerInput {
    /// Short tag used in error messages and tracing.
    pub fn kind_tag(&self) -> &'static str {
        match self {
            RunnerInput::Cli(_) => "cli",
            RunnerInput::Rest(_) => "rest",
        }
    }
}

/// Errors that originate from the runner layer itself (not from the
/// upstream model). Upstream / network / parsing errors flow through
/// [`RunResult::error`] as before.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    /// The runner was handed a [`RunnerInput`] variant it does not accept.
    /// Typed (rather than silently ignored) so misuse fails loudly during
    /// integration instead of producing empty runs in production.
    #[error("provider `{provider}` runner expected `{expected}` input, got `{got}`")]
    WrongInputKind {
        provider: String,
        expected: &'static str,
        got: &'static str,
    },
}

/// A tool the model may invoke. Mirrors the Anthropic / OpenAI schema shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema for the tool's input object.
    pub input_schema: JsonValue,
}

/// Constraint on tool selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Model decides whether to call a tool.
    Auto,
    /// Model must call some tool.
    Any,
    /// Model must call the named tool.
    Tool { name: String },
    /// Model must not call any tool.
    None,
}

/// A structured tool invocation captured from the model's output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: JsonValue,
}

/// A normalised streaming event emitted by any provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Caller-supplied identifier grouping all events for one run.
    pub session_id: SessionId,
    /// Provider that produced this event.
    pub provider: String,
    pub kind: EventKind,
}

/// The typed payload of an [`Event`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    /// Backend process / HTTP stream established.
    Connected { model: Option<String> },
    /// A chunk of generated text.
    Text { content: String },
    /// The model invoked a tool. `id` and `input` are present for REST
    /// providers (Anthropic, OpenAI) that supply a structured tool block,
    /// and absent for CLI providers (Claude wrapper, Codex) which only
    /// surface the name. Collapsing the previous `ToolCall` + `ToolUse`
    /// pair into one variant keeps the SSE wire shape stable across
    /// transports — orchestrators no longer need to dedupe two events
    /// for the same invocation.
    ToolUse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input: Option<JsonValue>,
    },
    /// Run finished successfully.
    Done {
        duration_ms: u64,
        cost_usd: f64,
        input_tokens: u32,
        output_tokens: u32,
    },
    /// Something went wrong.
    Error { message: String },
}

/// Records one tool invocation within a run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolCallEntry {
    pub name: String,
    pub duration_ms: u64,
    /// `"ok"` or `"error"`.
    pub status: String,
    pub error: Option<String>,
}

/// Aggregated result returned after a run completes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunResult {
    pub text: String,
    pub provider: String,
    pub model: Option<String>,
    /// Upstream CLI session id for resume support (Claude runner only).
    /// Not the agent-domain [`SessionId`] — this one is opaque, assigned by
    /// the `claude` binary itself, and meaningful only when fed back to a
    /// later run via [`CliCfg::resume_id`].
    pub session_id: Option<String>,
    pub duration_ms: u64,
    pub cost_usd: f64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub tool_calls: u32,
    pub tool_call_log: Vec<ToolCallEntry>,
    /// Structured tool invocations captured during the run (REST providers).
    pub tool_uses: Vec<ToolUse>,
    /// Set when the run ended with a fatal error.
    pub error: Option<String>,
}
