//! vue/warn-custom-directive
//!
//! Warn about custom directives usage.
//!
//! Custom directives (directives other than built-in Vue directives) should
//! be properly registered and documented. This rule warns about their usage
//! to ensure proper registration.
//!
//! ## Built-in Directives
//!
//! - `v-if`, `v-else`, `v-else-if`, `v-show`
//! - `v-for`, `v-on` (`@`), `v-bind` (`:`)
//! - `v-model`, `v-slot` (`#`)
//! - `v-pre`, `v-once`, `v-memo`, `v-cloak`
//! - `v-html`, `v-text`
//!
//! ## Examples
//!
//! ### Triggers Warning
//! ```vue
//! <div v-focus></div>
//! <input v-mask="'###-####'" />
//! <div v-click-outside="handleClose"></div>
//! ```
//!
//! ### Valid (Built-in)
//! ```vue
//! <div v-if="show"></div>
//! <input v-model="value" />
//! <button @click="onClick">Click</button>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{DirectiveNode, ElementNode};

static META: RuleMeta = RuleMeta {
    name: "vue/warn-custom-directive",
    description: "Warn about custom directives that need registration",
    category: RuleCategory::Recommended,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Built-in Vue directives
const BUILTIN_DIRECTIVES: &[&str] = &[
    "if", "else", "else-if", "show", "for", "on", "bind", "model", "slot", "pre", "once", "memo",
    "cloak", "html", "text", "is",
];

/// Warn about custom directives
#[derive(Default)]
pub struct WarnCustomDirective;

impl Rule for WarnCustomDirective {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_directive<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        _element: &ElementNode<'a>,
        directive: &DirectiveNode<'a>,
    ) {
        let name = directive.name.as_str();

        // Check if this is a custom directive (not built-in)
        if !BUILTIN_DIRECTIVES.contains(&name) {
            ctx.warn_with_help(
                format!(
                    "Custom directive 'v-{}' detected. Ensure it is properly registered.",
                    name
                ),
                &directive.loc,
                format!(
                    "Register the directive globally with `app.directive('{}', {{ ... }})` \
                     or locally in the component's `directives` option",
                    name
                ),
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
        registry.register(Box::new(WarnCustomDirective));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_builtin_v_if() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div v-if="show"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_builtin_v_model() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input v-model="value" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_warns_custom_directive() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div v-focus></div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("v-focus"));
    }

    #[test]
    fn test_warns_custom_directive_with_arg() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div v-click-outside="handler"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
