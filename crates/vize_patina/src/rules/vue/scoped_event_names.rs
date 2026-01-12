//! vue/scoped-event-names
//!
//! Recommend using scoped event names for better organization.
//!
//! When a component emits multiple related events, grouping them
//! with a scope prefix (e.g., `audio:play`, `audio:pause`) improves
//! readability and organization.
//!
//! This rule suggests using the `context:event` format when multiple
//! events share a common context.
//!
//! ## Examples
//!
//! ### Not Recommended
//! ```vue
//! <script setup>
//! defineEmits(['playAudio', 'pauseAudio', 'reloadAudio'])
//! </script>
//! ```
//!
//! ### Recommended
//! ```vue
//! <script setup>
//! defineEmits(['audio:play', 'audio:pause', 'audio:reload'])
//! </script>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{DirectiveNode, ElementNode, ExpressionNode};

static META: RuleMeta = RuleMeta {
    name: "vue/scoped-event-names",
    description: "Recommend scoped event names using context:event format",
    category: RuleCategory::Recommended,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Known context prefixes that suggest grouping
const COMMON_SUFFIXES: &[&str] = &[
    "Audio", "Video", "Form", "Dialog", "Modal", "Menu", "List", "Item", "User", "Data", "File",
    "Image", "Input", "Select", "Button",
];

/// Recommend scoped event names
pub struct ScopedEventNames;

impl ScopedEventNames {
    /// Check if an event name could benefit from scoping
    fn could_benefit_from_scope(event_name: &str) -> Option<&'static str> {
        for suffix in COMMON_SUFFIXES {
            let lower_suffix = suffix.to_lowercase();
            let lower_name = event_name.to_lowercase();

            // Check camelCase pattern (e.g., "playAudio")
            if lower_name.ends_with(&lower_suffix) && lower_name.len() > lower_suffix.len() {
                return Some(suffix);
            }
            // Check kebab-case pattern (e.g., "play-audio")
            if lower_name.ends_with(&format!("-{}", lower_suffix)) {
                return Some(suffix);
            }
        }
        None
    }

    /// Check if event is already scoped
    fn is_scoped(event_name: &str) -> bool {
        event_name.contains(':')
    }
}

impl Rule for ScopedEventNames {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_directive<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        _element: &ElementNode<'a>,
        directive: &DirectiveNode<'a>,
    ) {
        // Only check v-on directives
        if directive.name.as_str() != "on" {
            return;
        }

        // Get the event name from the argument
        let event_name = match &directive.arg {
            Some(ExpressionNode::Simple(s)) => s.content.as_str(),
            _ => return,
        };

        // Skip if already scoped
        if Self::is_scoped(event_name) {
            return;
        }

        // Check if this event could benefit from scoping
        if let Some(context) = Self::could_benefit_from_scope(event_name) {
            let suggested = format!(
                "{}:{}",
                context.to_lowercase(),
                event_name
                    .strip_suffix(context)
                    .or_else(|| event_name.strip_suffix(&context.to_lowercase()))
                    .unwrap_or(event_name)
                    .trim_end_matches('-')
                    .to_lowercase()
            );

            ctx.warn_with_help(
                format!(
                    "Consider using scoped event name for better organization: \"{}\"",
                    event_name
                ),
                &directive.loc,
                format!(
                    "Use \"{}\" format instead (e.g., \"{}\")",
                    "context:event", suggested
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
        registry.register(Box::new(ScopedEventNames));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_scoped_event() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<button @audio:play="handlePlay"></button>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_warn_unscoped_event_with_suffix() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<button @playAudio="handlePlay"></button>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("scoped"));
    }

    #[test]
    fn test_valid_simple_event() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<button @click="handleClick"></button>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_update_event() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<input @update:modelValue="handleUpdate" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }
}
