//! musea/prefer-design-tokens
//!
//! Warn when CSS style blocks contain hardcoded values that match known
//! design token primitive values. Components should reference tokens via
//! CSS custom properties instead of duplicating raw values.
//!
//! ## Examples
//!
//! ### Invalid
//! ```css
//! .button {
//!   background: #3b82f6;  /* matches primitive token color.primary */
//! }
//! ```
//!
//! ### Valid
//! ```css
//! .button {
//!   background: var(--color-primary);
//! }
//! ```

use memchr::memmem;
use std::collections::HashMap;

use super::{MuseaLintResult, MuseaRuleMeta};
use crate::diagnostic::{Fix, LintDiagnostic, Severity, TextEdit};

static META: MuseaRuleMeta = MuseaRuleMeta {
    name: "musea/prefer-design-tokens",
    description: "Prefer design token CSS variables over hardcoded primitive values",
    default_severity: Severity::Warning,
};

/// A known design token for matching against hardcoded values
#[derive(Debug, Clone)]
pub struct TokenInfo {
    /// Token path (e.g., "color.primary")
    pub path: String,
    /// CSS variable name (e.g., "--color-primary")
    pub var_name: String,
    /// Tier: "primitive" or "semantic"
    pub tier: String,
}

/// Configuration for the prefer-design-tokens rule
#[derive(Debug, Clone, Default)]
pub struct PreferDesignTokensConfig {
    /// Map of normalized value -> token info
    /// Multiple tokens may map to the same value
    pub value_map: HashMap<String, Vec<TokenInfo>>,
}

impl PreferDesignTokensConfig {
    /// Add a token to the configuration
    pub fn add_token(&mut self, value: &str, path: &str, tier: &str) {
        let normalized = normalize_value(value);
        let var_name = format!("--{}", path.replace('.', "-"));
        let info = TokenInfo {
            path: path.to_string(),
            var_name,
            tier: tier.to_string(),
        };
        self.value_map.entry(normalized).or_default().push(info);
    }
}

/// Rule: prefer design token CSS variables
pub struct PreferDesignTokens {
    config: PreferDesignTokensConfig,
}

impl PreferDesignTokens {
    pub fn new(config: PreferDesignTokensConfig) -> Self {
        Self { config }
    }

