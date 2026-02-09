//! NAPI bindings for type checking.

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::typecheck::{type_check_sfc, TypeCheckOptions, TypeSeverity};

/// Type check options for NAPI
#[napi(object)]
#[derive(Default)]
pub struct TypeCheckOptionsNapi {
    pub filename: Option<String>,
    pub strict: Option<bool>,
    pub include_virtual_ts: Option<bool>,
    pub check_props: Option<bool>,
    pub check_emits: Option<bool>,
    pub check_template_bindings: Option<bool>,
    pub check_reactivity: Option<bool>,
    pub check_setup_context: Option<bool>,
    pub check_invalid_exports: Option<bool>,
    pub check_fallthrough_attrs: Option<bool>,
}

/// Related location for diagnostic (NAPI)
#[napi(object)]
pub struct RelatedLocationNapi {
    pub message: String,
    pub start: u32,
    pub end: u32,
    pub filename: Option<String>,
}

/// Type diagnostic for NAPI
#[napi(object)]
pub struct TypeDiagnosticNapi {
    pub severity: String,
    pub message: String,
    pub start: u32,
    pub end: u32,
    pub code: Option<String>,
    pub help: Option<String>,
    pub related: Vec<RelatedLocationNapi>,
}

/// Type check result for NAPI
#[napi(object)]
pub struct TypeCheckResultNapi {
    pub diagnostics: Vec<TypeDiagnosticNapi>,
    pub virtual_ts: Option<String>,
    pub error_count: u32,
    pub warning_count: u32,
    pub analysis_time_ms: Option<f64>,
}

/// Apply NAPI options to TypeCheckOptions.
fn apply_napi_options(opts: &TypeCheckOptionsNapi, check_opts: &mut TypeCheckOptions) {
    check_opts.strict = opts.strict.unwrap_or(false);
    check_opts.include_virtual_ts = opts.include_virtual_ts.unwrap_or(false);
    check_opts.check_props = opts.check_props.unwrap_or(true);
    check_opts.check_emits = opts.check_emits.unwrap_or(true);
    check_opts.check_template_bindings = opts.check_template_bindings.unwrap_or(true);
    check_opts.check_reactivity = opts.check_reactivity.unwrap_or(true);
    check_opts.check_setup_context = opts.check_setup_context.unwrap_or(true);
    check_opts.check_invalid_exports = opts.check_invalid_exports.unwrap_or(true);
    check_opts.check_fallthrough_attrs = opts.check_fallthrough_attrs.unwrap_or(true);
}

/// Perform type checking on a Vue SFC
///
/// This performs AST-based type analysis without requiring a TypeScript compiler.
/// For full type checking, use the CLI with tsgo integration.
#[napi(js_name = "typeCheck")]
pub fn type_check_napi(
    source: String,
    options: Option<TypeCheckOptionsNapi>,
) -> Result<TypeCheckResultNapi> {
    let opts = options.unwrap_or_default();
    let filename = opts
        .filename
        .as_deref()
        .unwrap_or("anonymous.vue")
        .to_string();

    let mut check_opts = TypeCheckOptions::new(filename);
    apply_napi_options(&opts, &mut check_opts);

    let result = type_check_sfc(&source, &check_opts);

    Ok(TypeCheckResultNapi {
        diagnostics: result
            .diagnostics
            .into_iter()
            .map(|d| TypeDiagnosticNapi {
                severity: match d.severity {
                    TypeSeverity::Error => "error".to_string(),
                    TypeSeverity::Warning => "warning".to_string(),
                    TypeSeverity::Info => "info".to_string(),
                    TypeSeverity::Hint => "hint".to_string(),
                },
                message: d.message,
                start: d.start,
                end: d.end,
                code: d.code,
                help: d.help,
                related: d
                    .related
                    .into_iter()
                    .map(|r| RelatedLocationNapi {
                        message: r.message,
                        start: r.start,
                        end: r.end,
                        filename: r.filename,
                    })
                    .collect(),
            })
            .collect(),
        virtual_ts: result.virtual_ts,
        error_count: result.error_count as u32,
        warning_count: result.warning_count as u32,
        analysis_time_ms: result.analysis_time_ms,
    })
}

/// Type check capabilities info
#[napi(object)]
pub struct TypeCheckCapabilityNapi {
    pub name: String,
    pub description: String,
    pub severity: String,
}

/// Type check capabilities result
#[napi(object)]
pub struct TypeCheckCapabilitiesNapi {
    pub mode: String,
    pub description: String,
    pub checks: Vec<TypeCheckCapabilityNapi>,
    pub notes: Vec<String>,
}

