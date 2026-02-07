//! Configuration file loading for vize.
//!
//! Reads `vize.config.json` from the current working directory.
//! Also provides JSON Schema generation for editor autocompletion.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level vize configuration.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct VizeConfig {
    /// JSON Schema reference (for editor autocompletion).
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Type checking configuration.
    #[serde(default)]
    pub check: CheckConfig,
}

/// Configuration for the `check` command.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CheckConfig {
    /// Template global variables to declare (e.g., `["$t", "$route"]`).
    ///
    /// Each entry is either:
    /// - `"$t"` — declares as `any`
    /// - `"$t:(...args: any[]) => string"` — declares with a specific type
    ///
    /// When omitted or null, no plugin globals are declared (empty by default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub globals: Option<Vec<String>>,
}

/// Load `vize.config.json` from the given directory (or CWD if None).
pub fn load_config(dir: Option<&Path>) -> VizeConfig {
    let base = dir
        .map(|d| d.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let config_path = base.join("vize.config.json");

    if !config_path.exists() {
        return VizeConfig::default();
    }

    match std::fs::read_to_string(&config_path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!(
                    "\x1b[33mWarning:\x1b[0m Failed to parse {}: {}",
                    config_path.display(),
                    e
                );
                VizeConfig::default()
            }
        },
        Err(e) => {
            eprintln!(
                "\x1b[33mWarning:\x1b[0m Failed to read {}: {}",
                config_path.display(),
                e
            );
            VizeConfig::default()
        }
    }
}

/// JSON Schema for `vize.config.json`.
pub const VIZE_CONFIG_SCHEMA: &str = r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Vize Configuration",
  "description": "Configuration file for vize - High-performance Vue.js toolchain",
  "type": "object",
  "properties": {
    "$schema": {
      "type": "string",
      "description": "JSON Schema reference for editor autocompletion"
    },
    "check": {
      "type": "object",
      "description": "Type checking configuration",
      "properties": {
        "globals": {
          "type": "array",
          "description": "Template global variables to declare. Each entry is \"$name\" (typed as any) or \"$name:TypeAnnotation\" (with explicit type).",
          "items": {
            "type": "string",
            "pattern": "^\\$?[a-zA-Z_][a-zA-Z0-9_]*(:.+)?$"
          },
          "examples": [
            ["$t", "$d", "$n", "$route", "$router"],
            ["$t:(...args: any[]) => string", "$route:any"]
          ]
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}"#;

/// Write the JSON Schema to `node_modules/.vize/vize.config.schema.json`.
pub fn write_schema(dir: Option<&Path>) {
    let base = dir
        .map(|d| d.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let schema_dir = base.join("node_modules/.vize");
    if std::fs::create_dir_all(&schema_dir).is_ok() {
        let schema_path = schema_dir.join("vize.config.schema.json");
        let _ = std::fs::write(&schema_path, VIZE_CONFIG_SCHEMA);
    }
}
