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

use serde::Serialize;
use vize_carton::Bump;

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

/// Check props typing.
fn check_props_typing(
    summary: &vize_croquis::AnalysisSummary,
    script_offset: u32,
    result: &mut SfcTypeCheckResult,
    strict: bool,
) {
    use vize_croquis::macros::MacroKind;

    // Find defineProps call
    let define_props = summary
        .macros
        .all_calls()
        .iter()
        .find(|c| matches!(c.kind, MacroKind::DefineProps));

    let Some(define_props) = define_props else {
        return;
    };

    // Check if defineProps has type arguments (TypeScript generic)
    if define_props.type_args.is_some() {
        // Props are fully typed via TypeScript
        return;
    }

    let props = summary.macros.props();

    // defineProps() called without type argument and without runtime props
    if props.is_empty() {
        let (start, end) = (
            define_props.start + script_offset,
            define_props.end + script_offset,
        );

        result.add_diagnostic(SfcTypeDiagnostic {
            severity: if strict {
                SfcTypeSeverity::Error
            } else {
                SfcTypeSeverity::Warning
            },
            message: "defineProps() should have a type definition".to_string(),
            start,
            end,
            code: Some("untyped-props".to_string()),
            help: Some("Use defineProps<{ propName: Type }>() to define prop types".to_string()),
            related: Vec::new(),
        });
        return;
    }

    // Check each prop for runtime type
    for prop in props {
        if prop.prop_type.is_none() {
            let (start, end) = (
                define_props.start + script_offset,
                define_props.end + script_offset,
            );

            result.add_diagnostic(SfcTypeDiagnostic {
                severity: if strict {
                    SfcTypeSeverity::Error
                } else {
                    SfcTypeSeverity::Warning
                },
                message: format!("Prop '{}' should have a type definition", prop.name),
                start,
                end,
                code: Some("untyped-prop".to_string()),
                help: Some(
                    "Use defineProps<{ propName: Type }>() or define runtime type".to_string(),
                ),
                related: Vec::new(),
            });
            break; // Only report once per defineProps
        }
    }
}

/// Check emits typing.
fn check_emits_typing(
    summary: &vize_croquis::AnalysisSummary,
    script_offset: u32,
    result: &mut SfcTypeCheckResult,
    strict: bool,
) {
    use vize_croquis::macros::MacroKind;

    // Find defineEmits call
    let define_emits = summary
        .macros
        .all_calls()
        .iter()
        .find(|c| matches!(c.kind, MacroKind::DefineEmits));

    let Some(define_emits) = define_emits else {
        return;
    };

    // Check if defineEmits has type arguments
    if define_emits.type_args.is_some() {
        // Emits are fully typed via TypeScript
        return;
    }

    let emits = summary.macros.emits();

    // defineEmits() called without type argument and without runtime emits
    if emits.is_empty() {
        let (start, end) = (
            define_emits.start + script_offset,
            define_emits.end + script_offset,
        );

        result.add_diagnostic(SfcTypeDiagnostic {
            severity: if strict {
                SfcTypeSeverity::Error
            } else {
                SfcTypeSeverity::Warning
            },
            message: "defineEmits() should have a type definition".to_string(),
            start,
            end,
            code: Some("untyped-emits".to_string()),
            help: Some("Use defineEmits<{ (e: 'event', payload: Type): void }>()".to_string()),
            related: Vec::new(),
        });
        return;
    }

    // Check each emit for payload type
    for emit in emits {
        if emit.payload_type.is_none() {
            let (start, end) = (
                define_emits.start + script_offset,
                define_emits.end + script_offset,
            );

            result.add_diagnostic(SfcTypeDiagnostic {
                severity: if strict {
                    SfcTypeSeverity::Error
                } else {
                    SfcTypeSeverity::Warning
                },
                message: format!("Emit '{}' should have a type definition", emit.name),
                start,
                end,
                code: Some("untyped-emit".to_string()),
                help: Some("Use defineEmits<{ event: [payload: Type] }>()".to_string()),
                related: Vec::new(),
            });
            break; // Only report once per defineEmits
        }
    }
}

