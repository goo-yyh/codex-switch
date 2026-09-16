//! CC Switch modules are kept in their original layout. This file is the local API.
//! See upstream-files.json and LICENSE for provenance.
#![allow(dead_code)]

mod provider;
mod proxy {
    pub mod circuit_breaker;
    pub mod log_codes;
    pub mod types {
        // Minimal shape adapter for the upstream config conversion; the public API uses CircuitBreakerConfig directly.
        pub struct AppProxyConfig {
            pub circuit_failure_threshold: u32,
            pub circuit_success_threshold: u32,
            pub circuit_timeout_seconds: u32,
            pub circuit_error_rate_threshold: f64,
            pub circuit_min_requests: u32,
        }
    }
    pub mod error {
        #[derive(Debug, thiserror::Error)]
        pub enum ProxyError {
            #[error("{0}")]
            TransformError(String),
        }
    }
    pub mod json_canonical;
    pub mod sse;
    pub mod tool_media;
    pub mod providers {
        pub mod codex_chat_common;
        pub mod codex_chat_history;
        pub mod codex_responses_sse;
        pub mod streaming_codex_chat;
        pub mod transform;
        pub mod transform_codex_chat;
        pub mod transform_codex_chat_moonshot_schema;
    }
}

pub use provider::CodexChatReasoningConfig as ReasoningConfig;
pub use proxy::error::ProxyError as BridgeError;
pub use proxy::providers::codex_chat_history::CodexChatHistoryStore as HistoryStore;
use proxy::providers::{streaming_codex_chat, transform_codex_chat as chat};
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct ToolContext(chat::CodexToolContext);

pub fn chat_request(
    body: Value,
    reasoning: Option<&ReasoningConfig>,
    endpoint: &str,
) -> Result<(Value, ToolContext), BridgeError> {
    let context = ToolContext(chat::build_codex_tool_context_from_request(&body));
    let mut request = chat::responses_to_chat_completions_with_reasoning(body, reasoning)?;
    let schema = &proxy::providers::transform_codex_chat_moonshot_schema::upstream_requires_ref_sibling_all_of;
    if schema(endpoint) {
        proxy::providers::transform_codex_chat_moonshot_schema::wrap_ref_siblings_in_chat_tools(
            &mut request,
        );
    }
    Ok((request, context))
}

pub fn chat_response(body: Value, context: &ToolContext) -> Result<Value, BridgeError> {
    chat::chat_completion_to_response_with_context(body, &context.0)
}

pub fn chat_stream<E: std::error::Error + Send + 'static>(
    stream: impl futures::Stream<Item = Result<bytes::Bytes, E>> + Send + 'static,
    context: ToolContext,
) -> impl futures::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send {
    streaming_codex_chat::create_responses_sse_stream_from_chat_with_context(stream, context.0)
}

pub mod catalog;
mod model_capabilities;

mod claude_desktop_config {
    pub const ONE_M_CONTEXT_MARKER: &str = "[1m]";
}

pub use proxy::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};

impl ToolContext {
    pub fn contains_response_tool(&self, item: &Value) -> bool {
        if item["type"] == "tool_search_call" {
            return self
                .0
                .chat_tools()
                .iter()
                .filter_map(|t| t["function"]["name"].as_str())
                .any(|name| {
                    self.0
                        .lookup_chat_name(name)
                        .is_some_and(|spec| matches!(spec.kind, chat::CodexToolKind::ToolSearch))
                });
        }
        let Some(name) = item["name"].as_str() else {
            return false;
        };
        let chat_name = self
            .0
            .chat_name_for_response_function(name, item["namespace"].as_str());
        self.0.lookup_chat_name(&chat_name).is_some()
    }
}
