use std::path::PathBuf;

use serde::Deserialize;

use crate::domain::{ChunkKind, RascalError};

#[derive(Deserialize)]
struct RawConfig {
    api_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    #[serde(default = "default_threshold")]
    threshold: f32,
    #[serde(default = "default_kinds")]
    check_kinds: Vec<String>,
}

fn default_threshold() -> f32 {
    0.75
}

fn default_kinds() -> Vec<String> {
    vec!["function".into(), "impl_block".into(), "trait".into()]
}

pub struct Config {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    pub threshold: f32,
    pub check_kinds: Vec<ChunkKind>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            threshold: default_threshold(),
            check_kinds: vec![ChunkKind::Function, ChunkKind::ImplBlock, ChunkKind::Trait],
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, RascalError> {
        let path = config_path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = std::fs::read_to_string(&path)
            .map_err(|e| RascalError::Config(format!("cannot read {}: {e}", path.display())))?;

        let parsed: RawConfig =
            toml::from_str(&raw).map_err(|e| RascalError::Config(e.to_string()))?;

        let check_kinds = parsed
            .check_kinds
            .iter()
            .filter_map(|s| parse_kind(s))
            .collect();

        Ok(Self {
            api_url: parsed.api_url.unwrap_or_default(),
            api_key: parsed.api_key.unwrap_or_default(),
            model: parsed.model.unwrap_or_default(),
            threshold: parsed.threshold,
            check_kinds,
        })
    }
}

fn parse_kind(s: &str) -> Option<ChunkKind> {
    match s {
        "function" => Some(ChunkKind::Function),
        "impl_block" => Some(ChunkKind::ImplBlock),
        "trait" => Some(ChunkKind::Trait),
        "struct" => Some(ChunkKind::Struct),
        "enum" => Some(ChunkKind::Enum),
        _ => None,
    }
}

fn config_path() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("rascal")
        .join("config.toml")
}