/// Check template bindings for undefined references.
fn check_template_bindings(
    summary: &vize_croquis::AnalysisSummary,
    template_offset: u32,
    result: &mut SfcTypeCheckResult,
    _strict: bool,
) {
    // Report undefined references using croquis scope analysis
    for undef_ref in &summary.undefined_refs {
        result.add_diagnostic(SfcTypeDiagnostic {
            severity: SfcTypeSeverity::Error,
            message: format!(
                "Undefined reference '{}' in {}",
                undef_ref.name, undef_ref.context
            ),
            start: undef_ref.offset + template_offset,
            end: undef_ref.offset + template_offset + undef_ref.name.len() as u32,
            code: Some("undefined-binding".to_string()),
            help: Some(format!(
                "Make sure '{}' is defined in script setup or imported",
                undef_ref.name
            )),
            related: Vec::new(),
        });
    }
}

/// Generate virtual TypeScript using croquis scope information.
///
/// This generates TypeScript code that reflects the actual scope chain
/// from croquis analysis. It wraps everything in a generic setup function
/// to support `<script setup generic="T">` and creates nested scopes
/// using real JavaScript scoping (closures, for-of, etc.).
fn generate_virtual_ts_with_scopes(
    summary: &vize_croquis::AnalysisSummary,
    script_content: Option<&str>,
    script_offset: u32,
    _template_ast: Option<&vize_relief::ast::RootNode<'_>>,
    template_offset: u32,
) -> String {
    let mut ts = String::with_capacity(8192);

    // Extract imports from script content for hoisting
    let (hoisted_imports, script_without_imports, import_end_offset) =
        if let Some(script) = script_content {
            extract_imports_with_offset(script)
        } else {
            (Vec::new(), String::new(), 0)
        };

    // Header
    ts.push_str("// ============================================\n");
    ts.push_str("// Virtual TypeScript for Vue SFC Type Checking\n");
    ts.push_str("// Generated by vize_canon with croquis scopes\n");
    ts.push_str("// ============================================\n\n");

    // Vue Runtime Type Declarations
    ts.push_str("// ========== Vue Runtime Types ==========\n");
    ts.push_str(VUE_RUNTIME_TYPES);
    ts.push_str("\n\n");

    // Hoisted imports from script content
    if !hoisted_imports.is_empty() {
        ts.push_str("// ========== Hoisted Imports ==========\n");
        for import in &hoisted_imports {
            ts.push_str(import);
            ts.push('\n');
        }
        ts.push('\n');
    }

    // ========== Props Interface ==========
    ts.push_str("// ========== Props Interface ==========\n");
    let props = summary.macros.props();
    if !props.is_empty() {
        ts.push_str("interface __VizeProps {\n");
        for prop in props {
            let type_str = prop
                .prop_type
                .as_ref()
                .map(|t| t.as_str())
                .unwrap_or("unknown");
            let optional = if prop.required { "" } else { "?" };
            ts.push_str(&format!("  {}{}: {};\n", prop.name, optional, type_str));
        }
        ts.push_str("}\n\n");
    } else {
        ts.push_str("interface __VizeProps {}\n\n");
    }

    // ========== Emits Interface ==========
    ts.push_str("// ========== Emits Interface ==========\n");
    let emits = summary.macros.emits();
    if !emits.is_empty() {
        ts.push_str("interface __VizeEmits {\n");
        for emit in emits {
            let payload_type = emit
                .payload_type
                .as_ref()
                .map(|t| t.as_str())
                .unwrap_or("void");
            ts.push_str(&format!(
                "  (e: '{}', payload: {}): void;\n",
                emit.name, payload_type
            ));
        }
        ts.push_str("}\n\n");
    } else {
        ts.push_str("interface __VizeEmits {}\n\n");
    }

    // ========== Slots Interface ==========
    ts.push_str("// ========== Slots Interface ==========\n");
    ts.push_str("interface __VizeSlots {\n");
    ts.push_str("  [name: string]: ((props: Record<string, unknown>) => any) | undefined;\n");
    ts.push_str("}\n\n");

    // ========== Generic Setup Function ==========
    // This wraps the entire component in a generic function to support
    // <script setup generic="T extends SomeType">
    ts.push_str("// ========== Generic Setup Function ==========\n");
    ts.push_str("// Supports <script setup generic=\"T\">\n");
    ts.push_str("function __setup<\n");
    ts.push_str("  __Props extends __VizeProps = __VizeProps,\n");
    ts.push_str("  __Emits extends __VizeEmits = __VizeEmits,\n");
    ts.push_str("  __Slots extends __VizeSlots = __VizeSlots,\n");
    ts.push_str(">(\n");
    ts.push_str("  __props: __Props,\n");
    ts.push_str("  __ctx: {\n");
    ts.push_str("    emit: __Emits;\n");
    ts.push_str("    slots: __Slots;\n");
    ts.push_str("    attrs: Record<string, unknown>;\n");
    ts.push_str("    expose: (exposed?: Record<string, unknown>) => void;\n");
    ts.push_str("  }\n");
    ts.push_str(") {\n");

    // ========== Models ==========
    let models = summary.macros.models();
    if !models.is_empty() {
        ts.push_str("  // defineModel bindings\n");
        for model in models {
            let name = if model.name.is_empty() {
                "modelValue"
            } else {
                model.name.as_str()
            };
            let type_str = model
                .model_type
                .as_ref()
                .map(|t| t.as_str())
                .unwrap_or("unknown");
            ts.push_str(&format!(
                "  const {}: import('vue').ModelRef<{}> = undefined!;\n",
                name, type_str
            ));
        }
        ts.push('\n');
    }

    // ========== Script Setup Content (imports hoisted) ==========
    if !script_without_imports.is_empty() {
        ts.push_str("  // ========== Script Setup Content ==========\n");

        // Track positions for source mapping
        // NOTE: We output script content WITHOUT indentation so that character
        // positions map 1:1 with the source. This is required for accurate
        // diagnostic position mapping.
        let gen_start = ts.len();
        let src_start = script_offset as usize + import_end_offset;

        // Output script content directly (no indentation for accurate mapping)
        ts.push_str(&script_without_imports);
        if !script_without_imports.ends_with('\n') {
            ts.push('\n');
        }

        let gen_end = ts.len();
        let src_end = src_start + script_without_imports.len();

        // Emit source map marker for script content
        // Format: // @vize-map: genStart:genEnd -> srcStart:srcEnd
        ts.push_str(&format!(
            "  // @vize-map: {}:{} -> {}:{}\n",
            gen_start, gen_end, src_start, src_end
        ));
        ts.push('\n');
    }

    // ========== Template Function ==========
    // Template is a function inside setup to access all setup bindings
    // and Vue globals. This enables proper scope chain for type checking.
    ts.push_str("  // ========== Template Function ==========\n");
    ts.push_str("  function __template() {\n");
    ts.push_str("    // Vue instance context (template-scoped)\n");
    ts.push_str("    // Properties\n");
    ts.push_str("    const $data: Record<string, unknown> = {};\n");
    ts.push_str("    const $props = __props;\n");
    ts.push_str("    const $el: HTMLElement | undefined = undefined;\n");
    ts.push_str("    const $options: Record<string, unknown> = {};\n");
    ts.push_str("    const $parent: any = undefined;\n");
    ts.push_str("    const $root: any = undefined;\n");
    ts.push_str("    const $slots = __ctx.slots;\n");
    ts.push_str("    const $refs: Record<string, any> = {};\n");
    ts.push_str("    const $attrs = __ctx.attrs;\n");
    ts.push_str("    // Methods\n");
    ts.push_str("    const $emit = __ctx.emit;\n");
    ts.push_str("    const $watch: (source: string | (() => any), callback: (newVal: any, oldVal: any) => void, options?: { immediate?: boolean; deep?: boolean }) => () => void = undefined!;\n");
    ts.push_str("    const $forceUpdate: () => void = undefined!;\n");
    ts.push_str("    const $nextTick: (callback?: () => void) => Promise<void> = undefined!;\n\n");

    // Mark script setup bindings as used (for unused variable detection)
    // This ensures bindings used in template don't trigger "unused" warnings
    // Skip Props/PropsAliased as they are properties of the props object, not standalone variables
    ts.push_str("    // Script setup bindings (mark as used for unused detection)\n");
    for (name, binding_type) in summary.bindings.bindings.iter() {
        // Props bindings are accessed via props.name, not as standalone variables
        if !matches!(
            binding_type,
            vize_croquis::BindingType::Props | vize_croquis::BindingType::PropsAliased
        ) {
            ts.push_str(&format!("    void {};\n", name));
        }
    }
    ts.push('\n');

    // Generate nested template scopes inside __template function
    generate_template_scopes(&mut ts, summary, template_offset, 2);

    // Generate template expressions for type checking
    if !summary.template_expressions.is_empty() {
        ts.push_str("    // ========== Template Expressions ==========\n");
        for expr in &summary.template_expressions {
            let src_start = template_offset + expr.start;
            let src_end = template_offset + expr.end;

            // Output expression comment
            ts.push_str(&format!(
                "    // {:?} @{}:{}\n",
                expr.kind, src_start, src_end
            ));

            // Output expression with source map
            let gen_start = ts.len();
            ts.push_str(&format!("    void ({});\n", expr.content));
            let gen_end = ts.len();

            // Emit source map marker
            ts.push_str(&format!(
                "    // @vize-map: {}:{} -> {}:{}\n",
                gen_start, gen_end, src_start, src_end
            ));
        }
        ts.push('\n');
    }

    ts.push_str("  }\n\n");

    // ========== Return Statement ==========
    ts.push_str("  // Component render return\n");
    ts.push_str("  return __template;\n");

    ts.push_str("}\n\n");

    // ========== Undefined Reference Errors ==========
    // Filter out refs that are defined in a v-for scope (false positives from :key on v-for elements)
    let filtered_refs: Vec<_> = summary
        .undefined_refs
        .iter()
        .filter(|undef| {
            let ref_offset = undef.offset;
            // Check if this ref is within a v-for scope and matches the v-for variable
            !summary.scopes.iter().any(|scope| {
                if let vize_croquis::ScopeData::VFor(data) = scope.data() {
                    // Check if the ref is within the v-for scope span
                    let in_scope = ref_offset >= scope.span.start && ref_offset <= scope.span.end;
                    // Check if the ref name matches any v-for variable
                    let matches_var = data.value_alias == undef.name
                        || data.key_alias.as_ref() == Some(&undef.name)
                        || data.index_alias.as_ref() == Some(&undef.name);
                    in_scope && matches_var
                } else {
                    false
                }
            })
        })
        .collect();

    if !filtered_refs.is_empty() {
        ts.push_str("// ========== Undefined Reference Errors ==========\n");
        ts.push_str("function __undefinedReferenceErrors() {\n");

        for undef in &filtered_refs {
            let src_start = template_offset + undef.offset;
            let src_end = src_start + undef.name.len() as u32;

            ts.push_str(&format!(
                "  // @ts-expect-error TS2304: '{}' is not defined\n",
                undef.name
            ));
            ts.push_str(&format!(
                "  // Context: {} @[{}:{}]\n",
                undef.context, src_start, src_end
            ));
            ts.push_str(&format!("  void {};\n\n", undef.name));
        }

        ts.push_str("}\n");
    }

    ts
}

