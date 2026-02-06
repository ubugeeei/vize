//! vue/html-quotes
//!
//! Enforce consistent use of quotes in HTML attributes.
//!
//! ## Options
//!
//! - `"double"` (default): Require double quotes
//! - `"single"`: Require single quotes
//!
//! ## Examples
//!
//! ### Invalid (with double option)
//! ```vue
//! <div class='foo'></div>
//! ```
//!
//! ### Valid (with double option)
//! ```vue
//! <div class="foo"></div>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, PropNode};

static META: RuleMeta = RuleMeta {
    name: "vue/html-quotes",
    description: "Enforce quotes style of HTML attributes",
    category: RuleCategory::StronglyRecommended,
    fixable: true,
    default_severity: Severity::Warning,
};

/// Quote style preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HtmlQuotesOption {
    #[default]
    Double,
    Single,
}

/// Enforce HTML attribute quote style
pub struct HtmlQuotes {
    pub style: HtmlQuotesOption,
}

impl Default for HtmlQuotes {
    fn default() -> Self {
        Self {
            style: HtmlQuotesOption::Double,
        }
    }
}

impl Rule for HtmlQuotes {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if let Some(value) = &attr.value {
                    // Use the source span from the attribute location to check quote style.
                    // The attribute loc.source contains the full attribute including quotes.
                    // We need to look at the source around the value to find the quote character.
                    let start = value.loc.start.offset as usize;
                    if start == 0 || start > ctx.source.len() {
                        continue;
                    }

                    // Check the character just before the value content (the opening quote)
                    let quote_char = ctx.source.as_bytes().get(start.wrapping_sub(1)).copied();

                    match self.style {
                        HtmlQuotesOption::Double => {
                            if quote_char == Some(b'\'') {
                                ctx.warn_with_help(
                                    ctx.t("vue/html-quotes.message_double"),
                                    &value.loc,
                                    ctx.t("vue/html-quotes.help"),
                                );
                            }
                        }
                        HtmlQuotesOption::Single => {
                            if quote_char == Some(b'"') {
                                ctx.warn_with_help(
                                    ctx.t("vue/html-quotes.message_single"),
                                    &value.loc,
                                    ctx.t("vue/html-quotes.help"),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;

    fn create_linter() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(HtmlQuotes::default()));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_double_quotes() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div class="foo"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_no_value() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input disabled />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_single_option_warns_on_double_quotes() {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(HtmlQuotes {
            style: HtmlQuotesOption::Single,
        }));
        let linter = Linter::with_registry(registry);
        // Parser preserves double quotes, so single-quote preference should warn
        let result = linter.lint_template(r#"<div class="foo"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
