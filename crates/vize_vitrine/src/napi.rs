//! NAPI bindings for Vue compiler.

#[path = "napi_typecheck.rs"]
mod napi_typecheck;
pub use napi_typecheck::*;

use glob::glob;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use rayon::prelude::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use vize_carton::Bump;

use crate::{CompileResult, CompilerOptions};
use vize_atelier_core::{
    codegen::generate,
    options::{CodegenMode, CodegenOptions, TransformOptions},
    parser::parse,
    transform::transform,
};
use vize_atelier_vapor::{compile_vapor as vapor_compile, VaporCompilerOptions};

/// Compile Vue template to VDom render function
#[napi]
pub fn compile(template: String, options: Option<CompilerOptions>) -> Result<CompileResult> {
    let opts = options.unwrap_or_default();
    let allocator = Bump::new();

    // Parse
    let (mut root, errors) = parse(&allocator, &template);

    if !errors.is_empty() {
        return Err(Error::new(
            Status::GenericFailure,
            format!("Parse errors: {:?}", errors),
        ));
    }

    // Determine mode
    let is_module_mode = opts.mode.as_deref() == Some("module");

    // Transform
    // In module mode, prefix_identifiers defaults to true (like Vue)
    let transform_opts = TransformOptions {
        prefix_identifiers: opts.prefix_identifiers.unwrap_or(is_module_mode),
        hoist_static: opts.hoist_static.unwrap_or(false),
        cache_handlers: opts.cache_handlers.unwrap_or(false),
        scope_id: opts.scope_id.clone().map(|s| s.into()),
        ssr: opts.ssr.unwrap_or(false),
        ..Default::default()
    };
    transform(&allocator, &mut root, transform_opts);

    // Codegen
    let codegen_opts = CodegenOptions {
        mode: if is_module_mode {
            CodegenMode::Module
        } else {
            CodegenMode::Function
        },
        source_map: opts.source_map.unwrap_or(false),
        ssr: opts.ssr.unwrap_or(false),
        ..Default::default()
    };
    let result = generate(&root, codegen_opts);

    // Collect helpers
    let helpers: Vec<String> = root.helpers.iter().map(|h| h.name().to_string()).collect();

    // Build AST JSON
    let ast = build_ast_json(&root);

    Ok(CompileResult {
        code: result.code.to_string(),
        preamble: result.preamble.to_string(),
        ast,
        map: None,
        helpers,
        templates: None,
    })
}

/// Compile Vue template to Vapor mode
#[napi(js_name = "compileVapor")]
pub fn compile_vapor(template: String, options: Option<CompilerOptions>) -> Result<CompileResult> {
    let opts = options.unwrap_or_default();
    let allocator = Bump::new();

    // Use actual Vapor compiler
    let vapor_opts = VaporCompilerOptions {
        prefix_identifiers: opts.prefix_identifiers.unwrap_or(false),
        ssr: opts.ssr.unwrap_or(false),
        ..Default::default()
    };
    let result = vapor_compile(&allocator, &template, vapor_opts);

    if !result.error_messages.is_empty() {
        return Err(Error::new(
            Status::GenericFailure,
            result.error_messages.join("\n"),
        ));
    }

    Ok(CompileResult {
        code: result.code,
        preamble: String::new(),
        ast: serde_json::json!({}),
        map: None,
        helpers: vec![],
        templates: Some(result.templates.iter().map(|s| s.to_string()).collect()),
    })
}

/// Parse template to AST only
#[napi]
pub fn parse_template(
    template: String,
    _options: Option<CompilerOptions>,
) -> Result<serde_json::Value> {
    let allocator = Bump::new();

    let (root, errors) = parse(&allocator, &template);

    if !errors.is_empty() {
        return Err(Error::new(
            Status::GenericFailure,
            format!("Parse errors: {:?}", errors),
        ));
    }

    Ok(build_ast_json(&root))
}

/// SFC parse options for NAPI
#[napi(object)]
#[derive(Default)]
pub struct SfcParseOptionsNapi {
    pub filename: Option<String>,
}

/// SFC compile options for NAPI
#[napi(object)]
#[derive(Default)]
pub struct SfcCompileOptionsNapi {
    pub filename: Option<String>,
    pub source_map: Option<bool>,
    pub ssr: Option<bool>,
    /// Scope ID for scoped CSS (e.g., "data-v-abc123")
    pub scope_id: Option<String>,
}