/// Check if a scope is inside a Vue directive scope (v-for, v-slot, event handler).
/// Returns true if the scope's span is contained within any Vue directive scope.
fn is_inside_vue_directive(
    scope: &vize_croquis::Scope,
    summary: &vize_croquis::AnalysisSummary,
) -> bool {
    use vize_croquis::ScopeKind;

    let vue_directive_kinds = [
        ScopeKind::VFor,
        ScopeKind::VSlot,
        ScopeKind::EventHandler,
        ScopeKind::ClientOnly,
    ];

    // Check if this scope's span is contained within any Vue directive scope
    for other in summary.scopes.iter() {
        if vue_directive_kinds.contains(&other.kind) {
            // Check span containment (other contains scope)
            if other.span.start <= scope.span.start && scope.span.end <= other.span.end {
                return true;
            }
        }
    }
    false
}

/// Generate nested template scopes with proper JavaScript scoping.
/// Uses croquis scope chain to define identifiers in their correct scopes.
fn generate_template_scopes(
    ts: &mut String,
    summary: &vize_croquis::AnalysisSummary,
    template_offset: u32,
    base_indent: usize,
) {
    use vize_croquis::{ScopeData, ScopeKind};

    let indent = "  ".repeat(base_indent);

    // Generate template-specific scopes (v-for, v-slot, event handlers, etc.)
    for scope in summary.scopes.iter() {
        // Skip closures that are not inside a Vue directive scope
        // (e.g., top-level closures in interpolations don't need separate output)
        if scope.kind == ScopeKind::Closure && !is_inside_vue_directive(scope, summary) {
            continue;
        }

        match (scope.kind, scope.data()) {
            // v-for: Real for-of loop with type inference
            (ScopeKind::VFor, ScopeData::VFor(data)) => {
                let src_start = template_offset + scope.span.start;
                let src_end = template_offset + scope.span.end;

                ts.push_str(&format!(
                    "{}// v-for: {} in {} @{}:{}\n",
                    indent, data.value_alias, data.source, src_start, src_end
                ));

                // Use for-of with proper type inference - track source expression position
                let gen_start = ts.len();
                ts.push_str(&format!(
                    "{}for (const {} of {}) {{\n",
                    indent, data.value_alias, data.source
                ));
                let gen_end = ts.len();

                // Emit source map for the for-of statement (covers the source expression)
                ts.push_str(&format!(
                    "{}  // @vize-map: {}:{} -> {}:{}\n",
                    indent, gen_start, gen_end, src_start, src_end
                ));

                // Index and key variables inside the loop
                if let Some(ref index) = data.index_alias {
                    ts.push_str(&format!(
                        "{}  const {}: number = 0; // loop index\n",
                        indent, index
                    ));
                }
                if let Some(ref key) = data.key_alias {
                    ts.push_str(&format!(
                        "{}  const {}: string | number = undefined!; // loop key\n",
                        indent, key
                    ));
                }

                // Output all bindings defined in this scope
                for (name, binding) in scope.bindings() {
                    if name != data.value_alias.as_str()
                        && data.index_alias.as_ref().map(|i| i.as_str()) != Some(name)
                        && data.key_alias.as_ref().map(|k| k.as_str()) != Some(name)
                    {
                        ts.push_str(&format!(
                            "{}  void {}; // {:?}\n",
                            indent, name, binding.binding_type
                        ));
                    }
                }

                // Output :key expression for type checking
                // Key must be string | number for Vue's reconciliation
                if let Some(ref key_expr) = data.key_expression {
                    let key_gen_start = ts.len();
                    ts.push_str(&format!(
                        "{}  const __key: string | number = {}; // :key type constraint\n",
                        indent, key_expr
                    ));
                    let key_gen_end = ts.len();

                    // Source map for key expression
                    ts.push_str(&format!(
                        "{}  // @vize-map: {}:{} -> {}:{}\n",
                        indent, key_gen_start, key_gen_end, src_start, src_end
                    ));
                }

                // Use the iterator value
                ts.push_str(&format!("{}  void {};\n", indent, data.value_alias));

                ts.push_str(&format!("{}}}\n\n", indent));
            }

            // v-slot: IIFE receiving slot props
            (ScopeKind::VSlot, ScopeData::VSlot(data)) => {
                let slot_name = if data.name.is_empty() {
                    "default"
                } else {
                    data.name.as_str()
                };

                ts.push_str(&format!(
                    "{}// v-slot:{} @{}:{}\n",
                    indent,
                    slot_name,
                    template_offset + scope.span.start,
                    template_offset + scope.span.end
                ));

                // IIFE that receives slot props
                ts.push_str(&format!(
                    "{}((__slotProps: Parameters<NonNullable<__Slots['{}']>>[0]) => {{\n",
                    indent, slot_name
                ));

                if !data.prop_names.is_empty() {
                    ts.push_str(&format!("{}  const {{ ", indent));
                    for (i, prop) in data.prop_names.iter().enumerate() {
                        if i > 0 {
                            ts.push_str(", ");
                        }
                        ts.push_str(prop.as_str());
                    }
                    ts.push_str(" } = __slotProps;\n");

                    for prop in &data.prop_names {
                        ts.push_str(&format!("{}  void {};\n", indent, prop));
                    }
                }

                // Output additional bindings from croquis scope
                for (name, binding) in scope.bindings() {
                    if !data.prop_names.iter().any(|p| p.as_str() == name) {
                        ts.push_str(&format!(
                            "{}  void {}; // {:?}\n",
                            indent, name, binding.binding_type
                        ));
                    }
                }

                ts.push_str(&format!("{}}})({{}} as any);\n\n", indent));
            }

            // Event handler: Arrow function with typed $event
            (ScopeKind::EventHandler, ScopeData::EventHandler(data)) => {
                let event_type = get_event_type(&data.event_name);

                // Calculate source position for this handler
                let src_start = template_offset + scope.span.start;
                let src_end = template_offset + scope.span.end;

                ts.push_str(&format!(
                    "{}// @{} @{}:{}\n",
                    indent, data.event_name, src_start, src_end
                ));

                if data.has_implicit_event {
                    ts.push_str(&format!("{}(($event: {}) => {{\n", indent, event_type));

                    // Output the handler expression with $event and source map
                    if let Some(ref expr) = data.handler_expression {
                        let expr_str = expr.as_str();
                        // Check if it's a simple identifier (method reference)
                        let is_simple_identifier = expr_str
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '_' || c == '.');

                        let gen_start = ts.len();
                        if is_simple_identifier && !expr_str.contains('(') {
                            // Simple method reference like "handleClick" -> "handleClick($event)"
                            ts.push_str(&format!("{}  {}($event);\n", indent, expr_str));
                        } else {
                            // Expression already has parens or is complex, output as-is
                            ts.push_str(&format!("{}  {};\n", indent, expr_str));
                        }
                        let gen_end = ts.len();

                        // Emit source map for the expression
                        ts.push_str(&format!(
                            "{}  // @vize-map: {}:{} -> {}:{}\n",
                            indent, gen_start, gen_end, src_start, src_end
                        ));
                    }
                } else if !data.param_names.is_empty() {
                    ts.push_str(&format!("{}((", indent));
                    for (i, param) in data.param_names.iter().enumerate() {
                        if i > 0 {
                            ts.push_str(", ");
                        }
                        if param.as_str() == "$event"
                            || param.as_str() == "e"
                            || param.as_str() == "event"
                        {
                            ts.push_str(&format!("{}: {}", param, event_type));
                        } else {
                            ts.push_str(&format!("{}: unknown", param));
                        }
                    }
                    ts.push_str(") => {\n");

                    // Output the handler expression with source map
                    if let Some(ref expr) = data.handler_expression {
                        let gen_start = ts.len();
                        ts.push_str(&format!("{}  {};\n", indent, expr.as_str()));
                        let gen_end = ts.len();

                        ts.push_str(&format!(
                            "{}  // @vize-map: {}:{} -> {}:{}\n",
                            indent, gen_start, gen_end, src_start, src_end
                        ));
                    }
                } else {
                    ts.push_str(&format!("{}(() => {{\n", indent));

                    // Output the handler expression with source map
                    if let Some(ref expr) = data.handler_expression {
                        let gen_start = ts.len();
                        ts.push_str(&format!("{}  {};\n", indent, expr.as_str()));
                        let gen_end = ts.len();

                        ts.push_str(&format!(
                            "{}  // @vize-map: {}:{} -> {}:{}\n",
                            indent, gen_start, gen_end, src_start, src_end
                        ));
                    }
                }

                ts.push_str(&format!("{}}})();\n\n", indent));
            }

            // Callback: Arrow function
            (ScopeKind::Callback, ScopeData::Callback(data)) => {
                ts.push_str(&format!(
                    "{}// callback: {} @{}:{}\n",
                    indent,
                    data.context,
                    template_offset + scope.span.start,
                    template_offset + scope.span.end
                ));

                ts.push_str(&format!("{}((", indent));
                for (i, param) in data.param_names.iter().enumerate() {
                    if i > 0 {
                        ts.push_str(", ");
                    }
                    ts.push_str(&format!("{}: unknown", param));
                }
                ts.push_str(") => {\n");

                for param in &data.param_names {
                    ts.push_str(&format!("{}  void {};\n", indent, param));
                }

                // Output additional bindings from croquis scope
                for (name, binding) in scope.bindings() {
                    if !data.param_names.iter().any(|p| p.as_str() == name) {
                        ts.push_str(&format!(
                            "{}  void {}; // {:?}\n",
                            indent, name, binding.binding_type
                        ));
                    }
                }

                ts.push_str(&format!("{}}})(undefined!);\n\n", indent));
            }

            // Closure: Function expression
            (ScopeKind::Closure, ScopeData::Closure(data)) => {
                let async_kw = if data.is_async { "async " } else { "" };

                ts.push_str(&format!(
                    "{}// closure @{}:{}\n",
                    indent,
                    template_offset + scope.span.start,
                    template_offset + scope.span.end
                ));

                ts.push_str(&format!("{}{}((", indent, async_kw));
                for (i, param) in data.param_names.iter().enumerate() {
                    if i > 0 {
                        ts.push_str(", ");
                    }
                    ts.push_str(&format!("{}: unknown", param));
                }
                ts.push_str(") => {\n");

                for param in &data.param_names {
                    ts.push_str(&format!("{}  void {};\n", indent, param));
                }

                // Output additional bindings from croquis scope
                for (name, binding) in scope.bindings() {
                    if !data.param_names.iter().any(|p| p.as_str() == name) {
                        ts.push_str(&format!(
                            "{}  void {}; // {:?}\n",
                            indent, name, binding.binding_type
                        ));
                    }
                }

                ts.push_str(&format!("{}}})(undefined!);\n\n", indent));
            }

            // Client-only: onMounted etc.
            (ScopeKind::ClientOnly, ScopeData::ClientOnly(data)) => {
                ts.push_str(&format!(
                    "{}// {} (client-only) @{}:{}\n",
                    indent,
                    data.hook_name,
                    template_offset + scope.span.start,
                    template_offset + scope.span.end
                ));

                ts.push_str(&format!("{}{}(() => {{\n", indent, data.hook_name));
                ts.push_str(&format!(
                    "{}  // Browser APIs available: window, document\n",
                    indent
                ));

                // Output bindings from croquis scope
                for (name, binding) in scope.bindings() {
                    ts.push_str(&format!(
                        "{}  void {}; // {:?}\n",
                        indent, name, binding.binding_type
                    ));
                }

                ts.push_str(&format!("{}}});\n\n", indent));
            }

            _ => {}
        }
    }
}

