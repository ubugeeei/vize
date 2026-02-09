//! SFC type checking functionality for Vue Single File Components.
//!
//! This module provides AST-based type analysis for Vue SFCs.
//! It leverages croquis for semantic analysis and scope tracking.
//!
//! ## Features
//!
//! - Props type validation (defineProps)
//! - Emits type validation (defineEmits)
//! - Template binding validation (undefined references)
//! - Virtual TypeScript generation with scope-aware code
//!
//! ## Architecture
//!
//! ```text
//! Vue SFC (.vue)
//!     │
//!     ▼
//! ┌─────────────────────────────────────┐
//! │  vize_atelier_sfc::parse_sfc        │
//! └─────────────────────────────────────┘
//!     │
//!     ▼
//! ┌─────────────────────────────────────┐
//! │  vize_croquis::Analyzer             │
//! │  - Script analysis (bindings)       │
//! │  - Template analysis (scopes)       │
//! │  - Macro tracking (defineProps)     │
//! └─────────────────────────────────────┘
//!     │
//!     ▼
//! ┌─────────────────────────────────────┐
//! │  type_check_sfc()                   │
//! │  - check_props_typing()             │
//! │  - check_emits_typing()             │
//! │  - check_template_bindings()        │
//! │  - generate_virtual_ts_with_scopes()│
//! └─────────────────────────────────────┘
//! ```

mod checks;
mod virtual_ts;

use serde::Serialize;
use vize_carton::Bump;

use checks::{
    check_emits_typing, check_fallthrough_attrs, check_invalid_exports, check_props_typing,
    check_reactivity, check_setup_context, check_template_bindings,
};
use virtual_ts::generate_virtual_ts_with_scopes;

/// Type diagnostic severity.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SfcTypeSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Type diagnostic representing a type-related issue.
#[derive(Debug, Clone, Serialize)]
pub struct SfcTypeDiagnostic {
    /// Severity of the diagnostic
    pub severity: SfcTypeSeverity,
    /// Human-readable message
    pub message: String,
    /// Start offset in source
    pub start: u32,
    /// End offset in source
    pub end: u32,
    /// Optional error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Optional help text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Related locations (for multi-file issues)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<SfcRelatedLocation>,
}

/// Related location for diagnostics.
#[derive(Debug, Clone, Serialize)]
pub struct SfcRelatedLocation {
    pub message: String,
    pub start: u32,
    pub end: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// Type checking result.
#[derive(Debug, Clone, Serialize)]
pub struct SfcTypeCheckResult {
    /// List of diagnostics
    pub diagnostics: Vec<SfcTypeDiagnostic>,
    /// Generated virtual TypeScript (for debugging/IDE integration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtual_ts: Option<String>,
    /// Error count
    pub error_count: usize,
    /// Warning count
    pub warning_count: usize,
    /// Analysis time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis_time_ms: Option<f64>,
}

impl SfcTypeCheckResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            diagnostics: Vec::new(),
            virtual_ts: None,
            error_count: 0,
            warning_count: 0,
            analysis_time_ms: None,
        }
    }

    /// Add a diagnostic.
    pub fn add_diagnostic(&mut self, diagnostic: SfcTypeDiagnostic) {
        match diagnostic.severity {
            SfcTypeSeverity::Error => self.error_count += 1,
            SfcTypeSeverity::Warning => self.warning_count += 1,
            _ => {}
        }
        self.diagnostics.push(diagnostic);
    }

    /// Check if there are errors.
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }
}

/// Type checking options.
#[derive(Debug, Clone, Default)]
pub struct SfcTypeCheckOptions {
    /// Filename for error reporting
    pub filename: String,
    /// Whether to include virtual TypeScript in output
    pub include_virtual_ts: bool,
    /// Whether to check props types
    pub check_props: bool,
    /// Whether to check emits types
    pub check_emits: bool,
    /// Whether to check template bindings
    pub check_template_bindings: bool,
    /// Whether to check reactivity loss patterns
    pub check_reactivity: bool,
    /// Whether to check setup context violations
    pub check_setup_context: bool,
    /// Whether to check invalid exports in `<script setup>`
    pub check_invalid_exports: bool,
    /// Whether to check fallthrough attrs with multi-root
    pub check_fallthrough_attrs: bool,
    /// Strict mode - report more potential issues
    pub strict: bool,
}