/// SFC compile result for NAPI
#[napi(object)]
pub struct SfcCompileResultNapi {
    /// Generated JavaScript code
    pub code: String,
    /// Generated CSS (if any)
    pub css: Option<String>,
    /// Compilation errors
    pub errors: Vec<String>,
    /// Compilation warnings
    pub warnings: Vec<String>,
}

/// Parse SFC (.vue file) - returns lightweight result for speed
#[napi(js_name = "parseSfc")]
pub fn parse_sfc(env: Env, source: String, options: Option<SfcParseOptionsNapi>) -> Result<Object> {
    use vize_atelier_sfc::{parse_sfc as sfc_parse, SfcParseOptions};

    let opts = options.unwrap_or_default();
    let parse_opts = SfcParseOptions {
        filename: opts.filename.unwrap_or_else(|| "anonymous.vue".to_string()),
        ..Default::default()
    };

    match sfc_parse(&source, parse_opts) {
        Ok(descriptor) => {
            // Build JS object directly for speed (avoid JSON serialization)
            let mut obj = env.create_object()?;

            obj.set("filename", descriptor.filename.as_ref())?;
            obj.set("source", descriptor.source.as_ref())?;

            // Template
            if let Some(ref template) = descriptor.template {
                let mut tpl_obj = env.create_object()?;
                tpl_obj.set("content", template.content.as_ref())?;
                tpl_obj.set("lang", template.lang.as_deref())?;
                obj.set("template", tpl_obj)?;
            } else {
                obj.set("template", env.get_null()?)?;
            }

            // Script
            if let Some(ref script) = descriptor.script {
                let mut scr_obj = env.create_object()?;
                scr_obj.set("content", script.content.as_ref())?;
                scr_obj.set("lang", script.lang.as_deref())?;
                scr_obj.set("setup", script.setup)?;
                obj.set("script", scr_obj)?;
            } else {
                obj.set("script", env.get_null()?)?;
            }

            // Script Setup
            if let Some(ref script_setup) = descriptor.script_setup {
                let mut scr_obj = env.create_object()?;
                scr_obj.set("content", script_setup.content.as_ref())?;
                scr_obj.set("lang", script_setup.lang.as_deref())?;
                scr_obj.set("setup", script_setup.setup)?;
                obj.set("scriptSetup", scr_obj)?;
            } else {
                obj.set("scriptSetup", env.get_null()?)?;
            }

            // Styles
            let mut styles_arr = env.create_array(descriptor.styles.len() as u32)?;
            for (i, style) in descriptor.styles.iter().enumerate() {
                let mut style_obj = env.create_object()?;
                style_obj.set("content", style.content.as_ref())?;
                style_obj.set("lang", style.lang.as_deref())?;
                style_obj.set("scoped", style.scoped)?;
                style_obj.set("module", style.module.as_deref())?;
                styles_arr.set(i as u32, style_obj)?;
            }
            obj.set("styles", styles_arr)?;

            // Custom blocks
            let mut customs_arr = env.create_array(descriptor.custom_blocks.len() as u32)?;
            for (i, block) in descriptor.custom_blocks.iter().enumerate() {
                let mut block_obj = env.create_object()?;
                block_obj.set("type", block.block_type.as_ref())?;
                block_obj.set("content", block.content.as_ref())?;
                customs_arr.set(i as u32, block_obj)?;
            }
            obj.set("customBlocks", customs_arr)?;

            Ok(obj)
        }
        Err(e) => Err(Error::new(Status::GenericFailure, e.message)),
    }
}