/// Get TypeScript event type from event name.
fn get_event_type(event_name: &str) -> &'static str {
    match event_name {
        "click" | "dblclick" | "mousedown" | "mouseup" | "mousemove" | "mouseenter"
        | "mouseleave" | "mouseover" | "mouseout" | "contextmenu" => "MouseEvent",
        "keydown" | "keyup" | "keypress" => "KeyboardEvent",
        "input" | "change" | "beforeinput" => "InputEvent",
        "focus" | "blur" | "focusin" | "focusout" => "FocusEvent",
        "submit" | "reset" => "SubmitEvent",
        "scroll" => "Event",
        "wheel" => "WheelEvent",
        "touchstart" | "touchend" | "touchmove" | "touchcancel" => "TouchEvent",
        "drag" | "dragstart" | "dragend" | "dragenter" | "dragleave" | "dragover" | "drop" => {
            "DragEvent"
        }
        "pointerdown" | "pointerup" | "pointermove" | "pointerenter" | "pointerleave"
        | "pointerover" | "pointerout" | "pointercancel" => "PointerEvent",
        "animationstart" | "animationend" | "animationiteration" => "AnimationEvent",
        "transitionstart" | "transitionend" | "transitionrun" | "transitioncancel" => {
            "TransitionEvent"
        }
        "resize" => "UIEvent",
        "copy" | "cut" | "paste" => "ClipboardEvent",
        _ => "Event",
    }
}