impl SfcTypeCheckOptions {
    /// Create default options.
    pub fn new(filename: impl Into<String>) -> Self {
        Self {
            filename: filename.into(),
            include_virtual_ts: false,
            check_props: true,
            check_emits: true,
            check_template_bindings: true,
            check_reactivity: true,
            check_setup_context: true,
            check_invalid_exports: true,
            check_fallthrough_attrs: true,
            strict: false,
        }
    }

    /// Enable strict mode.
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Include virtual TypeScript in output.
    pub fn with_virtual_ts(mut self) -> Self {
        self.include_virtual_ts = true;
        self
    }
}

/// Perform type checking on a Vue SFC.
///
/// This performs AST-based type analysis using croquis for semantic analysis.
/// It checks:
/// - Props typing (defineProps)
/// - Emits typing (defineEmits)
/// - Template binding references
///
/// For full TypeScript type checking with tsgo, use `TypeCheckService`.
pub fn type_check_sfc(source: &str, options: &SfcTypeCheckOptions) -> SfcTypeCheckResult {
    use vize_atelier_core::parser::parse;
    use vize_atelier_sfc::{parse_sfc, SfcParseOptions};
    use vize_croquis::{Analyzer, AnalyzerOptions};

    // Use Instant for timing on native, skip on WASM
    #[cfg(not(target_arch = "wasm32"))]
    let start_time = std::time::Instant::now();

    let mut result = SfcTypeCheckResult::empty();

    // Parse SFC
    let parse_opts = SfcParseOptions {
        filename: options.filename.clone(),
        ..Default::default()
    };

    let descriptor = match parse_sfc(source, parse_opts) {
        Ok(d) => d,
        Err(e) => {
            result.add_diagnostic(SfcTypeDiagnostic {
                severity: SfcTypeSeverity::Error,
                message: format!("Failed to parse SFC: {}", e.message),
                start: 0,
                end: 0,
                code: Some("parse-error".to_string()),
                help: None,
                related: Vec::new(),
            });
            return result;
        }
    };

    // Get script content for virtual TS generation
    let script_content = descriptor
        .script_setup
        .as_ref()
        .map(|s| s.content.as_ref())
        .or_else(|| descriptor.script.as_ref().map(|s| s.content.as_ref()));

    // Create allocator for template parsing
    let allocator = Bump::new();

    // Create analyzer with full options
    let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());

    // Analyze script and get offset
    let script_offset: u32 = if let Some(ref script_setup) = descriptor.script_setup {
        analyzer.analyze_script_setup(&script_setup.content);
        script_setup.loc.start as u32
    } else if let Some(ref script) = descriptor.script {
        analyzer.analyze_script_plain(&script.content);
        script.loc.start as u32
    } else {
        0
    };

    // Analyze template and get AST
    let (template_offset, template_ast) = if let Some(ref template) = descriptor.template {
        let (root, _errors) = parse(&allocator, &template.content);
        analyzer.analyze_template(&root);
        (template.loc.start as u32, Some(root))
    } else {
        (0, None)
    };

    // Get analysis summary with scopes
    let summary = analyzer.finish();

    // Check props typing
    if options.check_props {
        check_props_typing(&summary, script_offset, &mut result, options.strict);
    }

    // Check emits typing
    if options.check_emits {
        check_emits_typing(&summary, script_offset, &mut result, options.strict);
    }

    // Check template bindings
    if options.check_template_bindings {
        check_template_bindings(&summary, template_offset, &mut result, options.strict);
    }

    // Check reactivity loss
    if options.check_reactivity {
        check_reactivity(&summary, script_offset, &mut result, options.strict);
    }

    // Check setup context violations
    if options.check_setup_context {
        check_setup_context(&summary, script_offset, &mut result);
    }

    // Check invalid exports in <script setup>
    if options.check_invalid_exports {
        check_invalid_exports(&summary, script_offset, &mut result);
    }

    // Check fallthrough attrs
    if options.check_fallthrough_attrs {
        check_fallthrough_attrs(&summary, &mut result, options.strict);
    }

    // Generate virtual TypeScript with scope information if requested
    if options.include_virtual_ts {
        result.virtual_ts = Some(generate_virtual_ts_with_scopes(
            &summary,
            script_content,
            script_offset,
            template_ast.as_ref(),
            template_offset,
        ));
    }

    // Record analysis time on native only
    #[cfg(not(target_arch = "wasm32"))]
    {
        result.analysis_time_ms = Some(start_time.elapsed().as_secs_f64() * 1000.0);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_check_empty_sfc() {
        let source = "<template><div>Hello</div></template>";
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        assert!(!result.has_errors());
        assert_eq!(result.error_count, 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_type_check_result() {
        let mut result = SfcTypeCheckResult::empty();
        assert_eq!(result.error_count, 0);
        assert!(!result.has_errors());

        result.add_diagnostic(SfcTypeDiagnostic {
            severity: SfcTypeSeverity::Error,
            message: "test".to_string(),
            start: 0,
            end: 0,
            code: None,
            help: None,
            related: Vec::new(),
        });

        assert_eq!(result.error_count, 1);
        assert!(result.has_errors());
    }

    #[test]
    fn test_type_check_options_default() {
        let options = SfcTypeCheckOptions::new("test.vue");
        assert_eq!(options.filename, "test.vue");
        assert!(!options.strict);
        assert!(options.check_props);
        assert!(options.check_emits);
        assert!(options.check_template_bindings);
        assert!(!options.include_virtual_ts);
    }

    #[test]
    fn test_type_check_options_strict() {
        let options = SfcTypeCheckOptions::new("test.vue").strict();
        assert!(options.strict);
    }

    #[test]
    fn test_type_check_options_with_virtual_ts() {
        let options = SfcTypeCheckOptions::new("test.vue").with_virtual_ts();
        assert!(options.include_virtual_ts);
    }

    #[test]
    fn test_type_check_with_typed_props() {
        let source = r#"<script setup lang="ts">
interface Props {
    count: number;
    name: string;
}
const props = defineProps<Props>();
</script>
<template>
    <div>{{ props.count }} - {{ props.name }}</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        assert!(!result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("untyped-prop")));
    }

    #[test]
    fn test_type_check_with_untyped_props_non_strict() {
        let source = r#"<script setup>
const props = defineProps(['count', 'name']);
</script>
<template>
    <div>{{ props.count }}</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        let has_untyped_prop_warning = result.diagnostics.iter().any(|d| {
            d.code.as_deref() == Some("untyped-prop") && d.severity == SfcTypeSeverity::Warning
        });
        assert!(has_untyped_prop_warning);
    }

    #[test]
    fn test_type_check_with_untyped_props_strict() {
        let source = r#"<script setup>
const props = defineProps(['count', 'name']);
</script>
<template>
    <div>{{ props.count }}</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").strict();
        let result = type_check_sfc(source, &options);
        let has_untyped_prop_error = result.diagnostics.iter().any(|d| {
            d.code.as_deref() == Some("untyped-prop") && d.severity == SfcTypeSeverity::Error
        });
        assert!(has_untyped_prop_error);
    }

    #[test]
    fn test_type_check_with_typed_emits() {
        let source = r#"<script setup lang="ts">
const emit = defineEmits<{
    (e: 'update', value: number): void;
    (e: 'close'): void;
}>();
</script>
<template>
    <button @click="emit('close')">Close</button>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        assert!(!result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("untyped-emit")));
    }

    #[test]
    fn test_type_check_disabled_props_check() {
        let source = r#"<script setup>
const props = defineProps(['count']);
</script>
<template>
    <div>{{ props.count }}</div>
</template>"#;
        let mut options = SfcTypeCheckOptions::new("test.vue");
        options.check_props = false;
        let result = type_check_sfc(source, &options);
        assert!(!result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("untyped-prop")));
    }

    #[test]
    fn test_type_check_undefined_binding() {
        let source = r#"<script setup>
const count = ref(0);
</script>
<template>
    <div>{{ undefinedVar }}</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        let has_undefined_error = result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("undefined-binding"));
        assert!(has_undefined_error);
    }

    #[test]
    fn test_type_check_defined_binding() {
        let source = r#"<script setup>
const count = ref(0);
</script>
<template>
    <div>{{ count }}</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        assert!(!result.diagnostics.iter().any(|d| {
            d.code.as_deref() == Some("undefined-binding") && d.message.contains("count")
        }));
    }

    #[test]
    fn test_type_check_virtual_ts_generation() {
        let source = r#"<script setup lang="ts">
const props = defineProps<{ count: number }>();
const message = ref('Hello');
</script>
<template>
    <div>{{ props.count }} - {{ message }}</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").with_virtual_ts();
        let result = type_check_sfc(source, &options);
        assert!(result.virtual_ts.is_some());
        let virtual_ts = result.virtual_ts.unwrap();
        assert!(virtual_ts.contains("Virtual TypeScript"));
        assert!(virtual_ts.contains("Generated by vize"));
    }

    #[test]
    fn test_type_severity_serialization() {
        assert_eq!(
            serde_json::to_string(&SfcTypeSeverity::Error).unwrap(),
            "\"error\""
        );
        assert_eq!(
            serde_json::to_string(&SfcTypeSeverity::Warning).unwrap(),
            "\"warning\""
        );
        assert_eq!(
            serde_json::to_string(&SfcTypeSeverity::Info).unwrap(),
            "\"info\""
        );
        assert_eq!(
            serde_json::to_string(&SfcTypeSeverity::Hint).unwrap(),
            "\"hint\""
        );
    }

    #[test]
    fn test_type_check_result_warning_count() {
        let mut result = SfcTypeCheckResult::empty();

        result.add_diagnostic(SfcTypeDiagnostic {
            severity: SfcTypeSeverity::Warning,
            message: "warning 1".to_string(),
            start: 0,
            end: 0,
            code: None,
            help: None,
            related: Vec::new(),
        });

        result.add_diagnostic(SfcTypeDiagnostic {
            severity: SfcTypeSeverity::Warning,
            message: "warning 2".to_string(),
            start: 0,
            end: 0,
            code: None,
            help: None,
            related: Vec::new(),
        });

        assert_eq!(result.error_count, 0);
        assert_eq!(result.warning_count, 2);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_type_check_result_mixed_diagnostics() {
        let mut result = SfcTypeCheckResult::empty();

        result.add_diagnostic(SfcTypeDiagnostic {
            severity: SfcTypeSeverity::Error,
            message: "error".to_string(),
            start: 0,
            end: 0,
            code: None,
            help: None,
            related: Vec::new(),
        });

        result.add_diagnostic(SfcTypeDiagnostic {
            severity: SfcTypeSeverity::Warning,
            message: "warning".to_string(),
            start: 0,
            end: 0,
            code: None,
            help: None,
            related: Vec::new(),
        });

        result.add_diagnostic(SfcTypeDiagnostic {
            severity: SfcTypeSeverity::Info,
            message: "info".to_string(),
            start: 0,
            end: 0,
            code: None,
            help: None,
            related: Vec::new(),
        });

        assert_eq!(result.error_count, 1);
        assert_eq!(result.warning_count, 1);
        assert_eq!(result.diagnostics.len(), 3);
        assert!(result.has_errors());
    }

    #[test]
    fn test_type_check_v_for_destructuring() {
        let source = r#"<script setup lang="ts">
import { ref } from 'vue'
interface Item { id: number; name: string }
const items = ref<Item[]>([])
</script>
<template>
  <div v-for="{ id, name } in items" :key="id">
    {{ id }}: {{ name }}
  </div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").with_virtual_ts();
        let result = type_check_sfc(source, &options);
        assert!(result.virtual_ts.is_some());
        // Should not have undefined binding errors for destructured vars
        assert!(!result.diagnostics.iter().any(|d| {
            d.code.as_deref() == Some("undefined-binding")
                && (d.message.contains("id") || d.message.contains("name"))
        }));
    }

    #[test]
    fn test_type_check_nested_v_if_v_else() {
        let source = r#"<script setup lang="ts">
import { ref } from 'vue'
const show = ref(true)
const count = ref(0)
const message = ref('')
</script>
<template>
  <div v-if="show">{{ count }}</div>
  <div v-else>{{ message }}</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").with_virtual_ts();
        let result = type_check_sfc(source, &options);
        // All bindings are defined, should have no undefined binding errors
        assert!(!result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("undefined-binding")));
    }

    #[test]
    fn test_type_check_scoped_slots() {
        let source = r#"<script setup lang="ts">
import MyList from './MyList.vue'
</script>
<template>
  <MyList>
    <template #default="{ item }">
      {{ item }}
    </template>
  </MyList>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").with_virtual_ts();
        let result = type_check_sfc(source, &options);
        assert!(result.virtual_ts.is_some());
    }

    #[test]
    fn test_type_check_v_model() {
        let source = r#"<script setup lang="ts">
import { ref } from 'vue'
const text = ref('')
</script>
<template>
  <input v-model="text" />
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").with_virtual_ts();
        let result = type_check_sfc(source, &options);
        assert!(!result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("undefined-binding")));
    }

    #[test]
    fn test_type_check_template_refs() {
        let source = r#"<script setup lang="ts">
import { ref } from 'vue'
const inputRef = ref<HTMLInputElement | null>(null)
</script>
<template>
  <input ref="inputRef" />
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").with_virtual_ts();
        let result = type_check_sfc(source, &options);
        assert!(result.virtual_ts.is_some());
    }

    #[test]
    fn test_type_check_generic_component() {
        let source = r#"<script setup lang="ts" generic="T extends string">
const props = defineProps<{ items: T[] }>()
</script>
<template>
  <div v-for="item in props.items" :key="item">{{ item }}</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").with_virtual_ts();
        let result = type_check_sfc(source, &options);
        assert!(result.virtual_ts.is_some());
    }

    #[test]
    fn test_type_check_multiple_script_blocks() {
        let source = r#"<script lang="ts">
export default {
  name: 'MyComponent'
}
</script>
<script setup lang="ts">
import { ref } from 'vue'
const count = ref(0)
</script>
<template>
  <div>{{ count }}</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").with_virtual_ts();
        let result = type_check_sfc(source, &options);
        assert!(result.virtual_ts.is_some());
    }

    #[test]
    fn test_type_check_empty_template() {
        let source = r#"<script setup lang="ts">
import { ref } from 'vue'
const count = ref(0)
</script>
<template></template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_type_check_component_with_props_and_events() {
        let source = r#"<script setup lang="ts">
interface Props {
  title: string
  disabled?: boolean
}
interface Emits {
  (e: 'update:title', value: string): void
  (e: 'submit'): void
}
const props = defineProps<Props>()
const emit = defineEmits<Emits>()
</script>
<template>
  <div>
    <h1>{{ props.title }}</h1>
    <button :disabled="props.disabled" @click="emit('submit')">Submit</button>
  </div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").with_virtual_ts();
        let result = type_check_sfc(source, &options);
        assert!(result.virtual_ts.is_some());
        let vts = result.virtual_ts.unwrap();
        assert!(vts.contains("props.title"));
        assert!(vts.contains("props.disabled"));
    }

    // ========== Reactivity Loss Tests ==========

    #[test]
    fn test_check_reactivity_destructure_detected() {
        let source = r#"<script setup>
import { reactive } from 'vue'
const state = reactive({ count: 0 })
const { count } = state
</script>
<template><div>{{ count }}</div></template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        let has_reactivity = result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("reactivity-loss"));
        assert!(
            has_reactivity,
            "Should detect reactivity loss from destructuring"
        );
    }

    #[test]
    fn test_check_reactivity_no_issue() {
        let source = r#"<script setup>
import { ref } from 'vue'
const count = ref(0)
</script>
<template><div>{{ count }}</div></template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        let has_reactivity = result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("reactivity-loss"));
        assert!(!has_reactivity, "Should not detect reactivity loss");
    }

    #[test]
    fn test_check_reactivity_strict_severity() {
        let source = r#"<script setup>
import { reactive } from 'vue'
const state = reactive({ count: 0 })
const { count } = state
</script>
<template><div>{{ count }}</div></template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").strict();
        let result = type_check_sfc(source, &options);
        let has_error = result.diagnostics.iter().any(|d| {
            d.code.as_deref() == Some("reactivity-loss") && d.severity == SfcTypeSeverity::Error
        });
        assert!(has_error, "Strict mode should report as Error");
    }

    // ========== Invalid Export Tests ==========

    #[test]
    fn test_check_invalid_exports_detected() {
        let source = r#"<script setup>
export const foo = 'bar'
</script>
<template><div>Hello</div></template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        let has_invalid = result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("invalid-export"));
        assert!(has_invalid, "Should detect invalid export");
    }

    #[test]
    fn test_check_invalid_exports_type_export_ok() {
        let source = r#"<script setup>
export type Foo = { name: string }
const count = ref(0)
</script>
<template><div>{{ count }}</div></template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        let has_invalid = result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("invalid-export"));
        assert!(!has_invalid, "Type exports should be allowed");
    }

    #[test]
    fn test_check_invalid_exports_disabled() {
        let source = r#"<script setup>
export const foo = 'bar'
</script>
<template><div>Hello</div></template>"#;
        let mut options = SfcTypeCheckOptions::new("test.vue");
        options.check_invalid_exports = false;
        let result = type_check_sfc(source, &options);
        let has_invalid = result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("invalid-export"));
        assert!(!has_invalid, "Should not check when disabled");
    }

    // ========== Fallthrough Attrs Tests ==========

    #[test]
    fn test_check_fallthrough_attrs_multi_root() {
        let source = r#"<script setup>
const msg = 'hello'
</script>
<template>
  <div>first</div>
  <div>second</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        let has_fallthrough = result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("fallthrough-attrs"));
        assert!(has_fallthrough, "Should detect multi-root fallthrough");
    }

    #[test]
    fn test_check_fallthrough_attrs_single_root_ok() {
        let source = r#"<script setup>
const msg = 'hello'
</script>
<template>
  <div>{{ msg }}</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue");
        let result = type_check_sfc(source, &options);
        let has_fallthrough = result
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("fallthrough-attrs"));
        assert!(!has_fallthrough, "Single root should not warn");
    }

    #[test]
    fn test_check_fallthrough_attrs_strict() {
        let source = r#"<script setup>
const msg = 'hello'
</script>
<template>
  <div>first</div>
  <div>second</div>
</template>"#;
        let options = SfcTypeCheckOptions::new("test.vue").strict();
        let result = type_check_sfc(source, &options);
        let has_error = result.diagnostics.iter().any(|d| {
            d.code.as_deref() == Some("fallthrough-attrs") && d.severity == SfcTypeSeverity::Error
        });
        assert!(has_error, "Strict mode should report as Error");
    }
}
