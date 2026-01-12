//! vue/warn-custom-block
//!
//! Warn about custom blocks in SFC files.
//!
//! Custom blocks (blocks other than `<script>`, `<template>`, `<style>`)
//! require additional configuration and tooling support. This rule warns
//! about their usage to ensure they are intentional.
//!
//! ## Common Custom Blocks
//!
//! - `<i18n>` - Vue I18n translations
//! - `<docs>` - Component documentation
//! - `<story>` - Storybook stories
//! - `<test>` - Component tests
//!
//! ## Examples
//!
//! ### Triggers Warning
//! ```vue
//! <i18n>
//! { "en": { "hello": "Hello" } }
//! </i18n>
//!
//! <docs>
//! # MyComponent
//! This is a custom component.
//! </docs>
//! ```
//!
//! ## Note
//!
//! This rule is informational. Custom blocks are valid and useful when
//! properly configured with the appropriate Vite/Webpack plugins.

use crate::context::LintContext;
use crate::diagnostic::{LintDiagnostic, Severity};
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::RootNode;

static META: RuleMeta = RuleMeta {
    name: "vue/warn-custom-block",
    description: "Warn about custom blocks in SFC files",
    category: RuleCategory::Recommended,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Standard SFC block names
const STANDARD_BLOCKS: &[&str] = &["script", "template", "style"];

/// Warn about custom blocks
#[derive(Default)]
pub struct WarnCustomBlock;

impl Rule for WarnCustomBlock {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn run_on_template<'a>(&self, ctx: &mut LintContext<'a>, _root: &RootNode<'a>) {
        let source = ctx.source;

        // Find all top-level blocks by looking for < at start of line or after >
        let mut pos = 0;
        while pos < source.len() {
            // Find next < that could be a block start
            if let Some(tag_start) = source[pos..].find('<') {
                let abs_pos = pos + tag_start;

                // Skip if this is a closing tag
                if source[abs_pos..].starts_with("</") {
                    pos = abs_pos + 2;
                    continue;
                }

                // Get the tag name
                let rest = &source[abs_pos + 1..];
                let tag_end = rest
                    .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .unwrap_or(rest.len());
                let tag_name = &rest[..tag_end];

                // Check if this is a non-standard block at root level
                // Only check if we're likely at root level (check for preceding whitespace/newline)
                let before = &source[..abs_pos];
                let is_root_level = before.is_empty()
                    || before.ends_with('\n')
                    || before.trim_end().ends_with('>') && !before.contains('<');

                if is_root_level
                    && !tag_name.is_empty()
                    && !STANDARD_BLOCKS.contains(&tag_name)
                    && tag_name
                        .chars()
                        .next()
                        .map(|c| c.is_lowercase())
                        .unwrap_or(false)
                {
                    // Find the closing >
                    let close_pos = source[abs_pos..]
                        .find('>')
                        .map(|p| abs_pos + p + 1)
                        .unwrap_or(abs_pos + tag_end + 1);

                    ctx.report(
                        LintDiagnostic::warn(
                            META.name,
                            format!(
                                "Custom block <{}> detected. Ensure proper plugin configuration.",
                                tag_name
                            ),
                            abs_pos as u32,
                            close_pos as u32,
                        )
                        .with_help(format!(
                            "Custom blocks like <{}> require corresponding Vite/Webpack plugins to be processed",
                            tag_name
                        )),
                    );
                }

                pos = abs_pos + 1;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests would need SFC-level testing infrastructure
}