/// Extract import statements from script content for hoisting.
/// Returns (hoisted_imports, script_without_imports).
/// Extract imports from script and return (imports, remaining, byte offset where imports end).
fn extract_imports_with_offset(script: &str) -> (Vec<String>, String, usize) {
    let mut imports = Vec::new();
    let mut remaining_lines = Vec::new();
    let mut import_end_byte = 0;
    let mut current_byte = 0;

    for line in script.lines() {
        let line_len = line.len() + 1; // +1 for newline
        let trimmed = line.trim();
        // Check if line starts with import (including `import type`)
        if trimmed.starts_with("import ") || trimmed.starts_with("import{") {
            imports.push(line.to_string());
            import_end_byte = current_byte + line_len;
        } else {
            remaining_lines.push(line);
        }
        current_byte += line_len;
    }

    (imports, remaining_lines.join("\n"), import_end_byte)
}

/// Vue runtime type declarations for type checking.
const VUE_RUNTIME_TYPES: &str = r#"import type {
  Ref,
  ComputedRef,
  UnwrapRef,
  Reactive,
  ShallowRef,
  WritableComputedRef,
} from 'vue';

import {
  ref,
  reactive,
  computed,
  watch,
  watchEffect,
  unref,
  toRef,
  toRefs,
  shallowRef,
  triggerRef,
  customRef,
  readonly,
  onMounted,
  onUnmounted,
  onBeforeMount,
  onBeforeUnmount,
  onUpdated,
  onBeforeUpdate,
  onActivated,
  onDeactivated,
  onErrorCaptured,
  nextTick,
  getCurrentInstance,
  inject,
  provide,
} from 'vue';

type MaybeRef<T> = T | Ref<T>;"#;

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
        assert!(virtual_ts.contains("croquis scopes"));
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
}
