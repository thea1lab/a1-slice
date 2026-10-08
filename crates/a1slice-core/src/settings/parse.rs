//! Read a settings object, including the old provider:model keys.

use crate::types::{AppSettings, LlmProvider, VideoLanguage};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub(super) fn settings_from_object(raw: &str, object: &Map<String, Value>) -> AppSettings {
    let defaults = AppSettings::default();
    let provider = object
        .get("provider")
        .and_then(Value::as_str)
        .and_then(LlmProvider::parse)
        .unwrap_or(defaults.provider);
    let model = string_field(object, "model").unwrap_or(defaults.model);
    let user_hint = string_field(object, "userHint").unwrap_or(defaults.user_hint);
    let language = object
        .get("language")
        .and_then(Value::as_str)
        .and_then(VideoLanguage::parse)
        .unwrap_or(defaults.language);
    let entropy_thold = object
        .get("entropyThold")
        .and_then(finite_f64)
        .unwrap_or(defaults.entropy_thold);
    let max_context = object
        .get("maxContext")
        .and_then(finite_i64)
        .unwrap_or(defaults.max_context);
    let beam_size = object
        .get("beamSize")
        .and_then(finite_i64)
        .unwrap_or(defaults.beam_size);
    let temperature_inc = object
        .get("temperatureInc")
        .and_then(finite_f64)
        .unwrap_or(defaults.temperature_inc);
    let top_level_key = object.get("apiKey").and_then(Value::as_str).unwrap_or("");
    let api_keys = migrate_api_keys(raw, object.get("apiKeys"), provider, top_level_key);
    let api_key = api_keys
        .get(provider.as_str())
        .cloned()
        .unwrap_or_else(|| top_level_key.to_string());
    AppSettings {
        provider,
        model,
        api_key,
        api_keys,
        user_hint,
        language,
        entropy_thold,
        max_context,
        beam_size,
        temperature_inc,
    }
}

fn string_field(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_string)
}

fn finite_f64(value: &Value) -> Option<f64> {
    let number = value.as_f64()?;
    number.is_finite().then_some(number)
}

fn finite_i64(value: &Value) -> Option<i64> {
    if let Some(number) = value.as_i64() {
        return Some(number);
    }
    let number = finite_f64(value)?;
    if number.fract() == 0.0 && (i64::MIN as f64..=i64::MAX as f64).contains(&number) {
        Some(number as i64)
    } else {
        None
    }
}

/// Prefer the `provider` slot. Otherwise copy the top-level key, then a `provider:model` slot.
fn migrate_api_keys(
    raw: &str,
    api_keys: Option<&Value>,
    provider: LlmProvider,
    api_key: &str,
) -> BTreeMap<String, String> {
    let mut entries = ordered_api_key_entries(raw).unwrap_or_else(|| string_entries(api_keys));
    let provider_id = provider.as_str();
    let slot_filled = entries
        .iter()
        .any(|(id, value)| id == provider_id && !value.is_empty());
    if !slot_filled {
        if !api_key.is_empty() {
            set_entry(&mut entries, provider_id, api_key);
        } else if let Some(legacy) = entries
            .iter()
            .find(|(id, _)| id == provider_id || id.starts_with(&format!("{provider_id}:")))
            .map(|(_, value)| value.clone())
        {
            set_entry(&mut entries, provider_id, &legacy);
        }
    }
    entries.into_iter().collect()
}

fn string_entries(api_keys: Option<&Value>) -> Vec<(String, String)> {
    let Some(Value::Object(map)) = api_keys else {
        return Vec::new();
    };
    map.iter()
        .filter_map(|(id, value)| value.as_str().map(|text| (id.clone(), text.to_string())))
        .collect()
}

fn set_entry(entries: &mut Vec<(String, String)>, key: &str, value: &str) {
    if let Some(slot) = entries.iter_mut().find(|(id, _)| id == key) {
        slot.1 = value.to_string();
    } else {
        entries.push((key.to_string(), value.to_string()));
    }
}

