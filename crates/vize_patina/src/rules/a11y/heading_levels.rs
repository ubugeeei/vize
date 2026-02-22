//! a11y/heading-levels
//!
//! Detect heading level skipping (e.g., jumping from `<h1>` to `<h3>`).
//! Skipping heading levels can confuse screen reader users who navigate
//! by headings. Based on markuplint's `heading-levels` rule.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <h1>Title</h1>
//!   <h3>Subsection</h3>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <h1>Title</h1>
//!   <h2>Section</h2>
//!   <h3>Subsection</h3>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use crate::rules::html::helpers::walk_elements;
use vize_relief::ast::RootNode;

static META: RuleMeta = RuleMeta {
    name: "a11y/heading-levels",
    description: "Disallow skipping heading levels",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

#[derive(Default)]
pub struct HeadingLevels;

struct HeadingInfo {
    level: u8,
    start: u32,
    end: u32,
}

fn heading_level(tag: &str) -> Option<u8> {
    match tag {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

impl Rule for HeadingLevels {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn run_on_template<'a>(&self, ctx: &mut LintContext<'a>, root: &RootNode<'a>) {
        let mut headings: Vec<HeadingInfo> = Vec::new();

        walk_elements(&root.children, &mut |element| {
            if let Some(level) = heading_level(element.tag.as_str()) {
                headings.push(HeadingInfo {
                    level,
                    start: element.loc.start.offset,
                    end: element.loc.end.offset,
                });
            }
        });

        // Sort by document order (source offset)
        headings.sort_by_key(|h| h.start);

        let mut prev_level: u8 = 0;
        for heading in &headings {
            if prev_level > 0 && heading.level > prev_level + 1 {
                let message = ctx.t_fmt(
                    "a11y/heading-levels.message",
                    &[
                        ("from", &format!("h{prev_level}")),
                        ("to", &format!("h{}", heading.level)),
                    ],
                );
                let help = ctx.t("a11y/heading-levels.help");
                let diag = crate::diagnostic::LintDiagnostic::warn(
                    META.name,
                    message,
                    heading.start,
                    heading.end,
                )
                .with_help(help.into_owned());
                ctx.report(diag);
            }
            prev_level = heading.level;
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
        registry.register(Box::new(HeadingLevels));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_sequential() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<h1>Title</h1><h2>Section</h2><h3>Sub</h3>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_same_level() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<h2>Section A</h2><h2>Section B</h2>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_decrease() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<h1>Title</h1><h2>Section</h2><h3>Sub</h3><h2>Back</h2>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_single_heading() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<h3>Only heading</h3>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_no_headings() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>content</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_skip_h1_to_h3() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<h1>Title</h1><h3>Subsection</h3>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_skip_h2_to_h4() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<h1>T</h1><h2>S</h2><h4>Sub</h4>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_skip_h1_to_h4() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<h1>Title</h1><h4>Deep</h4>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_multiple_skips() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<h1>T</h1><h3>S</h3><h6>D</h6>"#, "test.vue");
        // h1→h3 skip, h3→h6 skip
        assert_eq!(result.warning_count, 2);
    }
}
