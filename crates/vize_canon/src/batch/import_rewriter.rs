//! Import rewriter for transforming .vue imports to .vue.ts.
//!
//! This module uses oxc to parse TypeScript/JavaScript files and rewrite
//! import paths that reference .vue files to .vue.ts.

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_ast_visit::walk;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Offset adjustment for source map.
#[derive(Debug, Clone)]
pub struct OffsetAdjustment {
    /// Original offset before rewrite.
    pub original_offset: u32,
    /// Adjustment amount (positive = added chars, negative = removed chars).
    pub adjustment: i32,
}

/// Result of import rewriting.
#[derive(Debug)]
pub struct RewriteResult {
    /// Rewritten code.
    pub code: String,
    /// Source map for position translation.
    pub source_map: ImportSourceMap,
}

/// Source map for import rewrites.
#[derive(Debug, Default)]
pub struct ImportSourceMap {
    adjustments: Vec<OffsetAdjustment>,
}

impl ImportSourceMap {
    /// Create a new import source map.
    pub fn new(adjustments: Vec<OffsetAdjustment>) -> Self {
        Self { adjustments }
    }

    /// Create an empty source map.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Get the original offset from a virtual offset.
    pub fn get_original_offset(&self, virtual_offset: u32) -> u32 {
        let mut cumulative: i32 = 0;
        for adj in &self.adjustments {
            let adjusted = (adj.original_offset as i32 + cumulative) as u32;
            if virtual_offset < adjusted {
                break;
            }
            cumulative += adj.adjustment;
        }
        (virtual_offset as i32 - cumulative) as u32
    }

    /// Get the virtual offset from an original offset.
    pub fn get_virtual_offset(&self, original_offset: u32) -> u32 {
        let mut cumulative: i32 = 0;
        for adj in &self.adjustments {
            if original_offset < adj.original_offset {
                break;
            }
            cumulative += adj.adjustment;
        }
        (original_offset as i32 + cumulative) as u32
    }
}

/// Import rewriter that transforms .vue imports to .vue.ts.
pub struct ImportRewriter;

impl ImportRewriter {
    /// Create a new import rewriter.
    pub fn new() -> Self {
        Self
    }

    /// Rewrite imports in the given source code.
    pub fn rewrite(&self, source: &str, source_type: SourceType) -> RewriteResult {
        let allocator = Allocator::default();
        let parser = Parser::new(&allocator, source, source_type);
        let result = parser.parse();

        let mut rewrites: Vec<(u32, u32, String)> = Vec::new();

        // Collect import/export rewrites
        for stmt in &result.program.body {
            match stmt {
                Statement::ImportDeclaration(decl) => {
                    if let Some(rewrite) = self.rewrite_module_specifier(&decl.source.value) {
                        rewrites.push((
                            decl.source.span.start + 1, // +1 to skip opening quote
                            decl.source.span.end - 1,   // -1 to skip closing quote
                            rewrite,
                        ));
                    }
                }
                Statement::ExportNamedDeclaration(decl) => {
                    if let Some(source) = &decl.source {
                        if let Some(rewrite) = self.rewrite_module_specifier(&source.value) {
                            rewrites.push((source.span.start + 1, source.span.end - 1, rewrite));
                        }
                    }
                }
                Statement::ExportAllDeclaration(decl) => {
                    if let Some(rewrite) = self.rewrite_module_specifier(&decl.source.value) {
                        rewrites.push((
                            decl.source.span.start + 1,
                            decl.source.span.end - 1,
                            rewrite,
                        ));
                    }
                }
                _ => {}
            }
        }

        // Collect dynamic imports
        let mut collector = DynamicImportCollector::new();
        collector.visit_program(&result.program);
        for (start, end, path) in collector.imports {
            if let Some(rewrite) = self.rewrite_module_specifier(&path) {
                rewrites.push((start, end, rewrite));
            }
        }

        // Sort by offset descending (process from end to start)
        rewrites.sort_by(|a, b| b.0.cmp(&a.0));

        let mut output = source.to_string();
        let mut adjustments = Vec::new();

        for (start, end, new_path) in rewrites {
            let original_len = (end - start) as i32;
            let new_len = new_path.len() as i32;

            output.replace_range(start as usize..end as usize, &new_path);

            adjustments.push(OffsetAdjustment {
                original_offset: start,
                adjustment: new_len - original_len,
            });
        }

        // Reverse to get ascending order
        adjustments.reverse();

        RewriteResult {
            code: output,
            source_map: ImportSourceMap::new(adjustments),
        }
    }