    pub fn meta() -> &'static MuseaRuleMeta {
        &META
    }

    /// Check a source file for hardcoded token values in style blocks
    pub fn check(&self, source: &str, result: &mut MuseaLintResult) {
        if self.config.value_map.is_empty() {
            return;
        }

        let bytes = source.as_bytes();

        // Find all <style> blocks
        let style_finder = memmem::Finder::new(b"<style");
        let style_close_finder = memmem::Finder::new(b"</style>");
        let mut search_start = 0;

        while let Some(style_pos) = style_finder.find(&bytes[search_start..]) {
            let abs_style_start = search_start + style_pos;

            // Find > to get end of opening tag
            let Some(tag_end_offset) = memchr::memchr(b'>', &bytes[abs_style_start..]) else {
                break;
            };
            let content_start = abs_style_start + tag_end_offset + 1;

            // Find </style>
            let Some(close_pos) = style_close_finder.find(&bytes[content_start..]) else {
                break;
            };
            let content_end = content_start + close_pos;

            // Extract and check the CSS content
            if let Ok(css_content) = std::str::from_utf8(&bytes[content_start..content_end]) {
                self.check_css_block(css_content, content_start, result);
            }

            search_start = content_end + 8; // skip </style>
        }
    }

    /// Check a CSS block for hardcoded token values
    fn check_css_block(&self, css: &str, block_offset: usize, result: &mut MuseaLintResult) {
        for (line_idx, line) in css.lines().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines, comments, selectors
            if trimmed.is_empty()
                || trimmed.starts_with("/*")
                || trimmed.starts_with("//")
                || trimmed.starts_with("}")
                || trimmed.starts_with("{")
                || trimmed.ends_with('{')
            {
                continue;
            }

            // Check for property: value pattern
            if let Some(colon_pos) = trimmed.find(':') {
                let value_part = trimmed[colon_pos + 1..].trim();
                // Remove trailing semicolon and !important
                let value_part = value_part
                    .trim_end_matches(';')
                    .trim_end_matches("!important")
                    .trim();

                // Skip values that already use var()
                if value_part.contains("var(") {
                    continue;
                }

                // Check individual value tokens (split on whitespace for shorthand properties)
                let line_byte_offset = css[..css
                    .lines()
                    .take(line_idx)
                    .map(|l| l.len() + 1)
                    .sum::<usize>()]
                    .len();
                let line_start = block_offset + line_byte_offset;
                let line_end = line_start + line.len();

                // Check the full value first
                let normalized_full = normalize_value(value_part);
                if let Some(tokens) = self.config.value_map.get(&normalized_full) {
                    // Prefer primitive tokens for warnings
                    let token = tokens
                        .iter()
                        .find(|t| t.tier == "primitive")
                        .unwrap_or(&tokens[0]);

                    let message = if token.tier == "primitive" {
                        format!(
                            "Hardcoded value '{}' matches primitive token '{}' — use var({})",
                            value_part, token.path, token.var_name
                        )
                    } else {
                        format!(
                            "Hardcoded value '{}' matches token '{}' — use var({})",
                            value_part, token.path, token.var_name
                        )
                    };

                    let fix = Fix::new(
                        format!("Replace with var({})", token.var_name),
                        TextEdit::replace(
                            line_start as u32,
                            line_end as u32,
                            line.replace(value_part, &format!("var({})", token.var_name)),
                        ),
                    );

                    result.add_diagnostic(
                        LintDiagnostic::warn(
                            META.name,
                            message,
                            line_start as u32,
                            line_end as u32,
                        )
                        .with_help(format!(
                            "Use `var({})` for consistent theming and maintainability",
                            token.var_name
                        ))
                        .with_fix(fix),
                    );
                    continue;
                }

                // Check individual tokens in shorthand values (e.g., "1px solid #3b82f6")
                for part in value_part.split_whitespace() {
                    let normalized = normalize_value(part);
                    if let Some(tokens) = self.config.value_map.get(&normalized) {
                        let token = tokens
                            .iter()
                            .find(|t| t.tier == "primitive")
                            .unwrap_or(&tokens[0]);

                        let message = if token.tier == "primitive" {
                            format!(
                                "Hardcoded value '{}' matches primitive token '{}' — use var({})",
                                part, token.path, token.var_name
                            )
                        } else {
                            format!(
                                "Hardcoded value '{}' matches token '{}' — use var({})",
                                part, token.path, token.var_name
                            )
                        };

                        let fix = Fix::new(
                            format!("Replace with var({})", token.var_name),
                            TextEdit::replace(
                                line_start as u32,
                                line_end as u32,
                                line.replace(part, &format!("var({})", token.var_name)),
                            ),
                        );

                        result.add_diagnostic(
                            LintDiagnostic::warn(
                                META.name,
                                message,
                                line_start as u32,
                                line_end as u32,
                            )
                            .with_help(format!(
                                "Use `var({})` for consistent theming and maintainability",
                                token.var_name
                            ))
                            .with_fix(fix),
                        );
                    }
                }
            }
        }
    }
}

