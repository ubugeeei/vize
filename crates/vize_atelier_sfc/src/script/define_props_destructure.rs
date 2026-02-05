//! defineProps destructure handling.
//!
//! Handles the props destructure pattern: `const { prop1, prop2 = default } = defineProps(...)`
//!
//! This module follows Vue.js core's definePropsDestructure.ts implementation.
//! Uses OXC for AST-based analysis and transformation.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BindingPattern, BindingPatternKind, Expression, ObjectPattern, Program, Statement,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use vize_carton::FxHashMap;

use crate::types::BindingType;

/// Props destructure binding info
#[derive(Debug, Clone)]
pub struct PropsDestructureBinding {
    /// Local variable name
    pub local: String,
    /// Default value expression (source text)
    pub default: Option<String>,
}

/// Props destructure bindings data
#[derive(Debug, Clone, Default)]
pub struct PropsDestructuredBindings {
    /// Map of prop key -> binding info
    pub bindings: FxHashMap<String, PropsDestructureBinding>,
    /// Rest spread identifier (if any)
    pub rest_id: Option<String>,
}

impl PropsDestructuredBindings {
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty() && self.rest_id.is_none()
    }
}

/// Process props destructure from an ObjectPattern
pub fn process_props_destructure(
    pattern: &ObjectPattern<'_>,
    source: &str,
) -> (
    PropsDestructuredBindings,
    FxHashMap<String, BindingType>,
    FxHashMap<String, String>,
) {
    let mut result = PropsDestructuredBindings::default();
    let mut binding_metadata: FxHashMap<String, BindingType> = FxHashMap::default();
    let mut props_aliases: FxHashMap<String, String> = FxHashMap::default();

    for prop in pattern.properties.iter() {
        let key = resolve_object_key(&prop.key, source);

        if let Some(key) = key {
            match &prop.value.kind {
                // Default value: { foo = 123 }
                BindingPatternKind::AssignmentPattern(assign) => {
                    if let BindingPatternKind::BindingIdentifier(id) = &assign.left.kind {
                        let local = id.name.to_string();
                        let default_expr = &source
                            [assign.right.span().start as usize..assign.right.span().end as usize];

                        result.bindings.insert(
                            key.clone(),
                            PropsDestructureBinding {
                                local: local.clone(),
                                default: Some(default_expr.to_string()),
                            },
                        );

                        // If local name differs from key, it's an alias
                        if local != key {
                            binding_metadata.insert(local.clone(), BindingType::PropsAliased);
                            props_aliases.insert(local, key);
                        } else {
                            // Same name - it's a prop
                            binding_metadata.insert(local.clone(), BindingType::Props);
                        }
                    }
                }
                // Simple destructure: { foo } or { foo: bar }
                BindingPatternKind::BindingIdentifier(id) => {
                    let local = id.name.to_string();

                    result.bindings.insert(
                        key.clone(),
                        PropsDestructureBinding {
                            local: local.clone(),
                            default: None,
                        },
                    );

                    // If local name differs from key, it's an alias
                    if local != key {
                        binding_metadata.insert(local.clone(), BindingType::PropsAliased);
                        props_aliases.insert(local, key);
                    } else {
                        // Same name - it's a prop
                        binding_metadata.insert(local.clone(), BindingType::Props);
                    }
                }
                _ => {
                    // Nested patterns not supported
                }
            }
        }
    }

    // Handle rest spread: { ...rest }
    if let Some(rest) = &pattern.rest {
        if let BindingPatternKind::BindingIdentifier(id) = &rest.argument.kind {
            let rest_name = id.name.to_string();
            result.rest_id = Some(rest_name.clone());
            binding_metadata.insert(rest_name, BindingType::SetupReactiveConst);
        }
    }

    (result, binding_metadata, props_aliases)
}

/// Resolve object key to string
fn resolve_object_key(key: &oxc_ast::ast::PropertyKey<'_>, _source: &str) -> Option<String> {
    match key {
        oxc_ast::ast::PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        oxc_ast::ast::PropertyKey::StringLiteral(lit) => Some(lit.value.to_string()),
        oxc_ast::ast::PropertyKey::NumericLiteral(lit) => Some(lit.value.to_string()),
        _ => None, // Computed keys not supported
    }
}

