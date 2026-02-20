//! Main linter entry point.
//!
//! High-performance Vue template linter with arena allocation.

use crate::context::LintContext;
use crate::diagnostic::{HelpLevel, LintDiagnostic, LintSummary};
use crate::rule::RuleRegistry;
use crate::visitor::LintVisitor;
use vize_armature::Parser;
use vize_carton::i18n::Locale;
use vize_carton::{Allocator, FxHashSet};

/// Lint result for a single file
#[derive(Debug, Clone)]
pub struct LintResult {
    /// Filename that was linted
    pub filename: String,
    /// Collected diagnostics
    pub diagnostics: Vec<LintDiagnostic>,
    /// Number of errors
    pub error_count: usize,
    /// Number of warnings
    pub warning_count: usize,
}

impl LintResult {
    /// Check if there are any errors
    #[inline]
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Check if there are any diagnostics
    #[inline]
    pub fn has_diagnostics(&self) -> bool {
        !self.diagnostics.is_empty()
    }
}

/// Main linter struct.
///
/// The linter is designed for high performance:
/// - Uses arena allocation for AST and context
/// - Pre-allocates vectors with expected capacity
/// - Minimizes allocations during traversal
pub struct Linter {
    registry: RuleRegistry,
    /// Estimated initial allocator capacity (in bytes)
    initial_capacity: usize,
    /// Locale for i18n messages
    locale: Locale,
    /// Optional set of enabled rule names (if None, all rules are enabled)
    enabled_rules: Option<FxHashSet<String>>,
    /// Help display level
    help_level: HelpLevel,
}

impl Linter {
    /// Default initial capacity for the arena (64KB)
    const DEFAULT_INITIAL_CAPACITY: usize = 64 * 1024;

    /// Create a new linter with recommended rules
    #[inline]
    pub fn new() -> Self {
        Self {
            registry: RuleRegistry::with_recommended(),
            initial_capacity: Self::DEFAULT_INITIAL_CAPACITY,
            locale: Locale::default(),
            enabled_rules: None,
            help_level: HelpLevel::default(),
        }
    }

    /// Create a linter with a custom rule registry
    #[inline]
    pub fn with_registry(registry: RuleRegistry) -> Self {
        Self {
            registry,
            initial_capacity: Self::DEFAULT_INITIAL_CAPACITY,
            locale: Locale::default(),
            enabled_rules: None,
            help_level: HelpLevel::default(),
        }
    }

    /// Set the initial allocator capacity
    #[inline]
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.initial_capacity = capacity;
        self
    }

    /// Set the locale for i18n messages
    #[inline]
    pub fn with_locale(mut self, locale: Locale) -> Self {
        self.locale = locale;
        self
    }

    /// Set enabled rules (if None, all rules are enabled)
    ///
    /// Pass a list of rule names to enable only those rules.
    /// Rules not in the list will be skipped during linting.
    #[inline]
    pub fn with_enabled_rules(mut self, rules: Option<Vec<String>>) -> Self {
        self.enabled_rules = rules.map(|r| r.into_iter().collect());
        self
    }

    /// Set the help display level
    #[inline]
    pub fn with_help_level(mut self, level: HelpLevel) -> Self {
        self.help_level = level;
        self
    }

    /// Get the current locale
    #[inline]
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// Check if a rule is enabled
    #[inline]
    pub fn is_rule_enabled(&self, rule_name: &str) -> bool {
        match &self.enabled_rules {
            Some(set) => set.contains(rule_name),
            None => true,
        }
    }

    /// Lint a Vue template source
    #[inline]
    pub fn lint_template(&self, source: &str, filename: &str) -> LintResult {
        // Create allocator sized for source (rough heuristic: 4x source size)
        let capacity = (source.len() * 4).max(self.initial_capacity);
        let allocator = Allocator::with_capacity(capacity);

        self.lint_template_with_allocator(&allocator, source, filename)
    }

    /// Lint a Vue template with a provided allocator (for reuse)
    pub fn lint_template_with_allocator(
        &self,
        allocator: &Allocator,
        source: &str,
        filename: &str,
    ) -> LintResult {
        // Parse the template
        let parser = Parser::new(allocator.as_bump(), source);
        let (root, _parse_errors) = parser.parse();

        // Create lint context with locale, help level, and enabled rules filter
        let mut ctx = LintContext::with_locale(allocator, source, filename, self.locale);
        ctx.set_enabled_rules(self.enabled_rules.clone());
        ctx.set_help_level(self.help_level);

        // Run visitor with all rules (filtering happens in context)
        let mut visitor = LintVisitor::new(&mut ctx, self.registry.rules());
        visitor.visit_root(&root);

        // Collect results (error/warning counts are cached)
        let error_count = ctx.error_count();
        let warning_count = ctx.warning_count();
        let diagnostics = ctx.into_diagnostics();

        LintResult {
            filename: filename.to_string(),
            diagnostics,
            error_count,
            warning_count,
        }
    }

    /// Lint multiple files and aggregate results
    pub fn lint_files(&self, files: &[(String, String)]) -> (Vec<LintResult>, LintSummary) {
        let mut results = Vec::with_capacity(files.len());
        let mut summary = LintSummary::default();

        // Reuse allocator across files for better memory efficiency
        let mut allocator = Allocator::with_capacity(self.initial_capacity);

        for (filename, source) in files {
            let result = self.lint_template_with_allocator(&allocator, source, filename);
            summary.error_count += result.error_count;
            summary.warning_count += result.warning_count;
            results.push(result);

            // Reset allocator for next file
            allocator.reset();
        }

        summary.file_count = files.len();
        (results, summary)
    }

    /// Get the rule registry
    #[inline]
    pub fn registry(&self) -> &RuleRegistry {
        &self.registry
    }

    /// Get all registered rules
    #[inline]
    pub fn rules(&self) -> &[Box<dyn crate::rule::Rule>] {
        self.registry.rules()
    }

    /// Lint a full Vue SFC file
    ///
    /// Uses ultra-fast template extraction optimized for linting.
    #[inline]
    pub fn lint_sfc(&self, source: &str, filename: &str) -> LintResult {
        // Fast template extraction using memchr
        let (content, byte_offset) = match extract_template_fast(source) {
            Some(r) => r,
            None => {
                return LintResult {
                    filename: filename.to_string(),
                    diagnostics: Vec::new(),
                    error_count: 0,
                    warning_count: 0,
                };
            }
        };

        let mut result = self.lint_template(&content, filename);

        // Adjust byte offsets in diagnostics to match original file positions
        if byte_offset > 0 {
            for diag in &mut result.diagnostics {
                diag.start += byte_offset;
                diag.end += byte_offset;
                for label in &mut diag.labels {
                    label.start += byte_offset;
                    label.end += byte_offset;
                }
            }
        }

        result
    }
}

