//! Diagnostics aggregation from multiple sources.
//!
//! Aggregates diagnostics from:
//! - SFC parser errors
//! - Template parser errors
//! - vize_patina (linter)
//! - Future: vize_canon (type checker)

use tower_lsp::lsp_types::{
    CodeDescription, Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range, Url,
};

use crate::server::ServerState;

/// Diagnostic source identifiers.
pub mod sources {
    pub const SFC_PARSER: &str = "vize/sfc";
    pub const TEMPLATE_PARSER: &str = "vize/template";
    pub const SCRIPT_PARSER: &str = "vize/script";
    pub const LINTER: &str = "vize/lint";
    pub const TYPE_CHECKER: &str = "vize/types";
    pub const MUSEA: &str = "vize/musea";
}

/// Diagnostic severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

impl From<Severity> for DiagnosticSeverity {
    fn from(s: Severity) -> Self {
        match s {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
            Severity::Information => DiagnosticSeverity::INFORMATION,
            Severity::Hint => DiagnosticSeverity::HINT,
        }
    }
}

/// Source position mapping from @vize-map comments.
#[cfg(feature = "native")]
#[derive(Debug, Clone)]
struct SourceMapping {
    /// Byte offset start in SFC
    start: u32,
    /// Byte offset end in SFC
    end: u32,
}

/// Virtual TypeScript generation result with position mapping info.
#[cfg(feature = "native")]
struct VirtualTsResult {
    /// Generated TypeScript code
    code: String,
    /// Line number where user code starts in virtual TS (0-indexed)
    user_code_start_line: u32,
    /// Line number where script starts in original SFC (1-indexed)
    sfc_script_start_line: u32,
    /// Line number where template scope starts in virtual TS (0-indexed)
    template_scope_start_line: u32,
    /// Line-to-source mappings from @vize-map comments
    /// Index is virtual TS line number (0-indexed), value is source position in SFC
    line_mappings: Vec<Option<SourceMapping>>,
    /// Number of import lines skipped from user code (to adjust line mapping)
    skipped_import_lines: u32,
}

/// Diagnostic service for collecting and aggregating diagnostics.
pub struct DiagnosticService;

impl DiagnosticService {
    /// Collect all diagnostics for a document.
    pub fn collect(state: &ServerState, uri: &Url) -> Vec<Diagnostic> {
        let Some(doc) = state.documents.get(uri) else {
            tracing::warn!("collect: document not found for {}", uri);
            return vec![];
        };

        let content = doc.text();
        let mut diagnostics = Vec::new();

        // Check if this is an Art file (*.art.vue)
        let path = uri.path();
        if path.ends_with(".art.vue") {
            // Use Musea-specific diagnostics for Art files
            diagnostics.extend(Self::collect_musea_diagnostics(uri, &content));
            return diagnostics;
        }

        // Standard SFC processing
        // Collect SFC parser diagnostics
        let sfc_diags = Self::collect_sfc_diagnostics(uri, &content);
        tracing::info!("collect: SFC parser diagnostics: {}", sfc_diags.len());
        diagnostics.extend(sfc_diags);

        // Collect template parser diagnostics
        let template_diags = Self::collect_template_diagnostics(uri, &content);
        tracing::info!(
            "collect: template parser diagnostics: {}",
            template_diags.len()
        );
        diagnostics.extend(template_diags);

        // Collect linter diagnostics (vize_patina)
        let lint_diags = Self::collect_lint_diagnostics(uri, &content);
        tracing::info!("collect: patina lint diagnostics: {}", lint_diags.len());
        diagnostics.extend(lint_diags);

        // Collect type checker diagnostics (vize_canon)
        let type_diags = super::TypeService::collect_diagnostics(state, uri);
        tracing::info!("collect: type checker diagnostics: {}", type_diags.len());
        diagnostics.extend(type_diags);

        // Also lint inline <art> blocks in regular .vue files
        let inline_art_diags = Self::collect_inline_art_diagnostics(uri, &content);
        tracing::info!(
            "collect: inline art diagnostics: {}",
            inline_art_diags.len()
        );
        diagnostics.extend(inline_art_diags);

        diagnostics
    }

