//! Inlay hints provider.
//!
//! Provides inlay hints for:
//! - Props destructure (show `#props.` prefix for destructured props in template and script)
//!
//! Uses vize_croquis for proper scope analysis to accurately identify destructured props.

use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Position, Range, Url};
use vize_croquis::{Analyzer, AnalyzerOptions};

use super::offset_to_position;

/// Inlay hint service.
pub struct InlayHintService;

impl InlayHintService {
    /// Get inlay hints for a document range.
    pub fn get_hints(content: &str, uri: &Url, range: Range) -> Vec<InlayHint> {
        let mut hints = Vec::new();

        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        let Ok(descriptor) = vize_atelier_sfc::parse_sfc(content, options) else {
            return hints;
        };

        // Use vize_croquis analyzer for proper scope analysis
        let Some(ref script_setup) = descriptor.script_setup else {
            return hints;
        };

        // Analyze the script setup using croquis
        let mut analyzer = Analyzer::with_options(AnalyzerOptions {
            analyze_script: true,
            ..Default::default()
        });
        analyzer.analyze_script_setup(&script_setup.content);
        let croquis = analyzer.finish();

        // Get all prop names from defineProps (for template hints)
        let all_prop_names: Vec<String> = croquis
            .macros
            .props()
            .iter()
            .map(|p| p.name.to_string())
            .collect();

        // Get props destructure info from the analysis (for script hints)
        let props_destructure = croquis.macros.props_destructure();

        // Collect local names of destructured props (for script)
        let destructured_local_names: Vec<&str> = props_destructure
            .map(|pd| pd.bindings.values().map(|b| b.local.as_str()).collect())
            .unwrap_or_default();

        // Get the defineProps call span to skip hints within the type definition
        let define_props_end = croquis
            .macros
            .define_props()
            .map(|call| call.end as usize)
            .unwrap_or(0);

        // Find usages of destructured props in script setup (only destructured ones)
        if !destructured_local_names.is_empty() {
            Self::collect_script_props_hints(
                &script_setup.content,
                script_setup.loc.start,
                content,
                &destructured_local_names,
                define_props_end,
                range,
                &mut hints,
            );
        }

        // Find usages of props in template (all props are available in template)
        if let Some(ref template) = descriptor.template {
            if !all_prop_names.is_empty() {
                let prop_refs: Vec<&str> = all_prop_names.iter().map(|s| s.as_str()).collect();
                Self::collect_template_props_hints(
                    &template.content,
                    template.loc.start,
                    content,
                    &prop_refs,
                    range,
                    &mut hints,
                );
            }
        }

        hints
    }

    /// Collect inlay hints for props usages in script setup.
    fn collect_script_props_hints(
        script: &str,
        script_offset: usize,
        full_content: &str,
        destructured_props: &[&str],
        define_props_end: usize,
        range: Range,
        hints: &mut Vec<InlayHint>,
    ) {
        // Find usages of destructured props after the defineProps call
        for &prop_name in destructured_props {
            Self::find_prop_usages_in_text(
                script,
                prop_name,
                script_offset,
                full_content,
                define_props_end,
                range,
                hints,
            );
        }
    }

