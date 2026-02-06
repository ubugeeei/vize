//! a11y/no-static-element-interactions
//!
//! Disallow event handlers on non-interactive/static elements.
//!
//! Static elements like `<div>`, `<span>` should not have event handlers
//! without also having an interactive role and tabindex.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <div @click="handleClick">Click</div>
//! ```
//!
//! ### Valid
//! ```vue
//! <button @click="handleClick">Click</button>
//! <div role="button" tabindex="0" @click="handleClick">Click</div>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, ExpressionNode, PropNode};

use super::helpers::{has_interactive_role, is_interactive_element};

static META: RuleMeta = RuleMeta {
    name: "a11y/no-static-element-interactions",
    description: "Disallow event handlers on static elements",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Disallow event handlers on static elements
#[derive(Default)]
pub struct NoStaticElementInteractions;

const INTERACTIVE_EVENTS: &[&str] = &[
    "click",
    "dblclick",
    "mousedown",
    "mouseup",
    "keydown",
    "keyup",
    "keypress",
];

fn has_interactive_event(element: &ElementNode) -> bool {
    for prop in &element.props {
        if let PropNode::Directive(dir) = prop {
            if dir.name == "on" {
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    if INTERACTIVE_EVENTS.contains(&arg.content.as_ref()) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

impl Rule for NoStaticElementInteractions {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        // Skip natively interactive elements
        if is_interactive_element(&element.tag) {
            return;
        }

        // Skip elements with interactive roles
        if has_interactive_role(element) {
            return;
        }

        // Check if element has interactive event handlers
        if has_interactive_event(element) {
            ctx.warn_with_help(
                ctx.t_fmt(
                    "a11y/no-static-element-interactions.message",
                    &[("tag", element.tag.as_str())],
                ),
                &element.loc,
                ctx.t("a11y/no-static-element-interactions.help"),
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
        registry.register(Box::new(NoStaticElementInteractions));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_button_with_click() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<button @click="handle">Click</button>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_div_with_role() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div role="button" @click="handle">Click</div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_div_no_events() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>Content</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_div_with_click() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div @click="handle">Click</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_span_with_keydown() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<span @keydown="handle">Content</span>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_component_skipped() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<MyDiv @click="handle">Click</MyDiv>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }
}
