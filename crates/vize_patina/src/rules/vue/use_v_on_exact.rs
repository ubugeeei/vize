//! vue/use-v-on-exact
//!
//! Enforce usage of `.exact` modifier on v-on when there are other
//! v-on handlers with key modifiers on the same element.
//!
//! When an element has both `@click` and `@click.ctrl`, the unmodified
//! `@click` will fire even when Ctrl is pressed. Adding `.exact` to the
//! unmodified handler prevents this overlap.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <button @click="handleClick" @click.ctrl="handleCtrlClick">Click</button>
//! ```
//!
//! ### Valid
//! ```vue
//! <button @click.exact="handleClick" @click.ctrl="handleCtrlClick">Click</button>
//! <button @click="handleClick">Click</button>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ExpressionNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "vue/use-v-on-exact",
    description: "Enforce `.exact` modifier on `v-on` when there are modifier-based handlers",
    category: RuleCategory::Essential,
    fixable: false,
    default_severity: Severity::Warning,
};

#[derive(Default)]
pub struct UseVOnExact;

/// System key modifiers that create overlapping handlers
const SYSTEM_MODIFIERS: &[&str] = &["ctrl", "shift", "alt", "meta"];

impl Rule for UseVOnExact {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        // Collect info about event handlers: (event_name, has_system_modifier, has_exact, prop_index)
        let mut handlers: Vec<(&str, bool, bool, usize)> = Vec::new();

        for (idx, prop) in element.props.iter().enumerate() {
            if let PropNode::Directive(dir) = prop {
                if dir.name != "on" {
                    continue;
                }

                let event_name = match &dir.arg {
                    Some(ExpressionNode::Simple(arg)) => arg.content.as_str(),
                    _ => continue,
                };

                let has_system_modifier = dir
                    .modifiers
                    .iter()
                    .any(|m| SYSTEM_MODIFIERS.contains(&m.content.as_str()));

                let has_exact_modifier =
                    dir.modifiers.iter().any(|m| m.content.as_str() == "exact");

                handlers.push((event_name, has_system_modifier, has_exact_modifier, idx));
            }
        }

        // For each unmodified handler, check if a sibling with the same event has a system modifier
        for &(event_name, has_system_modifier, has_exact, idx) in &handlers {
            if has_system_modifier || has_exact {
                continue;
            }

            let has_modified_sibling = handlers
                .iter()
                .any(|&(name, sys_mod, _, i)| name == event_name && sys_mod && i != idx);

            if has_modified_sibling {
                let loc = &element.props[idx].loc();
                ctx.warn_with_help(
                    ctx.t_fmt("vue/use-v-on-exact.message", &[("event", event_name)]),
                    loc,
                    ctx.t("vue/use-v-on-exact.help"),
                );
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
        registry.register(Box::new(UseVOnExact));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_single_click() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<button @click="handleClick">Click</button>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_exact() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<button @click.exact="handleClick" @click.ctrl="handleCtrlClick">Click</button>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_missing_exact() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<button @click="handleClick" @click.ctrl="handleCtrlClick">Click</button>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_different_events() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<button @click="handleClick" @keydown.ctrl="handleCtrlKey">Click</button>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_multiple_modifiers() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<button @click="a" @click.ctrl="b" @click.shift="c">Click</button>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }
}
