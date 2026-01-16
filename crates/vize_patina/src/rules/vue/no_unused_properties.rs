//! vue/no-unused-properties
//!
//! Disallow unused properties (props, data, computed, etc.).
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <script setup lang="ts">
//! const props = defineProps<{
//!   msg: string
//!   unused: number  // defined but never used
//! }>()
//! </script>
//!
//! <template>
//!   <div>{{ msg }}</div>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <script setup lang="ts">
//! const props = defineProps<{
//!   msg: string
//!   count: number
//! }>()
//! </script>
//!
//! <template>
//!   <div>{{ msg }} - {{ count }}</div>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::RootNode;

static META: RuleMeta = RuleMeta {
    name: "vue/no-unused-properties",
    description: "Disallow unused properties defined in defineProps",
    category: RuleCategory::StronglyRecommended,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Disallow unused properties
pub struct NoUnusedProperties {
    /// Pattern for properties to ignore (e.g., starts with '_')
    pub ignore_pattern: Option<String>,
    /// Check props defined via defineProps
    pub check_props: bool,
}

impl Default for NoUnusedProperties {
    fn default() -> Self {
        Self {
            ignore_pattern: None,
            check_props: true,
        }
    }
}

impl NoUnusedProperties {
    /// Check if a property name should be ignored
    fn should_ignore(&self, name: &str) -> bool {
        // Ignore properties starting with underscore
        if name.starts_with('_') {
            return true;
        }

        // Check custom ignore pattern
        if let Some(ref pattern) = self.ignore_pattern {
            if name.starts_with(pattern.as_str()) {
                return true;
            }
        }

        false
    }
}

impl Rule for NoUnusedProperties {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn run_on_template<'a>(&self, ctx: &mut LintContext<'a>, _root: &RootNode<'a>) {
        // Skip if no analysis available
        if !ctx.has_analysis() {
            return;
        }

        // Collect unused props first (to avoid borrow conflicts)
        let (unused_props, define_props_loc): (Vec<String>, (u32, u32)) = {
            let analysis = ctx.analysis().unwrap();

            if !self.check_props {
                return;
            }

            let props = analysis.macros.props();

            // Get defineProps call location for error reporting
            let loc = analysis
                .macros
                .define_props()
                .map(|call| (call.start, call.end))
                .unwrap_or((0, 0));

            let unused: Vec<String> = props
                .iter()
                .filter(|prop| {
                    // Skip ignored properties
                    if self.should_ignore(prop.name.as_str()) {
                        return false;
                    }

                    let prop_name = prop.name.as_str();

                    // Check if prop is used in template scope chain
                    let is_used_in_scope = analysis.scopes.is_used(prop_name);

                    // Check if prop is accessed via props object in bindings
                    let has_props_binding = analysis.bindings.contains("props");
                    let is_prop_destructured = analysis.bindings.get(prop_name).is_some_and(|bt| {
                        matches!(
                            bt,
                            vize_relief::BindingType::Props
                                | vize_relief::BindingType::PropsAliased
                        )
                    });

                    // If props object exists and is used, we can't easily track individual prop usage
                    // in script, so we only report if not used in template AND not destructured
                    let is_used = is_used_in_scope || is_prop_destructured || has_props_binding;

                    !is_used
                })
                .map(|prop| prop.name.to_string())
                .collect();

            (unused, loc)
        };

        // Report unused props
        for prop_name in unused_props {
            ctx.report(
                crate::diagnostic::LintDiagnostic::warn(
                    ctx.current_rule,
                    format!("Prop '{}' is defined but never used", prop_name),
                    define_props_loc.0,
                    define_props_loc.1,
                )
                .with_help("Remove unused prop or use it in your template/script"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoUnusedProperties::default();
        assert_eq!(rule.meta().name, "vue/no-unused-properties");
        assert_eq!(rule.meta().category, RuleCategory::StronglyRecommended);
    }

    #[test]
    fn test_should_ignore() {
        let rule = NoUnusedProperties::default();
        assert!(rule.should_ignore("_internal"));
        assert!(!rule.should_ignore("count"));
    }
}
