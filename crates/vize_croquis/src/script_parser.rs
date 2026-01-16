//! OXC-based script parser for high-performance AST analysis.
//!
//! Uses OXC parser to extract:
//! - Compiler macros (defineProps, defineEmits, etc.)
//! - Top-level bindings (const, let, function, class)
//! - Import statements
//! - Reactivity wrappers (ref, computed, reactive)
//! - Invalid exports in script setup
//! - Nested function scopes (arrow functions, callbacks)

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, BindingPatternKind, CallExpression, Declaration, Expression, ObjectPropertyKind,
    PropertyKey, Statement, TSType, VariableDeclarationKind,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};

use crate::analysis::{
    BindingMetadata, InvalidExport, InvalidExportKind, TypeExport, TypeExportKind,
};
use crate::macros::{
    EmitDefinition, MacroKind, MacroTracker, ModelDefinition, PropDefinition,
    PropsDestructuredBindings,
};
use crate::provide::{InjectPattern, ProvideInjectTracker, ProvideKey};
use crate::reactivity::{ReactiveKind, ReactivityTracker};
use crate::scope::{
    BlockKind, BlockScopeData, ClientOnlyScopeData, ClosureScopeData, ExternalModuleScopeData,
    NonScriptSetupScopeData, ScopeChain, ScriptSetupScopeData, VueGlobalScopeData,
};
use vize_carton::CompactString;
use vize_relief::BindingType;

/// Result of parsing a script setup block
#[derive(Debug, Default)]
pub struct ScriptParseResult {
    pub bindings: BindingMetadata,
    pub macros: MacroTracker,
    pub reactivity: ReactivityTracker,
    pub type_exports: Vec<TypeExport>,
    pub invalid_exports: Vec<InvalidExport>,
    /// Scope chain for tracking nested JavaScript scopes
    pub scopes: ScopeChain,
    /// Provide/Inject tracking
    pub provide_inject: ProvideInjectTracker,
}

/// Setup global scopes hierarchy:
/// - ~universal (JS globals) - root, @0:0 (meta)
/// - ~vue (Vue globals) - parent: ~universal, @0:0 (meta)
/// - ~mod (module = SFC) - parent: ~universal, covers entire source
fn setup_global_scopes(scopes: &mut ScopeChain, source_len: u32) {
    use crate::scope::{JsGlobalScopeData, JsRuntime};

    // Root is already ~js (JsGlobalUniversal) with common globals
    // Current scope is root (~js)

    // !client - Browser-only globals (WHATWG Living Standard + HTML timers)
    // Used as parent for onMounted, onUnmounted, etc.
    scopes.enter_js_global_scope(
        JsGlobalScopeData {
            runtime: JsRuntime::Browser,
            globals: vize_carton::smallvec![
                CompactString::const_new("alert"),
                CompactString::const_new("Audio"),
                CompactString::const_new("cancelAnimationFrame"),
                CompactString::const_new("cancelIdleCallback"),
                CompactString::const_new("CanvasRenderingContext2D"),
                CompactString::const_new("clearInterval"),
                CompactString::const_new("clearTimeout"),
                CompactString::const_new("close"),
                CompactString::const_new("confirm"),
                CompactString::const_new("customElements"),
                CompactString::const_new("document"),
                CompactString::const_new("Document"),
                CompactString::const_new("DocumentFragment"),
                CompactString::const_new("Element"),
                CompactString::const_new("FocusEvent"),
                CompactString::const_new("getComputedStyle"),
                CompactString::const_new("getSelection"),
                CompactString::const_new("history"),
                CompactString::const_new("HTMLElement"),
                CompactString::const_new("Image"),
                CompactString::const_new("indexedDB"),
                CompactString::const_new("InputEvent"),
                CompactString::const_new("IntersectionObserver"),
                CompactString::const_new("KeyboardEvent"),
                CompactString::const_new("localStorage"),
                CompactString::const_new("location"),
                CompactString::const_new("matchMedia"),
                CompactString::const_new("MediaQueryList"),
                CompactString::const_new("MouseEvent"),
                CompactString::const_new("MutationObserver"),
                CompactString::const_new("navigator"),
                CompactString::const_new("Node"),
                CompactString::const_new("NodeList"),
                CompactString::const_new("open"),
                CompactString::const_new("PerformanceObserver"),
                CompactString::const_new("PointerEvent"),
                CompactString::const_new("print"),
                CompactString::const_new("prompt"),
                CompactString::const_new("queueMicrotask"),
                CompactString::const_new("requestAnimationFrame"),
                CompactString::const_new("requestIdleCallback"),
                CompactString::const_new("ResizeObserver"),
                CompactString::const_new("screen"),
                CompactString::const_new("self"),
                CompactString::const_new("sessionStorage"),
                CompactString::const_new("setInterval"),
                CompactString::const_new("setTimeout"),
                CompactString::const_new("ShadowRoot"),
                CompactString::const_new("TouchEvent"),
                CompactString::const_new("WebGL2RenderingContext"),
                CompactString::const_new("WebGLRenderingContext"),
                CompactString::const_new("WebSocket"),
                CompactString::const_new("window"),
                CompactString::const_new("XMLHttpRequest"),
            ],
        },
        0,
        0,
    );
    scopes.exit_scope(); // Back to ~univ

    // #server - Server-only globals (WinterCG extensions, ESM-based)
    // Reserved for future SSR/Server Components support
    scopes.enter_js_global_scope(
        JsGlobalScopeData {
            runtime: JsRuntime::Node,
            globals: vize_carton::smallvec![
                CompactString::const_new("Buffer"),
                CompactString::const_new("clearImmediate"),
                CompactString::const_new("process"),
                CompactString::const_new("setImmediate"),
            ],
        },
        0,
        0,
    );
    scopes.exit_scope(); // Back to ~univ

    // ~vue - Vue globals (parent: ~univ, meta scope)
    scopes.enter_vue_global_scope(
        VueGlobalScopeData {
            globals: vize_carton::smallvec![
                CompactString::const_new("$attrs"),
                CompactString::const_new("$data"),
                CompactString::const_new("$el"),
                CompactString::const_new("$emit"),
                CompactString::const_new("$forceUpdate"),
                CompactString::const_new("$nextTick"),
                CompactString::const_new("$options"),
                CompactString::const_new("$parent"),
                CompactString::const_new("$props"),
                CompactString::const_new("$refs"),
                CompactString::const_new("$root"),
                CompactString::const_new("$slots"),
                CompactString::const_new("$watch"),
            ],
        },
        0,
        0,
    );
    scopes.exit_scope(); // Back to ~univ

    // ~mod - module scope (parent: ~js, covers entire SFC)
    scopes.enter_module_scope(0, source_len);
    // Stay in module scope - setup/plain will be created as children
}

/// Parse script setup source code using OXC parser.
///
/// This is a high-performance alternative to string-based analysis,
/// providing accurate AST-based detection with proper span tracking.
pub fn parse_script_setup(source: &str) -> ScriptParseResult {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("script.ts").unwrap_or_default();

    let ret = Parser::new(&allocator, source, source_type).parse();

    if ret.panicked {
        return ScriptParseResult::default();
    }

    let source_len = source.len() as u32;

    let mut result = ScriptParseResult {
        bindings: BindingMetadata::script_setup(),
        scopes: ScopeChain::with_capacity(16),
        ..Default::default()
    };

    // Setup global scope hierarchy (universal → mod)
    setup_global_scopes(&mut result.scopes, source_len);

    // Enter script setup scope (parent: ~mod)
    result.scopes.enter_script_setup_scope(
        ScriptSetupScopeData {
            is_ts: true,
            is_async: false,
            generic: None, // TODO: Extract from <script setup generic="T">
        },
        0,
        source_len,
    );

    // Process all statements
    for stmt in ret.program.body.iter() {
        process_statement(&mut result, stmt, source);
    }

    result
}

