//! vue/sfc-element-order
//!
//! Enforce a consistent order of top-level elements in SFC.
//!
//! Single-File Components should have a consistent order of their
//! top-level tags. The recommended order is:
//!
//! 1. `<script>` (optional, if using both script and script setup)
//! 2. `<script setup>`
//! 3. `<template>`
//! 4. `<style>`
//!
//! Or alternatively:
//! 1. `<template>`
//! 2. `<script>`
//! 3. `<style>`
//!
//! This rule enforces script(s) -> template -> style order.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>...</template>
//! <style>...</style>
//! <script setup>...</script>
//! ```
//!
//! ### Valid
//! ```vue
//! <script setup>...</script>
//! <template>...</template>
//! <style>...</style>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, RootNode};

static META: RuleMeta = RuleMeta {
    name: "vue/sfc-element-order",
    description: "Enforce consistent order of SFC top-level elements",
    category: RuleCategory::Recommended,
    fixable: false,
    default_severity: Severity::Warning,
};

/// SFC element types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SfcElementType {
    Script,   // <script> or <script setup>
    Template, // <template>
    Style,    // <style>
    Other,    // custom blocks
}

impl SfcElementType {
    fn from_tag(tag: &str) -> Self {
        match tag {
            "script" => SfcElementType::Script,
            "template" => SfcElementType::Template,
            "style" => SfcElementType::Style,
            _ => SfcElementType::Other,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            SfcElementType::Script => "<script>",
            SfcElementType::Template => "<template>",
            SfcElementType::Style => "<style>",
            SfcElementType::Other => "custom block",
        }
    }
}

/// Enforce SFC element order
pub struct SfcElementOrder;

impl Rule for SfcElementOrder {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn run_on_template<'a>(&self, ctx: &mut LintContext<'a>, root: &RootNode<'a>) {
        // This rule checks the root-level elements
        // In an SFC, the root contains script, template, and style blocks
        let mut found_elements: Vec<(SfcElementType, &ElementNode)> = Vec::new();

        for child in root.children.iter() {
            if let vize_relief::ast::TemplateChildNode::Element(element) = child {
                let element_type = SfcElementType::from_tag(element.tag.as_str());
                if element_type != SfcElementType::Other {
                    found_elements.push((element_type, element));
                }
            }
        }

        // Check order
        for i in 1..found_elements.len() {
            let (curr_type, curr_element) = found_elements[i];
            let (prev_type, _) = found_elements[i - 1];

            // Script should always come before template and style
            // Template should always come before style
            if curr_type < prev_type {
                ctx.warn_with_help(
                    format!(
                        "{} should be placed before {} in SFC",
                        curr_type.name(),
                        prev_type.name()
                    ),
                    &curr_element.loc,
                    "Recommended order: <script> -> <template> -> <style>",
                );
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
        registry.register(Box::new(SfcElementOrder));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_order_script_template_style() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<script setup></script><template><div></div></template><style></style>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_template_before_script() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<template><div></div></template><script setup></script>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("<script>"));
    }

    #[test]
    fn test_invalid_style_before_template() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<script setup></script><style></style><template><div></div></template>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }
}