fn ordered_api_key_entries(raw: &str) -> Option<Vec<(String, String)>> {
    let mut parser = Parser { s: raw, i: 0 };
    parser.skip_ws();
    if !parser.eat(b'{') {
        return None;
    }
    loop {
        parser.skip_ws();
        if parser.eat(b'}') {
            return None;
        }
        let key = parser.string()?;
        parser.skip_ws();
        if !parser.eat(b':') {
            return None;
        }
        parser.skip_ws();
        if key == "apiKeys" {
            return parser.string_object();
        }
        parser.skip_value()?;
        parser.skip_ws();
        if parser.eat(b'}') {
            return None;
        }
        if !parser.eat(b',') {
            return None;
        }
    }
}

struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while let Some(ch) = self.s[self.i..].chars().next() {
            if ch.is_ascii_whitespace() {
                self.i += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.as_bytes().get(self.i).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn string(&mut self) -> Option<String> {
        if !self.eat(b'"') {
            return None;
        }
        let bytes = self.s.as_bytes();
        let mut out = String::new();
        while self.i < bytes.len() {
            let ch = bytes[self.i];
            if ch == b'"' {
                self.i += 1;
                return Some(out);
            }
            if ch == b'\\' {
                self.i += 1;
                let escaped = *bytes.get(self.i)?;
                self.i += 1;
                match escaped {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = self.s.get(self.i..self.i + 4)?;
                        let code = u16::from_str_radix(hex, 16).ok()?;
                        self.i += 4;
                        if (0xD800..=0xDBFF).contains(&code) {
                            if bytes.get(self.i) == Some(&b'\\')
                                && bytes.get(self.i + 1) == Some(&b'u')
                            {
                                self.i += 2;
                                let low_hex = self.s.get(self.i..self.i + 4)?;
                                let low = u16::from_str_radix(low_hex, 16).ok()?;
                                self.i += 4;
                                let combined = 0x10000
                                    + (((code as u32) - 0xD800) << 10)
                                    + ((low as u32) - 0xDC00);
                                out.push(char::from_u32(combined)?);
                            }
                        } else {
                            out.push(char::from_u32(code as u32)?);
                        }
                    }
                    _ => return None,
                }
                continue;
            }
            let ch = self.s[self.i..].chars().next()?;
            out.push(ch);
            self.i += ch.len_utf8();
        }
        None
    }

    fn string_object(&mut self) -> Option<Vec<(String, String)>> {
        if !self.eat(b'{') {
            return None;
        }
        let mut entries = Vec::new();
        loop {
            self.skip_ws();
            if self.eat(b'}') {
                return Some(entries);
            }
            let key = self.string()?;
            self.skip_ws();
            if !self.eat(b':') {
                return None;
            }
            self.skip_ws();
            if self.peek() == Some(b'"') {
                entries.push((key, self.string()?));
            } else {
                self.skip_value()?;
            }
            self.skip_ws();
            if self.eat(b'}') {
                return Some(entries);
            }
            if !self.eat(b',') {
                return None;
            }
        }
    }

    fn skip_value(&mut self) -> Option<()> {
        self.skip_ws();
        match self.peek()? {
            b'"' => {
                self.string()?;
                Some(())
            }
            b'{' => self.skip_container(b'{', b'}'),
            b'[' => self.skip_container(b'[', b']'),
            b't' => self.eat_lit("true"),
            b'f' => self.eat_lit("false"),
            b'n' => self.eat_lit("null"),
            b'-' | b'0'..=b'9' => self.skip_number(),
            _ => None,
        }
    }

    fn eat_lit(&mut self, literal: &str) -> Option<()> {
        if self.s[self.i..].starts_with(literal) {
            self.i += literal.len();
            Some(())
        } else {
            None
        }
    }

    fn skip_number(&mut self) -> Option<()> {
        let bytes = self.s.as_bytes();
        if bytes.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        let start = self.i;
        while self.i < bytes.len()
            && matches!(
                bytes[self.i],
                b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-'
            )
        {
            self.i += 1;
        }
        (self.i > start).then_some(())
    }

    fn skip_container(&mut self, open: u8, close: u8) -> Option<()> {
        if !self.eat(open) {
            return None;
        }
        let mut depth = 1;
        let bytes = self.s.as_bytes();
        while self.i < bytes.len() && depth > 0 {
            let ch = bytes[self.i];
            if ch == b'"' {
                self.string()?;
                continue;
            }
            self.i += 1;
            if ch == open {
                depth += 1;
            } else if ch == close {
                depth -= 1;
            }
        }
        (depth == 0).then_some(())
    }
}
