//! Import processing utilities.
//!
//! This module handles processing import statements, including
//! removing TypeScript type-only imports and extracting identifiers.

use oxc_allocator::Allocator;
use oxc_ast::ast::{ImportDeclarationSpecifier, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Known type-only exports from Vue that should never appear in runtime imports.
/// These are types that exist in Vue's TypeScript definitions but not in the runtime bundle.
const VUE_TYPE_ONLY_EXPORTS: &[&str] = &[
    "PropType",
    "Ref",
    "ComputedRef",
    "WritableComputedRef",
    "ShallowRef",
    "UnwrapRef",
    "UnwrapNestedRefs",
    "ToRef",
    "ToRefs",
    "MaybeRef",
    "MaybeRefOrGetter",
    "StyleValue",
    "ComponentPublicInstance",
    "ComponentCustomProperties",
    "ComponentOptionsBase",
    "DefineComponent",
    "SetupContext",
    "EmitsOptions",
    "ObjectEmitsOptions",
    "ExtractPropTypes",
    "ExtractDefaultPropTypes",
    "SlotsType",
    "VNodeChild",
    "VNodeProps",
    "RendererNode",
    "RendererElement",
    "HTMLAttributes",
    "InputHTMLAttributes",
    "WatchSource",
    "WatchCallback",
    "WatchEffect",
    "WatchOptions",
    "WatchStopHandle",
    "InjectionKey",
    "App",
    "Plugin",
    "Directive",
    "DirectiveBinding",
    "FunctionalComponent",
    "AsyncComponentLoader",
    "AsyncComponentOptions",
    "RenderFunction",
    "ComponentInternalInstance",
    "CSSProperties",
];

/// Internal helper for word-boundary identifier matching.
/// When `exclude_property_access` is true, occurrences preceded by '.' are skipped
/// (e.g. `Enum.Member` won't match `Member`). When false, they match
/// (needed for `$setup.ComponentName` in render functions).
fn contains_identifier_inner(text: &str, name: &str, exclude_property_access: bool) -> bool {
    let name_bytes = name.as_bytes();
    let text_bytes = text.as_bytes();
    let name_len = name_bytes.len();
    let text_len = text_bytes.len();

    if name_len == 0 || text_len < name_len {
        return false;
    }

    let mut i = 0;
    let mut in_string = false;
    let mut string_char: u8 = 0;
    let mut prev: u8 = 0;

    while i < text_len {
        let c = text_bytes[i];

        // Skip string literals
        if in_string {
            if c == string_char && prev != b'\\' {
                in_string = false;
            }
            prev = c;
            i += 1;
            continue;
        }

        if c == b'\'' || c == b'"' || c == b'`' {
            in_string = true;
            string_char = c;
            prev = c;
            i += 1;
            continue;
        }

        // Skip single-line comments: // ...
        if c == b'/' && i + 1 < text_len && text_bytes[i + 1] == b'/' {
            i += 2;
            while i < text_len && text_bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Skip multi-line comments: /* ... */
        if c == b'/' && i + 1 < text_len && text_bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < text_len {
                if text_bytes[i] == b'*' && text_bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        if i + name_len <= text_len && &text_bytes[i..i + name_len] == name_bytes {
            // Check word boundary before
            let before_ok = if i == 0 {
                true
            } else {
                let bc = text_bytes[i - 1];
                let base = !bc.is_ascii_alphanumeric() && bc != b'_' && bc != b'$';
                if exclude_property_access {
                    base && bc != b'.'
                } else {
                    base
                }
            };
            // Check word boundary after
            let after_ok = if i + name_len >= text_len {
                true
            } else {
                let ac = text_bytes[i + name_len];
                !ac.is_ascii_alphanumeric() && ac != b'_' && ac != b'$'
            };
            if before_ok && after_ok {
                return true;
            }
        }

        prev = c;
        i += 1;
    }
    false
}

/// Check if an identifier appears as a whole word in the given text,
/// excluding occurrences inside string literals.
/// This allows matches after '.' (e.g. `$setup.ComponentName` matches `ComponentName`).
pub fn contains_identifier(text: &str, name: &str) -> bool {
    contains_identifier_inner(text, name, false)
}

/// Like `contains_identifier`, but also excludes occurrences preceded by '.'
/// (property access). Use this to check if a name is used as a standalone
/// identifier, not as a member of another object.
pub fn contains_identifier_standalone(text: &str, name: &str) -> bool {
    contains_identifier_inner(text, name, true)
}

/// Check if a specifier name is a known Vue type-only export
fn is_vue_type_only_export(name: &str, source: &str) -> bool {
    if source == "vue" || source == "@vue/runtime-core" || source == "@vue/runtime-dom" {
        VUE_TYPE_ONLY_EXPORTS.contains(&name)
    } else {
        false
    }
}

/// Process import statement to remove TypeScript type-only imports using OXC
/// Returns None if the entire import should be removed, Some(processed) otherwise
pub fn process_import_for_types(import: &str) -> Option<String> {
    let import = import.trim();

    // Parse the import statement with OXC
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let parser = Parser::new(&allocator, import, source_type);
    let result = parser.parse();

    if result.errors.is_empty() {
        for stmt in &result.program.body {
            if let Statement::ImportDeclaration(decl) = stmt {
                // Skip type-only imports: import type { ... } from '...'
                if decl.import_kind.is_type() {
                    return None;
                }

                let source = decl.source.value.as_str();

                // Check if there are any specifiers
                if let Some(specifiers) = &decl.specifiers {
                    // Filter out type-only specifiers and known Vue type-only exports
                    let value_specifiers: Vec<&ImportDeclarationSpecifier> = specifiers
                        .iter()
                        .filter(|spec| match spec {
                            ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                if s.import_kind.is_type() {
                                    return false;
                                }
                                let imported_name = s.imported.name().as_str();
                                !is_vue_type_only_export(imported_name, source)
                            }
                            _ => true,
                        })
                        .collect();

                    if value_specifiers.is_empty() {
                        // All specifiers were type imports
                        return None;
                    }

                    if value_specifiers.len() != specifiers.len() {
                        // Some specifiers were filtered out, rebuild the import
                        let source = decl.source.value.as_str();
                        let mut default_name: Option<String> = None;
                        let mut namespace_name: Option<String> = None;
                        let mut named: Vec<String> = Vec::new();

                        for spec in &value_specifiers {
                            match spec {
                                ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                    default_name = Some(s.local.name.to_string());
                                }
                                ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                    namespace_name = Some(s.local.name.to_string());
                                }
                                ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                    let imported = s.imported.name().as_str();
                                    let local = s.local.name.as_str();
                                    if imported == local {
                                        named.push(imported.to_string());
                                    } else {
                                        named.push(format!("{} as {}", imported, local));
                                    }
                                }
                            }
                        }

                        let mut new_import = String::with_capacity(source.len() + 40);
                        new_import.push_str("import ");

                        if let Some(ref ns) = namespace_name {
                            new_import.push_str("* as ");
                            new_import.push_str(ns);
                        } else {
                            if let Some(ref def) = default_name {
                                new_import.push_str(def);
                                if !named.is_empty() {
                                    new_import.push_str(", ");
                                }
                            }
                            if !named.is_empty() {
                                new_import.push_str("{ ");
                                new_import.push_str(&named.join(", "));
                                new_import.push_str(" }");
                            }
                        }

                        new_import.push_str(" from '");
                        new_import.push_str(source);
                        new_import.push_str("'\n");
                        return Some(new_import);
                    }
                }
            }
        }
    }

    // Regular import or parse failed, return as-is
    Some(import.to_string() + "\n")
}