/// Ultra-fast template extraction using memchr for SIMD-accelerated search
#[inline]
fn extract_template_fast(source: &str) -> Option<(String, u32)> {
    let bytes = source.as_bytes();

    // Find <template using memchr (SIMD accelerated)
    let start_pattern = b"<template";

    // Find first <template occurrence
    let start_idx = memchr::memmem::find(bytes, start_pattern)?;

    // Find > after <template (end of opening tag)
    let tag_end = memchr::memchr(b'>', &bytes[start_idx..])? + start_idx;
    let content_start = tag_end + 1;

    // Find matching </template> - handle nesting with simple depth tracking
    let mut depth = 1u32;
    let mut pos = content_start;

    while pos < bytes.len() && depth > 0 {
        // Find next < character
        let next_lt = match memchr::memchr(b'<', &bytes[pos..]) {
            Some(p) => pos + p,
            None => break,
        };

        // Check if it's <template or </template
        if bytes.len() > next_lt + 9 && &bytes[next_lt..next_lt + 9] == b"<template" {
            // Check if self-closing
            if let Some(gt) = memchr::memchr(b'>', &bytes[next_lt..]) {
                let tag_end_pos = next_lt + gt;
                if tag_end_pos > 0 && bytes[tag_end_pos - 1] != b'/' {
                    depth += 1;
                }
                pos = tag_end_pos + 1;
            } else {
                pos = next_lt + 9;
            }
        } else if bytes.len() > next_lt + 11 && &bytes[next_lt..next_lt + 11] == b"</template>" {
            depth -= 1;
            if depth == 0 {
                let content = std::str::from_utf8(&bytes[content_start..next_lt]).ok()?;
                return Some((content.to_string(), content_start as u32));
            }
            pos = next_lt + 11;
        } else {
            pos = next_lt + 1;
        }
    }

    None
}

