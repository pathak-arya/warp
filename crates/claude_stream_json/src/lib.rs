//! Parser for the `claude --print --output-format stream-json` event stream.
//!
//! Warp launches the local Claude Code CLI in headless streaming mode and
//! consumes its stdout line-by-line. Each line is one JSON object; this crate
//! turns that raw stream into a small set of semantic [`AgentStreamEvent`]s
//! that the GUI can map onto Warp agent blocks (text turns, tool calls, tool
//! results, and the terminal result marker).
//!
//! The parser is intentionally lenient: unknown event types and unknown
//! content items are ignored rather than erroring, so a newer Claude CLI that
//! adds event kinds does not break rendering. A line that is not valid JSON is
//! surfaced as [`AgentStreamEvent::Unparseable`] so the caller can decide
//! whether to log it.

use serde::Deserialize;

/// A semantic event distilled from one or more raw stream-json lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStreamEvent {
    /// Session initialization (`type: "system", subtype: "init"`), carrying the
    /// Claude session id so callers can offer resume later.
    SessionStarted { session_id: Option<String> },
    /// A chunk of assistant-visible text (`assistant` message, `text` content).
    /// This is the content that may contain `$...$` / `$$...$$` math.
    AssistantText { text: String },
    /// Assistant reasoning/thinking text (`thinking` content). Kept distinct so
    /// the GUI can render it collapsed, matching Warp's reasoning styling.
    AssistantThinking { text: String },
    /// The assistant invoked a tool.
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// A tool returned a result (`user` message, `tool_result` content).
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    /// The turn finished (`type: "result"`). `text` is Claude's final answer
    /// text; `is_error` reflects a non-success subtype.
    Result { text: String, is_error: bool },
    /// A line that could not be parsed as JSON. Carries the raw line.
    Unparseable { line: String },
}

// ---- Raw wire types (a permissive subset of Claude's stream-json schema) ----

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RawEvent {
    System(SystemEvent),
    Assistant(MessageEvent),
    User(MessageEvent),
    Result(ResultEvent),
    // Any other event type (rate_limit_event, etc.) deserializes here and is
    // dropped by the mapper.
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct SystemEvent {
    #[serde(default)]
    subtype: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Deserialize)]
struct MessageEvent {
    message: InnerMessage,
}

#[derive(Deserialize)]
struct InnerMessage {
    #[serde(default)]
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text {
        #[serde(default)]
        text: String,
    },
    Thinking {
        #[serde(default)]
        thinking: String,
    },
    ToolUse {
        #[serde(default)]
        id: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
    ToolResult {
        #[serde(default)]
        tool_use_id: String,
        #[serde(default)]
        content: serde_json::Value,
        #[serde(default)]
        is_error: bool,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct ResultEvent {
    #[serde(default)]
    subtype: Option<String>,
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    is_error: bool,
}

/// Parse a single stream-json line into zero or more semantic events.
///
/// One raw line can yield multiple events (an assistant message may carry both
/// thinking and text content, plus tool uses), which is why this returns a
/// `Vec`. Blank lines yield an empty vec.
pub fn parse_line(line: &str) -> Vec<AgentStreamEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let raw: RawEvent = match serde_json::from_str(trimmed) {
        Ok(raw) => raw,
        Err(_) => {
            return vec![AgentStreamEvent::Unparseable {
                line: trimmed.to_string(),
            }];
        }
    };

    match raw {
        RawEvent::System(system) => {
            if system.subtype.as_deref() == Some("init") {
                vec![AgentStreamEvent::SessionStarted {
                    session_id: system.session_id,
                }]
            } else {
                Vec::new()
            }
        }
        RawEvent::Assistant(event) | RawEvent::User(event) => event
            .message
            .content
            .into_iter()
            .filter_map(content_to_event)
            .collect(),
        RawEvent::Result(result) => vec![AgentStreamEvent::Result {
            text: result.result.unwrap_or_default(),
            is_error: result.is_error || result.subtype.as_deref() != Some("success"),
        }],
        RawEvent::Other => Vec::new(),
    }
}

fn content_to_event(block: ContentBlock) -> Option<AgentStreamEvent> {
    match block {
        // Claude emits an empty thinking block on turns with no reasoning;
        // drop empties so the GUI doesn't render a blank reasoning section.
        ContentBlock::Text { text } if !text.is_empty() => {
            Some(AgentStreamEvent::AssistantText { text })
        }
        ContentBlock::Thinking { thinking } if !thinking.is_empty() => {
            Some(AgentStreamEvent::AssistantThinking { text: thinking })
        }
        ContentBlock::ToolUse { id, name, input } => {
            Some(AgentStreamEvent::ToolUse { id, name, input })
        }
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => Some(AgentStreamEvent::ToolResult {
            tool_use_id,
            content: tool_result_text(content),
            is_error,
        }),
        ContentBlock::Text { .. } | ContentBlock::Thinking { .. } | ContentBlock::Other => None,
    }
}

/// Tool-result content is either a plain string or an array of content items
/// (`{type:"text", text:...}`). Flatten to a display string.
fn tool_result_text(content: serde_json::Value) -> String {
    match content {
        serde_json::Value::String(text) => text,
        serde_json::Value::Array(items) => items
            .into_iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