    /// Collect diagnostics asynchronously (includes tsgo diagnostics when available).
    #[cfg(feature = "native")]
    pub async fn collect_async(state: &ServerState, uri: &Url) -> Vec<Diagnostic> {
        tracing::info!("collect_async: {}", uri);

        // Start with sync diagnostics (patina, etc.)
        let mut diagnostics = Self::collect(state, uri);
        tracing::info!("sync diagnostics count: {}", diagnostics.len());

        // Try to get tsgo diagnostics (with timeout, skip on failure)
        // Use 10s timeout - polling for diagnostics internally uses 5s
        let tsgo_future = Self::collect_tsgo_diagnostics(state, uri);
        match tokio::time::timeout(std::time::Duration::from_secs(10), tsgo_future).await {
            Ok(tsgo_diags) => {
                tracing::info!("tsgo diagnostics count: {}", tsgo_diags.len());
                diagnostics.extend(tsgo_diags);
            }
            Err(_) => {
                tracing::warn!("tsgo diagnostics timed out for {}", uri);
            }
        }

        diagnostics
    }

    /// Collect diagnostics from tsgo LSP.
    #[cfg(feature = "native")]
    async fn collect_tsgo_diagnostics(state: &ServerState, uri: &Url) -> Vec<Diagnostic> {
        tracing::info!("collect_tsgo_diagnostics: {}", uri);

        // Only process .vue files
        if !uri.path().ends_with(".vue") {
            tracing::debug!("skipping non-vue file: {}", uri);
            return vec![];
        }

        // Get document content
        let Some(doc) = state.documents.get(uri) else {
            tracing::warn!("document not found: {}", uri);
            return vec![];
        };
        let content = doc.text();

        // Get tsgo bridge
        tracing::info!("getting tsgo bridge...");
        let Some(bridge) = state.get_tsgo_bridge().await else {
            tracing::warn!("tsgo bridge not available");
            return vec![];
        };
        tracing::info!("tsgo bridge acquired");

        // Generate virtual TypeScript
        let Some(virtual_result) = Self::generate_virtual_ts(uri, &content) else {
            tracing::warn!("failed to generate virtual ts for {}", uri);
            return vec![];
        };
        let virtual_ts = &virtual_result.code;
        let user_code_start_line = virtual_result.user_code_start_line;
        let sfc_script_start_line = virtual_result.sfc_script_start_line;
        let template_scope_start_line = virtual_result.template_scope_start_line;
        let line_mappings = &virtual_result.line_mappings;
        tracing::info!(
            "generated virtual ts ({} bytes), user_code_start={}, sfc_script_start={}, template_scope_start={}, mappings_count={}",
            virtual_ts.len(),
            user_code_start_line,
            sfc_script_start_line,
            template_scope_start_line,
            line_mappings.iter().filter(|m| m.is_some()).count()
        );

        // Create virtual document name (used by tsgo bridge to create the full URI)
        let virtual_name = format!("{}.ts", uri.path());

        // Open or update document in tsgo (uses didChange if already open)
        tracing::info!("opening/updating virtual document: {}", virtual_name);
        let virtual_uri = match bridge
            .open_or_update_virtual_document(&virtual_name, virtual_ts)
            .await
        {
            Ok(uri) => {
                tracing::info!("virtual document opened/updated successfully: {}", uri);
                uri
            }
            Err(e) => {
                tracing::warn!("failed to open/update virtual document: {}", e);
                return vec![];
            }
        };

        // Get diagnostics (will poll for publishDiagnostics notification)
        tracing::info!(
            "waiting for diagnostics from tsgo bridge for {}",
            virtual_uri
        );
        let Ok(tsgo_diags) = bridge.get_diagnostics(&virtual_uri).await else {
            tracing::warn!("failed to get diagnostics from tsgo");
            return vec![];
        };

        tracing::info!(
            "tsgo returned {} raw diagnostics for {}",
            tsgo_diags.len(),
            virtual_uri
        );

        // Log each diagnostic for debugging
        for (i, diag) in tsgo_diags.iter().enumerate() {
            tracing::info!(
                "  raw diag[{}]: line {}-{}, message: {}",
                i,
                diag.range.start.line,
                diag.range.end.line,
                &diag.message[..diag.message.len().min(100)]
            );
        }

        // Helper to convert byte offset to (line, column) - both 0-indexed
        let offset_to_position = |offset: u32| -> (u32, u32) {
            let mut line = 0u32;
            let mut col = 0u32;
            let mut current = 0u32;

            for ch in content.chars() {
                if current >= offset {
                    break;
                }
                if ch == '\n' {
                    line += 1;
                    col = 0;
                } else {
                    col += 1;
                }
                current += ch.len_utf8() as u32;
            }

            (line, col)
        };

        // Convert to LSP diagnostics with proper position mapping
        tsgo_diags
            .into_iter()
            .filter_map(|diag| {
                // Skip diagnostics in preamble (before user script content)
                if diag.range.start.line < user_code_start_line {
                    tracing::debug!(
                        "skipping preamble diagnostic at line {} (user code starts at {}): {}",
                        diag.range.start.line,
                        user_code_start_line,
                        &diag.message[..diag.message.len().min(50)]
                    );
                    return None;
                }

                // Skip warnings about internal generated variables
                // TS6133: 'X' is declared but its value is never read
                // TS6196: 'X' is declared but never used
                let is_unused_warning = diag.message.contains("is declared but")
                    && (diag.message.contains("never read") || diag.message.contains("never used"));
                let is_internal_var = diag.message.contains("'__")
                    || diag.message.contains("'$event'")
                    || diag.message.contains("'$attrs'")
                    || diag.message.contains("'$slots'")
                    || diag.message.contains("'$refs'")
                    || diag.message.contains("'$emit'");

                if is_unused_warning && is_internal_var {
                    tracing::debug!(
                        "skipping internal variable warning: {}",
                        &diag.message[..diag.message.len().min(80)]
                    );
                    return None;
                }

                // Determine if this is a script error or template error
                let is_template_error = diag.range.start.line >= template_scope_start_line;

                let (start_line, end_line, start_char, end_char) = if is_template_error {
                    // Template scope error - try to find source mapping from @vize-map comments
                    let virtual_line = diag.range.start.line as usize;

                    // @vize-map comments are placed AFTER the code line they map.
                    // So for an error at line N, the mapping is at line N (from comment at N+1).
                    // Search forward (down) from the error line to find the mapping.
                    let mapping = (0..=10)
                        .filter_map(|offset| {
                            let search_line = virtual_line + offset;
                            line_mappings.get(search_line).and_then(|m| m.as_ref())
                        })
                        .next();

                    if let Some(src_mapping) = mapping {
                        // Found a source mapping - convert byte offset to line/column
                        let (start_line, start_col) = offset_to_position(src_mapping.start);
                        let (end_line, end_col) = offset_to_position(src_mapping.end);

                        tracing::info!(
                            "template error with mapping: virtual_line={} -> offset {}:{} -> sfc_line={} (message: {})",
                            diag.range.start.line,
                            src_mapping.start,
                            src_mapping.end,
                            start_line,
                            &diag.message[..diag.message.len().min(50)]
                        );
                        (start_line, end_line, start_col, end_col)
                    } else {
                        // No mapping found - skip this diagnostic
                        tracing::debug!(
                            "skipping unmapped template error at line {}: {}",
                            diag.range.start.line,
                            &diag.message[..diag.message.len().min(50)]
                        );
                        return None;
                    }
                } else {
                    // Script error - map using user code offset
                    let user_code_offset = diag.range.start.line.saturating_sub(user_code_start_line);
                    let user_code_offset_end = diag.range.end.line.saturating_sub(user_code_start_line);

                    // sfc_script_start_line is 1-indexed, convert to 0-indexed
                    // Add skipped_import_lines to account for import lines that were moved to module scope
                    let skipped_lines = virtual_result.skipped_import_lines;
                    let start = (sfc_script_start_line.saturating_sub(1)) + user_code_offset + skipped_lines;
                    let end = (sfc_script_start_line.saturating_sub(1)) + user_code_offset_end + skipped_lines;

                    // Adjust character offset: virtual TS adds 2 spaces of indentation
                    let start_ch = diag.range.start.character.saturating_sub(2);
                    let end_ch = diag.range.end.character.saturating_sub(2);

                    tracing::debug!(
                        "script error: virtual_line={} -> sfc_line={} (skipped_imports={}, message: {})",
                        diag.range.start.line,
                        start,
                        skipped_lines,
                        &diag.message[..diag.message.len().min(50)]
                    );
                    (start, end, start_ch, end_ch)
                };

                Some(Diagnostic {
                    range: Range {
                        start: Position {
                            line: start_line,
                            character: start_char,
                        },
                        end: Position {
                            line: end_line,
                            character: end_char,
                        },
                    },
                    severity: diag.severity.map(|s| match s {
                        1 => DiagnosticSeverity::ERROR,
                        2 => DiagnosticSeverity::WARNING,
                        3 => DiagnosticSeverity::INFORMATION,
                        _ => DiagnosticSeverity::HINT,
                    }),
                    source: Some("vize/tsgo".to_string()),
                    message: diag.message,
                    ..Default::default()
                })
            })
            .collect()
    }