/// Get type checking capabilities info
#[napi(js_name = "getTypeCheckCapabilities")]
pub fn get_type_check_capabilities_napi() -> TypeCheckCapabilitiesNapi {
    TypeCheckCapabilitiesNapi {
        mode: "ast-based".to_string(),
        description: "AST-based type analysis (no TypeScript compiler required)".to_string(),
        checks: vec![
            TypeCheckCapabilityNapi {
                name: "untyped-props".to_string(),
                description: "Detects props without type definitions".to_string(),
                severity: "warning".to_string(),
            },
            TypeCheckCapabilityNapi {
                name: "untyped-emits".to_string(),
                description: "Detects emits without type definitions".to_string(),
                severity: "warning".to_string(),
            },
            TypeCheckCapabilityNapi {
                name: "undefined-binding".to_string(),
                description: "Detects undefined template bindings".to_string(),
                severity: "error".to_string(),
            },
            TypeCheckCapabilityNapi {
                name: "reactivity-loss".to_string(),
                description:
                    "Detects patterns that lose reactivity (destructuring, spreading, reassigning)"
                        .to_string(),
                severity: "warning".to_string(),
            },
            TypeCheckCapabilityNapi {
                name: "setup-context-violation".to_string(),
                description: "Detects Vue APIs called outside setup context (CSRP, memory leaks)"
                    .to_string(),
                severity: "warning/error".to_string(),
            },
            TypeCheckCapabilityNapi {
                name: "invalid-export".to_string(),
                description: "Detects invalid value exports from <script setup>".to_string(),
                severity: "error".to_string(),
            },
            TypeCheckCapabilityNapi {
                name: "fallthrough-attrs".to_string(),
                description: "Detects multi-root components that may lose fallthrough attributes"
                    .to_string(),
                severity: "warning".to_string(),
            },
        ],
        notes: vec![
            "For full TypeScript type checking, use the CLI with tsgo integration".to_string(),
            "AST-based analysis catches common issues without external dependencies".to_string(),
        ],
    }
}

/// Batch type check result for NAPI
#[napi(object)]
pub struct BatchTypeCheckResultNapi {
    pub files_checked: u32,
    pub files_with_errors: u32,
    pub total_errors: u32,
    pub total_warnings: u32,
    pub time_ms: f64,
}

/// Batch type check SFC files matching a glob pattern (native multithreading)
#[napi(js_name = "typeCheckBatch")]
pub fn type_check_batch_napi(
    pattern: String,
    options: Option<TypeCheckOptionsNapi>,
) -> Result<BatchTypeCheckResultNapi> {
    use glob::glob;
    use rayon::prelude::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    let opts = options.unwrap_or_default();

    // Collect files matching the pattern
    let files: Vec<_> = glob(&pattern)
        .map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Invalid glob pattern: {}", e),
            )
        })?
        .filter_map(|entry| entry.ok())
        .filter(|path| path.extension().is_some_and(|ext| ext == "vue"))
        .collect();

    if files.is_empty() {
        return Ok(BatchTypeCheckResultNapi {
            files_checked: 0,
            files_with_errors: 0,
            total_errors: 0,
            total_warnings: 0,
            time_ms: 0.0,
        });
    }

    let files_checked = AtomicUsize::new(0);
    let files_with_errors = AtomicUsize::new(0);
    let total_errors = AtomicUsize::new(0);
    let total_warnings = AtomicUsize::new(0);

    let start = Instant::now();

    // Type check files in parallel using rayon
    files.par_iter().for_each(|path| {
        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return,
        };

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("anonymous.vue")
            .to_string();

        let mut check_opts = TypeCheckOptions::new(filename);
        apply_napi_options(&opts, &mut check_opts);
        check_opts.include_virtual_ts = false; // Don't generate virtual TS for batch

        let result = type_check_sfc(&source, &check_opts);

        files_checked.fetch_add(1, Ordering::Relaxed);
        if result.error_count > 0 {
            files_with_errors.fetch_add(1, Ordering::Relaxed);
        }
        total_errors.fetch_add(result.error_count, Ordering::Relaxed);
        total_warnings.fetch_add(result.warning_count, Ordering::Relaxed);
    });

    let elapsed = start.elapsed();

    Ok(BatchTypeCheckResultNapi {
        files_checked: files_checked.load(Ordering::Relaxed) as u32,
        files_with_errors: files_with_errors.load(Ordering::Relaxed) as u32,
        total_errors: total_errors.load(Ordering::Relaxed) as u32,
        total_warnings: total_warnings.load(Ordering::Relaxed) as u32,
        time_ms: elapsed.as_secs_f64() * 1000.0,
    })
}