    /// Rewrite a module specifier if it's a .vue import.
    fn rewrite_module_specifier(&self, path: &str) -> Option<String> {
        // Only rewrite relative .vue imports
        if path.ends_with(".vue") && (path.starts_with("./") || path.starts_with("../")) {
            Some(format!("{}.ts", path))
        } else {
            None
        }
    }
}

impl Default for ImportRewriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Visitor to collect dynamic imports.
struct DynamicImportCollector {
    imports: Vec<(u32, u32, String)>,
}

impl DynamicImportCollector {
    fn new() -> Self {
        Self {
            imports: Vec::new(),
        }
    }
}

impl<'a> Visit<'a> for DynamicImportCollector {
    fn visit_import_expression(&mut self, expr: &oxc_ast::ast::ImportExpression<'a>) {
        // Check if the source is a string literal
        if let Expression::StringLiteral(lit) = &expr.source {
            self.imports.push((
                lit.span.start + 1, // +1 to skip opening quote
                lit.span.end - 1,   // -1 to skip closing quote
                lit.value.to_string(),
            ));
        }
        walk::walk_import_expression(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_default_import() {
        let rewriter = ImportRewriter::new();
        let source = r#"import App from './App.vue';"#;
        let result = rewriter.rewrite(source, SourceType::ts());

        assert_eq!(result.code, r#"import App from './App.vue.ts';"#);
    }

    #[test]
    fn test_rewrite_named_import() {
        let rewriter = ImportRewriter::new();
        let source = r#"import { helper, type Props } from './helper.vue';"#;
        let result = rewriter.rewrite(source, SourceType::ts());

        assert_eq!(
            result.code,
            r#"import { helper, type Props } from './helper.vue.ts';"#
        );
    }

    #[test]
    fn test_rewrite_side_effect_import() {
        let rewriter = ImportRewriter::new();
        let source = r#"import './global.vue';"#;
        let result = rewriter.rewrite(source, SourceType::ts());

        assert_eq!(result.code, r#"import './global.vue.ts';"#);
    }

    #[test]
    fn test_no_rewrite_npm_import() {
        let rewriter = ImportRewriter::new();
        let source = r#"import { ref } from 'vue';"#;
        let result = rewriter.rewrite(source, SourceType::ts());

        assert_eq!(result.code, r#"import { ref } from 'vue';"#);
    }

    #[test]
    fn test_rewrite_export_from() {
        let rewriter = ImportRewriter::new();
        let source = r#"export { default as App } from './App.vue';"#;
        let result = rewriter.rewrite(source, SourceType::ts());

        assert_eq!(
            result.code,
            r#"export { default as App } from './App.vue.ts';"#
        );
    }

    #[test]
    fn test_rewrite_dynamic_import() {
        let rewriter = ImportRewriter::new();
        let source = r#"const App = () => import('./App.vue');"#;
        let result = rewriter.rewrite(source, SourceType::ts());

        assert_eq!(result.code, r#"const App = () => import('./App.vue.ts');"#);
    }

    #[test]
    fn test_rewrite_parent_path() {
        let rewriter = ImportRewriter::new();
        let source = r#"import Parent from '../Parent.vue';"#;
        let result = rewriter.rewrite(source, SourceType::ts());

        assert_eq!(result.code, r#"import Parent from '../Parent.vue.ts';"#);
    }

    #[test]
    fn test_source_map_offset() {
        let rewriter = ImportRewriter::new();
        let source = r#"import App from './App.vue';
import { ref } from 'vue';
const x = 1;"#;
        let result = rewriter.rewrite(source, SourceType::ts());

        // .vue -> .vue.ts adds 3 characters
        // Position after the rewrite should map back correctly
        let virtual_offset = 30; // After the first import
        let original_offset = result.source_map.get_original_offset(virtual_offset);

        // The adjustment is +3 (.ts added), so virtual - 3 = original
        assert!(original_offset < virtual_offset);
    }

    #[test]
    fn test_multiple_rewrites() {
        let rewriter = ImportRewriter::new();
        let source = r#"import App from './App.vue';
import Child from './Child.vue';
import { ref } from 'vue';"#;
        let result = rewriter.rewrite(source, SourceType::ts());

        assert!(result.code.contains("./App.vue.ts"));
        assert!(result.code.contains("./Child.vue.ts"));
        assert!(result.code.contains("from 'vue'"));
    }
}