/// Compile SFC (.vue file) to JavaScript - main use case
#[napi(js_name = "compileSfc")]
pub fn compile_sfc(
    source: String,
    options: Option<SfcCompileOptionsNapi>,
) -> Result<SfcCompileResultNapi> {
    use vize_atelier_sfc::{
        compile_sfc as sfc_compile, parse_sfc as sfc_parse, ScriptCompileOptions,
        SfcCompileOptions, SfcParseOptions, StyleCompileOptions, TemplateCompileOptions,
    };

    let opts = options.unwrap_or_default();
    let filename = opts.filename.unwrap_or_else(|| "anonymous.vue".to_string());

    // Parse
    let parse_opts = SfcParseOptions {
        filename: filename.clone(),
        ..Default::default()
    };

    let descriptor = match sfc_parse(&source, parse_opts) {
        Ok(d) => d,
        Err(e) => {
            return Ok(SfcCompileResultNapi {
                code: String::new(),
                css: None,
                errors: vec![e.message],
                warnings: vec![],
            });
        }
    };

    // Compile
    let has_scoped = descriptor.styles.iter().any(|s| s.scoped);
    // Preserve TypeScript in output - let Vite/esbuild handle TS transformation
    // This avoids issues with OXC transformer incorrectly removing code

    // Create compiler options with scope_id for scoped CSS
    let template_compiler_options = if has_scoped {
        opts.scope_id
            .as_ref()
            .map(|scope_id| vize_atelier_dom::DomCompilerOptions {
                scope_id: Some(scope_id.clone().into()),
                ..Default::default()
            })
    } else {
        None
    };

    let compile_opts = SfcCompileOptions {
        parse: SfcParseOptions {
            filename: filename.clone(),
            ..Default::default()
        },
        script: ScriptCompileOptions {
            id: Some(filename.clone()),
            is_ts: true, // Preserve TypeScript
            ..Default::default()
        },
        template: TemplateCompileOptions {
            id: Some(filename.clone()),
            scoped: has_scoped,
            ssr: opts.ssr.unwrap_or(false),
            is_ts: true, // Preserve TypeScript
            compiler_options: template_compiler_options,
            ..Default::default()
        },
        style: StyleCompileOptions {
            id: filename,
            scoped: has_scoped,
            ..Default::default()
        },
    };

    match sfc_compile(&descriptor, compile_opts) {
        Ok(result) => Ok(SfcCompileResultNapi {
            code: result.code,
            css: result.css,
            errors: result.errors.into_iter().map(|e| e.message).collect(),
            warnings: result.warnings.into_iter().map(|e| e.message).collect(),
        }),
        Err(e) => Ok(SfcCompileResultNapi {
            code: String::new(),
            css: None,
            errors: vec![e.message],
            warnings: vec![],
        }),
    }
}

/// Batch compile options for NAPI
#[napi(object)]
#[derive(Default)]
pub struct BatchCompileOptionsNapi {
    pub ssr: Option<bool>,
    pub threads: Option<u32>,
}

/// Batch compile result for NAPI
#[napi(object)]
pub struct BatchCompileResultNapi {
    /// Number of files compiled successfully
    pub success: u32,
    /// Number of files that failed
    pub failed: u32,
    /// Total input bytes
    pub input_bytes: u32,
    /// Total output bytes
    pub output_bytes: u32,
    /// Compilation time in milliseconds
    pub time_ms: f64,
}

/// Input file for batch compilation with results
#[napi(object)]
pub struct BatchFileInputNapi {
    /// File path
    pub path: String,
    /// Source code
    pub source: String,
}

/// Per-file result from batch compilation
#[napi(object)]
pub struct BatchFileResultNapi {
    /// File path
    pub path: String,
    /// Generated JavaScript code
    pub code: String,
    /// Generated CSS (if any)
    pub css: Option<String>,
    /// Scope ID for scoped styles
    pub scope_id: String,
    /// Whether the file has scoped styles
    pub has_scoped: bool,
    /// Compilation errors
    pub errors: Vec<String>,
    /// Compilation warnings
    pub warnings: Vec<String>,
    /// Hash of template content (for HMR)
    pub template_hash: Option<String>,
    /// Hash of style content (for HMR)
    pub style_hash: Option<String>,
    /// Hash of script content (for HMR)
    pub script_hash: Option<String>,
}

/// Batch compile result with per-file results
#[napi(object)]
pub struct BatchCompileResultWithFilesNapi {
    /// Per-file compilation results
    pub results: Vec<BatchFileResultNapi>,
    /// Number of files compiled successfully
    pub success_count: u32,
    /// Number of files that failed
    pub failed_count: u32,
    /// Compilation time in milliseconds
    pub time_ms: f64,
}