    /// Find usages of a prop in plain text (for script).
    fn find_prop_usages_in_text(
        text: &str,
        prop_name: &str,
        base_offset: usize,
        full_content: &str,
        define_props_end: usize,
        range: Range,
        hints: &mut Vec<InlayHint>,
    ) {
        if prop_name.is_empty() || text.is_empty() {
            return;
        }

        let mut search_pos = 0;

        while let Some(found) = text[search_pos..].find(prop_name) {
            let abs_pos = search_pos + found;

            // Bounds check
            if abs_pos + prop_name.len() > text.len() {
                break;
            }

            // Calculate SFC offset
            let sfc_offset = base_offset + abs_pos;

            // Skip if within defineProps call (including type definition)
            if sfc_offset <= base_offset + define_props_end {
                search_pos = abs_pos + 1;
                continue;
            }

            // Check word boundaries
            let before_ok = abs_pos == 0
                || text
                    .as_bytes()
                    .get(abs_pos - 1)
                    .map(|&b| !Self::is_ident_char(b))
                    .unwrap_or(true);
            let after_ok = text
                .as_bytes()
                .get(abs_pos + prop_name.len())
                .map(|&b| !Self::is_ident_char(b))
                .unwrap_or(true);

            // Check it's not preceded by "props." already
            let not_already_prefixed =
                abs_pos < 6 || &text[abs_pos.saturating_sub(6)..abs_pos] != "props.";

            // Check it's not in a string literal (simple check for quotes)
            let not_in_string = !Self::is_in_string(text, abs_pos);

            // Check it's not a property access (preceded by .)
            let not_property_access = abs_pos == 0
                || text
                    .as_bytes()
                    .get(abs_pos - 1)
                    .map(|&b| b != b'.')
                    .unwrap_or(true);

            // Check it's not part of an event name pattern like "update:title" (preceded by :)
            let not_event_name_part = abs_pos == 0
                || text
                    .as_bytes()
                    .get(abs_pos - 1)
                    .map(|&b| b != b':')
                    .unwrap_or(true);

            if before_ok
                && after_ok
                && not_already_prefixed
                && not_in_string
                && not_property_access
                && not_event_name_part
            {
                // Bounds check for full_content
                if sfc_offset >= full_content.len() {
                    search_pos = abs_pos + 1;
                    continue;
                }

                let (line, character) = offset_to_position(full_content, sfc_offset);

                let position = Position { line, character };

                // Check if within requested range
                if Self::position_in_range(position, range) {
                    hints.push(InlayHint {
                        position,
                        label: InlayHintLabel::String("#props.".to_string()),
                        kind: Some(InlayHintKind::TYPE),
                        text_edits: None,
                        tooltip: Some(tower_lsp::lsp_types::InlayHintTooltip::String(
                            "Destructured from defineProps".to_string(),
                        )),
                        padding_left: None,
                        padding_right: Some(true),
                        data: None,
                    });
                }
            }

            search_pos = abs_pos + 1;
        }
    }

    /// Simple check if a position is inside a string literal.
    fn is_in_string(text: &str, pos: usize) -> bool {
        if pos >= text.len() {
            return false;
        }

        let before = &text[..pos];
        let mut in_single = false;
        let mut in_double = false;
        let mut in_template = false;
        let mut prev_char = '\0';

        for c in before.chars() {
            if prev_char != '\\' {
                if c == '\'' && !in_double && !in_template {
                    in_single = !in_single;
                } else if c == '"' && !in_single && !in_template {
                    in_double = !in_double;
                } else if c == '`' && !in_single && !in_double {
                    in_template = !in_template;
                }
            }
            prev_char = c;
        }

        in_single || in_double || in_template
    }

    /// Collect inlay hints for props usages in template.
    fn collect_template_props_hints(
        template: &str,
        template_offset: usize,
        full_content: &str,
        destructured_props: &[&str],
        range: Range,
        hints: &mut Vec<InlayHint>,
    ) {
        // Find mustache expressions {{ ... }}
        Self::collect_mustache_hints(
            template,
            template_offset,
            full_content,
            destructured_props,
            range,
            hints,
        );

        // Find Vue directive expressions (:prop="...", v-bind:prop="...", @event="...", v-if="...", etc.)
        Self::collect_directive_hints(
            template,
            template_offset,
            full_content,
            destructured_props,
            range,
            hints,
        );
    }

