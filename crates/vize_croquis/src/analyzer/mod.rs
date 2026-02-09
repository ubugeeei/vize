//! High-performance Vue SFC analyzer.
//!
//! This module provides the `Analyzer` that produces `Croquis`.
//!
//! ## Performance Considerations
//!
//! - **Lazy analysis**: Only analyze what's requested
//! - **Zero-copy**: Use borrowed strings where possible
//! - **Arena allocation**: Temporary data uses arena allocator
//! - **Efficient structures**: FxHashMap, SmallVec, phf
//! - **Incremental**: Can analyze script and template separately
//!
//! ## Usage
//!
//! ```ignore
//! let mut analyzer = Analyzer::new();
//!
//! // Analyze script (fast path if only script bindings needed)
//! analyzer.analyze_script(script_source);
//!
//! // Analyze template (requires parsed AST)
//! analyzer.analyze_template(&template_ast);
//!
//! // Get results
//! let summary = analyzer.finish();
//! ```

mod helpers;
mod template;

pub use helpers::{
    extract_identifiers_oxc, extract_inline_callback_params, extract_slot_props,
    is_builtin_directive, is_component_tag, is_keyword, parse_v_for_expression,
};

use crate::analysis::Croquis;
use vize_carton::CompactString;

/// Analysis options for controlling what gets analyzed.
///
/// Use this to skip unnecessary analysis passes for better performance.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnalyzerOptions {
    /// Analyze script bindings (defineProps, defineEmits, etc.)
    pub analyze_script: bool,
    /// Analyze template scopes (v-for, v-slot variables)
    pub analyze_template_scopes: bool,
    /// Track component and directive usage
    pub track_usage: bool,
    /// Detect undefined references (requires script + template)
    pub detect_undefined: bool,
    /// Analyze hoisting opportunities
    pub analyze_hoisting: bool,
    /// Collect template expressions for type checking
    pub collect_template_expressions: bool,
}

impl AnalyzerOptions {
    /// Full analysis (all features enabled)
    #[inline]
    pub const fn full() -> Self {
        Self {
            analyze_script: true,
            analyze_template_scopes: true,
            track_usage: true,
            detect_undefined: true,
            analyze_hoisting: true,
            collect_template_expressions: true,
        }
    }

    /// Minimal analysis for linting (fast)
    #[inline]
    pub const fn for_lint() -> Self {
        Self {
            analyze_script: true,
            analyze_template_scopes: true,
            track_usage: true,
            detect_undefined: true,
            analyze_hoisting: false,
            collect_template_expressions: false,
        }
    }

    /// Analysis for compilation (needs hoisting)
    #[inline]
    pub const fn for_compile() -> Self {
        Self {
            analyze_script: true,
            analyze_template_scopes: true,
            track_usage: true,
            detect_undefined: false,
            analyze_hoisting: true,
            collect_template_expressions: false,
        }
    }
}

/// High-performance Vue SFC analyzer.
///
/// Uses lazy evaluation and efficient data structures to minimize overhead.
pub struct Analyzer {
    pub(crate) options: AnalyzerOptions,
    pub(crate) summary: Croquis,
    /// Track if script was analyzed (for undefined detection)
    pub(crate) script_analyzed: bool,
    /// Current v-if guard stack (for type narrowing in templates)
    pub(crate) vif_guard_stack: Vec<CompactString>,
}

impl Analyzer {
    /// Create a new analyzer with default options
    #[inline]
    pub fn new() -> Self {
        Self::with_options(AnalyzerOptions::default())
    }

    /// Create analyzer with specific options
    #[inline]
    pub fn with_options(options: AnalyzerOptions) -> Self {
        Self {
            options,
            summary: Croquis::new(),
            script_analyzed: false,
            vif_guard_stack: Vec::new(),
        }
    }

    /// Get the current v-if guard (combined from stack)
    pub(crate) fn current_vif_guard(&self) -> Option<CompactString> {
        if self.vif_guard_stack.is_empty() {
            None
        } else {
            Some(CompactString::new(self.vif_guard_stack.join(" && ")))
        }
    }

    /// Create analyzer for linting (optimized)
    #[inline]
    pub fn for_lint() -> Self {
        Self::with_options(AnalyzerOptions::for_lint())
    }

