//! a11y/no-refer-to-non-existent-id
//!
//! Disallow ID reference attributes that point to non-existent IDs in the
//! same template. Based on markuplint's `no-refer-to-non-existent-id` rule.
//!
//! Attributes like `for`, `aria-labelledby`, `aria-describedby`, etc.
//! reference other elements by their `id` attribute. If the referenced ID
//! does not exist in the template, the association is broken.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <label for="nonexistent">Name:</label>
//!   <input id="name-input" />
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <label for="name-input">Name:</label>
//!   <input id="name-input" />
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::{LintDiagnostic, Severity};
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_carton::FxHashSet;
use vize_croquis::analysis::ElementIdKind;
use vize_relief::ast::RootNode;

static META: RuleMeta = RuleMeta {
    name: "a11y/no-refer-to-non-existent-id",
    description: "Disallow references to non-existent IDs",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Pre-collected reference info (to avoid borrow conflicts with LintContext)
struct IdRefCheck {
    /// IDs being referenced (single or multi)
    ids: Vec<String>,
    /// Kind of reference
    kind_str: &'static str,
    /// Location info
    start: u32,
    end: u32,
}

#[derive(Default)]
pub struct NoReferToNonExistentId;

impl Rule for NoReferToNonExistentId {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn run_on_template<'a>(&self, ctx: &mut LintContext<'a>, _root: &RootNode<'a>) {
        // Phase 1: collect data from analysis (immutable borrow of ctx)
        let (defined_ids, ref_checks) = {
            let Some(analysis) = ctx.analysis() else {
                return;
            };

            // Collect all statically defined IDs
            let defined_ids: FxHashSet<String> = analysis
                .element_ids
                .iter()
                .filter(|info| info.kind == ElementIdKind::Id && info.is_static)
                .map(|info| info.value.to_string())
                .collect();

            // Collect all references to check
            let ref_checks: Vec<IdRefCheck> = analysis
                .element_ids
                .iter()
                .filter(|info| !info.kind.is_definition() && info.is_static)
                .map(|info| {
                    let is_multi_id = matches!(info.kind, ElementIdKind::AriaReference);
                    let ids = if is_multi_id {
                        info.value
                            .split_whitespace()
                            .map(|s| s.to_string())
                            .collect()
                    } else {
                        vec![info.value.to_string()]
                    };
                    IdRefCheck {
                        ids,
                        kind_str: info.kind.as_str(),
                        start: info.start,
                        end: info.end,
                    }
                })
                .collect();

            (defined_ids, ref_checks)
        };

        // Phase 2: report diagnostics (mutable borrow of ctx)
        for check in &ref_checks {
            for id in &check.ids {
                if !defined_ids.contains(id.as_str()) {
                    let message = ctx.t_fmt(
                        "a11y/no-refer-to-non-existent-id.message",
                        &[("id", id.as_str()), ("kind", check.kind_str)],
                    );
                    let help = ctx.t("a11y/no-refer-to-non-existent-id.help");
                    let diag = LintDiagnostic::warn(META.name, message, check.start, check.end)
                        .with_help(help.into_owned());
                    ctx.report(diag);
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
        registry.register(Box::new(NoReferToNonExistentId));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_matching_for_id() {
        let linter = create_linter();
        // Without semantic analysis, the rule is a no-op
        let result = linter.lint_template(
            r#"<label for="name">Name:</label><input id="name" />"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_no_references() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>Hello</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_dynamic_reference() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<label :for="inputId">Name:</label>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }
}