/// Batch compile SFC files matching a glob pattern (native multithreading)
#[napi(js_name = "compileSfcBatch")]
pub fn compile_sfc_batch(
    pattern: String,
    options: Option<BatchCompileOptionsNapi>,
) -> Result<BatchCompileResultNapi> {
    use std::time::Instant;
    use vize_atelier_sfc::{
        compile_sfc as sfc_compile, parse_sfc as sfc_parse, ScriptCompileOptions,
        SfcCompileOptions, SfcParseOptions, StyleCompileOptions, TemplateCompileOptions,
    };

    let opts = options.unwrap_or_default();
    let ssr = opts.ssr.unwrap_or(false);

    // Configure thread pool if specified
    if let Some(threads) = opts.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads as usize)
            .build_global()
            .ok(); // Ignore if already configured
    }

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
        return Err(Error::new(
            Status::GenericFailure,
            "No .vue files found matching the pattern",
        ));
    }

    let success = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let input_bytes = AtomicUsize::new(0);
    let output_bytes = AtomicUsize::new(0);

    let start = Instant::now();

    // Compile files in parallel using rayon
    files.par_iter().for_each(|path| {
        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                failed.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        input_bytes.fetch_add(source.len(), Ordering::Relaxed);

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("anonymous.vue")
            .to_string();

        // Parse
        let parse_opts = SfcParseOptions {
            filename: filename.clone(),
            ..Default::default()
        };

        let descriptor = match sfc_parse(&source, parse_opts) {
            Ok(d) => d,
            Err(_) => {
                failed.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        // Compile
        let has_scoped = descriptor.styles.iter().any(|s| s.scoped);
        let compile_opts = SfcCompileOptions {
            parse: SfcParseOptions {
                filename: filename.clone(),
                ..Default::default()
            },
            script: ScriptCompileOptions {
                id: Some(filename.clone()),
                ..Default::default()
            },
            template: TemplateCompileOptions {
                id: Some(filename.clone()),
                scoped: has_scoped,
                ssr,
                ..Default::default()
            },
            style: StyleCompileOptions {
                id: filename,
                scoped: has_scoped,
                ..Default::default()
            },
        };

        match sfc_compile(&descriptor, compile_opts) {
            Ok(result) => {
                success.fetch_add(1, Ordering::Relaxed);
                output_bytes.fetch_add(result.code.len(), Ordering::Relaxed);
            }
            Err(_) => {
                failed.fetch_add(1, Ordering::Relaxed);
            }
        }
    });

    let elapsed = start.elapsed();

    Ok(BatchCompileResultNapi {
        success: success.load(Ordering::Relaxed) as u32,
        failed: failed.load(Ordering::Relaxed) as u32,
        input_bytes: input_bytes.load(Ordering::Relaxed) as u32,
        output_bytes: output_bytes.load(Ordering::Relaxed) as u32,
        time_ms: elapsed.as_secs_f64() * 1000.0,
    })
}

/// Batch compile SFC files with per-file results (in-memory, native multithreading)
#[napi(js_name = "compileSfcBatchWithResults")]
pub fn compile_sfc_batch_with_results(
    files: Vec<BatchFileInputNapi>,
    options: Option<BatchCompileOptionsNapi>,
) -> Result<BatchCompileResultWithFilesNapi> {
    use std::sync::Mutex;
    use std::time::Instant;
    use vize_atelier_sfc::{
        compile_sfc as sfc_compile, parse_sfc as sfc_parse, ScriptCompileOptions,
        SfcCompileOptions, SfcParseOptions, StyleCompileOptions, TemplateCompileOptions,
    };

    let opts = options.unwrap_or_default();
    let ssr = opts.ssr.unwrap_or(false);

    // Configure thread pool if specified
    if let Some(threads) = opts.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads as usize)
            .build_global()
            .ok(); // Ignore if already configured
    }

    let results: Mutex<Vec<BatchFileResultNapi>> = Mutex::new(Vec::with_capacity(files.len()));
    let success_count = AtomicUsize::new(0);
    let failed_count = AtomicUsize::new(0);

    let start = Instant::now();

    // Compile files in parallel using rayon
    files.par_iter().for_each(|file| {
        let filename = &file.path;
        let source = &file.source;

        // Generate scope ID from filename
        let scope_id = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            filename.hash(&mut hasher);
            format!("{:08x}", hasher.finish() & 0xFFFFFFFF)
        };

        let has_scoped = source.contains("<style") && source.contains("scoped");

        // Parse
        let parse_opts = SfcParseOptions {
            filename: filename.clone(),
            ..Default::default()
        };

        let descriptor = match sfc_parse(source, parse_opts) {
            Ok(d) => d,
            Err(e) => {
                failed_count.fetch_add(1, Ordering::Relaxed);
                let mut guard = results.lock().unwrap();
                guard.push(BatchFileResultNapi {
                    path: filename.clone(),
                    code: String::new(),
                    css: None,
                    scope_id,
                    has_scoped,
                    errors: vec![e.message],
                    warnings: vec![],
                    template_hash: None,
                    style_hash: None,
                    script_hash: None,
                });
                return;
            }
        };

        // Compute hashes for HMR
        let template_hash = descriptor.template_hash();
        let style_hash = descriptor.style_hash();
        let script_hash = descriptor.script_hash();

        // Compile
        // Preserve TypeScript in output - let Vite/esbuild handle TS transformation
        let actual_has_scoped = descriptor.styles.iter().any(|s| s.scoped);
        // Create compiler options with scope_id for scoped CSS
        let template_compiler_options = if actual_has_scoped {
            Some(vize_atelier_dom::DomCompilerOptions {
                scope_id: Some(format!("data-v-{}", scope_id).into()),
                ..Default::default()
            })
        } else {
            None
        };

        let compile_opts = SfcCompileOptions {
            parse: SfcParseOptions {
                filename: filename.clone(),
                ..Default::default()
            },
            script: ScriptCompileOptions {
                id: Some(filename.clone()),
                is_ts: true, // Preserve TypeScript
                ..Default::default()
            },
            template: TemplateCompileOptions {
                id: Some(filename.clone()),
                scoped: actual_has_scoped,
                ssr,
                is_ts: true, // Preserve TypeScript
                compiler_options: template_compiler_options,
                ..Default::default()
            },
            style: StyleCompileOptions {
                id: filename.clone(),
                scoped: actual_has_scoped,
                ..Default::default()
            },
        };

        match sfc_compile(&descriptor, compile_opts) {
            Ok(result) => {
                success_count.fetch_add(1, Ordering::Relaxed);
                let mut guard = results.lock().unwrap();
                guard.push(BatchFileResultNapi {
                    path: filename.clone(),
                    code: result.code,
                    css: result.css,
                    scope_id,
                    has_scoped: actual_has_scoped,
                    errors: result.errors.into_iter().map(|e| e.message).collect(),
                    warnings: result.warnings.into_iter().map(|e| e.message).collect(),
                    template_hash,
                    style_hash,
                    script_hash,
                });
            }
            Err(e) => {
                failed_count.fetch_add(1, Ordering::Relaxed);
                let mut guard = results.lock().unwrap();
                guard.push(BatchFileResultNapi {
                    path: filename.clone(),
                    code: String::new(),
                    css: None,
                    scope_id,
                    has_scoped: actual_has_scoped,
                    errors: vec![e.message],
                    warnings: vec![],
                    template_hash,
                    style_hash,
                    script_hash,
                });
            }
        }
    });

    let elapsed = start.elapsed();
    let final_results = results.into_inner().unwrap();

    Ok(BatchCompileResultWithFilesNapi {
        results: final_results,
        success_count: success_count.load(Ordering::Relaxed) as u32,
        failed_count: failed_count.load(Ordering::Relaxed) as u32,
        time_ms: elapsed.as_secs_f64() * 1000.0,
    })
}

