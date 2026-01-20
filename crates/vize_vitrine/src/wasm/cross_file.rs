//! CrossFileAnalyzer WASM bindings.

use super::{to_js_value, utf8_byte_to_char_offset};
use vize_carton::Bump;
use wasm_bindgen::prelude::*;

/// Analyze multiple Vue SFC files for cross-file issues
#[wasm_bindgen(js_name = "analyzeCrossFile")]
pub fn analyze_cross_file_wasm(files: JsValue, options: JsValue) -> Result<JsValue, JsValue> {
    use vize_atelier_core::parser::parse;
    use vize_atelier_sfc::{parse_sfc, SfcParseOptions};
    use vize_croquis::cross_file::CrossFileAnalyzer;
    use vize_croquis::{Analyzer, AnalyzerOptions};

    // Parse options
    let cross_file_opts = parse_cross_file_options(&options);

    // Create analyzer
    let mut analyzer = CrossFileAnalyzer::new(cross_file_opts);

    // Parse files array from JsValue
    let files_array = js_sys::Array::from(&files);
    let mut file_data: Vec<(String, String)> = Vec::new();

    for i in 0..files_array.length() {
        let file_obj = files_array.get(i);
        let path = js_sys::Reflect::get(&file_obj, &JsValue::from_str("path"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| format!("file_{}.vue", i));
        let source = js_sys::Reflect::get(&file_obj, &JsValue::from_str("source"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();

        file_data.push((path, source));
    }

    // Process each file - for .vue files, analyze both script and template
    // Track script and template offsets for adjusting diagnostic positions later
    let mut script_offsets: std::collections::HashMap<u32, usize> =
        std::collections::HashMap::new();
    // Template spans: (tag_start, content_start) for template positioning
    // - tag_start: position of '<' in <template>
    // - content_start: position right after '>' in <template> (where content begins)
    let mut template_spans: std::collections::HashMap<u32, (usize, usize)> =
        std::collections::HashMap::new();

    for (path, source) in &file_data {
        let std_path = std::path::Path::new(path);
        let is_vue = std_path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("vue"));

        if is_vue {
            // Parse SFC to extract script and template content
            let parse_opts = SfcParseOptions {
                filename: path.clone(),
                ..Default::default()
            };
            if let Ok(descriptor) = parse_sfc(source, parse_opts) {
                // Create single-file analyzer with full options
                let mut single_analyzer = Analyzer::with_options(AnalyzerOptions::full());

                // Extract and analyze script content
                let (script_content, script_start): (&str, usize) =
                    if let Some(ref script_setup) = descriptor.script_setup {
                        single_analyzer.analyze_script_setup(&script_setup.content);
                        (&script_setup.content, script_setup.loc.start)
                    } else if let Some(ref script) = descriptor.script {
                        single_analyzer.analyze_script_plain(&script.content);
                        (&script.content, script.loc.start)
                    } else {
                        ("", 0)
                    };

                // Also analyze the regular <script> block for setup context violations
                // when it exists alongside <script setup>
                let plain_script_violations = if descriptor.script_setup.is_some() {
                    if let Some(ref script) = descriptor.script {
                        // Parse the plain script to detect setup context violations
                        let plain_result =
                            vize_croquis::script_parser::parse_script(&script.content);
                        // Extract violations with adjusted offsets
                        plain_result
                            .setup_context
                            .violations()
                            .iter()
                            .map(|v| {
                                vize_croquis::setup_context::SetupContextViolation {
                                    kind: v.kind,
                                    api_name: v.api_name.clone(),
                                    // Adjust offset to account for script block position
                                    start: v.start + script.loc.start as u32,
                                    end: v.end + script.loc.start as u32,
                                }
                            })
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // Analyze template for component usages (populates used_components)
                if let Some(ref template) = descriptor.template {
                    let allocator = Bump::new();
                    let (root, _errors) = parse(&allocator, &template.content);
                    single_analyzer.analyze_template(&root);
                }

                // Get complete analysis with used_components populated
                let mut analysis = single_analyzer.finish();

                // Merge setup context violations from plain script
                for violation in plain_script_violations {
                    analysis.setup_context.record_violation(
                        violation.kind,
                        violation.api_name,
                        violation.start,
                        violation.end,
                    );
                }

                // Record template opening tag span before adding file
                // Use tag_start and content start (which is right after '>') to cover just <template...>
                let template_span = descriptor
                    .template
                    .as_ref()
                    .map(|t| (t.loc.tag_start, t.loc.start))
                    .unwrap_or((0, 0));

                // Add file with pre-computed analysis
                let file_id = analyzer.add_file_with_analysis(std_path, script_content, analysis);

                // Record the script and template offsets for this file
                script_offsets.insert(file_id.as_u32(), script_start);
                template_spans.insert(file_id.as_u32(), template_span);
            }
        } else {
            // For .ts/.js files, use directly
            analyzer.add_file(std_path, source);
        }
    }

    // Rebuild component usage edges after all files are added
    // This ensures edges are created even when files are processed out of order
    analyzer.rebuild_component_edges();

    // Run cross-file analysis
    let result = analyzer.analyze();

    // Build file path map and content map for JSON output and offset conversion
    let mut file_paths: Vec<String> = Vec::new();
    let mut file_contents: Vec<String> = Vec::new();
    for (path, source) in &file_data {
        file_paths.push(path.clone());
        file_contents.push(source.clone());
    }
    // Also create a map from file_id to index in file_data
    let mut file_id_to_index: std::collections::HashMap<u32, usize> =
        std::collections::HashMap::new();
    for entry in analyzer.registry().iter() {
        // Find the matching file in file_data by path
        let entry_path = entry.path.to_string_lossy();
        for (idx, (path, _)) in file_data.iter().enumerate() {
            if path == entry_path.as_ref() || path.ends_with(entry_path.as_ref()) {
                file_id_to_index.insert(entry.id.as_u32(), idx);
                break;
            }
        }
    }

    // Convert diagnostics to JSON
    // Adjust offsets for .vue files to account for script/template block position
    let diagnostics: Vec<serde_json::Value> = result
        .diagnostics
        .iter()
        .map(|d| {
            let primary_file = file_paths
                .get(d.primary_file.as_u32() as usize)
                .cloned()
                .unwrap_or_default();

            // Determine if this diagnostic is template-related or script-related
            // Template-related diagnostics need template offset, script-related need script offset
            let is_template_diagnostic = is_template_related_diagnostic(&d.kind);
            // Some template diagnostics cover the entire <template> tag (e.g., multi-root)
            let is_template_tag_diagnostic = is_template_tag_span_diagnostic(&d.kind);

            // Adjust primary offset for SFC position (template or script)
            let (adjusted_primary_offset, adjusted_primary_end_offset) =
                if is_template_tag_diagnostic {
                    // For diagnostics that span the entire template tag, use tag_start and tag_end directly
                    let (tag_start, tag_end) = template_spans
                        .get(&d.primary_file.as_u32())
                        .copied()
                        .unwrap_or((0, 0));
                    (tag_start as u32, tag_end as u32)
                } else if is_template_diagnostic {
                    // For template-content diagnostics, add content_start offset
                    // (content_start is the position right after <template>)
                    let (_, content_start) = template_spans
                        .get(&d.primary_file.as_u32())
                        .copied()
                        .unwrap_or((0, 0));
                    (
                        d.primary_offset + content_start as u32,
                        d.primary_end_offset + content_start as u32,
                    )
                } else {
                    // For script diagnostics, add script offset and convert UTF-8 byte offset to char offset
                    let script_offset = script_offsets
                        .get(&d.primary_file.as_u32())
                        .copied()
                        .unwrap_or(0) as u32;

                    // Get the file content for UTF-8 to char offset conversion
                    let file_content = file_id_to_index
                        .get(&d.primary_file.as_u32())
                        .and_then(|idx| file_contents.get(*idx))
                        .map(|s| s.as_str())
                        .unwrap_or("");

                    // Calculate UTF-8 byte offsets first
                    let utf8_start = d.primary_offset + script_offset;
                    let utf8_end = d.primary_end_offset + script_offset;

                    // Convert to character offsets (handles emojis and multi-byte chars)
                    let char_start = utf8_byte_to_char_offset(file_content, utf8_start);
                    let char_end = utf8_byte_to_char_offset(file_content, utf8_end);

                    (char_start, char_end)
                };

            let related_locations: Vec<serde_json::Value> = d
                .related_files
                .iter()
                .map(
                    |(file_id, offset, message): &(
                        vize_croquis::cross_file::FileId,
                        u32,
                        vize_carton::CompactString,
                    )| {
                        let file_path = file_paths
                            .get(file_id.as_u32() as usize)
                            .cloned()
                            .unwrap_or_default();

                        // Related locations use script offsets (they reference components, not template positions)
                        let offset_adjustment =
                            script_offsets.get(&file_id.as_u32()).copied().unwrap_or(0) as u32;
                        let utf8_offset = offset + offset_adjustment;

                        // Convert to character offset
                        let related_content = file_id_to_index
                            .get(&file_id.as_u32())
                            .and_then(|idx| file_contents.get(*idx))
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        let adjusted_offset =
                            utf8_byte_to_char_offset(related_content, utf8_offset);

                        serde_json::json!({
                            "file": file_path,
                            "offset": adjusted_offset,
                            "message": message.as_str(),
                        })
                    },
                )
                .collect();

            let kind_str = diagnostic_kind_to_string(&d.kind);
            // Use the code() method from diagnostics.rs for unified code naming
            let code = d.code();

            serde_json::json!({
                "type": kind_str,
                "code": code,
                "severity": d.severity.display_name(),
                "message": d.message.as_str(),
                "file": primary_file,
                "offset": adjusted_primary_offset,
                "endOffset": adjusted_primary_end_offset,
                "relatedLocations": related_locations,
                "suggestion": d.suggestion.as_ref().map(|s| s.as_str()),
            })
        })
        .collect();

    // Convert circular dependencies
    let circular_deps: Vec<Vec<String>> = result
        .circular_deps
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .filter_map(|id| file_paths.get(id.as_u32() as usize).cloned())
                .collect()
        })
        .collect();

    // Build result JSON
    let output = serde_json::json!({
        "diagnostics": diagnostics,
        "circularDependencies": circular_deps,
        "stats": {
            "filesAnalyzed": result.stats.files_analyzed,
            "vueComponents": result.stats.vue_components,
            "dependencyEdges": result.stats.dependency_edges,
            "errorCount": result.stats.error_count,
            "warningCount": result.stats.warning_count,
            "infoCount": result.stats.info_count,
            "analysisTimeMs": result.stats.analysis_time_ms,
        },
        "filePaths": file_paths,
    });

    to_js_value(&output)
}

/// Parse CrossFileOptions from JsValue
fn parse_cross_file_options(options: &JsValue) -> vize_croquis::cross_file::CrossFileOptions {
    use vize_croquis::cross_file::CrossFileOptions;

    let get_bool = |key: &str| -> bool {
        js_sys::Reflect::get(options, &JsValue::from_str(key))
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    };

    let all_enabled = get_bool("all");
    if all_enabled {
        return CrossFileOptions::all();
    }

    CrossFileOptions {
        fallthrough_attrs: get_bool("fallthroughAttrs"),
        component_emits: get_bool("componentEmits"),
        event_bubbling: get_bool("eventBubbling"),
        provide_inject: get_bool("provideInject"),
        unique_ids: get_bool("uniqueIds"),
        server_client_boundary: get_bool("serverClientBoundary"),
        error_suspense_boundary: get_bool("errorSuspenseBoundary"),
        reactivity_tracking: get_bool("reactivityTracking"),
        setup_context: get_bool("setupContext"),
        circular_dependencies: get_bool("circularDependencies"),
        max_import_depth: js_sys::Reflect::get(options, &JsValue::from_str("maxImportDepth"))
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as usize),
        component_resolution: get_bool("componentResolution"),
        props_validation: get_bool("propsValidation"),
    }
}

/// Convert diagnostic kind to string type
fn diagnostic_kind_to_string(
    kind: &vize_croquis::cross_file::CrossFileDiagnosticKind,
) -> &'static str {
    use vize_croquis::cross_file::CrossFileDiagnosticKind::*;
    match kind {
        // Fallthrough attributes
        UnusedFallthroughAttrs { .. } => "fallthrough-attrs",
        InheritAttrsDisabledUnused => "fallthrough-attrs",
        MultiRootMissingAttrs => "fallthrough-attrs",
        // Component emits
        UndeclaredEmit { .. } => "component-emit",
        UnusedEmit { .. } => "component-emit",
        UnmatchedEventListener { .. } => "component-emit",
        // Event bubbling
        UnhandledEvent { .. } => "event-bubbling",
        EventModifierIssue { .. } => "event-bubbling",
        // Provide/Inject
        UnmatchedInject { .. } => "provide-inject",
        UnusedProvide { .. } => "provide-inject",
        ProvideInjectTypeMismatch { .. } => "provide-inject",
        ProvideInjectWithoutSymbol { .. } => "provide-inject",
        // Unique IDs
        DuplicateElementId { .. } => "unique-ids",
        NonUniqueIdInLoop { .. } => "unique-ids",
        // SSR boundary
        BrowserApiInSsr { .. } => "ssr-boundary",
        AsyncWithoutSuspense { .. } => "ssr-boundary",
        HydrationMismatchRisk { .. } => "ssr-boundary",
        // Error boundary
        UncaughtErrorBoundary => "error-boundary",
        MissingSuspenseBoundary => "error-boundary",
        SuspenseWithoutFallback => "error-boundary",
        // Circular dependency
        CircularDependency { .. } => "circular-dependency",
        DeepImportChain { .. } => "circular-dependency",
        // Component resolution
        UnregisteredComponent { .. } => "component-resolution",
        UnresolvedImport { .. } => "component-resolution",
        // Props validation
        UndeclaredProp { .. } => "props-validation",
        MissingRequiredProp { .. } => "props-validation",
        PropTypeMismatch { .. } => "props-validation",
        // Slot validation
        UndefinedSlot { .. } => "slot-validation",
        // Setup context violations
        ReactivityOutsideSetup { .. } => "setup-context",
        LifecycleOutsideSetup { .. } => "setup-context",
        WatcherOutsideSetup { .. } => "setup-context",
        DependencyInjectionOutsideSetup { .. } => "setup-context",
        ComposableOutsideSetup { .. } => "setup-context",
        // Reactivity loss
        SpreadBreaksReactivity { .. } => "reactivity-loss",
        ReassignmentBreaksReactivity { .. } => "reactivity-loss",
        ValueExtractionBreaksReactivity { .. } => "reactivity-loss",
        DestructuringBreaksReactivity { .. } => "reactivity-loss",
        // Reference escape
        ReactiveReferenceEscapes { .. } => "reference-escape",
        ReactiveObjectMutatedAfterEscape { .. } => "reference-escape",
        // Circular reactive dependency
        CircularReactiveDependency { .. } => "circular-reactive",
        // Watch patterns
        WatchMutationCanBeComputed { .. } => "watch-pattern",
        // DOM access
        DomAccessWithoutNextTick { .. } => "dom-access",
        // Ultra-strict: computed purity
        ComputedHasSideEffects { .. } => "computed-purity",
        // Ultra-strict: module scope
        ReactiveStateAtModuleScope { .. } => "module-scope",
        // Ultra-strict: template ref timing
        TemplateRefAccessedBeforeMount { .. } => "template-ref-timing",
        // Ultra-strict: async boundary
        AsyncBoundaryCrossing { .. } => "async-boundary",
        // Ultra-strict: closure capture
        ClosureCapturesReactive { .. } => "closure-capture",
        // Ultra-strict: object identity
        ObjectIdentityComparison { .. } => "object-identity",
        // Ultra-strict: state export
        ReactiveStateExported { .. } => "state-export",
        // Ultra-strict: shallow reactive
        ShallowReactiveDeepAccess { .. } => "shallow-reactive",
        // Ultra-strict: toRaw mutation
        ToRawMutation { .. } => "to-raw-mutation",
        // Ultra-strict: event listener
        EventListenerWithoutCleanup { .. } => "event-listener-cleanup",
        // Ultra-strict: array mutation
        ArrayMutationNotTriggering { .. } => "array-mutation",
        // Ultra-strict: Pinia
        PiniaGetterWithoutStoreToRefs { .. } => "pinia-store-refs",
        // Ultra-strict: watchEffect
        WatchEffectWithAsync { .. } => "watch-effect-async",
        // Setup context violation (unified)
        SetupContextViolation { .. } => "setup-context",
    }
}

