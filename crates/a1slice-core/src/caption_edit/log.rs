//! Hide the agent's JSON noise and keep the text a person should see.

use serde_json::Value;

use super::text::{js_string, js_trim, split_lines, strip_ansi};
use super::types::AgentLogState;

pub fn empty_agent_log_state() -> AgentLogState {
    AgentLogState {
        hide_json: false,
        final_text: None,
        pieces: Vec::new(),
    }
}

pub fn visible_log_line(line: &str, hide_json: &mut bool) -> Option<String> {
    let trimmed = js_trim(&strip_ansi(line)).to_string();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("CAPTIONS_JSON_START") {
        *hide_json = true;
        return Some("Writing the edited captions.".to_string());
    }
    if trimmed.contains("CAPTIONS_JSON_END") {
        *hide_json = false;
        return None;
    }
    if *hide_json || trimmed.starts_with("```") {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("NOTE:") {
        return Some(js_trim(rest).to_string());
    }
    Some(trimmed)
}

pub fn interpret_agent_line(
    line: &str,
    state: &mut AgentLogState,
    stream_json: bool,
) -> Vec<String> {
    if !stream_json {
        state.pieces.push(line.to_string());
        return visible_log_line(line, &mut state.hide_json)
            .into_iter()
            .collect();
    }
    let event: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => {
            return visible_log_line(line, &mut state.hide_json)
                .into_iter()
                .collect()
        }
    };
    if event.get("type").and_then(Value::as_str) == Some("result") {
        if let Some(result) = event.get("result").and_then(Value::as_str) {
            state.final_text = Some(result.to_string());
            if !state.pieces.is_empty() {
                return Vec::new();
            }
            let mut logs = Vec::new();
            for part in split_lines(result) {
                if let Some(shown) = visible_log_line(part, &mut state.hide_json) {
                    logs.push(shown);
                }
            }
            return logs;
        }
    }
    if event.get("type").and_then(Value::as_str) != Some("assistant") {
        return Vec::new();
    }
    let Some(message) = event.get("message") else {
        return Vec::new();
    };
    if message.is_null() || !message.is_object() {
        return Vec::new();
    }
    let Some(content) = message.get("content").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut logs = Vec::new();
    for block in content {
        let Some(row) = block.as_object() else {
            continue;
        };
        if row.get("type").and_then(Value::as_str) == Some("tool_use") {
            let name = row
                .get("name")
                .map(js_string)
                .unwrap_or_else(|| "undefined".to_string());
            logs.push(if name == "Read" {
                "Reading the captions.".to_string()
            } else {
                format!("Using {name}.")
            });
        }
        if row.get("type").and_then(Value::as_str) == Some("text") {
            if let Some(text) = row.get("text").and_then(Value::as_str) {
                state.pieces.push(text.to_string());
                for part in split_lines(text) {
                    if let Some(shown) = visible_log_line(part, &mut state.hide_json) {
                        logs.push(shown);
                    }
                }
            }
        }
    }
    logs
}

pub fn agent_output_text(state: &AgentLogState) -> String {
    if let Some(text) = &state.final_text {
        return text.clone();
    }
    state.pieces.join("\n")
}
