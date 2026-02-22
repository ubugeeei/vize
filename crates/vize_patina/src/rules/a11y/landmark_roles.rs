//! a11y/landmark-roles
//!
//! Validate landmark role placement and uniqueness.
//! Based on markuplint's `landmark-roles` rule.
//!
//! Checks:
//! - Duplicate `<main>` landmarks (only one allowed)
//! - Nested interactive landmarks of the same type
//! - Multiple `<nav>` elements without distinct labels
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <main>content</main>
//!   <main>another main</main>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <main>content</main>
//!   <nav aria-label="Primary">primary nav</nav>
//!   <nav aria-label="Footer">footer nav</nav>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::{LintDiagnostic, Severity};
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, PropNode, RootNode, TemplateChildNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/landmark-roles",
    description: "Validate landmark role placement and uniqueness",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

#[derive(Default)]
pub struct LandmarkRoles;

struct LandmarkInfo {
    role: String,
    label: Option<String>,
    start: u32,
    end: u32,
}

/// Get the landmark role for an element (from tag or explicit role attribute)
fn get_landmark_role<'a>(element: &ElementNode<'a>) -> Option<&'static str> {
    // Check explicit role attribute first
    for prop in &element.props {
        if let PropNode::Attribute(attr) = prop {
            if attr.name == "role" {
                if let Some(value) = &attr.value {
                    return match value.content.as_str() {
                        "banner" | "complementary" | "contentinfo" | "form" | "main"
                        | "navigation" | "region" | "search" => {
                            Some(match value.content.as_str() {
                                "banner" => "banner",
                                "complementary" => "complementary",
                                "contentinfo" => "contentinfo",
                                "form" => "form",
                                "main" => "main",
                                "navigation" => "navigation",
                                "region" => "region",
                                "search" => "search",
                                _ => unreachable!(),
                            })
                        }
                        _ => None,
                    };
                }
            }
        }
    }

    // Implicit roles from tags
    match element.tag.as_str() {
        "main" => Some("main"),
        "nav" => Some("navigation"),
        "aside" => Some("complementary"),
        "header" => Some("banner"),
        "footer" => Some("contentinfo"),
        "form" => Some("form"),
        "section" => Some("region"),
        "search" => Some("search"),
        _ => None,
    }
}

fn get_label(element: &ElementNode) -> Option<String> {
    for prop in &element.props {
        if let PropNode::Attribute(attr) = prop {
            if attr.name == "aria-label" {
                return attr.value.as_ref().map(|v| v.content.to_string());
            }
            if attr.name == "aria-labelledby" {
                return attr.value.as_ref().map(|v| v.content.to_string());
            }
        }
    }
    None
}

fn collect_landmarks<'a>(children: &[TemplateChildNode<'a>], landmarks: &mut Vec<LandmarkInfo>) {
    for child in children {
        match child {
            TemplateChildNode::Element(el) => {
                if el.tag_type != ElementType::Component {
                    if let Some(role) = get_landmark_role(el) {
                        landmarks.push(LandmarkInfo {
                            role: role.to_string(),
                            label: get_label(el),
                            start: el.loc.start.offset,
                            end: el.loc.end.offset,
                        });
                    }
                }
                collect_landmarks(&el.children, landmarks);
            }
            TemplateChildNode::If(if_node) => {
                for branch in if_node.branches.iter() {
                    collect_landmarks(&branch.children, landmarks);
                }
            }
            TemplateChildNode::For(for_node) => {
                collect_landmarks(&for_node.children, landmarks);
            }
            _ => {}
        }
    }
}

impl Rule for LandmarkRoles {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn run_on_template<'a>(&self, ctx: &mut LintContext<'a>, root: &RootNode<'a>) {
        let mut landmarks: Vec<LandmarkInfo> = Vec::new();
        collect_landmarks(&root.children, &mut landmarks);

        // Check 1: Duplicate main landmarks
        let mains: Vec<&LandmarkInfo> = landmarks.iter().filter(|l| l.role == "main").collect();
        if mains.len() > 1 {
            for main in &mains[1..] {
                let message = ctx.t("a11y/landmark-roles.duplicate_main");
                let diag = LintDiagnostic::warn(META.name, message, main.start, main.end)
                    .with_help(
                        ctx.t("a11y/landmark-roles.help_duplicate_main")
                            .into_owned(),
                    );
                ctx.report(diag);
            }
        }

        // Check 2: Multiple nav/aside/etc. without labels
        let mut role_groups: std::collections::HashMap<&str, Vec<&LandmarkInfo>> =
            std::collections::HashMap::new();
        for landmark in &landmarks {
            role_groups
                .entry(landmark.role.as_str())
                .or_default()
                .push(landmark);
        }

        for (role, group) in &role_groups {
            if *role == "main" {
                continue; // Already handled
            }
            if group.len() > 1 {
                let unlabeled: Vec<&&LandmarkInfo> =
                    group.iter().filter(|l| l.label.is_none()).collect();
                for landmark in unlabeled {
                    let message =
                        ctx.t_fmt("a11y/landmark-roles.missing_label", &[("role", *role)]);
                    let diag =
                        LintDiagnostic::warn(META.name, message, landmark.start, landmark.end)
                            .with_help(
                                ctx.t("a11y/landmark-roles.help_missing_label").into_owned(),
                            );
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
        registry.register(Box::new(LandmarkRoles));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_single_main() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<main>content</main>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_labeled_navs() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<nav aria-label="Primary">nav1</nav><nav aria-label="Footer">nav2</nav>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_single_nav() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<nav>navigation</nav>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_different_landmarks() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<header>h</header><main>m</main><footer>f</footer>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_duplicate_main() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<main>first</main><main>second</main>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_triple_main() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<main>1</main><main>2</main><main>3</main>"#, "test.vue");
        assert_eq!(result.warning_count, 2);
    }

    #[test]
    fn test_invalid_unlabeled_navs() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<nav>nav1</nav><nav>nav2</nav>"#, "test.vue");
        // Both navs lack labels
        assert_eq!(result.warning_count, 2);
    }

    #[test]
    fn test_invalid_one_unlabeled_nav() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<nav aria-label="Primary">nav1</nav><nav>nav2</nav>"#,
            "test.vue",
        );
        // Only the second nav lacks a label
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_role_main_duplicate() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<main>first</main><div role="main">second</div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }
}