    /// Generate virtual TypeScript for a Vue SFC.
    #[cfg(feature = "native")]
    fn generate_virtual_ts(uri: &Url, content: &str) -> Option<VirtualTsResult> {
        use vize_atelier_sfc::{parse_sfc, SfcParseOptions};
        use vize_canon::virtual_ts::generate_virtual_ts;
        use vize_croquis::{Analyzer, AnalyzerOptions};

        let options = SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        let descriptor = parse_sfc(content, options).ok()?;

        // Get script block info
        let (script_content, sfc_script_start_line) = descriptor
            .script_setup
            .as_ref()
            .map(|s| (s.content.as_ref(), s.loc.start_line as u32))
            .or_else(|| {
                descriptor
                    .script
                    .as_ref()
                    .map(|s| (s.content.as_ref(), s.loc.start_line as u32))
            })?;

        let template_block = descriptor.template.as_ref()?;
        let template_offset = template_block.loc.start as u32;

        let allocator = vize_carton::Bump::new();
        let (template_ast, _) = vize_armature::parse(&allocator, &template_block.content);

        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());
        analyzer.analyze_script(script_content);
        analyzer.analyze_template(&template_ast);

        let summary = analyzer.finish();
        let output = generate_virtual_ts(
            &summary,
            Some(script_content),
            Some(&template_ast),
            template_offset,
        );
        let code = output.code;