// ============================================================================
// Musea (Art file) bindings
// ============================================================================

/// Art parse options for NAPI
#[napi(object)]
#[derive(Default)]
pub struct ArtParseOptionsNapi {
    pub filename: Option<String>,
}

/// Art metadata for NAPI
#[napi(object)]
pub struct ArtMetadataNapi {
    pub title: String,
    pub description: Option<String>,
    pub component: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub status: String,
    pub order: Option<u32>,
}

/// Art variant for NAPI
#[napi(object)]
pub struct ArtVariantNapi {
    pub name: String,
    pub template: String,
    pub is_default: bool,
    pub skip_vrt: bool,
}

/// Art descriptor for NAPI
#[napi(object)]
pub struct ArtDescriptorNapi {
    pub filename: String,
    pub metadata: ArtMetadataNapi,
    pub variants: Vec<ArtVariantNapi>,
    pub has_script_setup: bool,
    pub has_script: bool,
    pub style_count: u32,
}

/// CSF output for NAPI
#[napi(object)]
pub struct CsfOutputNapi {
    pub code: String,
    pub filename: String,
}

/// Parse Art file (*.art.vue)
#[napi(js_name = "parseArt")]
pub fn parse_art(
    source: String,
    options: Option<ArtParseOptionsNapi>,
) -> Result<ArtDescriptorNapi> {
    use vize_musea::{parse_art as musea_parse, ArtParseOptions, ArtStatus, Bump};

    let allocator = Bump::new();
    let opts = options.unwrap_or_default();
    let parse_opts = ArtParseOptions {
        filename: opts
            .filename
            .unwrap_or_else(|| "anonymous.art.vue".to_string()),
    };

    let descriptor = musea_parse(&allocator, &source, parse_opts)
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;

    // Convert to owned types before allocator is dropped
    let metadata = ArtMetadataNapi {
        title: descriptor.metadata.title.to_string(),
        description: descriptor.metadata.description.map(|d| d.to_string()),
        component: descriptor.metadata.component.map(|c| c.to_string()),
        category: descriptor.metadata.category.map(|c| c.to_string()),
        tags: descriptor
            .metadata
            .tags
            .iter()
            .map(|t| t.to_string())
            .collect(),
        status: match descriptor.metadata.status {
            ArtStatus::Draft => "draft".to_string(),
            ArtStatus::Ready => "ready".to_string(),
            ArtStatus::Deprecated => "deprecated".to_string(),
        },
        order: descriptor.metadata.order,
    };

    let variants: Vec<ArtVariantNapi> = descriptor
        .variants
        .iter()
        .map(|v| ArtVariantNapi {
            name: v.name.to_string(),
            template: v.template.to_string(),
            is_default: v.is_default,
            skip_vrt: v.skip_vrt,
        })
        .collect();

    let result = ArtDescriptorNapi {
        filename: descriptor.filename.to_string(),
        metadata,
        variants,
        has_script_setup: descriptor.script_setup.is_some(),
        has_script: descriptor.script.is_some(),
        style_count: descriptor.styles.len() as u32,
    };

    // descriptor is dropped here, then allocator is dropped
    Ok(result)
}