    /// Create analyzer for compilation
    #[inline]
    pub fn for_compile() -> Self {
        Self::with_options(AnalyzerOptions::for_compile())
    }

    /// Analyze script setup source code.
    ///
    /// This uses OXC parser to extract:
    /// - defineProps/defineEmits/defineModel calls
    /// - Top-level bindings (const, let, function, class)
    /// - Import statements
    /// - Reactivity wrappers (ref, reactive, computed)
    ///
    /// Performance: OXC provides high-performance AST parsing with accurate span tracking.
    pub fn analyze_script(&mut self, source: &str) -> &mut Self {
        self.analyze_script_setup(source)
    }

    /// Analyze script setup source code.
    pub fn analyze_script_setup(&mut self, source: &str) -> &mut Self {
        if !self.options.analyze_script {
            return self;
        }

        self.script_analyzed = true;

        // Use OXC-based parser for accurate AST analysis
        let result = crate::script_parser::parse_script_setup(source);

        // Merge results into summary
        self.summary.bindings = result.bindings;
        self.summary.macros = result.macros;
        self.summary.reactivity = result.reactivity;
        self.summary.type_exports = result.type_exports;
        self.summary.invalid_exports = result.invalid_exports;
        self.summary.scopes = result.scopes;
        self.summary.provide_inject = result.provide_inject;
        self.summary.binding_spans = result.binding_spans;
        self.summary.setup_context = result.setup_context;

        self
    }

    /// Analyze non-script-setup (Options API) source code.
    pub fn analyze_script_plain(&mut self, source: &str) -> &mut Self {
        if !self.options.analyze_script {
            return self;
        }

        self.script_analyzed = true;

        // Use OXC-based parser for non-script-setup
        let result = crate::script_parser::parse_script(source);

        // Merge results into summary
        self.summary.bindings = result.bindings;
        self.summary.macros = result.macros;
        self.summary.reactivity = result.reactivity;
        self.summary.type_exports = result.type_exports;
        self.summary.invalid_exports = result.invalid_exports;
        self.summary.scopes = result.scopes;
        self.summary.provide_inject = result.provide_inject;
        self.summary.binding_spans = result.binding_spans;
        self.summary.setup_context = result.setup_context;

        self
    }

    /// Finish analysis and return the summary.
    ///
    /// Consumes the analyzer.
    #[inline]
    pub fn finish(self) -> Croquis {
        self.summary
    }

    /// Get a reference to the current summary (without consuming).
    #[inline]
    pub fn summary(&self) -> &Croquis {
        &self.summary
    }

