//! css/require-font-display
//!
//! Require font-display property in @font-face rules.

use lightningcss::rules::font_face::FontFaceProperty;
use lightningcss::rules::CssRule as LCssRule;
use lightningcss::stylesheet::StyleSheet;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{CssLintResult, CssRule, CssRuleMeta};

static META: CssRuleMeta = CssRuleMeta {
    name: "css/require-font-display",
    description: "Require font-display in @font-face rules",
    default_severity: Severity::Warning,
};

/// Require font-display rule
pub struct RequireFontDisplay;

impl CssRule for RequireFontDisplay {
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

impl RequireFontDisplay {
    fn check_rule(&self, rule: &LCssRule, offset: usize, result: &mut CssLintResult) {
        match rule {
            LCssRule::FontFace(font_face) => {
                // font-display is stored as Custom property in lightningcss
                let has_font_display = font_face.properties.iter().any(|prop| {
                    if let FontFaceProperty::Custom(custom) = prop {
                        custom.name.as_ref() == "font-display"
                    } else {
                        false
                    }
                });

                if !has_font_display {
                    result.add_diagnostic(
                        LintDiagnostic::warn(
                            META.name,
                            "@font-face rule is missing font-display property",
                            offset as u32,
                            (offset + 10) as u32,
                        )
                        .with_help(
                            "Add `font-display: swap;` to prevent FOIT (Flash of Invisible Text)",
                        ),
                    );
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::css::CssLinter;

    fn create_linter() -> CssLinter {
        let mut linter = CssLinter::new();
        linter.add_rule(Box::new(RequireFontDisplay));
        linter
    }

    #[test]
    fn test_valid_with_font_display() {
        let linter = create_linter();
        let result = linter.lint(
            "@font-face { font-family: 'MyFont'; font-display: swap; src: url('f.woff2'); }",
            0,
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_missing_font_display() {
        let linter = create_linter();
        let result = linter.lint(
            "@font-face { font-family: 'MyFont'; src: url('font.woff2'); }",
            0,
        );
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_no_font_face() {
        let linter = create_linter();
        let result = linter.lint(".button { color: red; }", 0);
        assert_eq!(result.warning_count, 0);
    }
}
