//! Scope walking functions for tracking nested JavaScript scopes.
//!
//! These functions recursively walk the AST to discover:
//! - Arrow functions and function expressions (closure scopes)
//! - Block statements (if, for, while, try/catch, etc.)
//! - Client-only lifecycle hooks (onMounted, etc.)
//! - Reactivity losses (destructuring, spreading, reassignment)

use oxc_ast::ast::{
    Argument, AssignmentTarget, BindingPattern, CallExpression, Expression, ObjectPropertyKind,
    Statement,
};
use oxc_span::GetSpan;

use crate::scope::{BlockKind, BlockScopeData, ClientOnlyScopeData, ClosureScopeData};
use crate::ScopeBinding;
use vize_carton::CompactString;
use vize_relief::BindingType;

use super::extract::detect_provide_inject_call;
use super::ScriptParseResult;

/// Check if a function name is a client-only lifecycle hook
#[inline]
pub(super) fn is_client_only_hook(name: &str) -> bool {
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
pub(super) fn walk_expression(result: &mut ScriptParseResult, expr: &Expression<'_>, source: &str) {
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
                        // Check for reactive spread: { ...state }
                        if let Expression::Identifier(id) = &spread.argument {
                            let var_name = CompactString::new(id.name.as_str());
                            if result.reactivity.is_reactive(var_name.as_str()) {
                                result.reactivity.record_spread(
                                    var_name,
                                    spread.span.start,
                                    spread.span.end,
                                );
                            }
                        }
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
            // Check for reactive variable reassignment: state = newValue
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left {
                let var_name = CompactString::new(id.name.as_str());
                if result.reactivity.is_reactive(var_name.as_str()) {
                    // Use id.span for the variable name, assign.span for the full expression
                    result
                        .reactivity
                        .record_reassign(var_name, id.span.start, assign.span.end);
                }
            }
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
pub(super) fn walk_call_arguments(
    result: &mut ScriptParseResult,
    call: &CallExpression<'_>,
    source: &str,
) {
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
pub(super) fn add_binding_pattern_to_scope(
    result: &mut ScriptParseResult,
    pattern: &oxc_ast::ast::BindingPattern<'_>,
    offset: u32,
) {
    let mut names = vize_carton::SmallVec::<[CompactString; 4]>::new();
    extract_param_names(pattern, &mut names);
    for name in names {
        result
            .scopes
            .add_binding(name, ScopeBinding::new(BindingType::SetupConst, offset));
    }
}

/// Walk a statement to find nested scopes
#[inline]
pub(super) fn walk_statement(result: &mut ScriptParseResult, stmt: &Statement<'_>, source: &str) {
    match stmt {
        Statement::ExpressionStatement(expr_stmt) => {
            walk_expression(result, &expr_stmt.expression, source);
        }
        Statement::VariableDeclaration(var_decl) => {
            // Add variable bindings to current scope and check for reactivity losses
            for decl in var_decl.declarations.iter() {
                add_binding_pattern_to_scope(result, &decl.id, decl.span.start);
                if let Some(init) = &decl.init {
                    walk_expression(result, init, source);

                    // Check for ref.value extraction: const x = someRef.value
                    // This also applies in block scopes (e.g., { const x = countRef.value })
                    super::extract::check_ref_value_extraction(result, &decl.id, init);
                }
            }
        }
        // Nested function declarations
        Statement::FunctionDeclaration(func) => {
            // Add function name as binding
            if let Some(id) = &func.id {
                result.scopes.add_binding(
                    CompactString::new(id.name.as_str()),
                    ScopeBinding::new(BindingType::SetupConst, func.span.start),
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
                    ScopeBinding::new(BindingType::SetupConst, class.span.start),
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
                            ScopeBinding::new(BindingType::SetupConst, handler.span.start),
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
pub(super) fn extract_function_params(
    params: &oxc_ast::ast::FormalParameters<'_>,
) -> vize_carton::SmallVec<[CompactString; 4]> {
    let mut names = vize_carton::SmallVec::new();

    for param in params.items.iter() {
        extract_param_names(&param.pattern, &mut names);
    }

    if let Some(rest) = &params.rest {
        extract_param_names(&rest.rest.argument, &mut names);
    }

    names
}

/// Extract parameter names from a binding pattern
#[inline]
pub(super) fn extract_param_names(
    pattern: &oxc_ast::ast::BindingPattern<'_>,
    names: &mut vize_carton::SmallVec<[CompactString; 4]>,
) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            names.push(CompactString::new(id.name.as_str()));
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in obj.properties.iter() {
                extract_param_names(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                extract_param_names(&rest.argument, names);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                extract_param_names(elem, names);
            }
            if let Some(rest) = &arr.rest {
                extract_param_names(&rest.argument, names);
            }
        }
        BindingPattern::AssignmentPattern(assign) => {
            extract_param_names(&assign.left, names);
        }
    }
}
