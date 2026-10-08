//! Load and save ~/.a1slice/settings.json.

use crate::project::a1slice_dir;
use crate::types::AppSettings;
use serde_json::{Map, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::parse::settings_from_object;

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
