//! Rewrite default export to a variable declaration.
//!
//! This module transforms `export default` declarations to variable declarations,
//! allowing the compiler to inject properties like render functions.

use oxc_allocator::Allocator;
use oxc_ast::ast::{ExportDefaultDeclarationKind, Statement};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

/// Rewrite `export default` to a const declaration with the given name.
/// Returns (rewritten_code, has_default_export)
pub fn rewrite_default(input: &str, as_name: &str, is_ts: bool) -> (String, bool) {
    let source_type = if is_ts {
        SourceType::ts()
    } else {
        SourceType::mjs()
    };

    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, input, source_type).parse();

    if !ret.errors.is_empty() {
        // If parsing fails, return original code
        return (input.to_string(), false);
    }

    let program = ret.program;

    // Check if there's a default export
    let has_default = program.body.iter().any(|stmt| {
        matches!(stmt, Statement::ExportDefaultDeclaration(_))
            || matches!(stmt, Statement::ExportNamedDeclaration(decl)
                if decl.specifiers.iter().any(|s| {
                    matches!(&s.exported, oxc_ast::ast::ModuleExportName::IdentifierName(name) if name.name == "default")
                        || matches!(&s.exported, oxc_ast::ast::ModuleExportName::IdentifierReference(name) if name.name == "default")
                }))
    });

    if !has_default {
        // No default export - append empty object
        let mut output = input.to_string();
        output.push_str("\nconst ");
        output.push_str(as_name);
        output.push_str(" = {}");
        return (output, false);
    }

    // Find and rewrite the default export
    let mut output = String::new();
    let mut last_end = 0;

    for stmt in program.body.iter() {
        match stmt {
            Statement::ExportDefaultDeclaration(decl) => {
                // Copy everything before this statement
                output.push_str(&input[last_end..decl.span.start as usize]);

                match &decl.declaration {
                    ExportDefaultDeclarationKind::ClassDeclaration(class_decl) => {
                        // export default class Foo {} -> class Foo {} \n const as_name = Foo
                        if let Some(id) = &class_decl.id {
                            output.push_str("class ");
                            output.push_str(id.name.as_str());
                            // Copy the rest of the class declaration
                            let class_body_start = id.span.end as usize;
                            let class_body = &input[class_body_start..decl.span.end as usize];
                            output.push_str(class_body);
                            output.push_str("\nconst ");
                            output.push_str(as_name);
                            output.push_str(" = ");
                            output.push_str(id.name.as_str());
                        } else {
                            // Anonymous class - wrap in const
                            output.push_str("const ");
                            output.push_str(as_name);
                            output.push_str(" = ");
                            let class_start = class_decl.span.start as usize;
                            output.push_str(&input[class_start..decl.span.end as usize]);
                        }
                    }
                    ExportDefaultDeclarationKind::FunctionDeclaration(func_decl) => {
                        // export default function foo() {} -> function foo() {} \n const as_name = foo
                        if let Some(id) = &func_decl.id {
                            output.push_str("function ");
                            output.push_str(id.name.as_str());
                            // Copy the rest of the function
                            let func_body_start = id.span.end as usize;
                            let func_body = &input[func_body_start..decl.span.end as usize];
                            output.push_str(func_body);
                            output.push_str("\nconst ");
                            output.push_str(as_name);
                            output.push_str(" = ");
                            output.push_str(id.name.as_str());
                        } else {
                            // Anonymous function - wrap in const
                            output.push_str("const ");
                            output.push_str(as_name);
                            output.push_str(" = ");
                            let func_start = func_decl.span.start as usize;
                            output.push_str(&input[func_start..decl.span.end as usize]);
                        }
                    }
                    _ => {
                        // export default {...} -> const as_name = {...}
                        output.push_str("const ");
                        output.push_str(as_name);
                        output.push_str(" = ");
                        let expr_start = decl.declaration.span().start as usize;
                        let expr_end = decl.declaration.span().end as usize;
                        output.push_str(&input[expr_start..expr_end]);
                    }
                }

                last_end = decl.span.end as usize;
            }
            Statement::ExportNamedDeclaration(named_decl) => {
                // Handle: export { foo as default }
                let has_default_specifier = named_decl.specifiers.iter().any(|s| {
                    matches!(&s.exported, oxc_ast::ast::ModuleExportName::IdentifierName(name) if name.name == "default")
                        || matches!(&s.exported, oxc_ast::ast::ModuleExportName::IdentifierReference(name) if name.name == "default")
                });

                if has_default_specifier {
                    // Copy everything before this statement
                    output.push_str(&input[last_end..named_decl.span.start as usize]);

                    if let Some(source) = &named_decl.source {
                        // export { default } from '...' or export { foo as default } from '...'
                        for specifier in &named_decl.specifiers {
                            let is_default = matches!(&specifier.exported,
                                oxc_ast::ast::ModuleExportName::IdentifierName(name) if name.name == "default")
                                || matches!(&specifier.exported,
                                    oxc_ast::ast::ModuleExportName::IdentifierReference(name) if name.name == "default");

                            if is_default {
                                let local_name = match &specifier.local {
                                    oxc_ast::ast::ModuleExportName::IdentifierName(name) => {
                                        name.name.as_str()
                                    }
                                    oxc_ast::ast::ModuleExportName::IdentifierReference(name) => {
                                        name.name.as_str()
                                    }
                                    _ => "default",
                                };

                                // Add import for the default
                                output.push_str("import { ");
                                output.push_str(local_name);
                                output.push_str(" as __VUE_DEFAULT__ } from '");
                                output.push_str(source.value.as_str());
                                output.push_str("'\n");
                            }
                        }

                        // Rebuild export without the default specifier
                        let other_specifiers: Vec<_> = named_decl
                            .specifiers
                            .iter()
                            .filter(|s| {
                                !matches!(&s.exported,
                                    oxc_ast::ast::ModuleExportName::IdentifierName(name) if name.name == "default")
                                    && !matches!(&s.exported,
                                        oxc_ast::ast::ModuleExportName::IdentifierReference(name) if name.name == "default")
                            })
                            .collect();

                        if !other_specifiers.is_empty() {
                            output.push_str("export { ");
                            for (i, spec) in other_specifiers.iter().enumerate() {
                                if i > 0 {
                                    output.push_str(", ");
                                }
                                let local = match &spec.local {
                                    oxc_ast::ast::ModuleExportName::IdentifierName(name) => {
                                        name.name.as_str()
                                    }
                                    oxc_ast::ast::ModuleExportName::IdentifierReference(name) => {
                                        name.name.as_str()
                                    }
                                    _ => continue,
                                };
                                let exported = match &spec.exported {
                                    oxc_ast::ast::ModuleExportName::IdentifierName(name) => {
                                        name.name.as_str()
                                    }
                                    oxc_ast::ast::ModuleExportName::IdentifierReference(name) => {
                                        name.name.as_str()
                                    }
                                    _ => continue,
                                };
                                if local == exported {
                                    output.push_str(local);
                                } else {
                                    output.push_str(local);
                                    output.push_str(" as ");
                                    output.push_str(exported);
                                }
                            }
                            output.push_str(" } from '");
                            output.push_str(source.value.as_str());
                            output.push_str("'\n");
                        }

                        output.push_str("const ");
                        output.push_str(as_name);
                        output.push_str(" = __VUE_DEFAULT__");
                    } else {
                        // export { foo as default } (no source)
                        for specifier in &named_decl.specifiers {
                            let is_default = matches!(&specifier.exported,
                                oxc_ast::ast::ModuleExportName::IdentifierName(name) if name.name == "default")
                                || matches!(&specifier.exported,
                                    oxc_ast::ast::ModuleExportName::IdentifierReference(name) if name.name == "default");

                            if is_default {
                                let local_name = match &specifier.local {
                                    oxc_ast::ast::ModuleExportName::IdentifierName(name) => {
                                        name.name.as_str()
                                    }
                                    oxc_ast::ast::ModuleExportName::IdentifierReference(name) => {
                                        name.name.as_str()
                                    }
                                    _ => "default",
                                };

                                // Rebuild export without the default specifier
                                let other_specifiers: Vec<_> = named_decl
                                    .specifiers
                                    .iter()
                                    .filter(|s| {
                                        !matches!(&s.exported,
                                            oxc_ast::ast::ModuleExportName::IdentifierName(name) if name.name == "default")
                                            && !matches!(&s.exported,
                                                oxc_ast::ast::ModuleExportName::IdentifierReference(name) if name.name == "default")
                                    })
                                    .collect();

                                if !other_specifiers.is_empty() {
                                    output.push_str("export { ");
                                    for (i, spec) in other_specifiers.iter().enumerate() {
                                        if i > 0 {
                                            output.push_str(", ");
                                        }
                                        let local = match &spec.local {
                                            oxc_ast::ast::ModuleExportName::IdentifierName(
                                                name,
                                            ) => name.name.as_str(),
                                            oxc_ast::ast::ModuleExportName::IdentifierReference(
                                                name,
                                            ) => name.name.as_str(),
                                            _ => continue,
                                        };
                                        let exported = match &spec.exported {
                                            oxc_ast::ast::ModuleExportName::IdentifierName(
                                                name,
                                            ) => name.name.as_str(),
                                            oxc_ast::ast::ModuleExportName::IdentifierReference(
                                                name,
                                            ) => name.name.as_str(),
                                            _ => continue,
                                        };
                                        if local == exported {
                                            output.push_str(local);
                                        } else {
                                            output.push_str(local);
                                            output.push_str(" as ");
                                            output.push_str(exported);
                                        }
                                    }
                                    output.push_str(" }\n");
                                }

                                output.push_str("const ");
                                output.push_str(as_name);
                                output.push_str(" = ");
                                output.push_str(local_name);
                                break;
                            }
                        }
                    }

                    last_end = named_decl.span.end as usize;
                }
            }
            _ => {}
        }
    }

    // Copy remaining content
    if last_end < input.len() {
        output.push_str(&input[last_end..]);
    }

    (output, has_default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_default_object() {
        let (result, has_default) = rewrite_default("export default {}", "_sfc_main", false);
        assert!(has_default);
        assert!(result.contains("const _sfc_main = {}"));
        assert!(!result.contains("export default"));
    }

    #[test]
    fn test_rewrite_default_with_other_code() {
        let input = r#"
import { ref } from 'vue'

const count = ref(0)

export default {
  name: 'MyComponent'
}
"#;
        let (result, has_default) = rewrite_default(input, "_sfc_main", false);
        assert!(has_default);
        assert!(result.contains("const _sfc_main = {"));
        assert!(result.contains("name: 'MyComponent'"));
        assert!(!result.contains("export default"));
    }

    #[test]
    fn test_rewrite_default_class() {
        let (result, has_default) =
            rewrite_default("export default class Foo {}", "_sfc_main", false);
        assert!(has_default);
        assert!(result.contains("class Foo {}"));
        assert!(result.contains("const _sfc_main = Foo"));
    }

    #[test]
    fn test_no_default_export() {
        let (result, has_default) = rewrite_default("export const a = {}", "_sfc_main", false);
        assert!(!has_default);
        assert!(result.contains("const _sfc_main = {}"));
    }

    #[test]
    fn test_named_default_export() {
        let input = "const a = 1\nexport { a as default }";
        let (result, has_default) = rewrite_default(input, "_sfc_main", false);
        assert!(has_default);
        assert!(result.contains("const _sfc_main = a"));
    }
}
