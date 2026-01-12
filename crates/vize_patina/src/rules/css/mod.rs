//! CSS lint rules for Vue.js SFC style blocks.
//!
//! These rules check CSS/SCSS/Less code in `<style>` blocks using lightning-css
//! for high-performance parsing.
//!
//! ## Enabling CSS Rules
//!
//! CSS rules can be enabled in your configuration:
//!
//! ```toml
//! [rules.css]
//! "no-important" = "warn"
//! "no-id-selectors" = "warn"
//! ```

mod no_id_selectors;
mod no_important;
mod prefer_logical_properties;
mod require_font_display;

use lightningcss::stylesheet::{ParserOptions, StyleSheet};

use crate::diagnostic::{LintDiagnostic, Severity};

pub use no_id_selectors::NoIdSelectors;
pub use no_important::NoImportant;
pub use prefer_logical_properties::PreferLogicalProperties;
pub use require_font_display::RequireFontDisplay;

/// Metadata for a CSS rule
pub struct CssRuleMeta {
    /// Rule name (e.g., "css/no-important")
    pub name: &'static str,
    /// Rule description
    pub description: &'static str,
    /// Default severity
    pub default_severity: Severity,
}

/// Result of linting a style block
#[derive(Debug, Default)]
pub struct CssLintResult {
    pub diagnostics: Vec<LintDiagnostic>,
    pub error_count: usize,
    pub warning_count: usize,
}

impl CssLintResult {
    pub fn add_diagnostic(&mut self, diagnostic: LintDiagnostic) {
        match diagnostic.severity {
            Severity::Error => self.error_count += 1,
            Severity::Warning => self.warning_count += 1,
        }
        self.diagnostics.push(diagnostic);
    }
}

/// Trait for CSS lint rules
pub trait CssRule: Send + Sync {
    /// Get rule metadata
    fn meta(&self) -> &'static CssRuleMeta;

    /// Check the CSS content using the parsed stylesheet
    ///
    /// * `source` - The original CSS source
    /// * `stylesheet` - The parsed stylesheet from lightning-css
    /// * `offset` - The offset of the style block in the original file
    /// * `result` - Accumulator for diagnostics
    fn check<'i>(
        &self,
        source: &'i str,
        stylesheet: &StyleSheet<'i, 'i>,
        offset: usize,
        result: &mut CssLintResult,
    );
}

/// Linter for style blocks using lightning-css
pub struct CssLinter {
    rules: Vec<Box<dyn CssRule>>,
}

impl CssLinter {
    /// Create a new CSS linter with no rules
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Create a CSS linter with all available rules
    pub fn with_all_rules() -> Self {
        Self {
            rules: vec![
                Box::new(NoImportant),
                Box::new(NoIdSelectors),
                Box::new(PreferLogicalProperties),
                Box::new(RequireFontDisplay),
            ],
        }
    }

    /// Add a rule to the linter
    pub fn add_rule(&mut self, rule: Box<dyn CssRule>) {
        self.rules.push(rule);
    }

    /// Lint a style block
    pub fn lint(&self, source: &str, offset: usize) -> CssLintResult {
        let mut result = CssLintResult::default();

        // Parse CSS with lightning-css
        let stylesheet = match StyleSheet::parse(source, ParserOptions::default()) {
            Ok(ss) => ss,
            Err(_) => {
                // If parsing fails, skip CSS linting
                return result;
            }
        };

        for rule in &self.rules {
            rule.check(source, &stylesheet, offset, &mut result);
        }

        result
    }
}

impl Default for CssLinter {
    fn default() -> Self {
        Self::new()
    }
}
