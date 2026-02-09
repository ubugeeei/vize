//! Type checking service for Vue SFC files.
//!
//! Integrates vize_vitrine's strict type checker with the LSP server.
//! Uses croquis for semantic analysis and provides comprehensive type diagnostics.
//! Also supports batch type checking via tsgo CLI.

use tower_lsp::lsp_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location,
    NumberOrString, Position, Range, Url,
};
use vize_vitrine::{type_check_sfc, TypeCheckOptions, TypeSeverity};

use super::IdeContext;
use crate::server::ServerState;

/// Batch type check result summary.
#[cfg(feature = "native")]
#[derive(Debug, Clone, Default)]
pub struct BatchTypeCheckSummary {
    /// Total number of files checked.
    pub file_count: usize,
    /// Number of errors.
    pub error_count: usize,
    /// Number of warnings.
    pub warning_count: usize,
    /// Whether the check succeeded (exit code 0).
    pub success: bool,
}

/// Type checking options for LSP.
#[derive(Debug, Clone)]
pub struct LspTypeCheckOptions {
    /// Enable strict mode (treats warnings as errors)
    pub strict: bool,
    /// Check props type definitions
    pub check_props: bool,
    /// Check emits type definitions
    pub check_emits: bool,
    /// Check template bindings
    pub check_template_bindings: bool,
    /// Check reactivity loss patterns
    pub check_reactivity: bool,
    /// Check setup context violations
    pub check_setup_context: bool,
    /// Check invalid exports in `<script setup>`
    pub check_invalid_exports: bool,
    /// Check fallthrough attrs with multi-root
    pub check_fallthrough_attrs: bool,
}

impl Default for LspTypeCheckOptions {
    fn default() -> Self {
        Self {
            strict: true, // Strict by default for IDE integration
            check_props: true,
            check_emits: true,
            check_template_bindings: true,
            check_reactivity: true,
            check_setup_context: true,
            check_invalid_exports: true,
            check_fallthrough_attrs: true,
        }
    }
}

/// Type checking service for providing type diagnostics and information.
pub struct TypeService;

impl TypeService {
    /// Collect type diagnostics for a document using the strict type checker.
    pub fn collect_diagnostics(state: &ServerState, uri: &Url) -> Vec<Diagnostic> {
        Self::collect_diagnostics_with_options(state, uri, &LspTypeCheckOptions::default())
    }