/// Extract all identifiers from an import statement (including default imports)
pub fn extract_import_identifiers(import: &str) -> Vec<String> {
    let import = import.trim();
    let mut identifiers = Vec::new();

    // Parse the import statement with OXC
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let parser = Parser::new(&allocator, import, source_type);
    let result = parser.parse();

    if result.errors.is_empty() {
        for stmt in &result.program.body {
            if let Statement::ImportDeclaration(decl) = stmt {
                // Skip type-only imports
                if decl.import_kind.is_type() {
                    continue;
                }

                if let Some(specifiers) = &decl.specifiers {
                    for spec in specifiers {
                        match spec {
                            ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                // Skip type-only specifiers
                                if !s.import_kind.is_type() {
                                    identifiers.push(s.local.name.to_string());
                                }
                            }
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                identifiers.push(s.local.name.to_string());
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                identifiers.push(s.local.name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    identifiers
}

/// Strip unused import specifiers from compiled SFC code.
/// Scans the non-import portion of the code and removes any import specifiers
/// that are not referenced (used only as types, already compiled away, etc.).
/// Side-effect imports (`import 'foo'`) are always preserved.
pub fn strip_unused_imports(code: &str) -> String {
    let mut import_lines: Vec<(usize, usize)> = Vec::new(); // (start, end) line indices
    let mut non_import_code = String::new();

    // Split code into import lines and non-import code.
    // Include __returned__ in usage check — it now only contains properly filtered
    // bindings, and standalone identifiers there (e.g. `{ ComponentName }`) allow
    // `contains_identifier_standalone` to detect them even though the render function
    // references them via `$setup.ComponentName` (preceded by '.').
    let mut in_import = false;
    let mut import_start = 0;
    for (i, line) in code.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("import ") || trimmed.starts_with("import{") {
            in_import = true;
            import_start = i;
        }

        if in_import {
            if trimmed.ends_with(';')
                || (trimmed.contains(" from ") && !trimmed.ends_with(','))
                || trimmed.ends_with('\'')
                || trimmed.ends_with('"')
            {
                import_lines.push((import_start, i));
                in_import = false;
            }
        } else {
            non_import_code.push_str(line);
            non_import_code.push('\n');
        }
    }

    let lines: Vec<&str> = code.lines().collect();
    let mut result = String::with_capacity(code.len());

    for &(start, end) in &import_lines {
        let import_text: String = lines[start..=end].join("\n");
        let trimmed_import = import_text.trim();

        // Parse the import
        let allocator = Allocator::default();
        let parser = Parser::new(&allocator, trimmed_import, SourceType::ts());
        let parse_result = parser.parse();

        if parse_result.errors.is_empty() {
            let mut handled = false;
            for stmt in &parse_result.program.body {
                if let Statement::ImportDeclaration(decl) = stmt {
                    if let Some(specifiers) = &decl.specifiers {
                        // Filter to only specifiers used in non-import code
                        let used_specifiers: Vec<&ImportDeclarationSpecifier> = specifiers
                            .iter()
                            .filter(|spec| {
                                let local_name = match spec {
                                    ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                        s.local.name.as_str()
                                    }
                                    ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                        s.local.name.as_str()
                                    }
                                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                        s.local.name.as_str()
                                    }
                                };
                                // Check if name is used in non-import code as a standalone identifier
                                // (excludes property access like Enum.Member, but matches
                                // standalone refs in __returned__ block)
                                contains_identifier_standalone(&non_import_code, local_name)
                            })
                            .collect();

                        if used_specifiers.is_empty() && !specifiers.is_empty() {
                            // No specifiers are used, skip entire import
                            handled = true;
                            break;
                        }

                        if used_specifiers.len() < specifiers.len() {
                            // Some specifiers unused, rebuild import
                            let source = decl.source.value.as_str();
                            let mut has_default = false;
                            let mut named: Vec<String> = Vec::new();

                            for spec in &used_specifiers {
                                match spec {
                                    ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                        has_default = true;
                                        result.push_str("import ");
                                        result.push_str(s.local.name.as_str());
                                    }
                                    ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                        let imported = s.imported.name().as_str();
                                        let local = s.local.name.as_str();
                                        if imported == local {
                                            named.push(imported.to_string());
                                        } else {
                                            named.push(format!("{} as {}", imported, local));
                                        }
                                    }
                                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                        result.push_str("import * as ");
                                        result.push_str(s.local.name.as_str());
                                        result.push_str(" from '");
                                        result.push_str(source);
                                        result.push_str("'\n");
                                        handled = true;
                                    }
                                }
                            }

                            if !handled {
                                if has_default && !named.is_empty() {
                                    result.push_str(", { ");
                                    result.push_str(&named.join(", "));
                                    result.push_str(" }");
                                } else if !named.is_empty() {
                                    result.push_str("import { ");
                                    result.push_str(&named.join(", "));
                                    result.push_str(" }");
                                }
                                result.push_str(" from '");
                                result.push_str(source);
                                result.push_str("'\n");
                                handled = true;
                            }
                            break;
                        }
                    }
                }
            }
            if !handled {
                result.push_str(trimmed_import);
                result.push('\n');
            }
        } else {
            result.push_str(trimmed_import);
            result.push('\n');
        }
    }

    // Append non-import code (includes __returned__ block)
    result.push_str(&non_import_code);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // contains_identifier tests (9 cases)
    // =========================================================================

    #[test]
    fn contains_identifier_basic_match() {
        assert!(contains_identifier("const Foo = 1", "Foo"));
    }

    #[test]
    fn contains_identifier_basic_no_match() {
        assert!(!contains_identifier("const Bar = 1", "Foo"));
    }

    #[test]
    fn contains_identifier_word_boundary_no_partial_match() {
        // "BillingType" should NOT match inside "canChangeBillingType"
        assert!(!contains_identifier("canChangeBillingType", "BillingType"));
    }

    #[test]
    fn contains_identifier_skips_string_literal() {
        // "Project" inside a string literal should be skipped
        assert!(!contains_identifier(
            r#"const msg = "Duplicate Project?""#,
            "Project"
        ));
    }

    #[test]
    fn contains_identifier_skips_single_line_comment() {
        // "FormFile" inside a single-line comment should be skipped
        assert!(!contains_identifier("// FormFile\nconst x = 1", "FormFile"));
    }

    #[test]
    fn contains_identifier_skips_multiline_comment() {
        // "FormFile" inside a multi-line comment should be skipped
        assert!(!contains_identifier(
            "/* FormFile */\nconst x = 1",
            "FormFile"
        ));
    }

    #[test]
    fn contains_identifier_matches_after_dot() {
        // $setup.Foo — "Foo" should match (non-standalone mode)
        assert!(contains_identifier("$setup.Foo", "Foo"));
    }

    #[test]
    fn contains_identifier_dollar_boundary() {
        // "$setup" should NOT match "setup" because $ is a word character
        assert!(!contains_identifier("$setup", "setup"));
    }

    #[test]
    fn contains_identifier_escaped_quote_in_string() {
        // `"it\'s"` — escaped quote doesn't close the string; Bar outside matches
        assert!(contains_identifier(r#"'it\'s' + Bar"#, "Bar"));
    }

    #[test]
    fn contains_identifier_trailing_dollar_boundary() {
        // "Foo$bar" — "Foo" should NOT match because followed by $
        assert!(!contains_identifier("Foo$bar", "Foo"));
    }

    // =========================================================================
    // contains_identifier_standalone tests (4 cases)
    // =========================================================================

    #[test]
    fn contains_identifier_standalone_excludes_property_access() {
        // Enum.Member — "Member" should NOT match in standalone mode
        assert!(!contains_identifier_standalone("Enum.Member", "Member"));
    }

    #[test]
    fn contains_identifier_standalone_excludes_setup_dot() {
        // $setup.Foo — "Foo" should NOT match in standalone mode
        assert!(!contains_identifier_standalone("$setup.Foo", "Foo"));
    }

    #[test]
    fn contains_identifier_standalone_matches_in_returned() {
        // Standalone identifier in __returned__ block should match
        let code = "const __returned__ = { Foo }\nrender($setup.Foo)";
        assert!(contains_identifier_standalone(code, "Foo"));
    }

    #[test]
    fn contains_identifier_standalone_skips_comments() {
        // Identifier inside comment should be skipped even in standalone mode
        assert!(!contains_identifier_standalone("// Foo\nconst x = 1", "Foo"));
    }

    // =========================================================================
    // process_import_for_types tests (7 cases)
    // =========================================================================

    #[test]
    fn process_import_for_types_removes_type_import() {
        // import type { Foo } from 'bar' → None
        let result = process_import_for_types("import type { Foo } from 'bar'");
        assert!(result.is_none());
    }

    #[test]
    fn process_import_for_types_filters_inline_type() {
        // import { type Foo, Bar } from 'bar' → only Bar remains
        let result =
            process_import_for_types("import { type Foo, Bar } from 'bar'").unwrap();
        assert!(result.contains("Bar"));
        assert!(!result.contains("Foo"));
    }

    #[test]
    fn process_import_for_types_removes_vue_type_export() {
        // import { PropType } from 'vue' → None (VUE_TYPE_ONLY_EXPORTS)
        let result = process_import_for_types("import { PropType } from 'vue'");
        assert!(result.is_none());
    }

    #[test]
    fn process_import_for_types_filters_vue_mixed() {
        // import { Ref, ref } from 'vue' → only ref remains
        let result =
            process_import_for_types("import { Ref, ref } from 'vue'").unwrap();
        assert!(result.contains("ref"));
        assert!(!result.contains("Ref"));
    }

    #[test]
    fn process_import_for_types_keeps_default_with_type_named() {
        // import Foo, { type Bar } from 'baz' → default Foo preserved
        let result =
            process_import_for_types("import Foo, { type Bar } from 'baz'").unwrap();
        assert!(result.contains("Foo"));
        assert!(!result.contains("Bar"));
    }

    #[test]
    fn process_import_for_types_regular_import_unchanged() {
        // Regular import should pass through
        let result =
            process_import_for_types("import { ref, computed } from 'vue'").unwrap();
        assert!(result.contains("ref"));
        assert!(result.contains("computed"));
    }

    #[test]
    fn process_import_for_types_double_quotes_to_single() {
        // Double-quote input should be converted to single quotes in output
        let result =
            process_import_for_types("import { type Foo, Bar } from \"baz\"").unwrap();
        // The rebuilt import uses single quotes
        assert!(result.contains("from 'baz'"));
    }

    // =========================================================================
    // strip_unused_imports tests (7 cases)
    // =========================================================================

    #[test]
    fn strip_unused_imports_removes_unused() {
        let code = "import { unused } from 'foo'\nconst x = 1\n";
        let result = strip_unused_imports(code);
        assert!(!result.contains("unused"));
        assert!(result.contains("const x = 1"));
    }

    #[test]
    fn strip_unused_imports_keeps_used() {
        let code = "import { used } from 'foo'\nconst x = used()\n";
        let result = strip_unused_imports(code);
        assert!(result.contains("used"));
    }

    #[test]
    fn strip_unused_imports_removes_property_access_only() {
        // Enum.Member — Member is only used as property access, import should be removed
        let code = "import { Member } from 'enums'\nconst x = Enum.Member\n";
        let result = strip_unused_imports(code);
        assert!(!result.contains("import"));
    }

    #[test]
    fn strip_unused_imports_keeps_returned_standalone() {
        // Standalone identifier in __returned__ should count as used
        let code =
            "import { Comp } from './comp'\nconst __returned__ = { Comp }\nrender($setup.Comp)\n";
        let result = strip_unused_imports(code);
        assert!(result.contains("Comp"));
        assert!(result.contains("import"));
    }

    #[test]
    fn strip_unused_imports_skips_comment_usage() {
        // Identifier appearing only in a comment should NOT count as usage
        let code = "import { helper } from 'utils'\n// helper is deprecated\nconst x = 1\n";
        let result = strip_unused_imports(code);
        assert!(!result.contains("import"));
    }

    #[test]
    fn strip_unused_imports_multiline_import() {
        // Multi-line import should be parsed correctly
        let code = "import {\n  foo,\n  bar\n} from 'lib'\nconst x = foo()\n";
        let result = strip_unused_imports(code);
        assert!(result.contains("foo"));
        assert!(!result.contains("bar"));
    }

    #[test]
    fn strip_unused_imports_preserves_side_effect() {
        // Side-effect import should always be preserved
        let code = "import 'polyfill'\nconst x = 1\n";
        let result = strip_unused_imports(code);
        assert!(result.contains("import 'polyfill'"));
    }

    // =========================================================================
    // extract_import_identifiers tests (3 cases)
    // =========================================================================

    #[test]
    fn extract_import_identifiers_named() {
        let ids = extract_import_identifiers("import { foo, bar } from 'lib'");
        assert_eq!(ids, vec!["foo", "bar"]);
    }

    #[test]
    fn extract_import_identifiers_default() {
        let ids = extract_import_identifiers("import Foo from 'lib'");
        assert_eq!(ids, vec!["Foo"]);
    }

    #[test]
    fn extract_import_identifiers_skips_type_only() {
        let ids = extract_import_identifiers("import type { Foo } from 'lib'");
        assert!(ids.is_empty());
    }
}
