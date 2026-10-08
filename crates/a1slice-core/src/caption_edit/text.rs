//! Clocks, fences, and the small string helpers the editor shares.

use serde_json::Value;

use super::apply::caption_file_lines;

pub(super) fn strip_stamp(line: &str) -> String {
    match match_time_prefix(line) {
        Some(index) => js_trim(&line[index..]).to_string(),
        None => js_trim(line).to_string(),
    }
}

pub(super) fn has_caption_stamp(line: &str) -> bool {
    match_time_prefix(line).is_some_and(|index| index < line.len())
}

fn match_time_prefix(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut digits = 0;
    while digits < bytes.len() && bytes[digits].is_ascii_digit() {
        digits += 1;
    }
    if digits > 0 && digits < bytes.len() && bytes[digits] == b':' {
        if let Some(index) = match_min_sec(line, digits + 1) {
            return Some(index);
        }
    }
    match_min_sec(line, 0)
}

fn match_min_sec(line: &str, start: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    if start >= bytes.len() || !bytes[start].is_ascii_digit() {
        return None;
    }
    let mut consumed = start + 1;
    if consumed < bytes.len() && bytes[consumed].is_ascii_digit() {
        consumed += 1;
    }
    if let Some(index) = after_minutes(line, consumed) {
        return Some(index);
    }
    if consumed == start + 2 {
        after_minutes(line, start + 1)
    } else {
        None
    }
}

fn after_minutes(line: &str, colon_at: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    if colon_at >= bytes.len() || bytes[colon_at] != b':' {
        return None;
    }
    let seconds = colon_at + 1;
    if seconds + 1 >= bytes.len()
        || !bytes[seconds].is_ascii_digit()
        || !bytes[seconds + 1].is_ascii_digit()
    {
        return None;
    }
    let after = seconds + 2;
    let whitespace = leading_ws_len(&line[after..]);
    if whitespace == 0 {
        return None;
    }
    Some(after + whitespace)
}

pub(super) fn strip_ansi(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            let mut j = i + 2;
            while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b';') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'm' {
                i = j + 1;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

pub(super) fn caption_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<String>> = None;
    for line in split_lines(&strip_ansi(text)) {
        if js_trim(line).is_empty() {
            if let Some(cur) = current.take() {
                if !cur.is_empty() {
                    let joined = cur.join("\n");
                    blocks.push(js_trim(&joined).to_string());
                }
            }
            continue;
        }
        if let Some(cur) = current.as_mut() {
            cur.push(line.to_string());
            continue;
        }
        if has_caption_stamp(js_trim(line)) {
            current = Some(vec![line.to_string()]);
        }
    }
    if let Some(cur) = current.take() {
        if !cur.is_empty() {
            let joined = cur.join("\n");
            blocks.push(js_trim(&joined).to_string());
        }
    }
    blocks
        .into_iter()
        .filter(|block| !caption_file_lines(block).is_empty())
        .collect()
}

pub(super) fn first_fence<'a>(output: &'a str, langs: &[&str]) -> Option<&'a str> {
    let mut index = 0;
    while index < output.len() {
        if output[index..].starts_with("```") {
            if let Some(content) = fence_at(output, index + 3, langs) {
                return Some(content);
            }
        }
        index += output[index..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
    }
    None
}

fn fence_at<'a>(output: &'a str, after_ticks: usize, langs: &[&str]) -> Option<&'a str> {
    let rest = &output[after_ticks..];
    let mut options: Vec<&str> = langs.to_vec();
    options.push("");
    for lang in options {
        if !rest.starts_with(lang) {
            continue;
        }
        let mut content_at = after_ticks + lang.len();
        content_at += leading_ws_len(&output[content_at..]);
        if let Some(close) = output[content_at..].find("```") {
            return Some(&output[content_at..content_at + close]);
        }
    }
    None
}

pub(super) fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Number(number) => {
            if let Some(int) = number.as_i64() {
                int.to_string()
            } else if let Some(uint) = number.as_u64() {
                uint.to_string()
            } else if let Some(float) = number.as_f64() {
                format!("{float}")
            } else {
                "undefined".to_string()
            }
        }
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

pub(super) fn finite_f64(value: &Value) -> Option<f64> {
    let number = value.as_f64()?;
    number.is_finite().then_some(number)
}

pub(super) fn js_round(n: f64) -> i64 {
    if !n.is_finite() {
        return 0;
    }
    let floor = n.floor();
    let rounded = if n - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded >= i64::MAX as f64 {
        i64::MAX
    } else if rounded <= i64::MIN as f64 {
        i64::MIN
    } else {
        rounded as i64
    }
}

pub(super) fn js_floor_nonneg(n: f64) -> i64 {
    if !n.is_finite() {
        return 0;
    }
    let floor = n.floor();
    if floor <= 0.0 {
        0
    } else if floor >= i64::MAX as f64 {
        i64::MAX
    } else {
        floor as i64
    }
}

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    ) || ('\u{2000}'..='\u{200A}').contains(&c)
}

pub(super) fn js_trim(s: &str) -> &str {
    let Some(start) = s.find(|c: char| !is_js_ws(c)) else {
        return "";
    };
    let end = s
        .rfind(|c: char| !is_js_ws(c))
        .map(|index| index + s[index..].chars().next().map(|c| c.len_utf8()).unwrap_or(0))
        .unwrap_or(s.len());
    &s[start..end]
}

fn leading_ws_len(s: &str) -> usize {
    let mut size = 0;
    for c in s.chars() {
        if !is_js_ws(c) {
            break;
        }
        size += c.len_utf8();
    }
    size
}

pub(super) fn split_js_ws(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (index, c) in s.char_indices() {
        if is_js_ws(c) {
            if let Some(from) = start.take() {
                out.push(&s[from..index]);
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(from) = start {
        out.push(&s[from..]);
    }
    out
}

pub(super) fn split_lines(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let mut end = i;
            if end > start && bytes[end - 1] == b'\r' {
                end -= 1;
            }
            lines.push(&text[start..end]);
            i += 1;
            start = i;
        } else {
            i += 1;
        }
    }
    lines.push(&text[start..]);
    lines
}