    /// Collect type diagnostics with custom options.
    pub fn collect_diagnostics_with_options(
        state: &ServerState,
        uri: &Url,
        lsp_options: &LspTypeCheckOptions,
    ) -> Vec<Diagnostic> {
        let Some(doc) = state.documents.get(uri) else {
            return vec![];
        };

        let content = doc.text();

        // Use vize_vitrine's strict type checker
        let options = TypeCheckOptions {
            filename: uri.path().to_string(),
            strict: lsp_options.strict,
            check_props: lsp_options.check_props,
            check_emits: lsp_options.check_emits,
            check_template_bindings: lsp_options.check_template_bindings,
            check_reactivity: lsp_options.check_reactivity,
            check_setup_context: lsp_options.check_setup_context,
            check_invalid_exports: lsp_options.check_invalid_exports,
            check_fallthrough_attrs: lsp_options.check_fallthrough_attrs,
            include_virtual_ts: false,
        };

        let result = type_check_sfc(&content, &options);

        // Convert to LSP diagnostics
        result
            .diagnostics
            .into_iter()
            .map(|diag| {
                let (start_line, start_col) = offset_to_line_col(&content, diag.start as usize);
                let (end_line, end_col) = offset_to_line_col(&content, diag.end as usize);

                // Build related information if present
                let related_information: Option<Vec<DiagnosticRelatedInformation>> = if diag
                    .related
                    .is_empty()
                {
                    None
                } else {
                    Some(
                        diag.related
                            .iter()
                            .map(|rel| {
                                let (rel_start_line, rel_start_col) =
                                    offset_to_line_col(&content, rel.start as usize);
                                let (rel_end_line, rel_end_col) =
                                    offset_to_line_col(&content, rel.end as usize);

                                DiagnosticRelatedInformation {
                                    location: Location {
                                        uri: rel
                                            .filename
                                            .as_ref()
                                            .and_then(|f| Url::parse(&format!("file://{}", f)).ok())
                                            .unwrap_or_else(|| uri.clone()),
                                        range: Range {
                                            start: Position {
                                                line: rel_start_line,
                                                character: rel_start_col,
                                            },
                                            end: Position {
                                                line: rel_end_line,
                                                character: rel_end_col,
                                            },
                                        },
                                    },
                                    message: rel.message.clone(),
                                }
                            })
                            .collect(),
                    )
                };

                // Build help message
                let message = if let Some(ref help) = diag.help {
                    format!("{}\n\nHelp: {}", diag.message, help)
                } else {
                    diag.message.clone()
                };

                // Build code description URL
                let code_description = diag.code.as_ref().map(|code| CodeDescription {
                    href: Url::parse(&format!(
                        "https://github.com/ubugeeei/vize/wiki/type-errors#{}",
                        code
                    ))
                    .unwrap_or_else(|_| Url::parse("https://github.com/ubugeeei/vize").unwrap()),
                });

                Diagnostic {
                    range: Range {
                        start: Position {
                            line: start_line,
                            character: start_col,
                        },
                        end: Position {
                            line: end_line,
                            character: end_col,
                        },
                    },
                    severity: Some(match diag.severity {
                        TypeSeverity::Error => DiagnosticSeverity::ERROR,
                        TypeSeverity::Warning => DiagnosticSeverity::WARNING,
                        TypeSeverity::Info => DiagnosticSeverity::INFORMATION,
                        TypeSeverity::Hint => DiagnosticSeverity::HINT,
                    }),
                    code: diag.code.map(NumberOrString::String),
                    code_description,
                    source: Some("vize/types".to_string()),
                    message,
                    related_information,
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Collect diagnostics using the legacy vize_canon type checker.
    /// This is kept for backwards compatibility and can be removed later.
    #[deprecated(
        note = "Use collect_diagnostics which uses the stricter vize_vitrine type checker"
    )]
    pub fn collect_diagnostics_legacy(state: &ServerState, uri: &Url) -> Vec<Diagnostic> {
        let Some(doc) = state.documents.get(uri) else {
            return vec![];
        };

        let content = doc.text();

        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        let Ok(descriptor) = vize_atelier_sfc::parse_sfc(&content, options) else {
            return vec![];
        };

        let Some(ref template) = descriptor.template else {
            return vec![];
        };

        // Build type context from script
        let ctx = Self::build_type_context(&descriptor);

        // Run type checker
        let checker = vize_canon::TypeChecker::new();
        let result = checker.check_template(&template.content, &ctx);

        // Template block offset
        let template_start_line = template.loc.start_line as u32;

        // Convert to LSP diagnostics
        result
            .diagnostics
            .into_iter()
            .map(|diag| {
                let (start_line, start_col) =
                    offset_to_line_col(&template.content, diag.start as usize);
                let (end_line, end_col) = offset_to_line_col(&template.content, diag.end as usize);

                Diagnostic {
                    range: Range {
                        start: Position {
                            line: template_start_line + start_line - 1,
                            character: start_col,
                        },
                        end: Position {
                            line: template_start_line + end_line - 1,
                            character: end_col,
                        },
                    },
                    severity: Some(match diag.severity {
                        vize_canon::TypeSeverity::Error => DiagnosticSeverity::ERROR,
                        vize_canon::TypeSeverity::Warning => DiagnosticSeverity::WARNING,
                    }),
                    code: Some(NumberOrString::Number(diag.code.code() as i32)),
                    source: Some("vize/types".to_string()),
                    message: diag.message,
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Get type information at a specific position.
    pub fn get_type_at(ctx: &IdeContext) -> Option<vize_canon::TypeInfo> {
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(&ctx.content, options).ok()?;
        let template = descriptor.template.as_ref()?;

        // Check if offset is in template
        let template_start = template.loc.start;
        let template_end = template.loc.end;

        if ctx.offset < template_start || ctx.offset > template_end {
            return None;
        }

        // Convert SFC offset to template-relative offset
        let template_offset = ctx.offset - template_start;

        // Build type context
        let type_ctx = Self::build_type_context(&descriptor);

        // Get type at position
        let checker = vize_canon::TypeChecker::new();
        checker.get_type_at(&template.content, template_offset as u32, &type_ctx)
    }

    /// Get type-aware completions.
    pub fn get_completions(ctx: &IdeContext) -> Vec<vize_canon::CompletionItem> {
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        let Ok(descriptor) = vize_atelier_sfc::parse_sfc(&ctx.content, options) else {
            return vec![];
        };

        let Some(ref template) = descriptor.template else {
            return vec![];
        };

        // Check if offset is in template
        let template_start = template.loc.start;
        let template_end = template.loc.end;

        if ctx.offset < template_start || ctx.offset > template_end {
            return vec![];
        }

        let template_offset = ctx.offset - template_start;

        // Build type context
        let type_ctx = Self::build_type_context(&descriptor);

        // Get completions
        let checker = vize_canon::TypeChecker::new();
        checker.get_completions(&template.content, template_offset as u32, &type_ctx)
    }

    /// Build type context from SFC descriptor.
    fn build_type_context(descriptor: &vize_atelier_sfc::SfcDescriptor) -> vize_canon::TypeContext {
        let mut ctx = vize_canon::TypeContext::new();

        // Extract bindings from script setup
        if let Some(ref script_setup) = descriptor.script_setup {
            Self::extract_bindings_from_script(&script_setup.content, &mut ctx);
        }

        // Extract bindings from regular script (for Options API)
        if let Some(ref script) = descriptor.script {
            Self::extract_bindings_from_script(&script.content, &mut ctx);
        }

        // Add Vue built-in globals
        Self::add_vue_globals(&mut ctx);

        ctx
    }

    /// Extract bindings from script content.
    fn extract_bindings_from_script(script: &str, ctx: &mut vize_canon::TypeContext) {
        // Use simple pattern matching to extract bindings
        // This is a simplified version - full implementation would use a proper parser

        // Find const/let/var declarations
        for pattern in ["const ", "let ", "var "] {
            let mut pos = 0;
            while let Some(start) = script[pos..].find(pattern) {
                let abs_start = pos + start + pattern.len();
                let remaining = &script[abs_start..];

                // Extract the identifier
                if let Some(ident) = Self::extract_identifier(remaining) {
                    let kind = match pattern.trim() {
                        "const" => vize_canon::BindingKind::Const,
                        "let" => vize_canon::BindingKind::Let,
                        "var" => vize_canon::BindingKind::Var,
                        _ => vize_canon::BindingKind::Const,
                    };

                    // Try to infer type
                    let type_info = Self::infer_binding_type(remaining, &ident);

                    ctx.add_binding(
                        ident.clone(),
                        vize_canon::Binding::new(ident, type_info, kind),
                    );
                }

                pos = abs_start + 1;
            }
        }

        // Find function declarations
        let mut pos = 0;
        while let Some(start) = script[pos..].find("function ") {
            let abs_start = pos + start + 9;
            let remaining = &script[abs_start..];

            if let Some(ident) = Self::extract_identifier(remaining) {
                ctx.add_binding(
                    ident.clone(),
                    vize_canon::Binding::new(
                        ident,
                        vize_canon::TypeInfo::new(
                            "(...args: any[]) => any",
                            vize_canon::TypeKind::Function,
                        ),
                        vize_canon::BindingKind::Function,
                    ),
                );
            }

            pos = abs_start + 1;
        }

        // Find ref(), computed(), reactive() calls
        for (fn_name, kind) in [
            ("ref(", vize_canon::BindingKind::Ref),
            ("computed(", vize_canon::BindingKind::Computed),
            ("reactive(", vize_canon::BindingKind::Reactive),
        ] {
            let mut search_pos = 0;
            while let Some(fn_pos) = script[search_pos..].find(fn_name) {
                let abs_fn_pos = search_pos + fn_pos;

                // Look backwards for the binding name
                if let Some(binding_name) = Self::find_binding_before(script, abs_fn_pos) {
                    let type_info = match kind {
                        vize_canon::BindingKind::Ref => {
                            vize_canon::TypeInfo::new("Ref<unknown>", vize_canon::TypeKind::Ref)
                        }
                        vize_canon::BindingKind::Computed => vize_canon::TypeInfo::new(
                            "ComputedRef<unknown>",
                            vize_canon::TypeKind::Computed,
                        ),
                        vize_canon::BindingKind::Reactive => vize_canon::TypeInfo::new(
                            "Reactive<unknown>",
                            vize_canon::TypeKind::Reactive,
                        ),
                        _ => vize_canon::TypeInfo::unknown(),
                    };

                    ctx.add_binding(
                        binding_name.clone(),
                        vize_canon::Binding::new(binding_name, type_info, kind),
                    );
                }

                search_pos = abs_fn_pos + fn_name.len();
            }
        }
    }

    /// Extract an identifier from the start of a string.
    fn extract_identifier(s: &str) -> Option<String> {
        let s = s.trim_start();
        if s.is_empty() {
            return None;
        }

        let bytes = s.as_bytes();
        let first = bytes[0] as char;

        // Must start with letter, underscore, or $
        if !first.is_ascii_alphabetic() && first != '_' && first != '$' {
            return None;
        }

        let mut end = 1;
        while end < bytes.len() {
            let c = bytes[end] as char;
            if !c.is_ascii_alphanumeric() && c != '_' && c != '$' {
                break;
            }
            end += 1;
        }

        Some(s[..end].to_string())
    }

    /// Find the binding name before a function call like ref().
    fn find_binding_before(script: &str, fn_pos: usize) -> Option<String> {
        // Look for pattern like "const name = ref("
        let before = &script[..fn_pos];
        let trimmed = before.trim_end();

        // Should end with "= "
        if !trimmed.ends_with('=') {
            return None;
        }

        let before_eq = trimmed[..trimmed.len() - 1].trim_end();

        // Find the identifier before =
        let mut end = before_eq.len();
        let bytes = before_eq.as_bytes();

        while end > 0 {
            let c = bytes[end - 1] as char;
            if !c.is_ascii_alphanumeric() && c != '_' && c != '$' {
                break;
            }
            end -= 1;
        }

        if end < before_eq.len() {
            Some(before_eq[end..].to_string())
        } else {
            None
        }
    }

    /// Infer type from binding initialization.
    fn infer_binding_type(after_ident: &str, _ident: &str) -> vize_canon::TypeInfo {
        let trimmed = after_ident.trim_start();

        // Check for type annotation
        if trimmed.starts_with(':') {
            // Has type annotation - extract it
            if let Some(eq_pos) = trimmed.find('=') {
                let type_str = trimmed[1..eq_pos].trim();
                return vize_canon::TypeInfo::new(type_str, vize_canon::TypeKind::Unknown);
            }
        }

        // Check for = and infer from value
        if let Some(stripped) = trimmed.strip_prefix('=') {
            let value = stripped.trim_start();

            // String literal
            if value.starts_with('"') || value.starts_with('\'') || value.starts_with('`') {
                return vize_canon::TypeInfo::string();
            }

            // Number
            if value
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                return vize_canon::TypeInfo::number();
            }

            // Boolean
            if value.starts_with("true") || value.starts_with("false") {
                return vize_canon::TypeInfo::boolean();
            }

            // ref()
            if value.starts_with("ref(") {
                return vize_canon::TypeInfo::new("Ref<unknown>", vize_canon::TypeKind::Ref);
            }

            // computed()
            if value.starts_with("computed(") {
                return vize_canon::TypeInfo::new(
                    "ComputedRef<unknown>",
                    vize_canon::TypeKind::Computed,
                );
            }

            // reactive()
            if value.starts_with("reactive(") {
                return vize_canon::TypeInfo::new(
                    "Reactive<unknown>",
                    vize_canon::TypeKind::Reactive,
                );
            }

            // Array literal
            if value.starts_with('[') {
                return vize_canon::TypeInfo::new("unknown[]", vize_canon::TypeKind::Array);
            }

            // Object literal
            if value.starts_with('{') {
                return vize_canon::TypeInfo::new("object", vize_canon::TypeKind::Object);
            }
        }

        vize_canon::TypeInfo::unknown()
    }

    /// Add Vue built-in globals to context.
    fn add_vue_globals(ctx: &mut vize_canon::TypeContext) {
        // Template globals
        ctx.globals.insert(
            "$slots".to_string(),
            vize_canon::TypeInfo::new("Slots", vize_canon::TypeKind::Object),
        );
        ctx.globals.insert(
            "$emit".to_string(),
            vize_canon::TypeInfo::new(
                "(event: string, ...args: any[]) => void",
                vize_canon::TypeKind::Function,
            ),
        );
        ctx.globals.insert(
            "$attrs".to_string(),
            vize_canon::TypeInfo::new("Record<string, unknown>", vize_canon::TypeKind::Object),
        );
        ctx.globals.insert(
            "$refs".to_string(),
            vize_canon::TypeInfo::new(
                "Record<string, Element | ComponentPublicInstance | null>",
                vize_canon::TypeKind::Object,
            ),
        );
        ctx.globals.insert(
            "$el".to_string(),
            vize_canon::TypeInfo::new("Element | null", vize_canon::TypeKind::Object),
        );
        ctx.globals.insert(
            "$parent".to_string(),
            vize_canon::TypeInfo::new(
                "ComponentPublicInstance | null",
                vize_canon::TypeKind::Object,
            ),
        );
        ctx.globals.insert(
            "$root".to_string(),
            vize_canon::TypeInfo::new("ComponentPublicInstance", vize_canon::TypeKind::Object),
        );
        ctx.globals.insert(
            "$data".to_string(),
            vize_canon::TypeInfo::new("object", vize_canon::TypeKind::Object),
        );
        ctx.globals.insert(
            "$options".to_string(),
            vize_canon::TypeInfo::new("ComponentOptions", vize_canon::TypeKind::Object),
        );
        ctx.globals.insert(
            "$props".to_string(),
            vize_canon::TypeInfo::new("object", vize_canon::TypeKind::Object),
        );
        ctx.globals.insert(
            "$watch".to_string(),
            vize_canon::TypeInfo::new("WatchStopHandle", vize_canon::TypeKind::Function),
        );
        ctx.globals.insert(
            "$forceUpdate".to_string(),
            vize_canon::TypeInfo::new("() => void", vize_canon::TypeKind::Function),
        );
        ctx.globals.insert(
            "$nextTick".to_string(),
            vize_canon::TypeInfo::new(
                "(callback?: () => void) => Promise<void>",
                vize_canon::TypeKind::Function,
            ),
        );
    }

    /// Run batch type checking on the entire project.
    ///
    /// This uses tsgo CLI to perform comprehensive TypeScript type checking
    /// on all Vue SFC files in the project. Results are cached for fast access.
    #[cfg(feature = "native")]
    pub fn run_batch_type_check(state: &ServerState) -> Option<BatchTypeCheckSummary> {
        let result = state.run_batch_type_check()?;

        Some(BatchTypeCheckSummary {
            file_count: state
                .get_batch_checker()
                .map(|c| c.read().file_count())
                .unwrap_or(0),
            error_count: result.error_count(),
            warning_count: result.warning_count(),
            success: result.success,
        })
    }

    /// Check if batch type checking is available.
    #[cfg(feature = "native")]
    pub fn is_batch_available(state: &ServerState) -> bool {
        state.get_batch_checker().is_some()
    }

    /// Get batch type check diagnostics for a specific file.
    #[cfg(feature = "native")]
    pub fn get_batch_diagnostics_for_file(
        state: &ServerState,
        uri: &Url,
    ) -> Vec<vize_canon::BatchDiagnostic> {
        let file_path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return vec![],
        };

        let cache = state.get_batch_cache();
        cache.get_diagnostics(&file_path)
    }

    /// Invalidate batch type check cache.
    /// Should be called when files are modified.
    #[cfg(feature = "native")]
    pub fn invalidate_batch_cache(state: &ServerState) {
        state.invalidate_batch_cache();
    }
}

/// Convert byte offset to (line, column) - line is 1-indexed, column is 0-indexed.
fn offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 1u32;
    let mut col = 0u32;
    let mut current_offset = 0;

    for ch in source.chars() {
        if current_offset >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        current_offset += ch.len_utf8();
    }

    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_identifier() {
        assert_eq!(
            TypeService::extract_identifier("count = 0"),
            Some("count".to_string())
        );
        assert_eq!(
            TypeService::extract_identifier("_private"),
            Some("_private".to_string())
        );
        assert_eq!(
            TypeService::extract_identifier("$refs"),
            Some("$refs".to_string())
        );
        assert_eq!(TypeService::extract_identifier("123abc"), None);
    }

    #[test]
    fn test_infer_binding_type() {
        let t = TypeService::infer_binding_type("= \"hello\"", "msg");
        assert_eq!(t.display, "string");

        let t = TypeService::infer_binding_type("= 42", "count");
        assert_eq!(t.display, "number");

        let t = TypeService::infer_binding_type("= true", "flag");
        assert_eq!(t.display, "boolean");

        let t = TypeService::infer_binding_type("= ref(0)", "count");
        assert_eq!(t.kind, vize_canon::TypeKind::Ref);
    }
}
