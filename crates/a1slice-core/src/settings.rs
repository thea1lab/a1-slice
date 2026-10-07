//! `~/.a1slice/settings.json`, including the legacy `provider:model` key migration.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::project::a1slice_dir;
use crate::types::{AppSettings, LlmProvider, VideoLanguage};

pub fn user_settings_path() -> PathBuf {
    a1slice_dir().join("settings.json")
}

pub fn load_settings(path: &Path) -> AppSettings {
    let Ok(raw) = fs::read_to_string(path) else {
        return AppSettings::default();
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return AppSettings::default();
    };
    let Some(object) = value.as_object() else {
        return AppSettings::default();
    };
    settings_from_object(&raw, object)
}

pub fn load_user_settings() -> AppSettings {
    load_settings(&user_settings_path())
}

pub fn save_settings(path: &Path, settings: &AppSettings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut root = match fs::read_to_string(path) {
        Ok(raw) => match serde_json::from_str::<Value>(&raw) {
            Ok(Value::Object(map)) => map,
            _ => Map::new(),
        },
        Err(_) => Map::new(),
    };
    let incoming = serde_json::to_value(settings)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    if let Value::Object(fields) = incoming {
        for (key, value) in fields {
            root.insert(key, value);
        }
    }
    let text = serde_json::to_string_pretty(&Value::Object(root))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(path, text)
}

pub fn save_user_settings(settings: &AppSettings) -> io::Result<()> {
    save_settings(&user_settings_path(), settings)
}

fn settings_from_object(raw: &str, object: &Map<String, Value>) -> AppSettings {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LlmProvider;
    use serde_json::Value;

    #[test]
    fn unknown_key_survives_load_save_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{"provider":"openai","model":"gpt-test","extraFlag":true,"userHint":"hi"}"#,
        )
        .unwrap();
        let loaded = load_settings(&path);
        assert_eq!(loaded.provider, LlmProvider::Openai);
        assert_eq!(loaded.model, "gpt-test");
        assert_eq!(loaded.user_hint, "hi");
        assert_eq!(loaded.entropy_thold, 2.8);
        assert_eq!(loaded.language, crate::types::VideoLanguage::Auto);
        save_settings(&path, &loaded).unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["extraFlag"], true);
        assert_eq!(saved["provider"], "openai");
        assert_eq!(saved["model"], "gpt-test");
        assert_eq!(saved["userHint"], "hi");
        assert!((saved["entropyThold"].as_f64().unwrap() - 2.8).abs() < 1e-9);
    }

    #[test]
    fn broken_json_falls_through_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        assert_eq!(
            load_settings(&dir.path().join("missing.json")),
            AppSettings::default()
        );
        fs::write(&path, "{").unwrap();
        assert_eq!(load_settings(&path), AppSettings::default());
        fs::write(&path, "null").unwrap();
        assert_eq!(load_settings(&path), AppSettings::default());
    }

    #[test]
    fn legacy_provider_model_key_is_copied_onto_the_provider() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{"provider":"openai","apiKey":"","apiKeys":{"openai:gpt-5-mini":"sk-legacy","claude":"sk-c"}}"#,
        )
        .unwrap();
        let loaded = load_settings(&path);
        assert_eq!(loaded.provider, LlmProvider::Openai);
        assert_eq!(loaded.model, "claude-haiku-4-5");
        assert_eq!(loaded.api_key, "sk-legacy");
        assert_eq!(
            loaded.api_keys.get("openai").map(String::as_str),
            Some("sk-legacy")
        );
        assert_eq!(
            loaded.api_keys.get("claude").map(String::as_str),
            Some("sk-c")
        );
        assert_eq!(
            loaded.api_keys.get("openai:gpt-5-mini").map(String::as_str),
            Some("sk-legacy")
        );

        fs::write(
            &path,
            r#"{"provider":"claude","apiKey":"sk-top","apiKeys":{"claude:old":"sk-old"}}"#,
        )
        .unwrap();
        let loaded = load_settings(&path);
        assert_eq!(loaded.api_key, "sk-top");
        assert_eq!(
            loaded.api_keys.get("claude").map(String::as_str),
            Some("sk-top")
        );

        fs::write(
            &path,
            r#"{"provider":"claude","apiKey":"sk-top","apiKeys":{"claude":"sk-map"}}"#,
        )
        .unwrap();
        assert_eq!(load_settings(&path).api_key, "sk-map");

        fs::write(
            &path,
            r#"{"apiKeys":{"claude:second":"b","claude:first":"a"}}"#,
        )
        .unwrap();
        assert_eq!(load_settings(&path).api_key, "b");

        fs::write(&path, r#"{"apiKeys":{"claude":"","claude:haiku":"sk-h"}}"#).unwrap();
        assert_eq!(load_settings(&path).api_key, "");

        fs::write(&path, r#"{"apiKeys":{"claude:haiku":"sk-h","claude":""}}"#).unwrap();
        assert_eq!(load_settings(&path).api_key, "sk-h");
    }

    #[test]
    fn save_creates_the_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("settings.json");
        save_settings(&path, &AppSettings::default()).unwrap();
        assert!(path.is_file());
        assert_eq!(load_settings(&path).provider, LlmProvider::Claude);
    }
}