    /// Collect hints from mustache expressions {{ ... }}.
    fn collect_mustache_hints(
        template: &str,
        template_offset: usize,
        full_content: &str,
        destructured_props: &[&str],
        range: Range,
        hints: &mut Vec<InlayHint>,
    ) {
        let mut pos = 0;

        while let Some(start) = template[pos..].find("{{") {
            let abs_start = pos + start + 2; // Skip "{{"

            if let Some(end) = template[abs_start..].find("}}") {
                let abs_end = abs_start + end;
                let expr = &template[abs_start..abs_end];

                for &prop in destructured_props {
                    Self::find_prop_usages_in_expr(
                        expr,
                        prop,
                        template_offset + abs_start,
                        full_content,
                        range,
                        hints,
                    );
                }

                pos = abs_end + 2;
            } else {
                break;
            }
        }
    }

    /// Collect hints from Vue directive attributes.
    fn collect_directive_hints(
        template: &str,
        template_offset: usize,
        full_content: &str,
        destructured_props: &[&str],
        range: Range,
        hints: &mut Vec<InlayHint>,
    ) {
        // Patterns for Vue directives:
        // :prop="...", v-bind:prop="...", @event="...", v-on:event="..."
        // v-if="...", v-else-if="...", v-for="...", v-show="...", v-model="..."
        // v-slot:name="...", #name="..."

        let directive_patterns = [
            "v-if=\"",
            "v-if='",
            "v-else-if=\"",
            "v-else-if='",
            "v-for=\"",
            "v-for='",
            "v-show=\"",
            "v-show='",
            "v-model=\"",
            "v-model='",
            "v-bind:",
            "v-on:",
            "v-slot:",
        ];

        let mut pos = 0;

        while pos < template.len() {
            let remaining = &template[pos..];

            // Find next directive or shorthand
            let mut next_match: Option<(usize, usize, char)> = None; // (position, skip_len, quote_char)

            // Check for shorthand patterns: :prop=", @event=", #slot="
            for (i, c) in remaining.char_indices() {
                if (c == ':' || c == '@' || c == '#') && i + 1 < remaining.len() {
                    // Check if followed by identifier and ="
                    let after = &remaining[i + 1..];
                    if let Some(eq_pos) = after.find('=') {
                        let attr_name = &after[..eq_pos];
                        // Validate it's a valid attribute name (alphanumeric, -, _)
                        if !attr_name.is_empty()
                            && attr_name
                                .chars()
                                .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ':')
                        {
                            let quote_start = i + 1 + eq_pos + 1;
                            if quote_start < remaining.len() {
                                let quote = remaining.as_bytes()[quote_start] as char;
                                if quote == '"' || quote == '\'' {
                                    next_match = Some((i, quote_start + 1, quote));
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // Check for full directive patterns
            for pattern in &directive_patterns {
                if let Some(found) = remaining.find(pattern) {
                    let pattern_end = found + pattern.len();
                    if pattern.ends_with(':') {
                        // v-bind:, v-on:, v-slot: - need to find ="
                        if let Some(eq_pos) = remaining[pattern_end..].find('=') {
                            let quote_pos = pattern_end + eq_pos + 1;
                            if quote_pos < remaining.len() {
                                let quote = remaining.as_bytes()[quote_pos] as char;
                                if quote == '"' || quote == '\'' {
                                    let new_match = (found, quote_pos + 1, quote);
                                    if next_match.is_none()
                                        || new_match.0 < next_match.as_ref().unwrap().0
                                    {
                                        next_match = Some(new_match);
                                    }
                                }
                            }
                        }
                    } else {
                        // v-if=", etc. - pattern already includes the quote
                        let quote = pattern.chars().last().unwrap();
                        let new_match = (found, pattern_end, quote);
                        if next_match.is_none() || new_match.0 < next_match.as_ref().unwrap().0 {
                            next_match = Some(new_match);
                        }
                    }
                }
            }

            let Some((_, expr_start, quote)) = next_match else {
                break;
            };

            let abs_start = pos + expr_start;

            // Find closing quote
            if let Some(end) = template[abs_start..].find(quote) {
                let abs_end = abs_start + end;
                let expr = &template[abs_start..abs_end];

                for &prop in destructured_props {
                    Self::find_prop_usages_in_expr(
                        expr,
                        prop,
                        template_offset + abs_start,
                        full_content,
                        range,
                        hints,
                    );
                }

                pos = abs_end + 1;
            } else {
                pos = abs_start + 1;
            }
        }
    }

    /// Find usages of a prop in an expression and add hints.
    fn find_prop_usages_in_expr(
        expr: &str,
        prop_name: &str,
        base_offset: usize,
        full_content: &str,
        range: Range,
        hints: &mut Vec<InlayHint>,
    ) {
        if prop_name.is_empty() || expr.is_empty() {
            return;
        }

        let mut search_pos = 0;

        while let Some(found) = expr[search_pos..].find(prop_name) {
            let abs_pos = search_pos + found;

            // Bounds check
            if abs_pos + prop_name.len() > expr.len() {
                break;
            }

            // Check word boundaries
            let before_ok = abs_pos == 0
                || expr
                    .as_bytes()
                    .get(abs_pos - 1)
                    .map(|&b| !Self::is_ident_char(b))
                    .unwrap_or(true);
            let after_ok = expr
                .as_bytes()
                .get(abs_pos + prop_name.len())
                .map(|&b| !Self::is_ident_char(b))
                .unwrap_or(true);

            // Check it's not preceded by "props." already
            let not_already_prefixed =
                abs_pos < 6 || &expr[abs_pos.saturating_sub(6)..abs_pos] != "props.";

            // Check it's not a property access (preceded by .)
            let not_property_access = abs_pos == 0
                || expr
                    .as_bytes()
                    .get(abs_pos - 1)
                    .map(|&b| b != b'.')
                    .unwrap_or(true);

            // Check it's not part of an event name pattern like "update:title" (preceded by :)
            let not_event_name_part = abs_pos == 0
                || expr
                    .as_bytes()
                    .get(abs_pos - 1)
                    .map(|&b| b != b':')
                    .unwrap_or(true);

            if before_ok
                && after_ok
                && not_already_prefixed
                && not_property_access
                && not_event_name_part
            {
                let sfc_offset = base_offset + abs_pos;

                // Bounds check for full_content
                if sfc_offset >= full_content.len() {
                    search_pos = abs_pos + 1;
                    continue;
                }

                let (line, character) = offset_to_position(full_content, sfc_offset);

                let position = Position { line, character };

                // Check if within requested range
                if Self::position_in_range(position, range) {
                    hints.push(InlayHint {
                        position,
                        label: InlayHintLabel::String("#props.".to_string()),
                        kind: Some(InlayHintKind::TYPE),
                        text_edits: None,
                        tooltip: Some(tower_lsp::lsp_types::InlayHintTooltip::String(
                            "Destructured from defineProps".to_string(),
                        )),
                        padding_left: None,
                        padding_right: Some(true),
                        data: None,
                    });
                }
            }

            search_pos = abs_pos + 1;
        }
    }

    /// Check if a position is within a range.
    fn position_in_range(pos: Position, range: Range) -> bool {
        if pos.line < range.start.line || pos.line > range.end.line {
            return false;
        }
        if pos.line == range.start.line && pos.character < range.start.character {
            return false;
        }
        if pos.line == range.end.line && pos.character > range.end.character {
            return false;
        }
        true
    }

    fn is_ident_char(c: u8) -> bool {
        c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_props_destructure_analysis() {
        let content = r#"<script setup lang="ts">
const { title, disabled } = defineProps<{
  title: string
  disabled?: boolean
}>()

console.log(title)
</script>

<template>
  <div>{{ title }}</div>
</template>"#;

        let uri = Url::parse("file:///test.vue").unwrap();
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 100,
                character: 0,
            },
        };

        let hints = InlayHintService::get_hints(content, &uri, range);

        // Should have hints for title in script (line 6) and template (line 10)
        assert!(!hints.is_empty(), "Should have inlay hints");

        // Verify all hints are #props.
        for hint in &hints {
            if let InlayHintLabel::String(label) = &hint.label {
                assert_eq!(label, "#props.");
            }
        }
    }

    #[test]
    fn test_props_destructure_with_alias() {
        let content = r#"<script setup lang="ts">
const { title: localTitle } = defineProps<{
  title: string
}>()

console.log(localTitle)
</script>

<template>
  <div>{{ localTitle }}</div>
</template>"#;

        let uri = Url::parse("file:///test.vue").unwrap();
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 100,
                character: 0,
            },
        };

        let hints = InlayHintService::get_hints(content, &uri, range);

        // Should have hints for localTitle (the alias), not title
        assert!(
            !hints.is_empty(),
            "Should have inlay hints for aliased prop"
        );
    }

    #[test]
    fn test_no_hints_in_define_props_type() {
        let content = r#"<script setup lang="ts">
const { title } = defineProps<{
  title: string
}>()
</script>

<template>
  <div>{{ title }}</div>
</template>"#;

        let uri = Url::parse("file:///test.vue").unwrap();
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 100,
                character: 0,
            },
        };

        let hints = InlayHintService::get_hints(content, &uri, range);

        // Check that no hints are in the defineProps type definition
        // (lines 1-3 in script, which is around line 1-4 in the file)
        for hint in &hints {
            assert!(
                hint.position.line > 3,
                "Hint should not be in defineProps type definition, found at line {}",
                hint.position.line
            );
        }
    }