/// Parse non-script-setup (Options API) source code using OXC parser.
pub fn parse_script(source: &str) -> ScriptParseResult {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("script.ts").unwrap_or_default();

    let ret = Parser::new(&allocator, source, source_type).parse();

    if ret.panicked {
        return ScriptParseResult::default();
    }

    let source_len = source.len() as u32;

    let mut result = ScriptParseResult {
        bindings: BindingMetadata::new(), // Not script setup
        scopes: ScopeChain::with_capacity(16),
        ..Default::default()
    };

    // Setup global scope hierarchy (universal → mod)
    setup_global_scopes(&mut result.scopes, source_len);

    // Enter non-script-setup scope (parent: ~mod)
    result.scopes.enter_non_script_setup_scope(
        NonScriptSetupScopeData {
            is_ts: true,
            has_define_component: false,
        },
        0,
        source_len,
    );

    // Process all statements
    for stmt in ret.program.body.iter() {
        process_statement(&mut result, stmt, source);
    }

    result
}

/// Process a single statement
fn process_statement(result: &mut ScriptParseResult, stmt: &Statement<'_>, source: &str) {
    match stmt {
        // Variable declarations: const, let, var
        Statement::VariableDeclaration(decl) => {
            for declarator in decl.declarations.iter() {
                process_variable_declarator(result, declarator, decl.kind, source);
            }
        }

        // Function declarations
        Statement::FunctionDeclaration(func) => {
            if let Some(id) = &func.id {
                let name = id.name.as_str();
                result.bindings.add(name, BindingType::SetupConst);
            }

            // Create closure scope and walk body
            let params = extract_function_params(&func.params);
            let name = func
                .id
                .as_ref()
                .map(|id| CompactString::new(id.name.as_str()));

            result.scopes.enter_closure_scope(
                ClosureScopeData {
                    name,
                    param_names: params,
                    is_arrow: false,
                    is_async: func.r#async,
                    is_generator: func.generator,
                },
                func.span.start,
                func.span.end,
            );

            if let Some(body) = &func.body {
                for stmt in body.statements.iter() {
                    walk_statement(result, stmt, source);
                }
            }

            result.scopes.exit_scope();
        }

        // Class declarations
        Statement::ClassDeclaration(class) => {
            if let Some(id) = &class.id {
                let name = id.name.as_str();
                result.bindings.add(name, BindingType::SetupConst);
            }
        }

        // Expression statements (may contain macro calls and callback scopes)
        Statement::ExpressionStatement(expr_stmt) => {
            if let Expression::CallExpression(call) = &expr_stmt.expression {
                process_call_expression(result, call, source);
            }
            // Walk the expression to find callback scopes
            walk_expression(result, &expr_stmt.expression, source);
        }

        // Module declarations (imports, exports)
        Statement::ImportDeclaration(import) => {
            let is_type_only = import.import_kind.is_type();

            // Create external module scope for this import
            let source_name = import.source.value.as_str();
            let span = import.span;

            result.scopes.enter_external_module_scope(
                ExternalModuleScopeData {
                    source: CompactString::new(source_name),
                    is_type_only,
                },
                span.start,
                span.end,
            );

            if let Some(specifiers) = &import.specifiers {
                for spec in specifiers.iter() {
                    let (name, is_type_spec) = match spec {
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(s) => {
                            (s.local.name.as_str(), s.import_kind.is_type())
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                            (s.local.name.as_str(), false)
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                            (s.local.name.as_str(), false)
                        }
                    };

                    // Add binding to external module scope
                    let binding_type = if is_type_only || is_type_spec {
                        BindingType::ExternalModule
                    } else {
                        BindingType::SetupConst
                    };
                    result.scopes.add_binding(
                        CompactString::new(name),
                        crate::ScopeBinding::new(binding_type, span.start),
                    );

                    // Only add to bindings if not type-only
                    if !is_type_only && !is_type_spec {
                        result.bindings.add(name, BindingType::SetupConst);
                    }
                }
            }

            result.scopes.exit_scope();
        }

        Statement::ExportNamedDeclaration(export) => {
            if let Some(decl) = &export.declaration {
                // Check if the declaration itself is a type declaration
                match decl {
                    Declaration::TSTypeAliasDeclaration(_)
                    | Declaration::TSInterfaceDeclaration(_) => {
                        // Type exports are valid in script setup
                        process_type_export(result, decl, stmt.span());
                    }
                    _ => {
                        // Check if it's a type-only export (export type { ... })
                        if export.export_kind.is_type() {
                            process_type_export(result, decl, stmt.span());
                        } else {
                            // Value exports are invalid in script setup
                            process_invalid_export(result, decl, stmt.span());
                        }
                    }
                }
            }
        }

        Statement::ExportDefaultDeclaration(export) => {
            // Default exports are invalid in script setup
            result.invalid_exports.push(InvalidExport {
                name: CompactString::new("default"),
                kind: InvalidExportKind::Default,
                start: export.span.start,
                end: export.span.end,
            });
        }

        // Type declarations at top level
        Statement::TSTypeAliasDeclaration(type_alias) => {
            // Type aliases are allowed (not bindings, but tracked)
            let name = type_alias.id.name.as_str();
            result.type_exports.push(TypeExport {
                name: CompactString::new(name),
                kind: TypeExportKind::Type,
                start: type_alias.span.start,
                end: type_alias.span.end,
                hoisted: true,
            });
        }

        Statement::TSInterfaceDeclaration(interface) => {
            // Interfaces are allowed (not bindings, but tracked)
            let name = interface.id.name.as_str();
            result.type_exports.push(TypeExport {
                name: CompactString::new(name),
                kind: TypeExportKind::Interface,
                start: interface.span.start,
                end: interface.span.end,
                hoisted: true,
            });
        }

        _ => {}
    }
}

/// Extract a CallExpression from an expression, unwrapping type assertions (as/satisfies)
fn extract_call_expression<'a>(expr: &'a Expression<'a>) -> Option<&'a CallExpression<'a>> {
    match expr {
        Expression::CallExpression(call) => Some(call),
        Expression::TSAsExpression(ts_as) => extract_call_expression(&ts_as.expression),
        Expression::TSSatisfiesExpression(ts_satisfies) => {
            extract_call_expression(&ts_satisfies.expression)
        }
        Expression::TSNonNullExpression(ts_non_null) => {
            extract_call_expression(&ts_non_null.expression)
        }
        Expression::ParenthesizedExpression(paren) => extract_call_expression(&paren.expression),
        _ => None,
    }
}