        // Count import lines in script content (these are moved to module scope)
        // Import lines are skipped from user setup code section
        let skipped_import_lines = Self::count_import_lines(script_content);

        // Find where user code starts in generated virtual TS
        // Look for "// User setup code" comment
        let user_code_start_line = code
            .lines()
            .enumerate()
            .find(|(_, line)| line.contains("// User setup code"))
            .map(|(i, _)| i as u32 + 1) // +1 because user code is on next line
            .unwrap_or(0);

        // Find where template scope starts in generated virtual TS
        // Look for "// Template Scope" or "// ========== Template Scope" comment
        let template_scope_start_line = code
            .lines()
            .enumerate()
            .find(|(_, line)| line.contains("Template Scope"))
            .map(|(i, _)| i as u32)
            .unwrap_or(u32::MAX);

        // Parse @vize-map comments to build line mappings
        // Format: // @vize-map: TYPE -> START:END
        // Where START:END are byte offsets in the SFC
        let line_mappings = Self::parse_vize_map_comments(&code);

        Some(VirtualTsResult {
            code,
            user_code_start_line,
            sfc_script_start_line,
            template_scope_start_line,
            line_mappings,
            skipped_import_lines,
        })
    }

    /// Count the number of import lines in script content.
    /// Handles multi-line imports.
    #[cfg(feature = "native")]
    fn count_import_lines(script: &str) -> u32 {
        let lines: Vec<&str> = script.lines().collect();
        let mut count = 0u32;
        let mut in_import = false;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.starts_with("import ") {
                in_import = true;
                count += 1;
                // Check if this is a single-line import
                if trimmed.ends_with(';') || trimmed.contains(" from ") {
                    in_import = false;
                }
            } else if in_import {
                count += 1;
                // Check if this line ends the import
                if trimmed.ends_with(';') {
                    in_import = false;
                }
            }
        }

        count
    }

    /// Parse @vize-map comments from generated virtual TS code.
    /// Returns a vector where index is line number and value is source mapping.
    #[cfg(feature = "native")]
    fn parse_vize_map_comments(code: &str) -> Vec<Option<SourceMapping>> {
        let mut mappings: Vec<Option<SourceMapping>> = vec![None; code.lines().count()];
        let mut found_count = 0;

        // Parse @vize-map comments without regex
        // Format: // @vize-map: TYPE -> START:END
        for (line_idx, line) in code.lines().enumerate() {
            // Find @vize-map comment
            if let Some(map_idx) = line.find("@vize-map:") {
                // Extract the part after @vize-map:
                let rest = &line[map_idx + "@vize-map:".len()..];

                // Find -> separator
                if let Some(arrow_idx) = rest.find("->") {
                    // Extract START:END part after ->
                    let offsets_part = rest[arrow_idx + 2..].trim();

                    // Parse START:END
                    if let Some(colon_idx) = offsets_part.find(':') {
                        let start_str = offsets_part[..colon_idx].trim();
                        let end_str = offsets_part[colon_idx + 1..].trim();

                        // Remove any trailing non-digit characters
                        let end_str = end_str
                            .chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect::<String>();

                        if let (Ok(start_val), Ok(end_val)) =
                            (start_str.parse::<u32>(), end_str.parse::<u32>())
                        {
                            // The mapping applies to the line BEFORE the comment
                            // (the actual code that will produce the error)
                            if line_idx > 0 {
                                mappings[line_idx - 1] = Some(SourceMapping {
                                    start: start_val,
                                    end: end_val,
                                });
                                found_count += 1;
                                tracing::debug!(
                                    "vize-map: line {} -> offset {}:{} (from: {})",
                                    line_idx - 1,
                                    start_val,
                                    end_val,
                                    &line[..line.len().min(80)]
                                );
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("parse_vize_map_comments: found {} mappings", found_count);
        mappings
    }

    /// Collect diagnostics for Art files (*.art.vue) using vize_patina's MuseaLinter.
    fn collect_musea_diagnostics(_uri: &Url, content: &str) -> Vec<Diagnostic> {
        use vize_patina::rules::musea::MuseaLinter;

        let linter = MuseaLinter::new();
        let result = linter.lint(content);

        result
            .diagnostics
            .into_iter()
            .map(|lint_diag| {
                // Convert byte offset to line/column
                let (start_line, start_col) = offset_to_line_col(content, lint_diag.start as usize);
                let (end_line, end_col) = offset_to_line_col(content, lint_diag.end as usize);

                // Build the diagnostic message with help text
                let message = if let Some(ref help) = lint_diag.help {
                    format!("{}\n\nHelp: {}", lint_diag.message, help)
                } else {
                    lint_diag.message.to_string()
                };

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
                    severity: Some(match lint_diag.severity {
                        vize_patina::Severity::Error => DiagnosticSeverity::ERROR,
                        vize_patina::Severity::Warning => DiagnosticSeverity::WARNING,
                    }),
                    code: Some(NumberOrString::String(lint_diag.rule_name.to_string())),
                    code_description: Some(CodeDescription {
                        href: Url::parse("https://github.com/ubugeeei/vize/wiki/musea-rules")
                            .unwrap_or_else(|_| {
                                Url::parse("https://github.com/ubugeeei/vize").unwrap()
                            }),
                    }),
                    source: Some(sources::MUSEA.to_string()),
                    message,
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Collect diagnostics for inline <art> custom blocks in regular .vue files.
    fn collect_inline_art_diagnostics(uri: &Url, content: &str) -> Vec<Diagnostic> {
        use vize_patina::rules::musea::MuseaLinter;

        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        let Ok(descriptor) = vize_atelier_sfc::parse_sfc(content, options) else {
            return vec![];
        };

        let mut diagnostics = Vec::new();

        for custom in &descriptor.custom_blocks {
            if custom.block_type != "art" {
                continue;
            }

            // Reconstruct the art block content including tags for the linter
            // The linter expects a full art file, so we wrap the content
            let art_content = format!(
                "<art{}>\n{}\n</art>",
                // Reconstruct attributes
                custom.attrs.iter().fold(String::new(), |mut acc, (k, v)| {
                    acc.push_str(&format!(" {}=\"{}\"", k, v));
                    acc
                }),
                custom.content
            );

            let linter = MuseaLinter::new();
            let result = linter.lint(&art_content);

            // Map diagnostics back to the original file positions
            let block_content_start = custom.loc.start;

            for lint_diag in result.diagnostics {
                // The lint_diag offsets are relative to art_content
                // We need to adjust: skip the reconstructed <art ...>\n prefix
                let art_tag_prefix_len = art_content.find('\n').unwrap_or(0) + 1;

                // Only process diagnostics that fall within the content area
                if (lint_diag.start as usize) < art_tag_prefix_len {
                    // Diagnostic is on the <art> tag itself - map to the original tag
                    let (start_line, start_col) = offset_to_line_col(content, custom.loc.tag_start);
                    let (end_line, end_col) =
                        offset_to_line_col(content, custom.loc.tag_end.min(content.len()));

                    let message = if let Some(ref help) = lint_diag.help {
                        format!("{}\n\nHelp: {}", lint_diag.message, help)
                    } else {
                        lint_diag.message.to_string()
                    };

                    diagnostics.push(Diagnostic {
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
                        severity: Some(match lint_diag.severity {
                            vize_patina::Severity::Error => DiagnosticSeverity::ERROR,
                            vize_patina::Severity::Warning => DiagnosticSeverity::WARNING,
                        }),
                        code: Some(NumberOrString::String(lint_diag.rule_name.to_string())),
                        source: Some(sources::MUSEA.to_string()),
                        message,
                        ..Default::default()
                    });
                } else {
                    // Diagnostic is in the content area - map offset to original file
                    let content_relative_start =
                        (lint_diag.start as usize).saturating_sub(art_tag_prefix_len);
                    let content_relative_end =
                        (lint_diag.end as usize).saturating_sub(art_tag_prefix_len);

                    let sfc_start = block_content_start + content_relative_start;
                    let sfc_end = block_content_start + content_relative_end;

                    let (start_line, start_col) =
                        offset_to_line_col(content, sfc_start.min(content.len()));
                    let (end_line, end_col) =
                        offset_to_line_col(content, sfc_end.min(content.len()));

                    let message = if let Some(ref help) = lint_diag.help {
                        format!("{}\n\nHelp: {}", lint_diag.message, help)
                    } else {
                        lint_diag.message.to_string()
                    };

                    diagnostics.push(Diagnostic {
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
                        severity: Some(match lint_diag.severity {
                            vize_patina::Severity::Error => DiagnosticSeverity::ERROR,
                            vize_patina::Severity::Warning => DiagnosticSeverity::WARNING,
                        }),
                        code: Some(NumberOrString::String(lint_diag.rule_name.to_string())),
                        source: Some(sources::MUSEA.to_string()),
                        message,
                        ..Default::default()
                    });
                }
            }
        }

        diagnostics
    }

    /// Collect SFC parser diagnostics.
    fn collect_sfc_diagnostics(uri: &Url, content: &str) -> Vec<Diagnostic> {
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        match vize_atelier_sfc::parse_sfc(content, options) {
            Ok(_) => vec![],
            Err(err) => {
                let range = if let Some(ref loc) = err.loc {
                    Range {
                        start: Position {
                            line: loc.start_line.saturating_sub(1) as u32,
                            character: loc.start_column.saturating_sub(1) as u32,
                        },
                        end: Position {
                            line: loc.end_line.saturating_sub(1) as u32,
                            character: loc.end_column.saturating_sub(1) as u32,
                        },
                    }
                } else {
                    Range::default()
                };

                vec![Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some(sources::SFC_PARSER.to_string()),
                    message: err.message,
                    ..Default::default()
                }]
            }
        }
    }

    /// Collect template parser diagnostics.
    fn collect_template_diagnostics(uri: &Url, content: &str) -> Vec<Diagnostic> {
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        let Ok(descriptor) = vize_atelier_sfc::parse_sfc(content, options) else {
            return vec![];
        };

        let Some(ref template) = descriptor.template else {
            return vec![];
        };

        let allocator = vize_carton::Bump::new();
        let (_, errors) = vize_armature::parse(&allocator, &template.content);

        errors
            .iter()
            .filter_map(|error| {
                let loc = error.loc.as_ref()?;

                // Adjust line numbers based on template block position
                let start_line =
                    (template.loc.start_line as u32) + loc.start.line.saturating_sub(1);
                let end_line = (template.loc.start_line as u32) + loc.end.line.saturating_sub(1);

                Some(Diagnostic {
                    range: Range {
                        start: Position {
                            line: start_line.saturating_sub(1),
                            character: loc.start.column.saturating_sub(1),
                        },
                        end: Position {
                            line: end_line.saturating_sub(1),
                            character: loc.end.column.saturating_sub(1),
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::Number(error.code as i32)),
                    source: Some(sources::TEMPLATE_PARSER.to_string()),
                    message: error.message.clone(),
                    ..Default::default()
                })
            })
            .collect()
    }

    /// Collect linter diagnostics from vize_patina.
    fn collect_lint_diagnostics(uri: &Url, content: &str) -> Vec<Diagnostic> {
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        let Ok(descriptor) = vize_atelier_sfc::parse_sfc(content, options) else {
            return vec![];
        };

        let Some(ref template) = descriptor.template else {
            return vec![];
        };

        // Create linter and lint the template content
        let linter = vize_patina::Linter::new();
        let result = linter.lint_template(&template.content, uri.path());

        // Convert lint diagnostics to LSP diagnostics
        result
            .diagnostics
            .into_iter()
            .map(|lint_diag| {
                // Convert byte offset to line/column within template
                let (start_line, start_col) =
                    offset_to_line_col(&template.content, lint_diag.start as usize);
                let (end_line, end_col) =
                    offset_to_line_col(&template.content, lint_diag.end as usize);

                // Adjust line numbers based on template block position in SFC
                let sfc_start_line = template.loc.start_line as u32 + start_line;
                let sfc_end_line = template.loc.start_line as u32 + end_line;

                // Build the diagnostic message with help text
                let message = if let Some(ref help) = lint_diag.help {
                    format!("{}\n\nHelp: {}", lint_diag.message, help)
                } else {
                    lint_diag.message.to_string()
                };

                Diagnostic {
                    range: Range {
                        start: Position {
                            line: sfc_start_line.saturating_sub(1),
                            character: start_col,
                        },
                        end: Position {
                            line: sfc_end_line.saturating_sub(1),
                            character: end_col,
                        },
                    },
                    severity: Some(match lint_diag.severity {
                        vize_patina::Severity::Error => DiagnosticSeverity::ERROR,
                        vize_patina::Severity::Warning => DiagnosticSeverity::WARNING,
                    }),
                    code: Some(NumberOrString::String(lint_diag.rule_name.to_string())),
                    code_description: Some(CodeDescription {
                        href: Url::parse(&format!(
                            "https://eslint.vuejs.org/rules/{}.html",
                            lint_diag
                                .rule_name
                                .strip_prefix("vue/")
                                .unwrap_or(lint_diag.rule_name)
                        ))
                        .unwrap_or_else(|_| Url::parse("https://eslint.vuejs.org/rules/").unwrap()),
                    }),
                    source: Some(sources::LINTER.to_string()),
                    message,
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Create a diagnostic from a custom error.
    pub fn create_diagnostic(
        range: Range,
        severity: Severity,
        source: &str,
        code: Option<i32>,
        message: String,
    ) -> Diagnostic {
        Diagnostic {
            range,
            severity: Some(severity.into()),
            code: code.map(NumberOrString::Number),
            source: Some(source.to_string()),
            message,
            ..Default::default()
        }
    }
}

/// Builder for creating diagnostics.
pub struct DiagnosticBuilder {
    range: Range,
    severity: Severity,
    source: String,
    code: Option<i32>,
    message: String,
    related_information: Vec<tower_lsp::lsp_types::DiagnosticRelatedInformation>,
}

impl DiagnosticBuilder {
    /// Create a new diagnostic builder.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            range: Range::default(),
            severity: Severity::Error,
            source: "vize".to_string(),
            code: None,
            message: message.into(),
            related_information: Vec::new(),
        }
    }

    /// Set the range.
    pub fn range(mut self, range: Range) -> Self {
        self.range = range;
        self
    }

    /// Set the severity.
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Set the source.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// Set the error code.
    pub fn code(mut self, code: i32) -> Self {
        self.code = Some(code);
        self
    }

    /// Add related information.
    pub fn related(
        mut self,
        location: tower_lsp::lsp_types::Location,
        message: impl Into<String>,
    ) -> Self {
        self.related_information
            .push(tower_lsp::lsp_types::DiagnosticRelatedInformation {
                location,
                message: message.into(),
            });
        self
    }

    /// Build the diagnostic.
    pub fn build(self) -> Diagnostic {
        Diagnostic {
            range: self.range,
            severity: Some(self.severity.into()),
            code: self.code.map(NumberOrString::Number),
            source: Some(self.source),
            message: self.message,
            related_information: if self.related_information.is_empty() {
                None
            } else {
                Some(self.related_information)
            },
            ..Default::default()
        }
    }
}

/// Convert byte offset to (line, column) - both 0-indexed for LSP.
fn offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
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
    fn test_diagnostic_builder() {
        let diagnostic = DiagnosticBuilder::new("Test error")
            .severity(Severity::Warning)
            .source("test")
            .code(42)
            .build();

        assert_eq!(diagnostic.message, "Test error");
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(diagnostic.source, Some("test".to_string()));
        assert_eq!(diagnostic.code, Some(NumberOrString::Number(42)));
    }

    #[test]
    fn test_severity_conversion() {
        assert_eq!(
            DiagnosticSeverity::from(Severity::Error),
            DiagnosticSeverity::ERROR
        );
        assert_eq!(
            DiagnosticSeverity::from(Severity::Warning),
            DiagnosticSeverity::WARNING
        );
        assert_eq!(
            DiagnosticSeverity::from(Severity::Information),
            DiagnosticSeverity::INFORMATION
        );
        assert_eq!(
            DiagnosticSeverity::from(Severity::Hint),
            DiagnosticSeverity::HINT
        );
    }
}