    #[test]
    fn test_is_in_string() {
        assert!(!InlayHintService::is_in_string("foo bar", 4));
        assert!(InlayHintService::is_in_string("'foo bar'", 4));
        assert!(InlayHintService::is_in_string("\"foo bar\"", 4));
        assert!(!InlayHintService::is_in_string("\"foo\" bar", 6));
        assert!(InlayHintService::is_in_string("`foo bar`", 4));
    }

    #[test]
    fn test_no_hints_in_event_name_pattern() {
        // Test that "title" in "update:title" event name does not get a hint
        let content = r#"<script setup lang="ts">
const { title } = defineProps<{
  title: string
}>()

const emit = defineEmits<{
  (e: 'update:title', value: string): void
}>()
</script>

<template>
  <input :value="title" @update:title="emit('update:title', $event)" />
</template>"#;

        let uri = Url::parse("file:///test.vue").unwrap();
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 100,
                character: 0,
            },
        };

        let hints = InlayHintService::get_hints(content, &uri, range);

        // Should have hints for title in :value="title" and possibly template
        // But NOT for title in 'update:title' event names
        for hint in &hints {
            // Get the position in the content
            let line = hint.position.line as usize;
            let lines: Vec<&str> = content.lines().collect();
            if line < lines.len() {
                let line_content = lines[line];
                // Verify the hint is not on a line containing 'update:title' pattern
                // where title immediately follows a colon
                let char_pos = hint.position.character as usize;
                if char_pos > 0 && char_pos <= line_content.len() {
                    let before_char = line_content.as_bytes().get(char_pos - 1);
                    assert_ne!(
                        before_char,
                        Some(&b':'),
                        "Hint should not be placed after colon (event name pattern)"
                    );
                }
            }
        }
    }

    #[test]
    fn test_props_without_destructure_in_template() {
        // Test that props defined without destructuring also get hints in template
        let content = r#"<script setup lang="ts">
const props = defineProps<{
  title: string
  count: number
}>()

// In script, we access via props.title (no hint needed for 'title' alone)
console.log(props.title)
</script>

<template>
  <div>{{ title }}</div>
  <span>{{ count }}</span>
</template>"#;

        let uri = Url::parse("file:///test.vue").unwrap();
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 100,
                character: 0,
            },
        };

        let hints = InlayHintService::get_hints(content, &uri, range);

        // Should have hints for title and count in template (lines 11 and 12)
        // Even though props are not destructured
        let template_hints: Vec<_> = hints.iter().filter(|h| h.position.line >= 11).collect();

        assert!(
            !template_hints.is_empty(),
            "Should have hints for props in template even without destructuring"
        );
    }
}
