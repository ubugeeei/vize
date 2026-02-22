//! a11y/no-i-for-icon
//!
//! Warn against using `<i>` element with icon framework CSS classes.
//!
//! The `<i>` element represents italic text in HTML semantics.
//! Using it for icons (e.g., Font Awesome, Material Icons) is a misuse
//! of HTML semantics and creates accessibility issues.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <i class="fas fa-home"></i>
//!   <i class="material-icons">home</i>
//!   <i class="bi bi-heart"></i>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <span class="fas fa-home" aria-hidden="true"></span>
//!   <i>italic text</i>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::ElementNode;

use super::helpers::get_static_attribute_value;

static META: RuleMeta = RuleMeta {
    name: "a11y/no-i-for-icon",
    description: "Disallow using <i> element for icons",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

#[derive(Default)]
pub struct NoIForIcon;

/// Check if a CSS class token indicates an icon framework usage
fn is_icon_class(class: &str) -> bool {
    // Font Awesome: fa, fas, far, fab, fal, fad, fa-*
    if class == "fa"
        || class == "fas"
        || class == "far"
        || class == "fab"
        || class == "fal"
        || class == "fad"
    {
        return true;
    }
    if class.starts_with("fa-") {
        return true;
    }

    // Material Icons: material-icons, material-symbols-*
    if class == "material-icons"
        || class.starts_with("material-symbols-")
        || class.starts_with("material-icons-")
    {
        return true;
    }

    // Bootstrap Icons: bi, bi-*
    if class == "bi" || class.starts_with("bi-") {
        return true;
    }

    // Iconify
    if class == "iconify" {
        return true;
    }

    // Generic: class name contains "icon"
    if class.contains("icon") {
        return true;
    }

    false
}

impl Rule for NoIForIcon {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag != "i" {
            return;
        }

        // Only check static class attribute; dynamic :class is not analyzable
        let Some(class_value) = get_static_attribute_value(element, "class") else {
            return;
        };

        let has_icon_class = class_value.split_whitespace().any(is_icon_class);
        if !has_icon_class {
            return;
        }

        let message = ctx.t("a11y/no-i-for-icon.message");
        let help = ctx.t("a11y/no-i-for-icon.help");
        ctx.warn_with_help(message, &element.loc, help);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;

    fn create_linter() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(NoIForIcon));
        Linter::with_registry(registry)
    }

    // ===== Valid cases =====

    #[test]
    fn test_valid_i_with_no_class() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<i>italic text</i>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_i_with_non_icon_class() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<i class="emphasis">text</i>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_span_with_icon_class() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<span class="fas fa-home" aria-hidden="true"></span>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_i_with_dynamic_class() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<i :class="iconClass"></i>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    // ===== Invalid cases =====

    #[test]
    fn test_invalid_font_awesome_fas() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<i class="fas fa-home"></i>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_font_awesome_fa() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<i class="fa fa-star"></i>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_font_awesome_far() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<i class="far fa-bell"></i>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_material_icons() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<i class="material-icons">home</i>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_material_symbols() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<i class="material-symbols-outlined">home</i>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_bootstrap_icons() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<i class="bi bi-heart"></i>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_iconify() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<i class="iconify" data-icon="mdi:home"></i>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_generic_icon_class() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<i class="my-icon"></i>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_is_icon_class_unit() {
        assert!(is_icon_class("fa"));
        assert!(is_icon_class("fas"));
        assert!(is_icon_class("far"));
        assert!(is_icon_class("fab"));
        assert!(is_icon_class("fal"));
        assert!(is_icon_class("fad"));
        assert!(is_icon_class("fa-home"));
        assert!(is_icon_class("material-icons"));
        assert!(is_icon_class("material-symbols-outlined"));
        assert!(is_icon_class("bi"));
        assert!(is_icon_class("bi-heart"));
        assert!(is_icon_class("iconify"));
        assert!(is_icon_class("my-icon"));
        assert!(!is_icon_class("emphasis"));
        assert!(!is_icon_class("bold"));
        assert!(!is_icon_class("text-red"));
    }
}