/// Transform Art to Storybook CSF 3.0
#[napi(js_name = "artToCsf")]
pub fn art_to_csf(source: String, options: Option<ArtParseOptionsNapi>) -> Result<CsfOutputNapi> {
    use vize_musea::{parse_art as musea_parse, transform_to_csf, ArtParseOptions, Bump};

    let allocator = Bump::new();
    let opts = options.unwrap_or_default();
    let parse_opts = ArtParseOptions {
        filename: opts
            .filename
            .unwrap_or_else(|| "anonymous.art.vue".to_string()),
    };

    let descriptor = musea_parse(&allocator, &source, parse_opts)
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;

    // transform_to_csf returns owned CsfOutput, so this is safe
    let csf = transform_to_csf(&descriptor);

    // Create result before descriptor and allocator are dropped
    let result = CsfOutputNapi {
        code: csf.code,
        filename: csf.filename,
    };

    Ok(result)
}

/// Doc options for NAPI
#[napi(object)]
#[derive(Default)]
pub struct DocOptionsNapi {
    pub include_source: Option<bool>,
    pub include_templates: Option<bool>,
    pub include_metadata: Option<bool>,
    pub include_toc: Option<bool>,
    pub toc_threshold: Option<u32>,
    pub base_path: Option<String>,
    pub title: Option<String>,
}

/// Doc output for NAPI
#[napi(object)]
pub struct DocOutputNapi {
    pub markdown: String,
    pub filename: String,
    pub title: String,
    pub category: Option<String>,
    pub variant_count: u32,
}

/// Catalog entry for NAPI
#[napi(object)]
pub struct CatalogEntryNapi {
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub status: String,
    pub variant_count: u32,
    pub doc_path: String,
    pub source_path: String,
}

/// Catalog output for NAPI
#[napi(object)]
pub struct CatalogOutputNapi {
    pub markdown: String,
    pub filename: String,
    pub component_count: u32,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
}

/// Generate component documentation from Art source
#[napi(js_name = "generateArtDoc")]
pub fn generate_art_doc(
    source: String,
    art_options: Option<ArtParseOptionsNapi>,
    doc_options: Option<DocOptionsNapi>,
) -> Result<DocOutputNapi> {
    use vize_musea::docs::{generate_component_doc, DocOptions};
    use vize_musea::{parse_art as musea_parse, ArtParseOptions, Bump};

    let allocator = Bump::new();
    let art_opts = art_options.unwrap_or_default();
    let parse_opts = ArtParseOptions {
        filename: art_opts
            .filename
            .unwrap_or_else(|| "anonymous.art.vue".to_string()),
    };

    let descriptor = musea_parse(&allocator, &source, parse_opts)
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;

    let doc_opts = doc_options.unwrap_or_default();
    let opts = DocOptions {
        include_source: doc_opts.include_source.unwrap_or(false),
        include_templates: doc_opts.include_templates.unwrap_or(true),
        include_metadata: doc_opts.include_metadata.unwrap_or(true),
        include_toc: doc_opts.include_toc.unwrap_or(true),
        toc_threshold: doc_opts.toc_threshold.unwrap_or(5) as usize,
        base_path: doc_opts.base_path.unwrap_or_default(),
        title: doc_opts.title,
        include_timestamp: false,
    };

    let output = generate_component_doc(&descriptor, &opts);

    Ok(DocOutputNapi {
        markdown: output.markdown,
        filename: output.filename,
        title: output.title,
        category: output.category,
        variant_count: output.variant_count as u32,
    })
}

