//! Statement and variable processing for Vue scripts.
//!
//! Handles processing of:
//! - Variable declarations (const, let, var)
//! - Function and class declarations
//! - Import and export statements
//! - Type declarations

use oxc_ast::ast::{
    Argument, BindingPatternKind, Declaration, Expression, PropertyKey, Statement,
    VariableDeclarationKind,
};
use oxc_span::GetSpan;

use crate::analysis::{InvalidExport, InvalidExportKind, TypeExport, TypeExportKind};
use crate::macros::PropsDestructuredBindings;
use crate::provide::InjectPattern;
use crate::scope::{BlockKind, BlockScopeData, ClosureScopeData, ExternalModuleScopeData};
use crate::ScopeBinding;
use vize_carton::CompactString;
use vize_relief::BindingType;

use super::extract::{
    check_ref_value_extraction, detect_reactivity_call, detect_setup_context_violation,
    extract_argument_source, extract_call_expression, extract_provide_key,
    get_binding_type_from_kind, process_call_expression, process_invalid_export,
    process_type_export,
};
use super::walk::{extract_function_params, walk_call_arguments, walk_expression, walk_statement};
use super::ScriptParseResult;
use crate::macros::MacroKind;
use crate::reactivity::ReactiveKind;

/// Process a single statement
pub fn process_statement(result: &mut ScriptParseResult, stmt: &Statement<'_>, source: &str) {
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
                result
                    .binding_spans
                    .insert(CompactString::new(name), (id.span.start, id.span.end));
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
                result
                    .binding_spans
                    .insert(CompactString::new(name), (id.span.start, id.span.end));
            }
        }

        // Expression statements (may contain macro calls and callback scopes)
        Statement::ExpressionStatement(expr_stmt) => {
            if let Expression::CallExpression(call) = &expr_stmt.expression {
                // Detect setup context violations (watch, onMounted, etc.)
                detect_setup_context_violation(result, call);
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
                    let (name, is_type_spec, local_span) = match spec {
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(s) => {
                            (s.local.name.as_str(), s.import_kind.is_type(), s.local.span)
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                            (s.local.name.as_str(), false, s.local.span)
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                            (s.local.name.as_str(), false, s.local.span)
                        }
                    };

                    // Record definition span for Go-to-Definition
                    result
                        .binding_spans
                        .insert(CompactString::new(name), (local_span.start, local_span.end));

                    // Determine binding type based on specifier kind:
                    // - Named imports (ImportSpecifier) → SetupMaybeRef (could be ref/reactive)
                    // - Default/Namespace imports → SetupConst
                    let binding_type = if is_type_only || is_type_spec {
                        BindingType::ExternalModule
                    } else {
                        match spec {
                            oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(_) => {
                                BindingType::SetupMaybeRef
                            }
                            _ => BindingType::SetupConst, // default/namespace
                        }
                    };
                    result.scopes.add_binding(
                        CompactString::new(name),
                        ScopeBinding::new(binding_type, span.start),
                    );

                    // Only add to bindings if not type-only
                    if !is_type_only && !is_type_spec {
                        result.bindings.add(name, binding_type);
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

        // Block statements at top level (scoped blocks)
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

        _ => {}
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

            // Record definition span for Go-to-Definition
            result
                .binding_spans
                .insert(CompactString::new(name), (id.span.start, id.span.end));

            // Check if the init is a macro or reactivity call
            // Use extract_call_expression to handle type assertions (as/satisfies)
            let call_extracted = if let Some(call) =
                declarator.init.as_ref().and_then(extract_call_expression)
            {
                // Check for macro calls (defineProps, defineEmits, etc.)
                if let Some(macro_kind) = process_call_expression(result, call, source) {
                    // Assign binding type based on macro kind
                    let binding_type = match macro_kind {
                        MacroKind::DefineProps | MacroKind::WithDefaults => {
                            BindingType::SetupReactiveConst
                        }
                        MacroKind::DefineModel => BindingType::SetupRef,
                        _ => get_binding_type_from_kind(kind),
                    };
                    // defineModel returns a ref, register in reactivity tracker
                    if macro_kind == MacroKind::DefineModel {
                        result
                            .reactivity
                            .register(CompactString::new(name), ReactiveKind::Ref, 0);
                    }
                    result.bindings.add(name, binding_type);
                    // Walk into the call's callback arguments to track nested scopes
                    walk_call_arguments(result, call, source);
                    return;
                }

                // Check for reactivity wrappers (also handles aliases)
                if let Some((reactive_kind, binding_type)) =
                    detect_reactivity_call(call, &result.reactivity_aliases)
                {
                    // Detect setup context violations for module-level state
                    detect_setup_context_violation(result, call);

                    result
                        .reactivity
                        .register(CompactString::new(name), reactive_kind, 0);
                    result.bindings.add(name, binding_type);
                    // Walk into the call's callback arguments to track nested scopes
                    walk_call_arguments(result, call, source);
                    return;
                }

                // Check for inject() call - track with local_name for indirect destructure detection
                // Also handles inject aliases (e.g., const a = inject; const state = a('key'))
                if let Expression::Identifier(callee_id) = &call.callee {
                    let callee_name = callee_id.name.as_str();
                    let is_inject =
                        callee_name == "inject" || result.inject_aliases.contains(callee_name);
                    if is_inject && !call.arguments.is_empty() {
                        // Detect setup context violation for inject
                        detect_setup_context_violation(result, call);

                        if let Some(key) = extract_provide_key(&call.arguments[0], source) {
                            let default_value = call.arguments.get(1).map(|arg| {
                                CompactString::new(extract_argument_source(arg, source))
                            });
                            let local_name = CompactString::new(name);
                            // Track inject variable name for indirect destructure detection
                            result.inject_var_names.insert(local_name.clone());
                            result.provide_inject.add_inject(
                                key,
                                local_name, // local_name is the binding name
                                default_value,
                                None, // expected_type
                                InjectPattern::Simple,
                                None, // from_composable
                                call.span.start,
                                call.span.end,
                            );
                            // Walk into the call's callback arguments to track nested scopes
                            walk_call_arguments(result, call, source);
                            // Add binding and return
                            let binding_type = get_binding_type_from_kind(kind);
                            result.bindings.add(name, binding_type);
                            return;
                        }
                    }
                }

                // Not a known macro/reactivity/inject, but still walk for nested scopes
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

                    // Check for ref.value extraction: const x = someRef.value
                    check_ref_value_extraction(result, &declarator.id, init);

                    // Check for Vue API aliases: const a = inject, const r = ref, etc.
                    if let Expression::Identifier(id) = init {
                        let api_name = id.name.as_str();
                        match api_name {
                            "inject" => {
                                result.inject_aliases.insert(CompactString::new(name));
                            }
                            "provide" => {
                                result.provide_aliases.insert(CompactString::new(name));
                            }
                            // Reactivity APIs
                            "ref" | "shallowRef" | "reactive" | "shallowReactive"
                            | "computed" | "readonly" | "shallowReadonly"
                            | "toRef" | "toRefs" | "toValue" | "toRaw"
                            | "isRef" | "isReactive" | "isReadonly" | "isProxy"
                            | "unref" | "triggerRef" | "customRef"
                            | "markRaw" | "effectScope" | "getCurrentScope" | "onScopeDispose"
                            // Watch APIs
                            | "watch" | "watchEffect" | "watchPostEffect" | "watchSyncEffect"
                            // Lifecycle hooks
                            | "onMounted" | "onUnmounted" | "onBeforeMount" | "onBeforeUnmount"
                            | "onUpdated" | "onBeforeUpdate" | "onActivated" | "onDeactivated"
                            | "onErrorCaptured" | "onRenderTracked" | "onRenderTriggered"
                            | "onServerPrefetch"
                            // Component APIs
                            | "defineComponent" | "defineAsyncComponent"
                            | "getCurrentInstance" | "nextTick"
                            // Types (for InjectionKey tracking)
                            | "InjectionKey" => {
                                result.reactivity_aliases.insert(
                                    CompactString::new(name),
                                    CompactString::new(api_name),
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Regular binding - for const, detect literal/function expressions
            let binding_type = if kind == VariableDeclarationKind::Const {
                if let Some(init) = &declarator.init {
                    if is_literal_expression(init) {
                        BindingType::LiteralConst
                    } else if is_function_expression(init) {
                        BindingType::SetupConst
                    } else {
                        BindingType::SetupMaybeRef
                    }
                } else {
                    BindingType::SetupConst
                }
            } else {
                get_binding_type_from_kind(kind)
            };
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

            // Check if this is indirect destructuring from an inject variable
            // e.g., const state = inject('state'); const { count } = state;
            let indirect_inject_var = declarator.init.as_ref().and_then(|init| {
                if let Expression::Identifier(id) = init {
                    let var_name = CompactString::new(id.name.as_str());
                    if result.inject_var_names.contains(&var_name) {
                        return Some((var_name, id.span.start));
                    }
                }
                None
            });

            // Check if this is destructuring from a reactive variable
            // e.g., const state = reactive({...}); const { count } = state;
            let reactive_destructure_var = declarator.init.as_ref().and_then(|init| {
                if let Expression::Identifier(id) = init {
                    let var_name = CompactString::new(id.name.as_str());
                    if result.reactivity.is_reactive(var_name.as_str()) {
                        return Some((var_name, id.span.start, id.span.end));
                    }
                }
                None
            });

            // Check if this is destructuring directly from reactive() or ref().value
            // e.g., const { count } = reactive({ count: 0 })
            let direct_reactive_call = declarator.init.as_ref().and_then(|init| {
                let call = extract_call_expression(init)?;
                if let Expression::Identifier(id) = &call.callee {
                    let name = id.name.as_str();
                    if matches!(name, "reactive" | "shallowReactive") {
                        return Some((CompactString::new(name), call.span.start, call.span.end));
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
            } else if let Some((inject_var, offset)) = indirect_inject_var {
                // Indirect destructuring: const { count } = injectVar
                let mut destructured_props: Vec<CompactString> = Vec::new();
                for prop in obj.properties.iter() {
                    if let Some(name) = get_binding_pattern_name(&prop.value.kind) {
                        destructured_props.push(CompactString::new(&name));
                    }
                }

                // Find the original inject entry and update it with indirect destructure info
                // We need to record this as a new pattern variant
                result.provide_inject.add_indirect_destructure(
                    inject_var.clone(),
                    destructured_props,
                    offset,
                );
            } else if let Some((source_name, start, end)) = reactive_destructure_var {
                // Destructuring reactive variable: const { count } = state
                let mut destructured_props: Vec<CompactString> = Vec::new();
                for prop in obj.properties.iter() {
                    if let Some(name) = get_binding_pattern_name(&prop.value.kind) {
                        destructured_props.push(CompactString::new(&name));
                    }
                }
                result
                    .reactivity
                    .record_destructure(source_name, destructured_props, start, end);
            } else if let Some((fn_name, start, end)) = direct_reactive_call {
                // Direct destructuring: const { count } = reactive({ count: 0 })
                let mut destructured_props: Vec<CompactString> = Vec::new();
                for prop in obj.properties.iter() {
                    if let Some(name) = get_binding_pattern_name(&prop.value.kind) {
                        destructured_props.push(CompactString::new(&name));
                    }
                }
                use crate::reactivity::{ReactivityLoss, ReactivityLossKind};
                result.reactivity.add_loss(ReactivityLoss {
                    kind: ReactivityLossKind::ReactiveDestructure {
                        source_name: fn_name,
                        destructured_props,
                    },
                    start,
                    end,
                });
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

/// Check if an expression is a literal value (number, string, boolean, null, template literal
/// without expressions, or unary minus on a numeric literal)
fn is_literal_expression(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_) => true,
        Expression::TemplateLiteral(tpl) => tpl.expressions.is_empty(),
        Expression::UnaryExpression(unary) => {
            unary.operator == oxc_ast::ast::UnaryOperator::UnaryNegation
                && matches!(unary.argument, Expression::NumericLiteral(_))
        }
        _ => false,
    }
}

/// Check if an expression is a function expression (arrow function or function expression)
fn is_function_expression(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    )
}
