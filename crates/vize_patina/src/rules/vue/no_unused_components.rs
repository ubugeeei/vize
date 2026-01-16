//! vue/no-unused-components
//!
//! Disallow registering components that are not used inside templates.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <script setup>
//! import MyButton from './MyButton.vue'  // imported but never used
//! </script>
//!
//! <template>
//!   <div>Hello</div>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <script setup>
//! import MyButton from './MyButton.vue'
//! </script>
//!
//! <template>
//!   <MyButton>Click me</MyButton>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_croquis::naming::is_pascal_case;
use vize_relief::ast::RootNode;
use vize_relief::BindingType;

static META: RuleMeta = RuleMeta {
    name: "vue/no-unused-components",
    description: "Disallow registering components that are not used inside templates",
    category: RuleCategory::Essential,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Disallow unused components
#[derive(Default)]
pub struct NoUnusedComponents {
    /// Pattern for components to ignore (e.g., starts with '_')
    pub ignore_pattern: Option<String>,
}

impl NoUnusedComponents {
    /// Check if a component name should be ignored
    fn should_ignore(&self, name: &str) -> bool {
        // Ignore components starting with underscore
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

    /// Check if a binding type indicates a component
    fn is_component_binding(binding_type: &BindingType) -> bool {
        matches!(
            binding_type,
            BindingType::SetupConst | BindingType::ExternalModule
        )
    }
}

impl Rule for NoUnusedComponents {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn run_on_template<'a>(&self, ctx: &mut LintContext<'a>, _root: &RootNode<'a>) {
        // Skip if no analysis available
        if !ctx.has_analysis() {
            return;
        }

        // Collect unused components first (to avoid borrow conflicts)
        let unused_components: Vec<String> = {
            let analysis = ctx.analysis().unwrap();

            // Collect registered components (PascalCase bindings that could be components)
            let registered_components: Vec<_> = analysis
                .bindings
                .iter()
                .filter(|(name, binding_type)| {
                    // Must be a component-like binding type
                    Self::is_component_binding(binding_type)
                        // Must be PascalCase (component naming convention)
                        && is_pascal_case(name)
                        // Not ignored
                        && !self.should_ignore(name)
                })
                .collect();

            // Find unused components
            registered_components
                .into_iter()
                .filter(|(name, _)| {
                    // Check if used in template (case-insensitive matching for kebab-case)
                    !analysis.used_components.iter().any(|used| {
                        // Exact match
                        used.as_str() == *name
                            // kebab-case match (MyComponent -> my-component)
                            || vize_croquis::naming::names_match(used.as_str(), name)
                    })
                })
                .map(|(name, _)| name.to_string())
                .collect()
        };

        // Report unused components
        for name in unused_components {
            ctx.report(
                crate::diagnostic::LintDiagnostic::warn(
                    ctx.current_rule,
                    format!(
                        "Component '{}' is registered but never used in template",
                        name
                    ),
                    0,
                    name.len() as u32,
                )
                .with_help("Remove the unused import or use the component in your template"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta() {
        let rule = NoUnusedComponents::default();
        assert_eq!(rule.meta().name, "vue/no-unused-components");
        assert_eq!(rule.meta().category, RuleCategory::Essential);
    }

    #[test]
    fn test_should_ignore() {
        let rule = NoUnusedComponents::default();
        assert!(rule.should_ignore("_Internal"));
        assert!(!rule.should_ignore("MyComponent"));
    }
}