/// Process a variable declarator
fn process_variable_declarator(
    result: &mut ScriptParseResult,
    declarator: &oxc_ast::ast::VariableDeclarator<'_>,
    kind: VariableDeclarationKind,
    source: &str,
) {
    // Handle destructuring patterns
    match &declarator.id.kind {
        BindingPatternKind::BindingIdentifier(id) => {
            let name = id.name.as_str();

            // Check if the init is a macro or reactivity call
            // Use extract_call_expression to handle type assertions (as/satisfies)
            let call_extracted =
                if let Some(call) = declarator.init.as_ref().and_then(extract_call_expression) {
                    // Check for macro calls (defineProps, defineEmits, etc.)
                    if process_call_expression(result, call, source) {
                        // Macro was processed, add binding
                        let binding_type = get_binding_type_from_kind(kind);
                        result.bindings.add(name, binding_type);
                        // Walk into the call's callback arguments to track nested scopes
                        walk_call_arguments(result, call, source);
                        return;
                    }

                    // Check for reactivity wrappers
                    if let Some((reactive_kind, binding_type)) = detect_reactivity_call(call) {
                        result
                            .reactivity
                            .register(CompactString::new(name), reactive_kind, 0);
                        result.bindings.add(name, binding_type);
                        // Walk into the call's callback arguments to track nested scopes
                        walk_call_arguments(result, call, source);
                        return;
                    }

                    // Not a known macro/reactivity, but still walk for nested scopes
                    // Note: detect_provide_inject_call is called inside walk_call_arguments
                    walk_call_arguments(result, call, source);
                    true // Call was extracted and processed
                } else {
                    false
                };

            // Walk other expression types for nested scopes
            // Skip if we already extracted and processed a call expression to avoid double processing
            if !call_extracted {
                if let Some(init) = &declarator.init {
                    walk_expression(result, init, source);
                }
            }

            // Regular binding
            let binding_type = get_binding_type_from_kind(kind);
            result.bindings.add(name, binding_type);
        }

        BindingPatternKind::ObjectPattern(obj) => {
            // Check if this is destructuring from defineProps or withDefaults(defineProps())
            let is_define_props = declarator.init.as_ref().is_some_and(|init| {
                match init {
                    Expression::CallExpression(call) => {
                        if let Expression::Identifier(id) = &call.callee {
                            let name = id.name.as_str();
                            if name == "defineProps" {
                                return true;
                            }
                            // withDefaults(defineProps<...>(), {...})
                            if name == "withDefaults" {
                                if let Some(Argument::CallExpression(inner)) =
                                    call.arguments.first()
                                {
                                    if let Expression::Identifier(inner_id) = &inner.callee {
                                        return inner_id.name.as_str() == "defineProps";
                                    }
                                }
                            }
                        }
                        false
                    }
                    _ => false,
                }
            });

            // Check if this is destructuring from inject() - this loses reactivity!
            let inject_call = declarator.init.as_ref().and_then(|init| {
                let call = extract_call_expression(init)?;
                if let Expression::Identifier(id) = &call.callee {
                    if id.name.as_str() == "inject" {
                        return Some(call);
                    }
                }
                None
            });

            // If inject(), track it with ObjectDestructure pattern
            if let Some(call) = inject_call {
                // Extract destructured property names
                let mut destructured_props: Vec<CompactString> = Vec::new();
                for prop in obj.properties.iter() {
                    if let Some(name) = get_binding_pattern_name(&prop.value.kind) {
                        destructured_props.push(CompactString::new(&name));
                    }
                }

                // Extract inject key
                if let Some(key) = call
                    .arguments
                    .first()
                    .and_then(|arg| extract_provide_key(arg, source))
                {
                    result.provide_inject.add_inject(
                        key,
                        CompactString::new("(destructured)"),
                        call.arguments
                            .get(1)
                            .map(|arg| CompactString::new(extract_argument_source(arg, source))),
                        None,
                        InjectPattern::ObjectDestructure(destructured_props.clone()),
                        None,
                        call.span.start,
                        call.span.end,
                    );
                }
            }

            // If defineProps, process it first to extract prop definitions
            if is_define_props {
                if let Some(Expression::CallExpression(call)) = &declarator.init {
                    process_call_expression(result, call, source);
                }
            }

            // Track props destructure bindings
            let mut props_destructure = if is_define_props {
                Some(PropsDestructuredBindings::default())
            } else {
                None
            };

            // Handle object destructuring
            for prop in obj.properties.iter() {
                // Get the key (prop name in defineProps)
                let key_name = match &prop.key {
                    PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
                    PropertyKey::StringLiteral(s) => Some(s.value.as_str()),
                    _ => None,
                };

                if let Some(local_name) = get_binding_pattern_name(&prop.value.kind) {
                    // If destructuring from defineProps, use Props binding type
                    let binding_type = if is_define_props {
                        BindingType::Props
                    } else {
                        get_binding_type_from_kind(kind)
                    };
                    result.bindings.add(&local_name, binding_type);

                    // Track destructure binding
                    if let Some(ref mut destructure) = props_destructure {
                        let key = key_name
                            .map(CompactString::new)
                            .unwrap_or_else(|| CompactString::new(&local_name));

                        // Extract default value if present (assignment pattern)
                        let default_value = if prop.shorthand {
                            // Check if the value is an assignment pattern with default
                            if let BindingPatternKind::AssignmentPattern(assign) = &prop.value.kind
                            {
                                Some(CompactString::new(
                                    &source[assign.right.span().start as usize
                                        ..assign.right.span().end as usize],
                                ))
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        destructure.insert(key, CompactString::new(&local_name), default_value);
                    }
                }
            }

            // Handle rest element
            if let Some(rest) = &obj.rest {
                if let Some(name) = get_binding_pattern_name(&rest.argument.kind) {
                    let binding_type = if is_define_props {
                        BindingType::Props
                    } else {
                        get_binding_type_from_kind(kind)
                    };
                    result.bindings.add(&name, binding_type);

                    // Track rest binding
                    if let Some(ref mut destructure) = props_destructure {
                        destructure.rest_id = Some(CompactString::new(&name));
                    }
                }
            }

            // Set props destructure in macro tracker
            if let Some(destructure) = props_destructure {
                if !destructure.is_empty() {
                    result.macros.set_props_destructure(destructure);
                }
            }
        }

        BindingPatternKind::ArrayPattern(arr) => {
            // Handle array destructuring
            for elem in arr.elements.iter().flatten() {
                if let Some(name) = get_binding_pattern_name(&elem.kind) {
                    let binding_type = get_binding_type_from_kind(kind);
                    result.bindings.add(&name, binding_type);
                }
            }
            if let Some(rest) = &arr.rest {
                if let Some(name) = get_binding_pattern_name(&rest.argument.kind) {
                    let binding_type = get_binding_type_from_kind(kind);
                    result.bindings.add(&name, binding_type);
                }
            }
        }

        BindingPatternKind::AssignmentPattern(assign) => {
            if let Some(name) = get_binding_pattern_name(&assign.left.kind) {
                let binding_type = get_binding_type_from_kind(kind);
                result.bindings.add(&name, binding_type);
            }
        }
    }
}

/// Get binding name from binding pattern kind
fn get_binding_pattern_name(kind: &BindingPatternKind<'_>) -> Option<String> {
    match kind {
        BindingPatternKind::BindingIdentifier(id) => Some(id.name.to_string()),
        BindingPatternKind::AssignmentPattern(assign) => {
            get_binding_pattern_name(&assign.left.kind)
        }
        _ => None,
    }
}

/// Process a call expression, returns true if it was a macro call
fn process_call_expression(
    result: &mut ScriptParseResult,
    call: &CallExpression<'_>,
    source: &str,
) -> bool {
    let callee_name = match &call.callee {
        Expression::Identifier(id) => id.name.as_str(),
        _ => return false,
    };

    let macro_kind = match MacroKind::from_name(callee_name) {
        Some(kind) => kind,
        None => return false,
    };

    let span = call.span;

    // Extract type arguments if present
    let type_args = call.type_parameters.as_ref().map(|tp| {
        let type_source = &source[tp.span.start as usize..tp.span.end as usize];
        CompactString::new(type_source)
    });

    // Extract runtime arguments
    let runtime_args = if !call.arguments.is_empty() {
        let args_start = call.arguments.first().map(|a| match a {
            Argument::SpreadElement(s) => s.span.start,
            Argument::Identifier(id) => id.span.start,
            _ => span.start,
        });
        let args_end = call.arguments.last().map(|a| match a {
            Argument::SpreadElement(s) => s.span.end,
            Argument::Identifier(id) => id.span.end,
            _ => span.end,
        });
        if let (Some(start), Some(end)) = (args_start, args_end) {
            Some(CompactString::new(&source[start as usize..end as usize]))
        } else {
            None
        }
    } else {
        None
    };

    // Add macro call
    result.macros.add_call(
        callee_name,
        macro_kind,
        span.start,
        span.end,
        runtime_args,
        type_args.clone(),
    );

    // Process macro-specific content
    match macro_kind {
        MacroKind::DefineProps => {
            // Extract props from type or runtime arguments
            if let Some(ref type_params) = call.type_parameters {
                extract_props_from_type(result, &type_params.params, source);
            } else if let Some(first_arg) = call.arguments.first() {
                extract_props_from_runtime(result, first_arg, source);
            }
        }

        MacroKind::DefineEmits => {
            // Extract emits from type or runtime arguments
            if let Some(ref type_params) = call.type_parameters {
                extract_emits_from_type(result, &type_params.params, source);
            } else if let Some(first_arg) = call.arguments.first() {
                extract_emits_from_runtime(result, first_arg, source);
            }
        }

        MacroKind::DefineModel => {
            // Extract model name (first string argument or 'modelValue' by default)
            let model_name = call
                .arguments
                .first()
                .and_then(|arg| {
                    if let Argument::StringLiteral(s) = arg {
                        Some(s.value.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or("modelValue");

            result.macros.add_model(ModelDefinition {
                name: CompactString::new(model_name),
                local_name: CompactString::new(model_name),
                model_type: None,
                required: false,
                default_value: None,
            });
        }

        MacroKind::WithDefaults => {
            // withDefaults wraps defineProps - find the inner call
            if let Some(Argument::CallExpression(inner_call)) = call.arguments.first() {
                process_call_expression(result, inner_call, source);
            }
        }

        _ => {}
    }

    true
}

/// Extract props from TypeScript type parameters
fn extract_props_from_type(
    result: &mut ScriptParseResult,
    type_params: &oxc_allocator::Vec<'_, TSType<'_>>,
    _source: &str,
) {
    for tp in type_params.iter() {
        if let TSType::TSTypeLiteral(lit) = tp {
            for member in lit.members.iter() {
                if let oxc_ast::ast::TSSignature::TSPropertySignature(prop) = member {
                    if let PropertyKey::StaticIdentifier(id) = &prop.key {
                        let name = id.name.as_str();
                        result.macros.add_prop(PropDefinition {
                            name: CompactString::new(name),
                            required: !prop.optional,
                            prop_type: None,
                            default_value: None,
                        });
                        result.bindings.add(name, BindingType::Props);
                    }
                }
            }
        }
    }
}

/// Extract props from runtime arguments (array or object)
fn extract_props_from_runtime(result: &mut ScriptParseResult, arg: &Argument<'_>, _source: &str) {
    match arg {
        // Array syntax: ['prop1', 'prop2']
        Argument::ArrayExpression(arr) => {
            for elem in arr.elements.iter() {
                if let oxc_ast::ast::ArrayExpressionElement::StringLiteral(s) = elem {
                    let name = s.value.as_str();
                    result.macros.add_prop(PropDefinition {
                        name: CompactString::new(name),
                        required: false,
                        prop_type: None,
                        default_value: None,
                    });
                    result.bindings.add(name, BindingType::Props);
                }
            }
        }

        // Object syntax: { prop1: Type, prop2: { type: Type, required: true } }
        Argument::ObjectExpression(obj) => {
            for prop in obj.properties.iter() {
                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                    if let PropertyKey::StaticIdentifier(id) = &p.key {
                        let name = id.name.as_str();
                        let required = detect_required_prop(&p.value);
                        result.macros.add_prop(PropDefinition {
                            name: CompactString::new(name),
                            required,
                            prop_type: None,
                            default_value: None,
                        });
                        result.bindings.add(name, BindingType::Props);
                    }
                }
            }
        }

        _ => {}
    }
}

/// Detect if a prop has required: true
fn detect_required_prop(value: &Expression<'_>) -> bool {
    if let Expression::ObjectExpression(obj) = value {
        for prop in obj.properties.iter() {
            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                if let PropertyKey::StaticIdentifier(id) = &p.key {
                    if id.name.as_str() == "required" {
                        if let Expression::BooleanLiteral(b) = &p.value {
                            return b.value;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Extract emits from TypeScript type parameters
fn extract_emits_from_type(
    result: &mut ScriptParseResult,
    type_params: &oxc_allocator::Vec<'_, TSType<'_>>,
    _source: &str,
) {
    for tp in type_params.iter() {
        if let TSType::TSTypeLiteral(lit) = tp {
            // Handle call signatures like { (e: 'update', value: string): void }
            for member in lit.members.iter() {
                if let oxc_ast::ast::TSSignature::TSCallSignatureDeclaration(call_sig) = member {
                    // First parameter is usually the event name: (e: 'eventName', ...)
                    if let Some(first_param) = call_sig.params.items.first() {
                        if let Some(type_ann) = &first_param.pattern.type_annotation {
                            if let TSType::TSLiteralType(lit_type) = &type_ann.type_annotation {
                                if let oxc_ast::ast::TSLiteral::StringLiteral(s) = &lit_type.literal
                                {
                                    result.macros.add_emit(EmitDefinition {
                                        name: CompactString::new(s.value.as_str()),
                                        payload_type: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Extract emits from runtime arguments (array)
fn extract_emits_from_runtime(result: &mut ScriptParseResult, arg: &Argument<'_>, _source: &str) {
    if let Argument::ArrayExpression(arr) = arg {
        for elem in arr.elements.iter() {
            if let oxc_ast::ast::ArrayExpressionElement::StringLiteral(s) = elem {
                result.macros.add_emit(EmitDefinition {
                    name: CompactString::new(s.value.as_str()),
                    payload_type: None,
                });
            }
        }
    }
}

/// Detect reactivity wrappers (ref, computed, reactive, etc.)
fn detect_reactivity_call(call: &CallExpression<'_>) -> Option<(ReactiveKind, BindingType)> {
    let callee_name = match &call.callee {
        Expression::Identifier(id) => id.name.as_str(),
        _ => return None,
    };

    match callee_name {
        "ref" | "shallowRef" => Some((ReactiveKind::Ref, BindingType::SetupRef)),
        "computed" => Some((ReactiveKind::Computed, BindingType::SetupRef)),
        "reactive" | "shallowReactive" => {
            Some((ReactiveKind::Reactive, BindingType::SetupReactiveConst))
        }
        "toRef" | "toRefs" => Some((ReactiveKind::Ref, BindingType::SetupMaybeRef)),
        _ => None,
    }
}

/// Detect provide() and inject() calls and track them
fn detect_provide_inject_call(
    result: &mut ScriptParseResult,
    call: &CallExpression<'_>,
    source: &str,
) {
    let callee_name = match &call.callee {
        Expression::Identifier(id) => id.name.as_str(),
        _ => return,
    };

    match callee_name {
        "provide" => {
            // provide(key, value)
            if call.arguments.len() >= 2 {
                let key = extract_provide_key(&call.arguments[0], source);
                let value = call
                    .arguments
                    .get(1)
                    .map(|arg| extract_argument_source(arg, source))
                    .unwrap_or_default();

                if let Some(key) = key {
                    result.provide_inject.add_provide(
                        key,
                        CompactString::new(&value),
                        None, // value_type
                        None, // from_composable
                        call.span.start,
                        call.span.end,
                    );
                }
            }
        }
        "inject" => {
            // inject(key) or inject(key, defaultValue)
            if !call.arguments.is_empty() {
                let key = extract_provide_key(&call.arguments[0], source);
                let default_value = call
                    .arguments
                    .get(1)
                    .map(|arg| CompactString::new(extract_argument_source(arg, source)));

                if let Some(key) = key {
                    result.provide_inject.add_inject(
                        key,
                        CompactString::new(""), // local_name (will be set by binding)
                        default_value,
                        None, // expected_type
                        InjectPattern::Simple,
                        None, // from_composable
                        call.span.start,
                        call.span.end,
                    );
                }
            }
        }
        _ => {}
    }
}

/// Extract a provide/inject key from an argument
fn extract_provide_key(arg: &Argument<'_>, source: &str) -> Option<ProvideKey> {
    match arg {
        Argument::StringLiteral(s) => {
            Some(ProvideKey::String(CompactString::new(s.value.as_str())))
        }
        Argument::Identifier(id) => {
            // Could be a Symbol or a variable reference - treat as Symbol for now
            Some(ProvideKey::Symbol(CompactString::new(id.name.as_str())))
        }
        _ => {
            // For complex expressions, extract source as string key
            let expr_source = extract_argument_source(arg, source);
            if !expr_source.is_empty() {
                Some(ProvideKey::String(CompactString::new(&expr_source)))
            } else {
                None
            }
        }
    }
}

/// Extract source code of an argument
fn extract_argument_source(arg: &Argument<'_>, source: &str) -> String {
    let span = match arg {
        Argument::SpreadElement(s) => s.span,
        Argument::Identifier(id) => id.span,
        Argument::StringLiteral(s) => s.span,
        Argument::NumericLiteral(n) => n.span,
        Argument::BooleanLiteral(b) => b.span,
        Argument::NullLiteral(n) => n.span,
        Argument::ArrayExpression(a) => a.span,
        Argument::ObjectExpression(o) => o.span,
        Argument::FunctionExpression(f) => f.span,
        Argument::ArrowFunctionExpression(a) => a.span,
        Argument::CallExpression(c) => c.span,
        _ => return String::new(),
    };
    source
        .get(span.start as usize..span.end as usize)
        .unwrap_or("")
        .to_string()
}

/// Get binding type from variable declaration kind
fn get_binding_type_from_kind(kind: VariableDeclarationKind) -> BindingType {
    match kind {
        VariableDeclarationKind::Const => BindingType::SetupConst,
        VariableDeclarationKind::Let => BindingType::SetupLet,
        VariableDeclarationKind::Var => BindingType::SetupLet,
        VariableDeclarationKind::Using => BindingType::SetupConst,
        VariableDeclarationKind::AwaitUsing => BindingType::SetupConst,
    }
}

/// Process type export (export type / export interface)
fn process_type_export(result: &mut ScriptParseResult, decl: &Declaration<'_>, span: Span) {
    match decl {
        Declaration::TSTypeAliasDeclaration(type_alias) => {
            result.type_exports.push(TypeExport {
                name: CompactString::new(type_alias.id.name.as_str()),
                kind: TypeExportKind::Type,
                start: span.start,
                end: span.end,
                hoisted: true,
            });
        }
        Declaration::TSInterfaceDeclaration(interface) => {
            result.type_exports.push(TypeExport {
                name: CompactString::new(interface.id.name.as_str()),
                kind: TypeExportKind::Interface,
                start: span.start,
                end: span.end,
                hoisted: true,
            });
        }
        _ => {}
    }
}

/// Process invalid export in script setup
fn process_invalid_export(result: &mut ScriptParseResult, decl: &Declaration<'_>, span: Span) {
    let (name, kind) = match decl {
        Declaration::VariableDeclaration(var_decl) => {
            let first_name = var_decl
                .declarations
                .first()
                .and_then(|d| {
                    if let BindingPatternKind::BindingIdentifier(id) = &d.id.kind {
                        Some(id.name.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or("unknown");

            let kind = match var_decl.kind {
                VariableDeclarationKind::Const => InvalidExportKind::Const,
                VariableDeclarationKind::Let => InvalidExportKind::Let,
                VariableDeclarationKind::Var => InvalidExportKind::Var,
                _ => InvalidExportKind::Const,
            };

            (first_name, kind)
        }
        Declaration::FunctionDeclaration(func) => {
            let name = func
                .id
                .as_ref()
                .map(|id| id.name.as_str())
                .unwrap_or("anonymous");
            (name, InvalidExportKind::Function)
        }
        Declaration::ClassDeclaration(class) => {
            let name = class
                .id
                .as_ref()
                .map(|id| id.name.as_str())
                .unwrap_or("anonymous");
            (name, InvalidExportKind::Class)
        }
        _ => return,
    };

    result.invalid_exports.push(InvalidExport {
        name: CompactString::new(name),
        kind,
        start: span.start,
        end: span.end,
    });
}

// =============================================================================
// Scope Walking Functions (for tracking nested JavaScript scopes)
// =============================================================================

/// Check if a function name is a client-only lifecycle hook
#[inline]
fn is_client_only_hook(name: &str) -> bool {
    matches!(
        name,
        "onMounted"
            | "onBeforeMount"
            | "onUnmounted"
            | "onBeforeUnmount"
            | "onUpdated"
            | "onBeforeUpdate"
            | "onActivated"
            | "onDeactivated"
    )
}

/// Walk an expression to find nested scopes (arrow functions, callbacks, etc.)
///
/// This is called recursively to build the scope chain for the script.
/// Performance: Only walks into expressions that might contain function scopes.
#[inline]
fn walk_expression(result: &mut ScriptParseResult, expr: &Expression<'_>, source: &str) {
    match expr {
        // Arrow functions create closure scopes (no `arguments`, no `this` binding)
        Expression::ArrowFunctionExpression(arrow) => {
            let params = extract_function_params(&arrow.params);

            result.scopes.enter_closure_scope(
                ClosureScopeData {
                    name: None,
                    param_names: params,
                    is_arrow: true,
                    is_async: arrow.r#async,
                    is_generator: false, // Arrow functions cannot be generators
                },
                arrow.span.start,
                arrow.span.end,
            );

            // Walk the body for nested scopes
            // Arrow function body is always a FunctionBody (not a variant)
            // but may have expression property set for concise arrows
            if arrow.expression {
                // Concise arrow: () => expr
                // The expression is the first statement's expression
                if let Some(Statement::ExpressionStatement(expr_stmt)) =
                    arrow.body.statements.first()
                {
                    walk_expression(result, &expr_stmt.expression, source);
                }
            } else {
                // Block arrow: () => { ... }
                for stmt in arrow.body.statements.iter() {
                    walk_statement(result, stmt, source);
                }
            }

            result.scopes.exit_scope();
        }

        // Function expressions create closure scopes
        Expression::FunctionExpression(func) => {
            let params = extract_function_params(&func.params);
            let name = func
                .id
                .as_ref()
                .map(|id| CompactString::new(id.name.as_str()));

            result.scopes.enter_closure_scope(
                ClosureScopeData {
                    name,
                    param_names: params,
                    is_arrow: false,
                    is_async: func.r#async,
                    is_generator: func.generator,
                },
                func.span.start,
                func.span.end,
            );

            // Walk the body for nested scopes
            if let Some(body) = &func.body {
                for stmt in body.statements.iter() {
                    walk_statement(result, stmt, source);
                }
            }

            result.scopes.exit_scope();
        }

        // Call expressions may contain callbacks as arguments
        Expression::CallExpression(call) => {
            walk_call_arguments(result, call, source);
        }

        // Member expressions - walk the object
        Expression::StaticMemberExpression(member) => {
            walk_expression(result, &member.object, source);
        }
        Expression::ComputedMemberExpression(member) => {
            walk_expression(result, &member.object, source);
            walk_expression(result, &member.expression, source);
        }

        // Chained expressions
        Expression::ChainExpression(chain) => match &chain.expression {
            oxc_ast::ast::ChainElement::CallExpression(call) => {
                walk_call_arguments(result, call, source);
            }
            oxc_ast::ast::ChainElement::TSNonNullExpression(expr) => {
                walk_expression(result, &expr.expression, source);
            }
            oxc_ast::ast::ChainElement::StaticMemberExpression(member) => {
                walk_expression(result, &member.object, source);
            }
            oxc_ast::ast::ChainElement::ComputedMemberExpression(member) => {
                walk_expression(result, &member.object, source);
                walk_expression(result, &member.expression, source);
            }
            oxc_ast::ast::ChainElement::PrivateFieldExpression(field) => {
                walk_expression(result, &field.object, source);
            }
        },

        // Conditional expression
        Expression::ConditionalExpression(cond) => {
            walk_expression(result, &cond.test, source);
            walk_expression(result, &cond.consequent, source);
            walk_expression(result, &cond.alternate, source);
        }

        // Logical/Binary expressions
        Expression::LogicalExpression(logical) => {
            walk_expression(result, &logical.left, source);
            walk_expression(result, &logical.right, source);
        }
        Expression::BinaryExpression(binary) => {
            walk_expression(result, &binary.left, source);
            walk_expression(result, &binary.right, source);
        }

        // Array/Object expressions
        Expression::ArrayExpression(arr) => {
            for elem in arr.elements.iter() {
                match elem {
                    oxc_ast::ast::ArrayExpressionElement::SpreadElement(spread) => {
                        walk_expression(result, &spread.argument, source);
                    }
                    oxc_ast::ast::ArrayExpressionElement::Elision(_) => {}
                    _ => {
                        if let Some(expr) = elem.as_expression() {
                            walk_expression(result, expr, source);
                        }
                    }
                }
            }
        }
        Expression::ObjectExpression(obj) => {
            for prop in obj.properties.iter() {
                match prop {
                    ObjectPropertyKind::ObjectProperty(p) => {
                        walk_expression(result, &p.value, source);
                    }
                    ObjectPropertyKind::SpreadProperty(spread) => {
                        walk_expression(result, &spread.argument, source);
                    }
                }
            }
        }

        // Await/Unary
        Expression::AwaitExpression(await_expr) => {
            walk_expression(result, &await_expr.argument, source);
        }
        Expression::UnaryExpression(unary) => {
            walk_expression(result, &unary.argument, source);
        }

        // Sequence expression
        Expression::SequenceExpression(seq) => {
            for expr in seq.expressions.iter() {
                walk_expression(result, expr, source);
            }
        }

        // Parenthesized
        Expression::ParenthesizedExpression(paren) => {
            walk_expression(result, &paren.expression, source);
        }

        // Assignment
        Expression::AssignmentExpression(assign) => {
            walk_expression(result, &assign.right, source);
        }

        // TypeScript type assertions (as, satisfies, !)
        Expression::TSAsExpression(ts_as) => {
            walk_expression(result, &ts_as.expression, source);
        }
        Expression::TSSatisfiesExpression(ts_satisfies) => {
            walk_expression(result, &ts_satisfies.expression, source);
        }
        Expression::TSNonNullExpression(ts_non_null) => {
            walk_expression(result, &ts_non_null.expression, source);
        }

        // Other expressions don't need walking for scopes
        _ => {}
    }
}

/// Walk call expression arguments to find callbacks
#[inline]
fn walk_call_arguments(result: &mut ScriptParseResult, call: &CallExpression<'_>, source: &str) {
    // First, walk the callee (might be a chained call like foo.bar().baz())
    walk_expression(result, &call.callee, source);

    // Check for provide/inject calls
    detect_provide_inject_call(result, call, source);

    // Check if this is a client-only lifecycle hook
    let is_lifecycle_hook = if let Expression::Identifier(id) = &call.callee {
        is_client_only_hook(id.name.as_str())
    } else {
        false
    };

    let hook_name = if is_lifecycle_hook {
        if let Expression::Identifier(id) = &call.callee {
            Some(id.name.as_str())
        } else {
            None
        }
    } else {
        None
    };

    // Then walk each argument
    for arg in call.arguments.iter() {
        match arg {
            Argument::SpreadElement(spread) => {
                walk_expression(result, &spread.argument, source);
            }
            _ => {
                if let Some(expr) = arg.as_expression() {
                    // If this is a lifecycle hook and the argument is a function,
                    // wrap it in a ClientOnly scope
                    if let Some(name) = hook_name {
                        match expr {
                            Expression::ArrowFunctionExpression(arrow) => {
                                // Enter client-only scope
                                result.scopes.enter_client_only_scope(
                                    ClientOnlyScopeData {
                                        hook_name: CompactString::new(name),
                                    },
                                    call.span.start,
                                    call.span.end,
                                );

                                // Now create the closure scope inside the client-only scope
                                let params = extract_function_params(&arrow.params);
                                result.scopes.enter_closure_scope(
                                    ClosureScopeData {
                                        name: None,
                                        param_names: params,
                                        is_arrow: true,
                                        is_async: arrow.r#async,
                                        is_generator: false,
                                    },
                                    arrow.span.start,
                                    arrow.span.end,
                                );

                                // Walk the body
                                if arrow.expression {
                                    if let Some(Statement::ExpressionStatement(expr_stmt)) =
                                        arrow.body.statements.first()
                                    {
                                        walk_expression(result, &expr_stmt.expression, source);
                                    }
                                } else {
                                    for stmt in arrow.body.statements.iter() {
                                        walk_statement(result, stmt, source);
                                    }
                                }

                                result.scopes.exit_scope(); // Exit closure scope
                                result.scopes.exit_scope(); // Exit client-only scope
                                continue;
                            }
                            Expression::FunctionExpression(func) => {
                                // Enter client-only scope
                                result.scopes.enter_client_only_scope(
                                    ClientOnlyScopeData {
                                        hook_name: CompactString::new(name),
                                    },
                                    call.span.start,
                                    call.span.end,
                                );

                                // Create closure scope inside client-only scope
                                let params = extract_function_params(&func.params);
                                let fn_name = func
                                    .id
                                    .as_ref()
                                    .map(|id| CompactString::new(id.name.as_str()));

                                result.scopes.enter_closure_scope(
                                    ClosureScopeData {
                                        name: fn_name,
                                        param_names: params,
                                        is_arrow: false,
                                        is_async: func.r#async,
                                        is_generator: func.generator,
                                    },
                                    func.span.start,
                                    func.span.end,
                                );

                                if let Some(body) = &func.body {
                                    for stmt in body.statements.iter() {
                                        walk_statement(result, stmt, source);
                                    }
                                }

                                result.scopes.exit_scope(); // Exit closure scope
                                result.scopes.exit_scope(); // Exit client-only scope
                                continue;
                            }
                            _ => {}
                        }
                    }
                    walk_expression(result, expr, source);
                }
            }
        }
    }
}

/// Add variable bindings from a binding pattern to the current scope
#[inline]
fn add_binding_pattern_to_scope(
    result: &mut ScriptParseResult,
    pattern: &oxc_ast::ast::BindingPattern<'_>,
    offset: u32,
) {
    let mut names = vize_carton::SmallVec::<[CompactString; 4]>::new();
    extract_param_names(pattern, &mut names);
    for name in names {
        result.scopes.add_binding(
            name,
            crate::ScopeBinding::new(BindingType::SetupConst, offset),
        );
    }
}

/// Walk a statement to find nested scopes
#[inline]
fn walk_statement(result: &mut ScriptParseResult, stmt: &Statement<'_>, source: &str) {
    match stmt {
        Statement::ExpressionStatement(expr_stmt) => {
            walk_expression(result, &expr_stmt.expression, source);
        }
        Statement::VariableDeclaration(var_decl) => {
            // Add variable bindings to current scope
            for decl in var_decl.declarations.iter() {
                add_binding_pattern_to_scope(result, &decl.id, decl.span.start);
                if let Some(init) = &decl.init {
                    walk_expression(result, init, source);
                }
            }
        }
        // Nested function declarations
        Statement::FunctionDeclaration(func) => {
            // Add function name as binding
            if let Some(id) = &func.id {
                result.scopes.add_binding(
                    CompactString::new(id.name.as_str()),
                    crate::ScopeBinding::new(BindingType::SetupConst, func.span.start),
                );
            }

            // Create closure scope
            let params = extract_function_params(&func.params);
            let name = func
                .id
                .as_ref()
                .map(|id| CompactString::new(id.name.as_str()));

            result.scopes.enter_closure_scope(
                ClosureScopeData {
                    name,
                    param_names: params,
                    is_arrow: false,
                    is_async: func.r#async,
                    is_generator: func.generator,
                },
                func.span.start,
                func.span.end,
            );

            if let Some(body) = &func.body {
                for stmt in body.statements.iter() {
                    walk_statement(result, stmt, source);
                }
            }

            result.scopes.exit_scope();
        }
        // Nested class declarations
        Statement::ClassDeclaration(class) => {
            // Add class name as binding
            if let Some(id) = &class.id {
                result.scopes.add_binding(
                    CompactString::new(id.name.as_str()),
                    crate::ScopeBinding::new(BindingType::SetupConst, class.span.start),
                );
            }
            // Walk class body for methods
            for element in class.body.body.iter() {
                if let oxc_ast::ast::ClassElement::MethodDefinition(method) = element {
                    if let Some(body) = &method.value.body {
                        let params = extract_function_params(&method.value.params);
                        result.scopes.enter_closure_scope(
                            ClosureScopeData {
                                name: None,
                                param_names: params,
                                is_arrow: false,
                                is_async: method.value.r#async,
                                is_generator: method.value.generator,
                            },
                            method.span.start,
                            method.span.end,
                        );
                        for stmt in body.statements.iter() {
                            walk_statement(result, stmt, source);
                        }
                        result.scopes.exit_scope();
                    }
                }
            }
        }
        Statement::ReturnStatement(ret) => {
            if let Some(arg) = &ret.argument {
                walk_expression(result, arg, source);
            }
        }
        Statement::BlockStatement(block) => {
            result.scopes.enter_block_scope(
                BlockScopeData {
                    kind: BlockKind::Block,
                },
                block.span.start,
                block.span.end,
            );
            for stmt in block.body.iter() {
                walk_statement(result, stmt, source);
            }
            result.scopes.exit_scope();
        }
        Statement::IfStatement(if_stmt) => {
            walk_expression(result, &if_stmt.test, source);

            // Consequent block
            result.scopes.enter_block_scope(
                BlockScopeData {
                    kind: BlockKind::If,
                },
                if_stmt.consequent.span().start,
                if_stmt.consequent.span().end,
            );
            walk_statement(result, &if_stmt.consequent, source);
            result.scopes.exit_scope();

            // Alternate block (else/else if)
            if let Some(alt) = &if_stmt.alternate {
                result.scopes.enter_block_scope(
                    BlockScopeData {
                        kind: BlockKind::Else,
                    },
                    alt.span().start,
                    alt.span().end,
                );
                walk_statement(result, alt, source);
                result.scopes.exit_scope();
            }
        }
        Statement::ForStatement(for_stmt) => {
            result.scopes.enter_block_scope(
                BlockScopeData {
                    kind: BlockKind::For,
                },
                for_stmt.span.start,
                for_stmt.span.end,
            );
            // Add loop variable bindings
            if let Some(init) = &for_stmt.init {
                match init {
                    oxc_ast::ast::ForStatementInit::VariableDeclaration(var_decl) => {
                        for decl in var_decl.declarations.iter() {
                            add_binding_pattern_to_scope(result, &decl.id, decl.span.start);
                            if let Some(init_expr) = &decl.init {
                                walk_expression(result, init_expr, source);
                            }
                        }
                    }
                    _ => {
                        // Expression init (e.g., for (i = 0; ...))
                        if let Some(expr) = init.as_expression() {
                            walk_expression(result, expr, source);
                        }
                    }
                }
            }
            if let Some(test) = &for_stmt.test {
                walk_expression(result, test, source);
            }
            if let Some(update) = &for_stmt.update {
                walk_expression(result, update, source);
            }
            walk_statement(result, &for_stmt.body, source);
            result.scopes.exit_scope();
        }
        Statement::ForInStatement(for_in) => {
            result.scopes.enter_block_scope(
                BlockScopeData {
                    kind: BlockKind::ForIn,
                },
                for_in.span.start,
                for_in.span.end,
            );
            // Add loop variable binding
            if let oxc_ast::ast::ForStatementLeft::VariableDeclaration(var_decl) = &for_in.left {
                for decl in var_decl.declarations.iter() {
                    add_binding_pattern_to_scope(result, &decl.id, decl.span.start);
                }
            }
            walk_expression(result, &for_in.right, source);
            walk_statement(result, &for_in.body, source);
            result.scopes.exit_scope();
        }
        Statement::ForOfStatement(for_of) => {
            result.scopes.enter_block_scope(
                BlockScopeData {
                    kind: BlockKind::ForOf,
                },
                for_of.span.start,
                for_of.span.end,
            );
            // Add loop variable binding
            if let oxc_ast::ast::ForStatementLeft::VariableDeclaration(var_decl) = &for_of.left {
                for decl in var_decl.declarations.iter() {
                    add_binding_pattern_to_scope(result, &decl.id, decl.span.start);
                }
            }
            walk_expression(result, &for_of.right, source);
            walk_statement(result, &for_of.body, source);
            result.scopes.exit_scope();
        }
        Statement::WhileStatement(while_stmt) => {
            result.scopes.enter_block_scope(
                BlockScopeData {
                    kind: BlockKind::While,
                },
                while_stmt.span.start,
                while_stmt.span.end,
            );
            walk_expression(result, &while_stmt.test, source);
            walk_statement(result, &while_stmt.body, source);
            result.scopes.exit_scope();
        }
        Statement::DoWhileStatement(do_while) => {
            result.scopes.enter_block_scope(
                BlockScopeData {
                    kind: BlockKind::DoWhile,
                },
                do_while.span.start,
                do_while.span.end,
            );
            walk_statement(result, &do_while.body, source);
            walk_expression(result, &do_while.test, source);
            result.scopes.exit_scope();
        }
        Statement::SwitchStatement(switch_stmt) => {
            walk_expression(result, &switch_stmt.discriminant, source);
            result.scopes.enter_block_scope(
                BlockScopeData {
                    kind: BlockKind::Switch,
                },
                switch_stmt.span.start,
                switch_stmt.span.end,
            );
            for case in switch_stmt.cases.iter() {
                if let Some(test) = &case.test {
                    walk_expression(result, test, source);
                }
                for stmt in case.consequent.iter() {
                    walk_statement(result, stmt, source);
                }
            }
            result.scopes.exit_scope();
        }
        Statement::TryStatement(try_stmt) => {
            // try block
            result.scopes.enter_block_scope(
                BlockScopeData {
                    kind: BlockKind::Try,
                },
                try_stmt.block.span.start,
                try_stmt.block.span.end,
            );
            for stmt in try_stmt.block.body.iter() {
                walk_statement(result, stmt, source);
            }
            result.scopes.exit_scope();

            // catch block
            if let Some(handler) = &try_stmt.handler {
                result.scopes.enter_block_scope(
                    BlockScopeData {
                        kind: BlockKind::Catch,
                    },
                    handler.span.start,
                    handler.span.end,
                );
                // Add catch parameter as binding if present
                if let Some(param) = &handler.param {
                    let mut names = vize_carton::SmallVec::<[CompactString; 4]>::new();
                    extract_param_names(&param.pattern, &mut names);
                    for name in names {
                        result.scopes.add_binding(
                            name,
                            crate::ScopeBinding::new(BindingType::SetupConst, handler.span.start),
                        );
                    }
                }
                for stmt in handler.body.body.iter() {
                    walk_statement(result, stmt, source);
                }
                result.scopes.exit_scope();
            }

            // finally block
            if let Some(finalizer) = &try_stmt.finalizer {
                result.scopes.enter_block_scope(
                    BlockScopeData {
                        kind: BlockKind::Finally,
                    },
                    finalizer.span.start,
                    finalizer.span.end,
                );
                for stmt in finalizer.body.iter() {
                    walk_statement(result, stmt, source);
                }
                result.scopes.exit_scope();
            }
        }
        Statement::WithStatement(with_stmt) => {
            walk_expression(result, &with_stmt.object, source);
            result.scopes.enter_block_scope(
                BlockScopeData {
                    kind: BlockKind::With,
                },
                with_stmt.body.span().start,
                with_stmt.body.span().end,
            );
            walk_statement(result, &with_stmt.body, source);
            result.scopes.exit_scope();
        }
        _ => {}
    }
}

/// Extract parameter names from function params
#[inline]
fn extract_function_params(
    params: &oxc_ast::ast::FormalParameters<'_>,
) -> vize_carton::SmallVec<[CompactString; 4]> {
    let mut names = vize_carton::SmallVec::new();

    for param in params.items.iter() {
        extract_param_names(&param.pattern, &mut names);
    }

    if let Some(rest) = &params.rest {
        extract_param_names(&rest.argument, &mut names);
    }

    names
}

/// Extract parameter names from a binding pattern
#[inline]
fn extract_param_names(
    pattern: &oxc_ast::ast::BindingPattern<'_>,
    names: &mut vize_carton::SmallVec<[CompactString; 4]>,
) {
    match &pattern.kind {
        BindingPatternKind::BindingIdentifier(id) => {
            names.push(CompactString::new(id.name.as_str()));
        }
        BindingPatternKind::ObjectPattern(obj) => {
            for prop in obj.properties.iter() {
                extract_param_names(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                extract_param_names(&rest.argument, names);
            }
        }
        BindingPatternKind::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                extract_param_names(elem, names);
            }
            if let Some(rest) = &arr.rest {
                extract_param_names(&rest.argument, names);
            }
        }
        BindingPatternKind::AssignmentPattern(assign) => {
            extract_param_names(&assign.left, names);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_define_props_type() {
        let result = parse_script_setup(
            r#"
            const props = defineProps<{
                msg: string
                count?: number
            }>()
        "#,
        );

        assert_eq!(result.macros.all_calls().len(), 1);
        assert_eq!(result.macros.props().len(), 2);

        let prop_names: Vec<_> = result
            .macros
            .props()
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert!(prop_names.contains(&"msg"));
        assert!(prop_names.contains(&"count"));
    }

    #[test]
    fn test_parse_define_props_runtime() {
        let result = parse_script_setup(
            r#"
            const props = defineProps(['foo', 'bar'])
        "#,
        );

        assert_eq!(result.macros.props().len(), 2);
    }

    #[test]
    fn test_parse_define_emits() {
        let result = parse_script_setup(
            r#"
            const emit = defineEmits(['update', 'delete'])
        "#,
        );

        assert_eq!(result.macros.all_calls().len(), 1);
        assert_eq!(result.macros.emits().len(), 2);
    }

    #[test]
    fn test_parse_reactivity() {
        let result = parse_script_setup(
            r#"
            const count = ref(0)
            const doubled = computed(() => count.value * 2)
            const state = reactive({ name: 'hello' })
        "#,
        );

        assert!(result.bindings.contains("count"));
        assert!(result.bindings.contains("doubled"));
        assert!(result.bindings.contains("state"));
        assert!(result.reactivity.is_reactive("count"));
        assert!(result.reactivity.is_reactive("doubled"));
        assert!(result.reactivity.is_reactive("state"));
    }

    #[test]
    fn test_parse_imports() {
        let result = parse_script_setup(
            r#"
            import { ref, computed } from 'vue'
            import MyComponent from './MyComponent.vue'
        "#,
        );

        assert!(result.bindings.contains("ref"));
        assert!(result.bindings.contains("computed"));
        assert!(result.bindings.contains("MyComponent"));
    }

    #[test]
    fn test_parse_invalid_exports() {
        let result = parse_script_setup(
            r#"
            export const foo = 'bar'
            export let count = 0
            export function hello() {}
            export class MyClass {}
            export default {}
        "#,
        );

        assert_eq!(result.invalid_exports.len(), 5);
    }

    #[test]
    fn test_parse_type_exports() {
        let result = parse_script_setup(
            r#"
            export type Props = { msg: string }
            export interface Emits {
                (e: 'update', value: string): void
            }
        "#,
        );

        assert_eq!(result.type_exports.len(), 2);
    }

    #[test]
    fn test_macro_span_tracking() {
        let source = "const props = defineProps<{ msg: string }>()";
        let result = parse_script_setup(source);

        let call = result.macros.all_calls().first().unwrap();
        assert!(call.start > 0);
        assert!(call.end > call.start);
        assert!(call.end as usize <= source.len());
    }

    #[test]
    fn test_nested_callback_scopes() {
        // Test: computed(() => list.map(item => item.value))
        // Should have: ScriptSetup > Callback (computed) > Callback (map)
        let result = parse_script_setup(
            r#"
            const items = computed(() => {
                return list.map(item => item.value)
            })
        "#,
        );

        // Should have at least 3 scopes:
        // 1. ScriptSetup scope (root)
        // 2. Callback scope (computed's arrow function)
        // 3. Callback scope (map's arrow function)
        assert!(
            result.scopes.len() >= 3,
            "Expected at least 3 scopes, got {}",
            result.scopes.len()
        );
    }

    #[test]
    fn test_deeply_nested_callbacks() {
        // More complex nesting: onMounted(() => { watch(() => state, () => { ... }) })
        let result = parse_script_setup(
            r#"
            onMounted(() => {
                watch(
                    () => state.value,
                    (newVal, oldVal) => {
                        console.log(newVal)
                    }
                )
            })
        "#,
        );

        // Should have at least 4 scopes:
        // 1. ScriptSetup scope (root)
        // 2. Callback scope (onMounted's arrow function)
        // 3. Callback scope (watch getter arrow function)
        // 4. Callback scope (watch callback arrow function)
        assert!(
            result.scopes.len() >= 4,
            "Expected at least 4 scopes for deeply nested callbacks, got {}",
            result.scopes.len()
        );
    }

    #[test]
    fn test_closure_params_extracted() {
        use crate::scope::{ScopeData, ScopeKind};

        let result = parse_script_setup(
            r#"
            const doubled = list.map((item, index) => item * index)
        "#,
        );

        // Find the closure scope for the map function
        let closure_scope = result.scopes.iter().find(|s| s.kind == ScopeKind::Closure);

        assert!(closure_scope.is_some(), "Should have a closure scope");

        if let ScopeData::Closure(data) = closure_scope.unwrap().data() {
            assert!(
                data.param_names.contains(&CompactString::new("item")),
                "Closure scope should have 'item' param"
            );
            assert!(
                data.param_names.contains(&CompactString::new("index")),
                "Closure scope should have 'index' param"
            );
            assert!(data.is_arrow, "Should be an arrow function");
        } else {
            panic!("Expected closure scope data");
        }
    }
}