/// Normalize a CSS value for comparison
fn normalize_value(value: &str) -> String {
    let v = value.trim().to_lowercase();

    // Normalize hex colors: #fff -> #ffffff
    if let Some(hex) = v.strip_prefix('#') {
        if hex.len() == 3 {
            let expanded: String = hex
                .chars()
                .flat_map(|c| std::iter::repeat_n(c, 2))
                .collect();
            return format!("#{}", expanded);
        }
        if hex.len() == 4 {
            // #rgba -> #rrggbbaa
            let expanded: String = hex
                .chars()
                .flat_map(|c| std::iter::repeat_n(c, 2))
                .collect();
            return format!("#{}", expanded);
        }
    }

    // Normalize leading zero: .5rem -> 0.5rem
    if v.starts_with('.') {
        return format!("0{}", v);
    }

    // Remove spaces in rgb/hsl functions
    if v.starts_with("rgb") || v.starts_with("hsl") {
        return v.replace(' ', "");
    }

    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_config() -> PreferDesignTokensConfig {
        let mut config = PreferDesignTokensConfig::default();
        config.add_token("#3b82f6", "color.primary", "primitive");
        config.add_token("#ef4444", "color.error", "primitive");
        config.add_token("0.5rem", "spacing.sm", "primitive");
        config.add_token("4px", "radius.sm", "primitive");
        config.add_token("#fff", "color.white", "primitive");
        config
    }

    #[test]
    fn test_detects_hardcoded_color() {
        let rule = PreferDesignTokens::new(create_config());
        let source = r#"<art title="Test" component="./Test.vue">
  <variant name="default"><Test /></variant>
</art>

<style scoped>
.test {
  background: #3b82f6;
}
</style>"#;

        let mut result = MuseaLintResult::default();
        rule.check(source, &mut result);
        assert_eq!(result.warning_count, 1, "Should detect hardcoded color");
        assert!(result.diagnostics[0].message.contains("color.primary"));
    }

    #[test]
    fn test_ignores_var_usage() {
        let rule = PreferDesignTokens::new(create_config());
        let source = r#"<art title="Test" component="./Test.vue">
  <variant name="default"><Test /></variant>
</art>

<style scoped>
.test {
  background: var(--color-primary);
}
</style>"#;

        let mut result = MuseaLintResult::default();
        rule.check(source, &mut result);
        assert_eq!(result.warning_count, 0, "Should not warn for var() usage");
    }

    #[test]
    fn test_detects_shorthand_value() {
        let rule = PreferDesignTokens::new(create_config());
        let source = r#"<art title="Test" component="./Test.vue">
  <variant name="default"><Test /></variant>
</art>

<style scoped>
.test {
  border: 1px solid #3b82f6;
}
</style>"#;

        let mut result = MuseaLintResult::default();
        rule.check(source, &mut result);
        assert_eq!(result.warning_count, 1, "Should detect color in shorthand");
    }

    #[test]
    fn test_normalizes_short_hex() {
        let rule = PreferDesignTokens::new(create_config());
        let source = r#"<style>
.test {
  color: #FFF;
}
</style>"#;

        let mut result = MuseaLintResult::default();
        rule.check(source, &mut result);
        assert_eq!(
            result.warning_count, 1,
            "Should normalize #FFF to #ffffff and match"
        );
    }

    #[test]
    fn test_provides_autofix() {
        let rule = PreferDesignTokens::new(create_config());
        let source = r#"<style>
.test {
  background: #3b82f6;
}
</style>"#;

        let mut result = MuseaLintResult::default();
        rule.check(source, &mut result);
        assert!(result.diagnostics[0].fix.is_some(), "Should provide a fix");
    }

    #[test]
    fn test_normalize_value() {
        assert_eq!(normalize_value("#fff"), "#ffffff");
        assert_eq!(normalize_value("#FFF"), "#ffffff");
        assert_eq!(normalize_value("#3b82f6"), "#3b82f6");
        assert_eq!(normalize_value(".5rem"), "0.5rem");
        assert_eq!(normalize_value("0.5rem"), "0.5rem");
        assert_eq!(normalize_value("rgb(255, 0, 0)"), "rgb(255,0,0)");
    }

    #[test]
    fn test_no_style_block() {
        let rule = PreferDesignTokens::new(create_config());
        let source = r#"<art title="Test" component="./Test.vue">
  <variant name="default"><Test /></variant>
</art>"#;

        let mut result = MuseaLintResult::default();
        rule.check(source, &mut result);
        assert_eq!(result.warning_count, 0, "Should handle no style block");
    }

    #[test]
    fn test_empty_config() {
        let rule = PreferDesignTokens::new(PreferDesignTokensConfig::default());
        let source = r#"<style>.test { color: #3b82f6; }</style>"#;

        let mut result = MuseaLintResult::default();
        rule.check(source, &mut result);
        assert_eq!(result.warning_count, 0, "Empty config should not warn");
    }
}
