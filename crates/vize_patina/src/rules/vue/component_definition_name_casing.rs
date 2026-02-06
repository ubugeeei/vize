//! vue/component-definition-name-casing
//!
//! Enforce specific casing for the component definition name.
//!
//! Component file names should use PascalCase (default).
//! This checks the filename of .vue files.
//!
//! ## Examples
//!
//! ### Invalid
//! ```text
//! my-component.vue     -> should be MyComponent.vue
//! myComponent.vue      -> should be MyComponent.vue
//! ```
//!
//! ### Valid
//! ```text
//! MyComponent.vue
//! index.vue
//! App.vue
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::RootNode;

static META: RuleMeta = RuleMeta {
    name: "vue/component-definition-name-casing",
    description: "Enforce PascalCase for component definition names",
    category: RuleCategory::StronglyRecommended,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Enforce PascalCase component definition names
#[derive(Default)]
pub struct ComponentDefinitionNameCasing;

fn is_pascal_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap();
    if !first.is_ascii_uppercase() {
        return false;
    }
    // Must not contain hyphens
    if s.contains('-') {
        return false;
    }
    // Must not be all uppercase (SCREAMING_CASE)
    if s.chars()
        .all(|c| c.is_ascii_uppercase() || !c.is_alphabetic())
    {
        return false;
    }
    true
}

/// Common exception filenames that don't need PascalCase
const EXCEPTION_NAMES: &[&str] = &["App"];

impl Rule for ComponentDefinitionNameCasing {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn run_on_template<'a>(&self, ctx: &mut LintContext<'a>, _root: &RootNode<'a>) {
        let filename = ctx.filename;
        if !filename.ends_with(".vue") {
            return;
        }

        // Extract the stem name (without extension and path)
        let stem = filename
            .rsplit('/')
            .next()
            .unwrap_or(filename)
            .rsplit('\\')
            .next()
            .unwrap_or(filename)
            .trim_end_matches(".vue");

        // Skip exception names
        if EXCEPTION_NAMES.contains(&stem) {
            return;
        }

        // Skip names starting with [ (dynamic routes)
        if stem.starts_with('[') {
            return;
        }

        // Skip single-word lowercase names (index, test, app, main, etc.)
        if stem.chars().all(|c| c.is_ascii_lowercase()) {
            return;
        }

        if !is_pascal_case(stem) {
            ctx.warn_with_help(
                ctx.t_fmt(
                    "vue/component-definition-name-casing.message",
                    &[("name", stem)],
                ),
                &_root.loc,
                ctx.t("vue/component-definition-name-casing.help"),
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
        registry.register(Box::new(ComponentDefinitionNameCasing));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_pascal_case() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>Content</div>"#, "MyComponent.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_index() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>Content</div>"#, "index.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_app() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>Content</div>"#, "App.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_kebab_case() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>Content</div>"#, "my-component.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_camel_case() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>Content</div>"#, "myComponent.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_with_path() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<div>Content</div>"#, "src/components/MyComponent.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_non_vue_file() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>Content</div>"#, "test.html");
        assert_eq!(result.warning_count, 0);
    }
}
