//! `~/.a1slice/settings.json`, including the legacy `provider:model` key migration.

mod io;
mod parse;

pub use io::{
    load_settings, load_user_settings, save_settings, save_user_settings, user_settings_path,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::types::{AppSettings, LlmProvider};
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
