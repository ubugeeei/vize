//! css/no-id-selectors
//!
//! Discourage use of ID selectors in CSS.
//!
//! ID selectors have very high specificity which makes styles hard to override.
//! They also can't be reused since IDs must be unique in a document.

use lightningcss::rules::CssRule as LCssRule;
use lightningcss::selector::{Component, Selector};
use lightningcss::stylesheet::StyleSheet;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{CssLintResult, CssRule, CssRuleMeta};

static META: CssRuleMeta = CssRuleMeta {
    name: "css/no-id-selectors",
    description: "Discourage use of ID selectors in CSS",
    default_severity: Severity::Warning,
};

/// No ID selectors rule
pub struct NoIdSelectors;

impl CssRule for NoIdSelectors {
    fn meta(&self) -> &'static CssRuleMeta {
        &META
    }

    fn check<'i>(
        &self,
        _source: &'i str,
        stylesheet: &StyleSheet<'i, 'i>,
        offset: usize,
        result: &mut CssLintResult,
    ) {
        for rule in &stylesheet.rules.0 {
            self.check_rule(rule, offset, result);
        }
    }
}

impl NoIdSelectors {
    fn check_rule(&self, rule: &LCssRule, offset: usize, result: &mut CssLintResult) {
        match rule {
            LCssRule::Style(style_rule) => {
                for selector in style_rule.selectors.0.iter() {
                    self.check_selector(selector, offset, result);
                }
            }
            LCssRule::Media(media) => {
                for rule in &media.rules.0 {
                    self.check_rule(rule, offset, result);
                }
            }
            LCssRule::Supports(supports) => {
                for rule in &supports.rules.0 {
                    self.check_rule(rule, offset, result);
                }
            }
            LCssRule::LayerBlock(layer) => {
                for rule in &layer.rules.0 {
                    self.check_rule(rule, offset, result);
                }
            }
            _ => {}
        }
    }

    fn check_selector(&self, selector: &Selector, offset: usize, result: &mut CssLintResult) {
        for component in selector.iter() {
            if let Component::ID(id) = component {
                result.add_diagnostic(
                    LintDiagnostic::warn(
                        META.name,
                        format!(
                            "Avoid ID selector '#{}' - use class selectors for better reusability",
                            id.0
                        ),
                        offset as u32,
                        (offset + id.0.len() + 1) as u32,
                    )
                    .with_help("Replace with a class selector for lower specificity"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::css::CssLinter;

    fn create_linter() -> CssLinter {
        let mut linter = CssLinter::new();
        linter.add_rule(Box::new(NoIdSelectors));
        linter
    }

    #[test]
    fn test_valid_class_selector() {
        let linter = create_linter();
        let result = linter.lint(".button { color: red; }", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_id_selector() {
        let linter = create_linter();
        let result = linter.lint("#header { color: red; }", 0);
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_color_hex() {
        let linter = create_linter();
        let result = linter.lint(".button { color: #ff0000; }", 0);
        assert_eq!(result.warning_count, 0);
    }
}