impl Default for Linter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_empty_template() {
        let linter = Linter::new();
        let result = linter.lint_template("", "test.vue");
        assert!(!result.has_errors());
        assert!(!result.has_diagnostics());
    }

    #[test]
    fn test_lint_simple_template() {
        let linter = Linter::new();
        let result = linter.lint_template("<div>Hello</div>", "test.vue");
        assert!(!result.has_errors());
    }

    #[test]
    fn test_lint_with_allocator_reuse() {
        let linter = Linter::new();
        let allocator = Allocator::with_capacity(1024);

        let result1 =
            linter.lint_template_with_allocator(&allocator, "<div>Hello</div>", "test1.vue");
        assert!(!result1.has_errors());

        // Allocator is borrowed, can't reset here, but demonstrates the API
    }

    #[test]
    fn test_lint_files_batch() {
        let linter = Linter::new();
        let files = vec![
            ("test1.vue".to_string(), "<div>Hello</div>".to_string()),
            ("test2.vue".to_string(), "<span>World</span>".to_string()),
        ];

        let (results, summary) = linter.lint_files(&files);
        assert_eq!(results.len(), 2);
        assert_eq!(summary.file_count, 2);
    }

    #[test]
    fn test_disable_next_line() {
        let linter = Linter::new();
        // Without disable comment - should have error
        let result = linter.lint_template(
            r#"<ul><li v-for="item in items">{{ item }}</li></ul>"#,
            "test.vue",
        );
        assert!(result.error_count > 0, "Should have error without key");

        // With disable comment - should suppress error
        let result = linter.lint_template(
            r#"<ul><!-- vize-disable-next-line -->
<li v-for="item in items">{{ item }}</li></ul>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 0, "Error should be suppressed");
    }

    #[test]
    fn test_disable_specific_rule() {
        let linter = Linter::new();
        // With specific rule disable
        let result = linter.lint_template(
            r#"<ul><!-- vize-disable-next-line vue/require-v-for-key -->
<li v-for="item in items">{{ item }}</li></ul>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 0, "Specific rule should be suppressed");
    }

    #[test]
    fn test_disable_all() {
        let linter = Linter::new();
        // With disable all
        let result = linter.lint_template(
            r#"<!-- vize-disable -->
<ul><li v-for="item in items">{{ item }}</li></ul>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 0, "All rules should be disabled");
    }

    #[test]
    fn test_lint_sfc_extracts_template() {
        let linter = Linter::new();
        // SFC with script and template - should only lint template content
        let sfc = r#"<script setup lang="ts">
interface Props {
  schema?: BaseSchema<FormShape, FormShape, any>;
}
</script>

<template>
  <div>Hello World</div>
</template>
"#;
        let result = linter.lint_sfc(sfc, "test.vue");
        // Should not report errors for TypeScript code in <script>
        assert_eq!(result.error_count, 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_lint_sfc_no_template() {
        let linter = Linter::new();
        // SFC without template - should return empty result
        let sfc = r#"<script setup lang="ts">
const foo = 'bar';
</script>
"#;
        let result = linter.lint_sfc(sfc, "test.vue");
        assert_eq!(result.error_count, 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_lint_sfc_byte_offset() {
        let linter = Linter::new();
        // SFC where template has an error - byte offset should be adjusted
        let sfc = r#"<script setup lang="ts">
const foo = 'bar';
</script>

<template>
  <ul><li v-for="item in items">{{ item }}</li></ul>
</template>
"#;
        let result = linter.lint_sfc(sfc, "test.vue");
        // Should have error for missing :key
        assert!(result.error_count > 0, "Should detect v-for without key");

        // The byte offset should point to the correct location in the original file
        if let Some(diag) = result.diagnostics.first() {
            // The diagnostic should point somewhere in the template section
            // Template starts after "<script>...</script>\n\n<template>\n"
            assert!(
                diag.start > 50,
                "Byte offset should be adjusted for template position"
            );
        }
    }

    #[test]
    fn test_lint_sfc_offset_line_conversion() {
        use crate::telegraph::LspEmitter;

        let linter = Linter::new();
        let sfc = r#"<script setup lang="ts">
const foo = 'bar';
</script>

<template>
  <ul><li v-for="item in items">{{ item }}</li></ul>
</template>
"#;
        let result = linter.lint_sfc(sfc, "test.vue");
        assert!(result.error_count > 0);

        // Debug: show template start
        let template_start = sfc.find("<template>").unwrap();
        eprintln!("Template <template> starts at byte: {}", template_start);

        // Debug: show content start (after <template>)
        let content_start = sfc.find("<template>").unwrap() + "<template>\n".len();
        eprintln!("Template content starts at byte: {}", content_start);

        // Debug: show diagnostics
        for (i, diag) in result.diagnostics.iter().enumerate() {
            eprintln!(
                "Diag[{}] rule={}, start={}, end={}",
                i, diag.rule_name, diag.start, diag.end
            );

            // Count newlines before start to get line number
            let before = &sfc[..diag.start as usize];
            let line_count = before.matches('\n').count();
            eprintln!("  -> Line (0-indexed): {}", line_count);
        }

        // Test LspEmitter conversion
        let lsp_diags = LspEmitter::to_lsp_diagnostics_with_source(&result, sfc);
        for (i, lsp) in lsp_diags.iter().enumerate() {
            eprintln!(
                "LSP[{}] line={}, col={}",
                i, lsp.range.start.line, lsp.range.start.character
            );
        }

        // Expected: line should be around 5 (0-indexed) for template content
        // Line 0: <script setup lang="ts">
        // Line 1: const foo = 'bar';
        // Line 2: </script>
        // Line 3: (empty)
        // Line 4: <template>
        // Line 5:   <ul>...
        if let Some(lsp) = lsp_diags.first() {
            assert_eq!(
                lsp.range.start.line, 5,
                "First diagnostic should be on line 5 (0-indexed)"
            );
        }
    }

    #[test]
    fn test_lint_sfc_with_nested_templates() {
        let linter = Linter::new();
        // SFC with nested template elements - should extract full content
        let sfc = r#"<script setup lang="ts">
const show = true;
</script>

<template>
  <div>
    <template v-if="show">
      <span>Visible</span>
    </template>
    <template v-else>
      <span>Hidden</span>
    </template>
  </div>
</template>
"#;
        let result = linter.lint_sfc(sfc, "test.vue");
        // Should not have errors - nested templates have v-if/v-else directives
        // Most importantly, should not report "no-lone-template" on the root <template>
        assert_eq!(
            result.error_count, 0,
            "Should not report errors for valid nested templates with directives"
        );
    }

    #[test]
    fn test_lint_sfc_with_nested_template_extraction() {
        // Test that nested templates are properly handled via parse_sfc
        let linter = Linter::new();
        let sfc = r#"<script></script>
<template>
  <div>
    <template v-if="x">nested</template>
  </div>
</template>"#;

        let result = linter.lint_sfc(sfc, "test.vue");
        // Should not report errors for the nested template with v-if directive
        assert_eq!(
            result.error_count, 0,
            "Should properly extract and lint nested templates"
        );
    }

    #[test]
    fn test_vize_todo_emits_warning() {
        let linter = Linter::new();
        let result = linter.lint_template(
            r#"<div><!-- @vize:todo fix this --><span>hello</span></div>"#,
            "test.vue",
        );
        assert_eq!(
            result.warning_count, 1,
            "Should emit 1 warning for @vize:todo"
        );
        assert_eq!(result.diagnostics[0].rule_name, "vize/todo");
        assert!(result.diagnostics[0].message.contains("TODO"));
    }

    #[test]
    fn test_vize_fixme_emits_error() {
        let linter = Linter::new();
        let result = linter.lint_template(
            r#"<div><!-- @vize:fixme broken --><span>hello</span></div>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 1, "Should emit 1 error for @vize:fixme");
        assert_eq!(result.diagnostics[0].rule_name, "vize/fixme");
        assert!(result.diagnostics[0].message.contains("FIXME"));
    }

    #[test]
    fn test_vize_deprecated_emits_warning() {
        let linter = Linter::new();
        let result = linter.lint_template(
            r#"<div><!-- @vize:deprecated use NewComp --><span>hello</span></div>"#,
            "test.vue",
        );
        assert_eq!(
            result.warning_count, 1,
            "Should emit 1 warning for @vize:deprecated"
        );
        assert_eq!(result.diagnostics[0].rule_name, "vize/deprecated");
        assert!(result.diagnostics[0].message.contains("Deprecated"));
    }

    #[test]
    fn test_vize_expected_suppresses_error() {
        let linter = Linter::new();
        // Without @vize:expected - should have error
        let result = linter.lint_template(
            r#"<ul><li v-for="item in items">{{ item }}</li></ul>"#,
            "test.vue",
        );
        assert!(result.error_count > 0, "Should have error without key");

        // With @vize:expected - should suppress error on next line
        let result = linter.lint_template(
            r#"<ul><!-- @vize:expected -->
<li v-for="item in items">{{ item }}</li></ul>"#,
            "test.vue",
        );
        assert_eq!(
            result.error_count, 0,
            "Error should be suppressed by @vize:expected"
        );
    }

    #[test]
    fn test_vize_ignore_start_end_region() {
        let linter = Linter::new();
        // With @vize:ignore-start/end - should suppress errors in region
        let result = linter.lint_template(
            r#"<!-- @vize:ignore-start -->
<ul><li v-for="item in items">{{ item }}</li></ul>
<!-- @vize:ignore-end -->"#,
            "test.vue",
        );
        assert_eq!(
            result.error_count, 0,
            "Errors in ignore region should be suppressed"
        );
    }

    #[test]
    fn test_vize_docs_no_lint_effect() {
        let linter = Linter::new();
        let result = linter.lint_template(
            r#"<div><!-- @vize:docs Component documentation --><span>hello</span></div>"#,
            "test.vue",
        );
        assert_eq!(
            result.error_count, 0,
            "Docs directive should not produce errors"
        );
        assert_eq!(
            result.warning_count, 0,
            "Docs directive should not produce warnings"
        );
    }
}