/// Generate catalog from multiple Art sources (high-performance batch)
#[napi(js_name = "generateArtCatalog")]
pub fn generate_art_catalog(
    sources: Vec<String>,
    doc_options: Option<DocOptionsNapi>,
) -> Result<CatalogOutputNapi> {
    use vize_musea::docs::{generate_catalog, CatalogEntry, DocOptions};
    use vize_musea::{parse_art as musea_parse, ArtParseOptions, Bump};

    // Single allocator for all parses - efficient memory usage
    let allocator = Bump::new();

    // Parse all sources and collect entries
    let mut entries = Vec::with_capacity(sources.len());
    for (idx, source) in sources.iter().enumerate() {
        let parse_opts = ArtParseOptions {
            filename: format!("component_{}.art.vue", idx),
        };

        if let Ok(descriptor) = musea_parse(&allocator, source, parse_opts) {
            entries.push(CatalogEntry::from_descriptor(&descriptor, ""));
        }
    }

    let doc_opts = doc_options.unwrap_or_default();
    let opts = DocOptions {
        include_source: doc_opts.include_source.unwrap_or(false),
        include_templates: doc_opts.include_templates.unwrap_or(true),
        include_metadata: doc_opts.include_metadata.unwrap_or(true),
        include_toc: doc_opts.include_toc.unwrap_or(true),
        toc_threshold: doc_opts.toc_threshold.unwrap_or(5) as usize,
        base_path: doc_opts.base_path.unwrap_or_default(),
        title: doc_opts.title,
        include_timestamp: false,
    };

    let output = generate_catalog(&entries, &opts);

    Ok(CatalogOutputNapi {
        markdown: output.markdown,
        filename: output.filename,
        component_count: output.component_count as u32,
        categories: output.categories,
        tags: output.tags,
    })
}

/// Batch generate docs with parallel processing
#[napi(js_name = "generateArtDocsBatch")]
pub fn generate_art_docs_batch(
    sources: Vec<String>,
    doc_options: Option<DocOptionsNapi>,
) -> Result<Vec<DocOutputNapi>> {
    use vize_musea::docs::{generate_component_doc, DocOptions};
    use vize_musea::{parse_art as musea_parse, ArtParseOptions, Bump};

    let doc_opts = doc_options.unwrap_or_default();
    let opts = DocOptions {
        include_source: doc_opts.include_source.unwrap_or(false),
        include_templates: doc_opts.include_templates.unwrap_or(true),
        include_metadata: doc_opts.include_metadata.unwrap_or(true),
        include_toc: doc_opts.include_toc.unwrap_or(true),
        toc_threshold: doc_opts.toc_threshold.unwrap_or(5) as usize,
        base_path: doc_opts.base_path.unwrap_or_default(),
        title: doc_opts.title,
        include_timestamp: false,
    };

    // Process in parallel using rayon
    let results: Vec<DocOutputNapi> = sources
        .par_iter()
        .enumerate()
        .filter_map(|(idx, source)| {
            let allocator = Bump::new();
            let parse_opts = ArtParseOptions {
                filename: format!("component_{}.art.vue", idx),
            };

            musea_parse(&allocator, source, parse_opts)
                .ok()
                .map(|descriptor| {
                    let output = generate_component_doc(&descriptor, &opts);
                    DocOutputNapi {
                        markdown: output.markdown,
                        filename: output.filename,
                        title: output.title,
                        category: output.category,
                        variant_count: output.variant_count as u32,
                    }
                })
        })
        .collect();

    Ok(results)
}

// ============================================================================
// Palette (Props Controls) bindings
// ============================================================================

/// Palette options for NAPI
#[napi(object)]
#[derive(Default)]
pub struct PaletteOptionsNapi {
    pub infer_options: Option<bool>,
    pub min_select_values: Option<u32>,
    pub max_select_values: Option<u32>,
    pub group_by_type: Option<bool>,
}

/// Select option for NAPI
#[napi(object)]
pub struct SelectOptionNapi {
    pub label: String,
    pub value: serde_json::Value,
}

/// Range config for NAPI
#[napi(object)]
pub struct RangeConfigNapi {
    pub min: f64,
    pub max: f64,
    pub step: Option<f64>,
}

/// Prop control for NAPI
#[napi(object)]
pub struct PropControlNapi {
    pub name: String,
    pub control: String,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub required: bool,
    pub options: Vec<SelectOptionNapi>,
    pub range: Option<RangeConfigNapi>,
    pub group: Option<String>,
}