/// Transform destructured props references in source code.
/// Rewrites `foo` to `__props.foo` for destructured props.
pub fn transform_destructured_props(
    source: &str,
    destructured: &PropsDestructuredBindings,
) -> String {
    if destructured.is_empty() {
        return source.to_string();
    }

    // Build map of local name -> prop key
    let mut local_to_key: FxHashMap<&str, &str> = FxHashMap::default();
    for (key, binding) in &destructured.bindings {
        local_to_key.insert(binding.local.as_str(), key.as_str());
    }

    // Try AST-based transformation first
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("script.ts").unwrap_or_default();
    let ret = Parser::new(&allocator, source, source_type).parse();

    if !ret.panicked {
        // Collect rewrites: (start, end, replacement)
        let mut rewrites: Vec<(usize, usize, String)> = Vec::new();

        // Walk the AST to find identifier references
        collect_identifier_rewrites(&ret.program, source, &local_to_key, &mut rewrites);

        // Apply rewrites if any found (empty rewrites means all props are shadowed or unused)
        if !rewrites.is_empty() {
            // Apply rewrites in reverse order to preserve positions
            rewrites.sort_by(|a, b| b.0.cmp(&a.0));

            let mut result = source.to_string();
            for (start, end, replacement) in rewrites {
                result.replace_range(start..end, &replacement);
            }
            return result;
        }

        // AST parsing succeeded but no rewrites needed (props are shadowed or unused)
        return source.to_string();
    }

    // Fallback: Simple text-based transformation
    // This handles cases where AST parsing failed
    transform_props_text_based(source, &local_to_key)
}

/// Text-based transformation fallback
fn transform_props_text_based(source: &str, local_to_key: &FxHashMap<&str, &str>) -> String {
    let mut result = source.to_string();

    // Sort by length (longest first) to avoid partial replacements
    let mut props: Vec<(&str, &str)> = local_to_key.iter().map(|(k, v)| (*k, *v)).collect();
    props.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (local, key) in props {
        result = replace_identifier(&result, local, &gen_props_access_exp(key));
    }

    result
}

