//! css/no-important
//!
//! Discourage use of !important in CSS.
//!
//! Using !important makes styles harder to override and maintain.
//! It's often a sign of specificity wars and can lead to CSS bloat.

use lightningcss::declaration::DeclarationBlock;
use lightningcss::rules::CssRule as LCssRule;
use lightningcss::stylesheet::StyleSheet;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{CssLintResult, CssRule, CssRuleMeta};

static META: CssRuleMeta = CssRuleMeta {
    name: "css/no-important",
    description: "Discourage use of !important in CSS",
    default_severity: Severity::Warning,
};

/// No !important rule
pub struct NoImportant;

impl CssRule for NoImportant {
    fn meta(&self) -> &'static CssRuleMeta {
        &META
    }

    fn check<'i>(
        &self,
        source: &'i str,
        stylesheet: &StyleSheet<'i, 'i>,
        offset: usize,
        result: &mut CssLintResult,
    ) {
        // Walk through all rules
        for rule in &stylesheet.rules.0 {
            self.check_rule(source, rule, offset, result);
        }
    }
}

impl NoImportant {
    fn check_rule(&self, source: &str, rule: &LCssRule, offset: usize, result: &mut CssLintResult) {
        match rule {
            LCssRule::Style(style_rule) => {
                self.check_declarations(source, &style_rule.declarations, offset, result);
            }
            LCssRule::Media(media) => {
                for rule in &media.rules.0 {
                    self.check_rule(source, rule, offset, result);
                }
            }
            LCssRule::Supports(supports) => {
                for rule in &supports.rules.0 {
                    self.check_rule(source, rule, offset, result);
                }
            }
            LCssRule::LayerBlock(layer) => {
                for rule in &layer.rules.0 {
                    self.check_rule(source, rule, offset, result);
                }
            }
            _ => {}
        }
    }

    fn check_declarations(
        &self,
        _source: &str,
        declarations: &DeclarationBlock,
        offset: usize,
        result: &mut CssLintResult,
    ) {
        // Check important declarations
        if !declarations.important_declarations.is_empty() {
            // Report at offset since we don't have precise location for each declaration
            result.add_diagnostic(
                LintDiagnostic::warn(
                    META.name,
                    "Avoid using !important as it makes styles harder to override",
                    offset as u32,
                    (offset + 10) as u32,
                )
                .with_help("Use more specific selectors or reorganize CSS specificity instead"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::css::CssLinter;

    fn create_linter() -> CssLinter {
        let mut linter = CssLinter::new();
        linter.add_rule(Box::new(NoImportant));
        linter
    }

    #[test]
    fn test_valid_no_important() {
        let linter = create_linter();
        let result = linter.lint(".button { color: red; }", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_important() {
        let linter = create_linter();
        let result = linter.lint(".button { color: red !important; }", 0);
        assert_eq!(result.warning_count, 1);
    }
}
