//! a11y/click-events-have-key-events
//!
//! Require keyboard event handlers with click events on non-interactive elements.
//!
//! Non-interactive elements with click handlers should also have keyboard event
//! handlers to ensure keyboard accessibility.
//!
//! Based on eslint-plugin-vuejs-accessibility click-events-have-key-events rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ExpressionNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/click-events-have-key-events",
    description: "Require keyboard event handlers with click events",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Require keyboard event handlers with click events
#[derive(Default)]
pub struct ClickEventsHaveKeyEvents;

impl ClickEventsHaveKeyEvents {
    /// Check if an element is natively interactive
    fn is_interactive_element(tag: &str) -> bool {
        matches!(
            tag,
            "a" | "button"
                | "input"
                | "select"
                | "textarea"
                | "details"
                | "summary"
                | "video"
                | "audio"
        )
    }

    /// Check if element has a role that makes it interactive
    fn has_interactive_role(element: &ElementNode) -> bool {
        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == "role" {
                    if let Some(value) = &attr.value {
                        return matches!(
                            value.content.as_ref(),
                            "button"
                                | "link"
                                | "checkbox"
                                | "menuitem"
                                | "menuitemcheckbox"
                                | "menuitemradio"
                                | "option"
                                | "radio"
                                | "searchbox"
                                | "switch"
                                | "textbox"
                                | "tab"
                                | "treeitem"
                                | "gridcell"
                        );
                    }
                }
            }
        }
        false
    }

    /// Check if element has a click handler
    fn has_click_handler(element: &ElementNode) -> bool {
        for prop in &element.props {
            if let PropNode::Directive(dir) = prop {
                if dir.name == "on" {
                    if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                        if arg.content == "click" {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if element has a keyboard event handler
    fn has_keyboard_handler(element: &ElementNode) -> bool {
        for prop in &element.props {
            if let PropNode::Directive(dir) = prop {
                if dir.name == "on" {
                    if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                        if matches!(arg.content.as_ref(), "keydown" | "keyup" | "keypress") {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

impl Rule for ClickEventsHaveKeyEvents {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        // Skip interactive elements - they have native keyboard support
        if Self::is_interactive_element(&element.tag) {
            return;
        }

        // Skip elements with interactive roles - they should have keyboard support
        if Self::has_interactive_role(element) {
            return;
        }

        // Check if has click but no keyboard handler
        if Self::has_click_handler(element) && !Self::has_keyboard_handler(element) {
            ctx.warn_with_help(
                ctx.t("a11y/click-events-have-key-events.message"),
                &element.loc,
                ctx.t("a11y/click-events-have-key-events.help"),
            );
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
        registry.register(Box::new(ClickEventsHaveKeyEvents));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_button() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<button @click="handleClick">Click</button>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_div_with_both() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div @click="handleClick" @keydown="handleKeydown">Click</div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_div_with_role_button() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div role="button" @click="handleClick">Click</div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_div_click_only() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div @click="handleClick">Click</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_span_click_only() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<span @click="toggle">Toggle</span>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
