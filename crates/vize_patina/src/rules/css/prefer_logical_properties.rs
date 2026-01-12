//! css/prefer-logical-properties
//!
//! Recommend using CSS logical properties for better internationalization.

use lightningcss::declaration::DeclarationBlock;
use lightningcss::properties::PropertyId;
use lightningcss::rules::CssRule as LCssRule;
use lightningcss::stylesheet::StyleSheet;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{CssLintResult, CssRule, CssRuleMeta};

static META: CssRuleMeta = CssRuleMeta {
    name: "css/prefer-logical-properties",
    description: "Recommend CSS logical properties for better i18n support",
    default_severity: Severity::Warning,
};

/// Prefer logical properties rule
pub struct PreferLogicalProperties;

impl CssRule for PreferLogicalProperties {
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

impl PreferLogicalProperties {
    fn check_rule(&self, rule: &LCssRule, offset: usize, result: &mut CssLintResult) {
        match rule {
            LCssRule::Style(style_rule) => {
                self.check_declarations(&style_rule.declarations, offset, result);
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

    fn check_declarations(
        &self,
        declarations: &DeclarationBlock,
        offset: usize,
        result: &mut CssLintResult,
    ) {
        // Check all declarations (both regular and important)
        for decl in declarations.declarations.iter() {
            self.check_property(decl.property_id(), offset, result);
        }
        for decl in declarations.important_declarations.iter() {
            self.check_property(decl.property_id(), offset, result);
        }
    }

    #[inline]
    fn check_property(&self, prop_id: PropertyId, offset: usize, result: &mut CssLintResult) {
        let (physical, logical) = match prop_id {
            PropertyId::MarginLeft => ("margin-left", "margin-inline-start"),
            PropertyId::MarginRight => ("margin-right", "margin-inline-end"),
            PropertyId::PaddingLeft => ("padding-left", "padding-inline-start"),
            PropertyId::PaddingRight => ("padding-right", "padding-inline-end"),
            PropertyId::BorderLeftWidth => ("border-left-width", "border-inline-start-width"),
            PropertyId::BorderRightWidth => ("border-right-width", "border-inline-end-width"),
            PropertyId::Left => ("left", "inset-inline-start"),
            PropertyId::Right => ("right", "inset-inline-end"),
            _ => return,
        };

        // Report at offset since PropertyId doesn't provide precise location
        result.add_diagnostic(
            LintDiagnostic::warn(
                META.name,
                format!(
                    "Consider using '{}' instead of '{}' for better RTL support",
                    logical, physical
                ),
                offset as u32,
                (offset + physical.len()) as u32,
            )
            .with_help(format!("Replace '{}' with '{}'", physical, logical)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::css::CssLinter;

    fn create_linter() -> CssLinter {
        let mut linter = CssLinter::new();
        linter.add_rule(Box::new(PreferLogicalProperties));
        linter
    }

    #[test]
    fn test_valid_logical_properties() {
        let linter = create_linter();
        let result = linter.lint(".button { margin-inline-start: 10px; }", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_warns_physical_properties() {
        let linter = create_linter();
        let result = linter.lint(".button { margin-left: 10px; }", 0);
        assert_eq!(result.warning_count, 1);
    }
}