/// Palette output for NAPI
#[napi(object)]
pub struct PaletteOutputNapi {
    pub title: String,
    pub controls: Vec<PropControlNapi>,
    pub groups: Vec<String>,
    pub json: String,
    pub typescript: String,
}

/// Generate props palette from Art source
#[napi(js_name = "generateArtPalette")]
pub fn generate_art_palette(
    source: String,
    art_options: Option<ArtParseOptionsNapi>,
    palette_options: Option<PaletteOptionsNapi>,
) -> Result<PaletteOutputNapi> {
    use vize_musea::palette::{generate_palette, ControlKind, PaletteOptions};
    use vize_musea::{parse_art as musea_parse, ArtParseOptions, Bump};

    let allocator = Bump::new();
    let art_opts = art_options.unwrap_or_default();
    let parse_opts = ArtParseOptions {
        filename: art_opts
            .filename
            .unwrap_or_else(|| "anonymous.art.vue".to_string()),
    };

    let descriptor = musea_parse(&allocator, &source, parse_opts)
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;

    let pal_opts = palette_options.unwrap_or_default();
    let opts = PaletteOptions {
        infer_options: pal_opts.infer_options.unwrap_or(true),
        min_select_values: pal_opts.min_select_values.unwrap_or(2) as usize,
        max_select_values: pal_opts.max_select_values.unwrap_or(10) as usize,
        group_by_type: pal_opts.group_by_type.unwrap_or(false),
    };

    let output = generate_palette(&descriptor, &opts);

    // Convert controls to NAPI types
    let controls: Vec<PropControlNapi> = output
        .palette
        .controls
        .iter()
        .map(|c| PropControlNapi {
            name: c.name.clone(),
            control: match c.control {
                ControlKind::Text => "text".to_string(),
                ControlKind::Number => "number".to_string(),
                ControlKind::Boolean => "boolean".to_string(),
                ControlKind::Range => "range".to_string(),
                ControlKind::Select => "select".to_string(),
                ControlKind::Radio => "radio".to_string(),
                ControlKind::Color => "color".to_string(),
                ControlKind::Date => "date".to_string(),
                ControlKind::Object => "object".to_string(),
                ControlKind::Array => "array".to_string(),
                ControlKind::File => "file".to_string(),
                ControlKind::Raw => "raw".to_string(),
            },
            default_value: c.default_value.clone(),
            description: c.description.clone(),
            required: c.required,
            options: c
                .options
                .iter()
                .map(|o| SelectOptionNapi {
                    label: o.label.clone(),
                    value: o.value.clone(),
                })
                .collect(),
            range: c.range.as_ref().map(|r| RangeConfigNapi {
                min: r.min,
                max: r.max,
                step: r.step,
            }),
            group: c.group.clone(),
        })
        .collect();

    Ok(PaletteOutputNapi {
        title: output.palette.title,
        controls,
        groups: output.palette.groups,
        json: output.json,
        typescript: output.typescript,
    })
}

/// Build AST JSON from root node
fn build_ast_json(root: &vize_atelier_core::RootNode<'_>) -> serde_json::Value {
    use vize_atelier_core::TemplateChildNode;

    let children: Vec<serde_json::Value> = root
        .children
        .iter()
        .map(|child| match child {
            TemplateChildNode::Element(el) => serde_json::json!({
                "type": "ELEMENT",
                "tag": el.tag.as_str(),
                "tagType": format!("{:?}", el.tag_type),
                "props": el.props.len(),
                "children": el.children.len(),
                "isSelfClosing": el.is_self_closing,
            }),
            TemplateChildNode::Text(text) => serde_json::json!({
                "type": "TEXT",
                "content": text.content.as_str(),
            }),
            TemplateChildNode::Comment(comment) => serde_json::json!({
                "type": "COMMENT",
                "content": comment.content.as_str(),
            }),
            TemplateChildNode::Interpolation(interp) => serde_json::json!({
                "type": "INTERPOLATION",
                "content": match &interp.content {
                    vize_atelier_core::ExpressionNode::Simple(exp) => exp.content.as_str(),
                    _ => "<compound>",
                }
            }),
            _ => serde_json::json!({
                "type": "UNKNOWN"
            }),
        })
        .collect();

    serde_json::json!({
        "type": "ROOT",
        "children": children,
        "helpers": root.helpers.iter().map(|h| h.name()).collect::<Vec<_>>(),
        "components": root.components.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
        "directives": root.directives.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
    })
}
