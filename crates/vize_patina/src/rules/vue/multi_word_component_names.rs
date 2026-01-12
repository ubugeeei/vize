//! vue/multi-word-component-names
//!
//! Require component names to be multi-word.
//!
//! This rule enforces that component names are always multi-word,
//! except for root `App` components. This prevents conflicts with
//! existing and future HTML elements, since all HTML elements are
//! a single word.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <Item />
//! <Table />
//! ```
//!
//! ### Valid
//! ```vue
//! <TodoItem />
//! <DataTable />
//! <AppHeader />
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::ElementNode;

static META: RuleMeta = RuleMeta {
    name: "vue/multi-word-component-names",
    description: "Require component names to be multi-word",
    category: RuleCategory::Essential,
    fixable: false,
    default_severity: Severity::Error,
};

/// Require component names to be multi-word
pub struct MultiWordComponentNames {
    /// Component names to ignore (e.g., "App", "Nuxt")
    pub ignore: Vec<&'static str>,
}

impl Default for MultiWordComponentNames {
    fn default() -> Self {
        Self {
            ignore: vec!["App", "Nuxt", "NuxtPage", "NuxtLayout"],
        }
    }
}

impl MultiWordComponentNames {
    fn is_custom_component(tag: &str) -> bool {
        // PascalCase components (e.g., MyComponent)
        if tag
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            return true;
        }
        // kebab-case components with hyphen (e.g., my-component)
        if tag.contains('-') && !tag.starts_with("v-") {
            return true;
        }
        false
    }

    fn is_multi_word(tag: &str) -> bool {
        // kebab-case: check for hyphen
        if tag.contains('-') {
            return true;
        }
        // PascalCase: count uppercase letters
        let uppercase_count = tag.chars().filter(|c| c.is_uppercase()).count();
        uppercase_count >= 2
    }
}

impl Rule for MultiWordComponentNames {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        let tag = element.tag.as_str();

        // Skip non-component elements
        if !Self::is_custom_component(tag) {
            return;
        }

        // Skip ignored components
        if self.ignore.contains(&tag) {
            return;
        }

        // Check if the component name is multi-word
        if !Self::is_multi_word(tag) {
            ctx.error_with_help(
                format!(
                    "Component name \"{}\" should be multi-word to avoid conflicts with HTML elements",
                    tag
                ),
                &element.loc,
                "Rename the component to use multiple words (e.g., \"TodoItem\" instead of \"Item\")",
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
        registry.register(Box::new(MultiWordComponentNames::default()));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_multi_word_pascal_case() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<TodoItem />"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_multi_word_kebab_case() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<todo-item></todo-item>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_ignored_app() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<App />"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_single_word_pascal_case() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<Item />"#, "test.vue");
        assert_eq!(result.error_count, 1);
        assert!(result.diagnostics[0].message.contains("multi-word"));
    }

    #[test]
    fn test_html_elements_ignored() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div><span>text</span></div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_three_words() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<UserProfileCard />"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }
}