/// Determine if a diagnostic is template-related (uses template offsets)
/// vs script-related (uses script offsets)
fn is_template_related_diagnostic(
    kind: &vize_croquis::cross_file::CrossFileDiagnosticKind,
) -> bool {
    use vize_croquis::cross_file::CrossFileDiagnosticKind::*;
    matches!(
        kind,
        // Template-based diagnostics (positions in template block)
        UnmatchedEventListener { .. }
            | UndeclaredProp { .. }
            | MissingRequiredProp { .. }
            | PropTypeMismatch { .. }
            | UndefinedSlot { .. }
            | UnregisteredComponent { .. }
            | UnusedFallthroughAttrs { .. }
            | MultiRootMissingAttrs
            | InheritAttrsDisabledUnused
    )
}

/// Determine if a diagnostic should span the entire <template> tag
/// (uses tag_start and tag_end directly, not relative offsets)
fn is_template_tag_span_diagnostic(
    kind: &vize_croquis::cross_file::CrossFileDiagnosticKind,
) -> bool {
    use vize_croquis::cross_file::CrossFileDiagnosticKind::*;
    matches!(
        kind,
        // These diagnostics apply to the entire template, not a specific location
        MultiRootMissingAttrs | InheritAttrsDisabledUnused | UnusedFallthroughAttrs { .. }
    )
}
