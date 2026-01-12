//! script/no-import-compiler-macros
//!
//! Disallow importing Vue compiler macros that are auto-imported.
//!
//! Vue compiler macros like `defineProps`, `defineEmits`, `defineExpose`,
//! `defineOptions`, `defineSlots`, and `withDefaults` are automatically
//! available in `<script setup>` and should not be explicitly imported.
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! import { defineProps, defineEmits } from 'vue'
//! import { withDefaults } from 'vue'
//!
//! const props = defineProps<Props>()
//! ```
//!
//! ### Valid
//! ```ts
//! // No import needed - compiler macros are auto-imported
//! const props = defineProps<Props>()
//! const emit = defineEmits<Emits>()
//!
//! // Regular imports are fine
//! import { ref, computed } from 'vue'
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/no-import-compiler-macros",
    description: "Disallow importing Vue compiler macros that are auto-imported",
    default_severity: Severity::Error,
};

/// Compiler macros that should not be imported
const COMPILER_MACROS: &[&str] = &[
    "defineProps",
    "defineEmits",
    "defineExpose",
    "defineOptions",
    "defineSlots",
    "defineModel",
    "withDefaults",
];

/// No import compiler macros rule
pub struct NoImportCompilerMacros;

impl ScriptRule for NoImportCompilerMacros {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Fast bailout: check if there's any import from vue
        if memmem::find(bytes, b"from 'vue'").is_none()
            && memmem::find(bytes, b"from \"vue\"").is_none()
        {
            return;
        }

        // Find import statements
        let import_finder = memmem::Finder::new(b"import ");
        let mut search_start = 0;

        while let Some(pos) = import_finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 7;

            // Find the end of this import statement
            let rest = &source[abs_pos..];
            let import_end = rest.find('\n').unwrap_or(rest.len());
            let import_line = &rest[..import_end];

            // Check if this is an import from 'vue'
            if !import_line.contains("from 'vue'") && !import_line.contains("from \"vue\"") {
                continue;
            }

            // Check for compiler macros in this import
            for macro_name in COMPILER_MACROS {
                if import_line.contains(macro_name) {
                    // Find the position of the macro name in the import
                    if let Some(macro_pos) = import_line.find(macro_name) {
                        result.add_diagnostic(
                            LintDiagnostic::error(
                                META.name,
                                format!(
                                    "Do not import compiler macro '{}' - it is automatically available in <script setup>",
                                    macro_name
                                ),
                                (offset + abs_pos + macro_pos) as u32,
                                (offset + abs_pos + macro_pos + macro_name.len()) as u32,
                            )
                            .with_help(format!(
                                "Remove '{}' from the import statement. Compiler macros are auto-imported.",
                                macro_name
                            )),
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::script::ScriptLinter;

    fn create_linter() -> ScriptLinter {
        let mut linter = ScriptLinter::new();
        linter.add_rule(Box::new(NoImportCompilerMacros));
        linter
    }

    #[test]
    fn test_valid_no_compiler_macros() {
        let linter = create_linter();
        let result = linter.lint("import { ref, computed } from 'vue'", 0);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_import_define_props() {
        let linter = create_linter();
        let result = linter.lint("import { defineProps } from 'vue'", 0);
        assert_eq!(result.error_count, 1);
        assert!(result.diagnostics[0].message.contains("defineProps"));
    }

    #[test]
    fn test_invalid_import_multiple_macros() {
        let linter = create_linter();
        let result = linter.lint("import { defineProps, defineEmits } from 'vue'", 0);
        assert_eq!(result.error_count, 2);
    }

    #[test]
    fn test_valid_usage_without_import() {
        let linter = create_linter();
        let result = linter.lint("const props = defineProps<Props>()", 0);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_other_package() {
        let linter = create_linter();
        let result = linter.lint("import { defineProps } from 'other-package'", 0);
        assert_eq!(result.error_count, 0);
    }
}
