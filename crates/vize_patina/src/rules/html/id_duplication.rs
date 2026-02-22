//! html/id-duplication
//!
//! Detect duplicate static `id` attribute values in the same template.
//! Based on markuplint's `id-duplication` rule.
//!
//! This is different from `vue/use-unique-element-ids` which warns about
//! any static ID. This rule only warns when the same ID literal appears
//! more than once.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <div id="content">first</div>
//!   <div id="content">second</div>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <div id="content">first</div>
//!   <div id="sidebar">second</div>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::{LintDiagnostic, Severity};
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode, RootNode, SourceLocation, TemplateChildNode};

static META: RuleMeta = RuleMeta {
    name: "html/id-duplication",
    description: "Disallow duplicate element IDs",
    category: RuleCategory::HtmlConformance,
    fixable: false,
    default_severity: Severity::Error,
};

#[derive(Default)]
pub struct IdDuplication;

struct IdEntry {
    value: String,
    loc: LocInfo,
}

#[derive(Clone)]
struct LocInfo {
    start: u32,
    end: u32,
}

impl Rule for IdDuplication {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn run_on_template<'a>(&self, ctx: &mut LintContext<'a>, root: &RootNode<'a>) {
        let mut ids: Vec<IdEntry> = Vec::new();

        collect_static_ids(&root.children, &mut ids);

        // Find duplicates
        let mut seen: std::collections::HashMap<&str, &LocInfo> = std::collections::HashMap::new();

        for entry in &ids {
            if let Some(first_loc) = seen.get(entry.value.as_str()) {
                let message = ctx.t_fmt(
                    "html/id-duplication.message",
                    &[("id", entry.value.as_str())],
                );
                let help = ctx.t("html/id-duplication.help");
                let diag =
                    LintDiagnostic::error(META.name, message, entry.loc.start, entry.loc.end)
                        .with_help(help.into_owned())
                        .with_label(
                            "first defined here".to_string(),
                            first_loc.start,
                            first_loc.end,
                        );
                ctx.report(diag);
            } else {
                seen.insert(&entry.value, &entry.loc);
            }
        }
    }
}

fn collect_static_ids<'a>(children: &[TemplateChildNode<'a>], ids: &mut Vec<IdEntry>) {
    for child in children {
        match child {
            TemplateChildNode::Element(el) => {
                collect_element_id(el, ids);
                collect_static_ids(&el.children, ids);
            }
            TemplateChildNode::If(if_node) => {
                for branch in if_node.branches.iter() {
                    collect_static_ids(&branch.children, ids);
                }
            }
            TemplateChildNode::For(for_node) => {
                collect_static_ids(&for_node.children, ids);
            }
            _ => {}
        }
    }
}

fn collect_element_id(element: &ElementNode, ids: &mut Vec<IdEntry>) {
    for prop in &element.props {
        if let PropNode::Attribute(attr) = prop {
            if attr.name == "id" {
                if let Some(value) = &attr.value {
                    ids.push(IdEntry {
                        value: value.content.to_string(),
                        loc: loc_info(&attr.loc),
                    });
                }
            }
        }
    }
}

fn loc_info(loc: &SourceLocation) -> LocInfo {
    LocInfo {
        start: loc.start.offset,
        end: loc.end.offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;

    fn create_linter() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(IdDuplication));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_unique_ids() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div id="a">A</div><div id="b">B</div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_no_ids() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>A</div><div>B</div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_dynamic_ids() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div :id="id1">A</div><div :id="id2">B</div>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_duplicate_ids() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div id="content">A</div><div id="content">B</div>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_triple_duplicate() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div id="x">A</div><div id="x">B</div><div id="x">C</div>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 2);
    }

    #[test]
    fn test_invalid_nested_duplicate() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div id="foo"><span id="foo">text</span></div>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 1);
    }
}