/// Replace identifier occurrences with proper word boundary checking
fn replace_identifier(source: &str, name: &str, replacement: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = source.chars().collect();
    let name_chars: Vec<char> = name.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check if we're at the start of the identifier
        if i + name_chars.len() <= chars.len() {
            let potential_match: String = chars[i..i + name_chars.len()].iter().collect();
            if potential_match == name {
                // Check word boundaries
                let before_ok = i == 0 || !is_identifier_char(chars[i - 1]);
                let after_ok = i + name_chars.len() >= chars.len()
                    || !is_identifier_char(chars[i + name_chars.len()]);

                // Check not preceded by . (member access) or __props already
                let not_member = i == 0 || chars[i - 1] != '.';

                if before_ok && after_ok && not_member {
                    result.push_str(replacement);
                    i += name_chars.len();
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Check if character can be part of an identifier
fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Collect identifier rewrites from AST
fn collect_identifier_rewrites<'a>(
    program: &Program<'a>,
    source: &str,
    local_to_key: &FxHashMap<&str, &str>,
    rewrites: &mut Vec<(usize, usize, String)>,
) {
    // Track local bindings that shadow destructured props
    let mut local_bindings: FxHashMap<String, bool> = FxHashMap::default();

    // Walk statements
    for stmt in program.body.iter() {
        collect_from_statement(stmt, source, local_to_key, &mut local_bindings, rewrites);
    }
}

fn collect_from_statement<'a>(
    stmt: &Statement<'a>,
    source: &str,
    local_to_key: &FxHashMap<&str, &str>,
    local_bindings: &mut FxHashMap<String, bool>,
    rewrites: &mut Vec<(usize, usize, String)>,
) {
    match stmt {
        Statement::VariableDeclaration(decl) => {
            for declarator in decl.declarations.iter() {
                // Check initializer BEFORE registering bindings
                // (so we don't accidentally skip references to props that will be shadowed)
                if let Some(init) = &declarator.init {
                    collect_from_expression(init, source, local_to_key, local_bindings, rewrites);
                }
                // Register local bindings
                register_binding_pattern(&declarator.id, local_bindings);
            }
        }
        Statement::ExpressionStatement(expr_stmt) => {
            collect_from_expression(
                &expr_stmt.expression,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
        }
        Statement::ReturnStatement(ret) => {
            if let Some(arg) = &ret.argument {
                collect_from_expression(arg, source, local_to_key, local_bindings, rewrites);
            }
        }
        Statement::IfStatement(if_stmt) => {
            collect_from_expression(
                &if_stmt.test,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
            // Walk consequent
            collect_from_statement(
                &if_stmt.consequent,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
            // Walk alternate if present
            if let Some(alt) = &if_stmt.alternate {
                collect_from_statement(alt, source, local_to_key, local_bindings, rewrites);
            }
        }
        Statement::BlockStatement(block) => {
            // Create new scope for block
            let mut inner_bindings = local_bindings.clone();
            // First pass: collect variable declarations in this block
            for block_stmt in block.body.iter() {
                if let Statement::VariableDeclaration(decl) = block_stmt {
                    for declarator in decl.declarations.iter() {
                        register_binding_pattern(&declarator.id, &mut inner_bindings);
                    }
                }
            }
            // Second pass: walk statements with updated bindings
            for block_stmt in block.body.iter() {
                collect_from_statement(
                    block_stmt,
                    source,
                    local_to_key,
                    &mut inner_bindings,
                    rewrites,
                );
            }
        }
        Statement::FunctionDeclaration(func) => {
            // Register function name as local binding
            if let Some(id) = &func.id {
                local_bindings.insert(id.name.to_string(), true);
            }
            // Walk function body with new scope
            if let Some(body) = &func.body {
                let mut inner_bindings = local_bindings.clone();
                // Register parameters
                for param in func.params.items.iter() {
                    register_binding_pattern(&param.pattern, &mut inner_bindings);
                }
                // Walk body statements
                for body_stmt in body.statements.iter() {
                    collect_from_statement(
                        body_stmt,
                        source,
                        local_to_key,
                        &mut inner_bindings,
                        rewrites,
                    );
                }
            }
        }
        Statement::ForStatement(for_stmt) => {
            let mut inner_bindings = local_bindings.clone();
            // Handle init
            if let Some(init) = &for_stmt.init {
                match init {
                    oxc_ast::ast::ForStatementInit::VariableDeclaration(decl) => {
                        for declarator in decl.declarations.iter() {
                            if let Some(init_expr) = &declarator.init {
                                collect_from_expression(
                                    init_expr,
                                    source,
                                    local_to_key,
                                    &inner_bindings,
                                    rewrites,
                                );
                            }
                            register_binding_pattern(&declarator.id, &mut inner_bindings);
                        }
                    }
                    _ => {
                        if let Some(expr) = init.as_expression() {
                            collect_from_expression(
                                expr,
                                source,
                                local_to_key,
                                &inner_bindings,
                                rewrites,
                            );
                        }
                    }
                }
            }
            // Handle test
            if let Some(test) = &for_stmt.test {
                collect_from_expression(test, source, local_to_key, &inner_bindings, rewrites);
            }
            // Handle update
            if let Some(update) = &for_stmt.update {
                collect_from_expression(update, source, local_to_key, &inner_bindings, rewrites);
            }
            // Handle body
            collect_from_statement(
                &for_stmt.body,
                source,
                local_to_key,
                &mut inner_bindings,
                rewrites,
            );
        }
        Statement::ForInStatement(for_in) => {
            let mut inner_bindings = local_bindings.clone();
            // Handle left (binding)
            if let oxc_ast::ast::ForStatementLeft::VariableDeclaration(decl) = &for_in.left {
                for declarator in decl.declarations.iter() {
                    register_binding_pattern(&declarator.id, &mut inner_bindings);
                }
            }
            // Handle right (collection being iterated)
            collect_from_expression(
                &for_in.right,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
            // Handle body
            collect_from_statement(
                &for_in.body,
                source,
                local_to_key,
                &mut inner_bindings,
                rewrites,
            );
        }
        Statement::ForOfStatement(for_of) => {
            let mut inner_bindings = local_bindings.clone();
            // Handle left (binding)
            if let oxc_ast::ast::ForStatementLeft::VariableDeclaration(decl) = &for_of.left {
                for declarator in decl.declarations.iter() {
                    register_binding_pattern(&declarator.id, &mut inner_bindings);
                }
            }
            // Handle right (collection being iterated)
            collect_from_expression(
                &for_of.right,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
            // Handle body
            collect_from_statement(
                &for_of.body,
                source,
                local_to_key,
                &mut inner_bindings,
                rewrites,
            );
        }
        Statement::WhileStatement(while_stmt) => {
            collect_from_expression(
                &while_stmt.test,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
            collect_from_statement(
                &while_stmt.body,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
        }
        Statement::DoWhileStatement(do_while) => {
            collect_from_statement(
                &do_while.body,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
            collect_from_expression(
                &do_while.test,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
        }
        Statement::TryStatement(try_stmt) => {
            // Walk try block
            for try_body_stmt in try_stmt.block.body.iter() {
                collect_from_statement(
                    try_body_stmt,
                    source,
                    local_to_key,
                    local_bindings,
                    rewrites,
                );
            }
            // Walk catch clause with new scope
            if let Some(handler) = &try_stmt.handler {
                let mut catch_bindings = local_bindings.clone();
                // Register catch parameter
                if let Some(param) = &handler.param {
                    register_binding_pattern(&param.pattern, &mut catch_bindings);
                }
                // Walk catch body
                for catch_stmt in handler.body.body.iter() {
                    collect_from_statement(
                        catch_stmt,
                        source,
                        local_to_key,
                        &mut catch_bindings,
                        rewrites,
                    );
                }
            }
            // Walk finally block
            if let Some(finalizer) = &try_stmt.finalizer {
                for finally_stmt in finalizer.body.iter() {
                    collect_from_statement(
                        finally_stmt,
                        source,
                        local_to_key,
                        local_bindings,
                        rewrites,
                    );
                }
            }
        }
        Statement::SwitchStatement(switch) => {
            collect_from_expression(
                &switch.discriminant,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
            for case in switch.cases.iter() {
                if let Some(test) = &case.test {
                    collect_from_expression(test, source, local_to_key, local_bindings, rewrites);
                }
                for case_stmt in case.consequent.iter() {
                    collect_from_statement(
                        case_stmt,
                        source,
                        local_to_key,
                        local_bindings,
                        rewrites,
                    );
                }
            }
        }
        Statement::ThrowStatement(throw) => {
            collect_from_expression(
                &throw.argument,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
        }
        Statement::LabeledStatement(labeled) => {
            collect_from_statement(
                &labeled.body,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
        }
        Statement::WithStatement(with) => {
            collect_from_expression(&with.object, source, local_to_key, local_bindings, rewrites);
            collect_from_statement(&with.body, source, local_to_key, local_bindings, rewrites);
        }
        _ => {}
    }
}

fn collect_from_expression<'a>(
    expr: &Expression<'a>,
    source: &str,
    local_to_key: &FxHashMap<&str, &str>,
    local_bindings: &FxHashMap<String, bool>,
    rewrites: &mut Vec<(usize, usize, String)>,
) {
    match expr {
        Expression::Identifier(id) => {
            let name = id.name.as_str();
            // Check if this is a destructured prop and not shadowed
            if let Some(key) = local_to_key.get(name) {
                if !local_bindings.contains_key(name) {
                    rewrites.push((
                        id.span.start as usize,
                        id.span.end as usize,
                        gen_props_access_exp(key),
                    ));
                }
            }
        }
        Expression::CallExpression(call) => {
            // Check arguments
            for arg in call.arguments.iter() {
                if let Some(expr) = arg.as_expression() {
                    collect_from_expression(expr, source, local_to_key, local_bindings, rewrites);
                }
            }
            // Check callee
            collect_from_expression(&call.callee, source, local_to_key, local_bindings, rewrites);
        }
        Expression::ArrowFunctionExpression(arrow) => {
            // Create new scope for arrow function
            let mut inner_bindings = local_bindings.clone();
            // Register parameters
            for param in arrow.params.items.iter() {
                register_binding_pattern(&param.pattern, &mut inner_bindings);
            }
            // Walk body statements - for expression bodies, OXC wraps the expression in a statement
            for stmt in arrow.body.statements.iter() {
                collect_from_statement(stmt, source, local_to_key, &mut inner_bindings, rewrites);
            }
        }
        Expression::FunctionExpression(func) => {
            // Create new scope for function
            let mut inner_bindings = local_bindings.clone();
            // Register parameters
            for param in func.params.items.iter() {
                register_binding_pattern(&param.pattern, &mut inner_bindings);
            }
            // Walk body statements
            if let Some(body) = &func.body {
                for stmt in body.statements.iter() {
                    collect_from_statement(
                        stmt,
                        source,
                        local_to_key,
                        &mut inner_bindings,
                        rewrites,
                    );
                }
            }
        }
        Expression::BinaryExpression(bin) => {
            collect_from_expression(&bin.left, source, local_to_key, local_bindings, rewrites);
            collect_from_expression(&bin.right, source, local_to_key, local_bindings, rewrites);
        }
        _ if expr.is_member_expression() => {
            // Handle MemberExpression via helper method
            if let Some(member) = expr.as_member_expression() {
                collect_from_expression(
                    member.object(),
                    source,
                    local_to_key,
                    local_bindings,
                    rewrites,
                );
            }
        }
        Expression::ObjectExpression(obj) => {
            for prop in obj.properties.iter() {
                match prop {
                    oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) => {
                        // Check for shorthand: { foo } should become { foo: __props.foo }
                        if p.shorthand {
                            if let oxc_ast::ast::PropertyKey::StaticIdentifier(id) = &p.key {
                                let name = id.name.as_str();
                                if let Some(key) = local_to_key.get(name) {
                                    if !local_bindings.contains_key(name) {
                                        // For shorthand, we need to expand it
                                        // { foo } -> { foo: __props.foo }
                                        let end = p.span.end as usize;
                                        let access = gen_props_access_exp(key);
                                        let mut suffix = String::with_capacity(access.len() + 2);
                                        suffix.push_str(": ");
                                        suffix.push_str(&access);
                                        rewrites.push((end, end, suffix));
                                    }
                                }
                            }
                        } else {
                            collect_from_expression(
                                &p.value,
                                source,
                                local_to_key,
                                local_bindings,
                                rewrites,
                            );
                        }
                    }
                    oxc_ast::ast::ObjectPropertyKind::SpreadProperty(spread) => {
                        collect_from_expression(
                            &spread.argument,
                            source,
                            local_to_key,
                            local_bindings,
                            rewrites,
                        );
                    }
                }
            }
        }
        Expression::ArrayExpression(arr) => {
            for elem in arr.elements.iter() {
                match elem {
                    oxc_ast::ast::ArrayExpressionElement::SpreadElement(spread) => {
                        collect_from_expression(
                            &spread.argument,
                            source,
                            local_to_key,
                            local_bindings,
                            rewrites,
                        );
                    }
                    oxc_ast::ast::ArrayExpressionElement::Elision(_) => {}
                    _ => {
                        if let Some(e) = elem.as_expression() {
                            collect_from_expression(
                                e,
                                source,
                                local_to_key,
                                local_bindings,
                                rewrites,
                            );
                        }
                    }
                }
            }
        }
        Expression::TemplateLiteral(template) => {
            for expr in template.expressions.iter() {
                collect_from_expression(expr, source, local_to_key, local_bindings, rewrites);
            }
        }
        Expression::ConditionalExpression(cond) => {
            collect_from_expression(&cond.test, source, local_to_key, local_bindings, rewrites);
            collect_from_expression(
                &cond.consequent,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
            collect_from_expression(
                &cond.alternate,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
        }
        Expression::LogicalExpression(log) => {
            collect_from_expression(&log.left, source, local_to_key, local_bindings, rewrites);
            collect_from_expression(&log.right, source, local_to_key, local_bindings, rewrites);
        }
        Expression::UnaryExpression(unary) => {
            collect_from_expression(
                &unary.argument,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
        }
        Expression::ParenthesizedExpression(paren) => {
            collect_from_expression(
                &paren.expression,
                source,
                local_to_key,
                local_bindings,
                rewrites,
            );
        }
        _ => {}
    }
}

fn register_binding_pattern(pattern: &BindingPattern<'_>, bindings: &mut FxHashMap<String, bool>) {
    match &pattern.kind {
        BindingPatternKind::BindingIdentifier(id) => {
            bindings.insert(id.name.to_string(), true);
        }
        BindingPatternKind::ObjectPattern(obj) => {
            for prop in obj.properties.iter() {
                register_binding_pattern(&prop.value, bindings);
            }
            if let Some(rest) = &obj.rest {
                register_binding_pattern(&rest.argument, bindings);
            }
        }
        BindingPatternKind::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                register_binding_pattern(elem, bindings);
            }
            if let Some(rest) = &arr.rest {
                register_binding_pattern(&rest.argument, bindings);
            }
        }
        BindingPatternKind::AssignmentPattern(assign) => {
            register_binding_pattern(&assign.left, bindings);
        }
    }
}

/// Generate prop access expression
pub fn gen_props_access_exp(key: &str) -> String {
    if is_simple_identifier(key) {
        let mut out = String::with_capacity(key.len() + 8);
        out.push_str("__props.");
        out.push_str(key);
        out
    } else {
        let mut out = String::with_capacity(key.len() + 10);
        out.push_str("__props[");
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{:?}", key);
        out.push(']');
        out
    }
}

/// Check if string is a simple identifier
fn is_simple_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' || c == '$' => {}
        _ => return false,
    }

    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bindings(names: &[&str]) -> PropsDestructuredBindings {
        let mut bindings = PropsDestructuredBindings::default();
        for name in names {
            bindings.bindings.insert(
                name.to_string(),
                PropsDestructureBinding {
                    local: name.to_string(),
                    default: None,
                },
            );
        }
        bindings
    }

    #[test]
    fn test_gen_props_access_exp() {
        assert_eq!(gen_props_access_exp("msg"), "__props.msg");
        assert_eq!(gen_props_access_exp("my-prop"), "__props[\"my-prop\"]");
    }

    #[test]
    fn test_transform_simple() {
        let bindings = make_bindings(&["msg"]);
        let source = "console.log(msg)";
        let result = transform_destructured_props(source, &bindings);
        assert!(result.contains("__props.msg"), "Got: {}", result);
    }

    #[test]
    fn test_transform_with_shadowing() {
        let bindings = make_bindings(&["msg"]);

        // msg is shadowed by the arrow function parameter
        let source = "const fn = (msg) => console.log(msg)";
        let result = transform_destructured_props(source, &bindings);
        // The msg inside the arrow function should NOT be rewritten
        assert!(!result.contains("__props"), "Got: {}", result);
    }

    #[test]
    fn test_transform_in_computed() {
        let bindings = make_bindings(&["count"]);

        // count inside computed arrow function should be rewritten
        let source = "const double = computed(() => count * 2)";
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.count"),
            "Expected __props.count, got: {}",
            result
        );
        assert_eq!(result, "const double = computed(() => __props.count * 2)");
    }

    #[test]
    fn test_transform_multiple_refs() {
        let bindings = make_bindings(&["foo", "bar"]);

        let source = "const result = foo + bar";
        let result = transform_destructured_props(source, &bindings);
        assert!(result.contains("__props.foo"), "Got: {}", result);
        assert!(result.contains("__props.bar"), "Got: {}", result);
    }

    // ==================== New test cases ====================

    #[test]
    fn test_transform_in_function_declaration() {
        let bindings = make_bindings(&["count"]);

        // count inside function declaration should be rewritten
        let source = r#"function double() {
    return count * 2
}"#;
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.count"),
            "Expected __props.count in function body, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_in_nested_arrow_function() {
        let bindings = make_bindings(&["msg"]);

        // msg inside nested arrow function should be rewritten
        let source = r#"const outer = () => {
    const inner = () => msg
    return inner()
}"#;
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.msg"),
            "Expected __props.msg in nested arrow, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_in_method() {
        let bindings = make_bindings(&["count"]);

        // count inside regular function expression should be rewritten
        let source = r#"const obj = {
    getCount: function() { return count }
}"#;
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.count"),
            "Expected __props.count in method, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_in_watch_callback() {
        let bindings = make_bindings(&["count"]);

        let source = r#"watch(() => count, (newVal) => {
    console.log(newVal)
})"#;
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.count"),
            "Expected __props.count in watch callback, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_shadowed_in_for_of() {
        let bindings = make_bindings(&["item"]);

        // item is shadowed in for...of
        let source = r#"for (const item of items) {
    console.log(item)
}"#;
        let result = transform_destructured_props(source, &bindings);
        // The item inside the loop should NOT be rewritten
        assert!(
            !result.contains("__props.item"),
            "item should be shadowed in for...of, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_shadowed_in_for_in() {
        let bindings = make_bindings(&["key"]);

        // key is shadowed in for...in
        let source = r#"for (const key in obj) {
    console.log(key)
}"#;
        let result = transform_destructured_props(source, &bindings);
        // The key inside the loop should NOT be rewritten
        assert!(
            !result.contains("__props.key"),
            "key should be shadowed in for...in, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_shadowed_in_catch() {
        let bindings = make_bindings(&["error"]);

        // error is shadowed in catch
        let source = r#"try {
    doSomething()
} catch (error) {
    console.log(error)
}"#;
        let result = transform_destructured_props(source, &bindings);
        // The error inside catch should NOT be rewritten
        assert!(
            !result.contains("__props.error"),
            "error should be shadowed in catch, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_in_block_scope() {
        let bindings = make_bindings(&["count"]);

        // count inside a block but not shadowed should be rewritten
        let source = r#"{
    const doubled = count * 2
    console.log(doubled)
}"#;
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.count"),
            "Expected __props.count in block scope, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_shadowed_in_block() {
        let bindings = make_bindings(&["count"]);

        // count is shadowed in block
        let source = r#"{
    const count = 10
    console.log(count)
}"#;
        let result = transform_destructured_props(source, &bindings);
        // The count inside the block should NOT be rewritten because it's shadowed
        assert!(
            !result.contains("__props.count"),
            "count should be shadowed in block, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_template_literal() {
        let bindings = make_bindings(&["name"]);

        let source = "const greeting = `Hello, ${name}!`";
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.name"),
            "Expected __props.name in template literal, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_ternary() {
        let bindings = make_bindings(&["show"]);

        let source = "const display = show ? 'visible' : 'hidden'";
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.show"),
            "Expected __props.show in ternary, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_logical_expression() {
        let bindings = make_bindings(&["enabled", "active"]);

        let source = "const isOn = enabled && active";
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.enabled"),
            "Expected __props.enabled, got: {}",
            result
        );
        assert!(
            result.contains("__props.active"),
            "Expected __props.active, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_object_shorthand() {
        let bindings = make_bindings(&["foo"]);

        let source = "const obj = { foo }";
        let result = transform_destructured_props(source, &bindings);
        // Should transform { foo } to { foo: __props.foo }
        assert!(
            result.contains("__props.foo"),
            "Expected __props.foo in object shorthand, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_array_access() {
        let bindings = make_bindings(&["items"]);

        let source = "const first = items[0]";
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.items"),
            "Expected __props.items in array access, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_member_expression() {
        let bindings = make_bindings(&["user"]);

        let source = "const name = user.name";
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.user"),
            "Expected __props.user in member expression, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_chained_member() {
        let bindings = make_bindings(&["data"]);

        let source = "const value = data.nested.value";
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.data"),
            "Expected __props.data in chained member, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_call_argument() {
        let bindings = make_bindings(&["count"]);

        let source = "doSomething(count, 'test')";
        let result = transform_destructured_props(source, &bindings);
        assert!(
            result.contains("__props.count"),
            "Expected __props.count as call argument, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_multiple_statements() {
        let bindings = make_bindings(&["msg", "count"]);

        let source = r#"console.log(msg)
const double = count * 2
return { msg, count }"#;
        let result = transform_destructured_props(source, &bindings);
        // Count occurrences of __props
        let props_count = result.matches("__props").count();
        assert!(
            props_count >= 4,
            "Expected at least 4 __props occurrences, got {}: {}",
            props_count,
            result
        );
    }

    #[test]
    fn test_no_transform_property_key() {
        let bindings = make_bindings(&["msg"]);

        // msg as property key should NOT be rewritten
        let source = "const obj = { msg: 'hello' }";
        let result = transform_destructured_props(source, &bindings);
        // The msg as key should stay as is
        assert!(
            !result.contains("__props.msg"),
            "Property key should not be transformed, got: {}",
            result
        );
    }

    #[test]
    fn test_no_transform_property_access() {
        let bindings = make_bindings(&["name"]);

        // .name should NOT be rewritten (it's property access, not reference)
        let source = "const userName = user.name";
        let result = transform_destructured_props(source, &bindings);
        // Only "name" after the dot should stay as is
        assert!(
            !result.contains("__props.name"),
            "Property access should not be transformed, got: {}",
            result
        );
    }

    #[test]
    fn test_transform_aliased_prop() {
        let mut bindings = PropsDestructuredBindings::default();
        // prop key is "message", local name is "msg"
        bindings.bindings.insert(
            "message".to_string(),
            PropsDestructureBinding {
                local: "msg".to_string(),
                default: None,
            },
        );

        let source = "console.log(msg)";
        let result = transform_destructured_props(source, &bindings);
        // Should rewrite msg to __props.message (the original key)
        assert!(
            result.contains("__props.message"),
            "Expected __props.message for aliased prop, got: {}",
            result
        );
    }

    // ==================== Snapshot tests ====================

    mod snapshots {
        use super::*;

        #[test]
        fn test_basic_usage() {
            let bindings = make_bindings(&["foo"]);
            let source = "console.log(foo)";
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_nested_scope() {
            let bindings = make_bindings(&["foo", "bar"]);
            let source = r#"function test(foo) {
    console.log(foo)
    console.log(bar)
}"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_multiple_variable_declarations() {
            let bindings = make_bindings(&["foo"]);
            let source = r#"const bar = 'fish', hello = 'world'
console.log(foo)"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_function_param_same_name() {
            let bindings = make_bindings(&["value"]);
            let source = r#"function test(value) {
    try {
    } catch {
    }
}
console.log(value)"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_aliasing() {
            let mut bindings = PropsDestructuredBindings::default();
            bindings.bindings.insert(
                "foo".to_string(),
                PropsDestructureBinding {
                    local: "x".to_string(),
                    default: None,
                },
            );
            bindings.bindings.insert(
                "foo".to_string(),
                PropsDestructureBinding {
                    local: "y".to_string(),
                    default: None,
                },
            );
            let source = r#"let a = x
let b = y"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_computed_property() {
            let bindings = make_bindings(&["count"]);
            let source = r#"const double = computed(() => count * 2)
const triple = computed(function() { return count * 3 })"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_watch_callback() {
            let bindings = make_bindings(&["count"]);
            let source = r#"watch(() => count, (newVal, oldVal) => {
    console.log('changed from', oldVal, 'to', newVal)
})"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_template_literal() {
            let bindings = make_bindings(&["name", "age"]);
            let source = r#"const greeting = `Hello ${name}, you are ${age} years old`"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_object_shorthand_props() {
            let bindings = make_bindings(&["foo", "bar"]);
            let source = r#"const obj = { foo, bar, baz: 123 }"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_complex_nested_function() {
            let bindings = make_bindings(&["data", "config"]);
            let source = r#"function processData() {
    const result = data.map(item => {
        const inner = config.map(c => c.value)
        return { item, inner }
    })
    return result
}"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_for_loops() {
            let bindings = make_bindings(&["items", "index"]);
            let source = r#"for (let i = 0; i < items.length; i++) {
    console.log(items[i])
}
for (const item of items) {
    console.log(item)
}
for (const index in items) {
    console.log(index)
}"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_try_catch_finally() {
            let bindings = make_bindings(&["error", "data"]);
            let source = r#"try {
    console.log(data)
} catch (error) {
    console.log(error)
} finally {
    console.log(error)
}"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_conditional_expression() {
            let bindings = make_bindings(&["show", "msg"]);
            let source = r#"const display = show ? msg : 'hidden'
const result = show && msg || 'default'"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_member_expression_chain() {
            let bindings = make_bindings(&["user"]);
            let source = r#"const name = user.profile.name
const email = user?.contact?.email
const id = user['id']"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_array_methods() {
            let bindings = make_bindings(&["items"]);
            let source = r#"const filtered = items.filter(x => x > 0)
const mapped = items.map(x => x * 2)
const reduced = items.reduce((acc, x) => acc + x, 0)"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_destructuring_in_params() {
            let bindings = make_bindings(&["data"]);
            let source = r#"const fn = ({ x, y }) => x + y
console.log(data)"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_switch_statement() {
            let bindings = make_bindings(&["value", "msg"]);
            let source = r#"switch (value) {
    case 1:
        console.log(msg)
        break
    case 2:
        console.log('two')
        break
    default:
        console.log(value)
}"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }

        #[test]
        fn test_while_and_do_while() {
            let bindings = make_bindings(&["count"]);
            let source = r#"while (count > 0) {
    console.log(count)
}
do {
    console.log(count)
} while (count > 0)"#;
            let result = transform_destructured_props(source, &bindings);
            insta::assert_snapshot!(result);
        }
    }
}