    /// Get a mutable reference to the current croquis (analysis result).
    ///
    /// This is primarily used for testing and advanced scenarios where
    /// the caller needs to inject data (e.g., used_components from template parsing).
    #[inline]
    pub fn croquis_mut(&mut self) -> &mut Croquis {
        &mut self.summary
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{InvalidExportKind, TypeExportKind};

    #[test]
    fn test_analyzer_script_bindings() {
        let mut analyzer = Analyzer::for_lint();
        analyzer.analyze_script(
            r#"
            const count = ref(0)
            const name = 'hello'
            let flag = true
            function handleClick() {}
        "#,
        );

        let summary = analyzer.finish();
        assert!(summary.bindings.contains("count"));
        assert!(summary.bindings.contains("name"));
        assert!(summary.bindings.contains("flag"));
        assert!(summary.bindings.contains("handleClick"));

        assert!(summary.reactivity.is_reactive("count"));
        assert!(summary.reactivity.needs_value_access("count"));
    }

    #[test]
    fn test_analyzer_define_props() {
        let mut analyzer = Analyzer::for_lint();
        analyzer.analyze_script(
            r#"
            const props = defineProps<{
                msg: string
                count?: number
            }>()
        "#,
        );

        let summary = analyzer.finish();
        assert_eq!(summary.macros.props().len(), 2);

        let prop_names: Vec<_> = summary
            .macros
            .props()
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert!(prop_names.contains(&"msg"));
        assert!(prop_names.contains(&"count"));
    }

    #[test]
    fn test_type_exports() {
        let mut analyzer = Analyzer::for_lint();
        analyzer.analyze_script(
            r#"
export type Props = {
    msg: string
}
export interface Emits {
    (e: 'update', value: string): void
}
const count = ref(0)
        "#,
        );

        let summary = analyzer.finish();
        assert_eq!(summary.type_exports.len(), 2);

        let type_export = &summary.type_exports[0];
        assert_eq!(type_export.name.as_str(), "Props");
        assert_eq!(type_export.kind, TypeExportKind::Type);
        assert!(type_export.hoisted);

        let interface_export = &summary.type_exports[1];
        assert_eq!(interface_export.name.as_str(), "Emits");
        assert_eq!(interface_export.kind, TypeExportKind::Interface);
        assert!(interface_export.hoisted);
    }

    #[test]
    fn test_invalid_exports() {
        let mut analyzer = Analyzer::for_lint();
        analyzer.analyze_script(
            r#"
export const foo = 'bar'
export let count = 0
export function hello() {}
export class MyClass {}
export default { foo: 'bar' }
const valid = ref(0)
        "#,
        );

        let summary = analyzer.finish();
        assert_eq!(summary.invalid_exports.len(), 5);

        let kinds: Vec<_> = summary.invalid_exports.iter().map(|e| e.kind).collect();
        assert!(kinds.contains(&InvalidExportKind::Const));
        assert!(kinds.contains(&InvalidExportKind::Let));
        assert!(kinds.contains(&InvalidExportKind::Function));
        assert!(kinds.contains(&InvalidExportKind::Class));
        assert!(kinds.contains(&InvalidExportKind::Default));

        let names: Vec<_> = summary
            .invalid_exports
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"count"));
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"MyClass"));
    }

    #[test]
    fn test_mixed_exports() {
        let mut analyzer = Analyzer::for_lint();
        analyzer.analyze_script(
            r#"
export type MyType = string
export const invalid = 123
export interface MyInterface { name: string }
        "#,
        );

        let summary = analyzer.finish();
        assert_eq!(summary.type_exports.len(), 2);
        assert_eq!(summary.invalid_exports.len(), 1);
        assert_eq!(summary.invalid_exports[0].name.as_str(), "invalid");
    }

    #[test]
    fn test_inject_detection_in_script_setup() {
        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
        analyzer.analyze_script_setup(
            r#"import { inject } from 'vue'

const theme = inject('theme')
const { name } = inject('user') as { name: string; id: number }"#,
        );

        let summary = analyzer.finish();
        let injects = summary.provide_inject.injects();

        assert_eq!(injects.len(), 2, "Should detect 2 inject calls");

        assert_eq!(
            injects[0].key,
            crate::provide::ProvideKey::String(vize_carton::CompactString::new("theme"))
        );

        assert_eq!(
            injects[1].key,
            crate::provide::ProvideKey::String(vize_carton::CompactString::new("user"))
        );
        assert!(
            matches!(
                &injects[1].pattern,
                crate::provide::InjectPattern::ObjectDestructure(_)
            ),
            "Should detect object destructure pattern"
        );
    }

    // ========== Snapshot Tests ==========

    #[test]
    fn test_full_analysis_snapshot() {
        use insta::assert_snapshot;

        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
        analyzer.analyze_script(
            r#"import { ref, computed, inject, provide } from 'vue'
import MyComponent from './MyComponent.vue'

const props = defineProps<{
    msg: string
    count?: number
}>()

const emit = defineEmits<{
    (e: 'update', value: string): void
    (e: 'delete'): void
}>()

const model = defineModel<string>()

const counter = ref(0)
const doubled = computed(() => counter.value * 2)
const theme = inject('theme')

provide('counter', counter)

function increment() {
    counter.value++
    emit('update', String(counter.value))
}

export type UserProps = { name: string }
"#,
        );

        let summary = analyzer.finish();

        // Build a readable snapshot
        let mut output = String::new();
        output.push_str("=== Bindings ===\n");
        for (name, ty) in summary.bindings.iter() {
            output.push_str(&std::format!("  {}: {:?}\n", name, ty));
        }

        output.push_str("\n=== Macros ===\n");
        output.push_str(&std::format!("  props: {}\n", summary.macros.props().len()));
        output.push_str(&std::format!("  emits: {}\n", summary.macros.emits().len()));
        output.push_str(&std::format!(
            "  models: {}\n",
            summary.macros.models().len()
        ));

        output.push_str("\n=== Reactivity ===\n");
        for source in summary.reactivity.sources() {
            output.push_str(&std::format!(
                "  {}: kind={:?}, needs_value={}\n",
                source.name,
                source.kind,
                source.kind.needs_value_access()
            ));
        }

        output.push_str("\n=== Provide/Inject ===\n");
        output.push_str(&std::format!(
            "  provides: {}\n",
            summary.provide_inject.provides().len()
        ));
        output.push_str(&std::format!(
            "  injects: {}\n",
            summary.provide_inject.injects().len()
        ));

        output.push_str("\n=== Type Exports ===\n");
        for te in &summary.type_exports {
            output.push_str(&std::format!("  {}: {:?}\n", te.name, te.kind));
        }

        assert_snapshot!(output);
    }

    #[test]
    fn test_props_emits_snapshot() {
        use insta::assert_snapshot;

        let mut analyzer = Analyzer::for_lint();
        analyzer.analyze_script(
            r#"
const props = defineProps({
    title: String,
    count: { type: Number, required: true },
    items: { type: Array, default: () => [] }
})

const emit = defineEmits(['update', 'delete', 'select'])
"#,
        );

        let summary = analyzer.finish();

        let mut output = String::new();
        output.push_str("=== Props ===\n");
        for prop in summary.macros.props() {
            output.push_str(&std::format!(
                "  {}: required={}, has_default={}\n",
                prop.name,
                prop.required,
                prop.default_value.is_some()
            ));
        }

        output.push_str("\n=== Emits ===\n");
        for emit in summary.macros.emits() {
            output.push_str(&std::format!("  {}\n", emit.name));
        }

        assert_snapshot!(output);
    }

    #[test]
    fn test_provide_inject_snapshot() {
        use insta::assert_snapshot;

        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
        analyzer.analyze_script(
            r#"import { provide, inject } from 'vue'

// Simple provide
provide('theme', 'dark')

// Provide with ref
const counter = ref(0)
provide('counter', counter)

// Provide with Symbol key
const KEY = Symbol('key')
provide(KEY, { value: 42 })

// Simple inject
const theme = inject('theme')

// Inject with default
const locale = inject('locale', 'en')

// Inject with destructure
const { name, id } = inject('user') as { name: string; id: number }
"#,
        );

        let summary = analyzer.finish();

        let mut output = String::new();
        output.push_str("=== Provides ===\n");
        for p in summary.provide_inject.provides() {
            output.push_str(&std::format!("  key: {:?}\n", p.key));
        }

        output.push_str("\n=== Injects ===\n");
        for i in summary.provide_inject.injects() {
            output.push_str(&std::format!(
                "  key: {:?}, has_default: {}, pattern: {:?}\n",
                i.key,
                i.default_value.is_some(),
                i.pattern
            ));
        }

        assert_snapshot!(output);
    }

    #[test]
    fn test_vif_guard_in_template() {
        use vize_armature::parse;
        use vize_carton::Bump;

        let allocator = Bump::new();
        let template = r#"<div>
            <p v-if="todo.description">{{ unwrapDescription(todo.description) }}</p>
            <span>{{ todo.title }}</span>
        </div>"#;

        let (root, errors) = parse(&allocator, template);
        assert!(errors.is_empty(), "Template should parse without errors");

        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
        analyzer.analyze_template(&root);
        let summary = analyzer.finish();

        // Find the interpolation expressions
        let expressions: Vec<_> = summary
            .template_expressions
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    crate::analysis::TemplateExpressionKind::Interpolation
                )
            })
            .collect();

        assert_eq!(expressions.len(), 2, "Should have 2 interpolations");

        // First interpolation is inside v-if, should have guard
        let inside_vif = expressions
            .iter()
            .find(|e| e.content.contains("unwrapDescription"))
            .expect("Should find unwrapDescription interpolation");
        assert!(
            inside_vif.vif_guard.is_some(),
            "Interpolation inside v-if should have vif_guard, got: {:?}",
            inside_vif.vif_guard
        );
        assert_eq!(
            inside_vif.vif_guard.as_deref(),
            Some("todo.description"),
            "vif_guard should be the v-if condition"
        );

        // Second interpolation is outside v-if, should NOT have guard
        let outside_vif = expressions
            .iter()
            .find(|e| e.content.contains("todo.title"))
            .expect("Should find todo.title interpolation");
        assert!(
            outside_vif.vif_guard.is_none(),
            "Interpolation outside v-if should NOT have vif_guard"
        );
    }
}
