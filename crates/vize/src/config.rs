//! Configuration file loading for vize.
//!
//! Reads `vize.config.json` from the current working directory.
//! Also provides JSON Schema generation for editor autocompletion.

use serde::{Deserialize, Serialize};
use std::path::Path;
use vize_glyph::FormatOptions;

/// Top-level vize configuration.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct VizeConfig {
    /// JSON Schema reference (for editor autocompletion).
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Type checking configuration.
    #[serde(default)]
    pub check: CheckConfig,

    /// Formatting configuration.
    #[serde(default)]
    pub fmt: FormatOptions,
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
    },
    "fmt": {
      "type": "object",
      "description": "Formatting configuration (Prettier-compatible)",
      "properties": {
        "printWidth": { "type": "integer", "default": 100, "description": "Maximum line width" },
        "tabWidth": { "type": "integer", "default": 2, "description": "Number of spaces per indentation level" },
        "useTabs": { "type": "boolean", "default": false, "description": "Use tabs instead of spaces" },
        "semi": { "type": "boolean", "default": true, "description": "Print semicolons at the ends of statements" },
        "singleQuote": { "type": "boolean", "default": false, "description": "Use single quotes instead of double quotes" },
        "jsxSingleQuote": { "type": "boolean", "default": false, "description": "Use single quotes in JSX" },
        "trailingComma": { "type": "string", "enum": ["none", "es5", "all"], "default": "all", "description": "Print trailing commas wherever possible" },
        "bracketSpacing": { "type": "boolean", "default": true, "description": "Print spaces between brackets in object literals" },
        "bracketSameLine": { "type": "boolean", "default": false, "description": "Put > of multi-line element at end of last line" },
        "arrowParens": { "type": "string", "enum": ["always", "avoid"], "default": "always", "description": "Include parens around sole arrow function parameter" },
        "endOfLine": { "type": "string", "enum": ["lf", "crlf", "cr", "auto"], "default": "lf", "description": "End of line style" },
        "quoteProps": { "type": "string", "enum": ["as-needed", "consistent", "preserve"], "default": "as-needed" },
        "singleAttributePerLine": { "type": "boolean", "default": false, "description": "Put each HTML attribute on its own line" },
        "vueIndentScriptAndStyle": { "type": "boolean", "default": false, "description": "Indent script and style tags in Vue files" },
        "sortAttributes": { "type": "boolean", "default": true, "description": "Sort HTML attributes in template" },
        "attributeSortOrder": { "type": "string", "enum": ["alphabetical", "as-written"], "default": "alphabetical", "description": "Sort order within attribute groups" },
        "mergeBindAndNonBindAttrs": { "type": "boolean", "default": false, "description": "Merge :xxx and xxx attributes for sorting" },
        "maxAttributesPerLine": { "type": "integer", "minimum": 1, "description": "Max attributes per line before wrapping" },
        "attributeGroups": { "type": "array", "items": { "type": "array", "items": { "type": "string" } }, "description": "Custom attribute sort groups (overrides Vue style guide order)" },
        "normalizeDirectiveShorthands": { "type": "boolean", "default": true, "description": "Normalize v-bind:/v-on:/v-slot: to :/@ /#" }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_config_returns_defaults_when_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = load_config(Some(dir.path()));
        assert_eq!(config.fmt.print_width, 100);
        assert_eq!(config.fmt.tab_width, 2);
        assert!(!config.fmt.use_tabs);
        assert!(config.fmt.semi);
        assert!(!config.fmt.single_quote);
        assert!(config.fmt.sort_attributes);
        assert!(config.fmt.normalize_directive_shorthands);
    }

    #[test]
    fn load_config_parses_fmt_section() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("vize.config.json");
        std::fs::write(
            &config_path,
            r#"{
                "fmt": {
                    "printWidth": 80,
                    "tabWidth": 4,
                    "useTabs": true,
                    "semi": false,
                    "singleQuote": true,
                    "sortAttributes": false,
                    "normalizeDirectiveShorthands": false
                }
            }"#,
        )
        .unwrap();

        let config = load_config(Some(dir.path()));
        assert_eq!(config.fmt.print_width, 80);
        assert_eq!(config.fmt.tab_width, 4);
        assert!(config.fmt.use_tabs);
        assert!(!config.fmt.semi);
        assert!(config.fmt.single_quote);
        assert!(!config.fmt.sort_attributes);
        assert!(!config.fmt.normalize_directive_shorthands);
    }

    #[test]
    fn load_config_partial_fmt_uses_defaults_for_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("vize.config.json");
        std::fs::write(&config_path, r#"{ "fmt": { "printWidth": 120 } }"#).unwrap();

        let config = load_config(Some(dir.path()));
        assert_eq!(config.fmt.print_width, 120);
        // defaults preserved
        assert_eq!(config.fmt.tab_width, 2);
        assert!(!config.fmt.use_tabs);
        assert!(config.fmt.semi);
    }

    #[test]
    fn load_config_returns_defaults_on_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("vize.config.json");
        std::fs::write(&config_path, "not valid json {{{").unwrap();

        let config = load_config(Some(dir.path()));
        // should fall back to defaults
        assert_eq!(config.fmt.print_width, 100);
        assert_eq!(config.fmt.tab_width, 2);
    }

    #[test]
    fn load_config_with_check_and_fmt() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("vize.config.json");
        std::fs::write(
            &config_path,
            r#"{
                "check": { "globals": ["$t", "$route"] },
                "fmt": { "singleQuote": true, "maxAttributesPerLine": 3 }
            }"#,
        )
        .unwrap();

        let config = load_config(Some(dir.path()));
        // check section
        let globals = config.check.globals.unwrap();
        assert_eq!(globals, vec!["$t", "$route"]);
        // fmt section
        assert!(config.fmt.single_quote);
        assert_eq!(config.fmt.max_attributes_per_line, Some(3));
    }
}

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
