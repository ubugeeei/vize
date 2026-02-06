//! vue/v-slot-style
//!
//! Enforce `v-slot` directive style.
//!
//! ## Options
//!
//! - `"shorthand"` (default): Prefer `#name` over `v-slot:name`
//! - `"longform"`: Prefer `v-slot:name` over `#name`
//!
//! ## Examples
//!
//! ### Invalid (with shorthand option)
//! ```vue
//! <template v-slot:header>...</template>
//! ```
//!
//! ### Valid (with shorthand option)
//! ```vue
//! <template #header>...</template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{DirectiveNode, ElementNode};

static META: RuleMeta = RuleMeta {
    name: "vue/v-slot-style",
    description: "Enforce `v-slot` directive style",
    category: RuleCategory::StronglyRecommended,
    fixable: true,
    default_severity: Severity::Warning,
};

/// Style preference for v-slot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VSlotStyleOption {
    #[default]
    Shorthand,
    Longform,
}

/// Enforce v-slot directive style
pub struct VSlotStyle {
    pub style: VSlotStyleOption,
}

impl Default for VSlotStyle {
    fn default() -> Self {
        Self {
            style: VSlotStyleOption::Shorthand,
        }
    }
}

impl Rule for VSlotStyle {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_directive<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        _element: &ElementNode<'a>,
        directive: &DirectiveNode<'a>,
    ) {
        if directive.name.as_str() != "slot" {
            return;
        }

        let raw_name = directive.raw_name.as_deref().unwrap_or("");
        let is_shorthand = raw_name.starts_with('#');

        match self.style {
            VSlotStyleOption::Shorthand => {
                if !is_shorthand && raw_name.starts_with("v-slot") {
                    ctx.warn_with_help(
                        ctx.t("vue/v-slot-style.message_shorthand"),
                        &directive.loc,
                        ctx.t("vue/v-slot-style.help"),
                    );
                }
            }
            VSlotStyleOption::Longform => {
                if is_shorthand {
                    ctx.warn_with_help(
                        ctx.t("vue/v-slot-style.message_longform"),
                        &directive.loc,
                        ctx.t("vue/v-slot-style.help"),
                    );
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
        registry.register(Box::new(VSlotStyle::default()));
        Linter::with_registry(registry)
    }

    fn create_linter_longform() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(VSlotStyle {
            style: VSlotStyleOption::Longform,
        }));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_shorthand() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<MyComponent><template #header>Header</template></MyComponent>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_longform_when_shorthand_preferred() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<MyComponent><template v-slot:header>Header</template></MyComponent>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_longform_when_longform_preferred() {
        let linter = create_linter_longform();
        let result = linter.lint_template(
            r#"<MyComponent><template v-slot:header>Header</template></MyComponent>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_shorthand_when_longform_preferred() {
        let linter = create_linter_longform();
        let result = linter.lint_template(
            r#"<MyComponent><template #header>Header</template></MyComponent>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }
}
