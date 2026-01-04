//! Expression transform.
//!
//! Transforms expressions by prefixing identifiers with `_ctx.` for proper
//! context binding in the compiled render function (script setup mode).

use once_cell::sync::Lazy;
use oxc_allocator::Allocator as OxcAllocator;
use oxc_ast::ast as oxc_ast_types;
use oxc_ast::Visit;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use rustc_hash::FxHashSet;
use vue_allocator::{Box, Bump, String};

use crate::ast::{CompoundExpressionNode, ConstantType, ExpressionNode, SimpleExpressionNode};
use crate::transform::TransformContext;

/// JavaScript global objects and functions that should NOT be prefixed with _ctx.
static JS_GLOBALS: Lazy<FxHashSet<&'static str>> = Lazy::new(|| {
    [
        // ES globals
        "Infinity",
        "undefined",
        "NaN",
        // Global objects
        "Array",
        "Boolean",
        "Date",
        "Error",
        "Function",
        "JSON",
        "Math",
        "Number",
        "Object",
        "Promise",
        "Proxy",
        "Reflect",
        "RegExp",
        "Set",
        "String",
        "Symbol",
        "Map",
        "WeakMap",
        "WeakSet",
        "BigInt",
        // Global functions
        "parseInt",
        "parseFloat",
        "isNaN",
        "isFinite",
        "decodeURI",
        "decodeURIComponent",
        "encodeURI",
        "encodeURIComponent",
        // Special
        "arguments",
        "console",
        "window",
        "document",
        "navigator",
        "globalThis",
        // Vue
        "$event",
        "$el",
        "$refs",
        "$slots",
        "$attrs",
        "$emit",
        "$props",
        "$setup",
        "$data",
        "$options",
        "$watch",
        "$forceUpdate",
        "$nextTick",
        // Internal render function parameters
        "_ctx",
        "_cache",
        // Common helpers
        "require",
        "import",
        "exports",
        "module",
    ]
    .into_iter()
    .collect()
});

/// Process expression with identifier prefixing and TypeScript stripping
pub fn process_expression<'a>(
    ctx: &mut TransformContext<'a>,
    exp: &ExpressionNode<'a>,
    as_params: bool,
) -> ExpressionNode<'a> {
    let allocator = ctx.allocator;

    // If not prefixing identifiers and not TypeScript, just clone
    if !ctx.options.prefix_identifiers && !ctx.options.is_ts {
        return clone_expression(exp, allocator);
    }

    match exp {
        ExpressionNode::Simple(simple) => {
            if simple.is_static {
                return clone_expression(exp, allocator);
            }

            let content = &simple.content;

            // Empty content
            if content.is_empty() {
                return clone_expression(exp, allocator);
            }

            // Strip TypeScript if needed, then optionally prefix identifiers
            let processed = if ctx.options.prefix_identifiers {
                // rewrite_expression handles both TS stripping and prefixing
                rewrite_expression(content, ctx, as_params)
            } else if ctx.options.is_ts {
                // Only strip TypeScript, no prefixing
                strip_typescript_from_expression(content)
            } else {
                content.to_string()
            };

            ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode {
                    content: String::new(&processed),
                    is_static: false,
                    const_type: simple.const_type,
                    loc: simple.loc.clone(),
                    js_ast: None,
                    hoisted: None,
                    identifiers: None,
                    is_handler_key: simple.is_handler_key,
                },
                allocator,
            ))
        }
        ExpressionNode::Compound(_compound) => {
            // For compound expressions, process each child
            clone_expression(exp, allocator)
        }
    }
}

/// Rewrite an expression string, prefixing identifiers with _ctx. where needed
fn rewrite_expression(
    content: &str,
    ctx: &TransformContext<'_>,
    _as_params: bool,
) -> std::string::String {
    // First, if this is TypeScript, strip type annotations
    let js_content = if ctx.options.is_ts {
        strip_typescript_from_expression(content)
    } else {
        content.to_string()
    };

    // Try to parse as a JavaScript expression
    let oxc_allocator = OxcAllocator::default();
    let source_type = SourceType::default().with_module(true);

    // Wrap in parentheses to make it a valid expression statement
    let wrapped = format!("({})", js_content);
    let parser = Parser::new(&oxc_allocator, &wrapped, source_type);
    let parse_result = parser.parse_expression();

    match parse_result {
        Ok(expr) => {
            // Successfully parsed - walk the AST and collect identifiers to rewrite
            let mut collector = IdentifierCollector::new(ctx);
            collector.visit_expression(&expr);

            // Combine prefix rewrites (from HashSet) with suffix rewrites
            // Each rewrite is (position, prefix, suffix)
            let mut all_rewrites: Vec<(usize, &str, &str)> = collector
                .rewrites
                .into_iter()
                .map(|(pos, prefix)| (pos, prefix, ""))
                .collect();

            // Add suffix rewrites (suffixes come after the identifier)
            for (pos, suffix) in collector.suffix_rewrites {
                all_rewrites.push((pos, "", suffix));
            }

            // Sort by position descending so we can replace from end to start
            all_rewrites.sort_by(|a, b| b.0.cmp(&a.0));

            // Apply rewrites
            let mut result = js_content.clone();
            for (pos, prefix, suffix) in all_rewrites {
                // Adjust position for the wrapping parenthesis we added
                let adjusted_pos = pos.saturating_sub(1);
                if adjusted_pos <= result.len() {
                    if !suffix.is_empty() {
                        // Insert suffix at the end of identifier
                        result.insert_str(adjusted_pos, suffix);
                    }
                    if !prefix.is_empty() {
                        // Insert prefix at the start of identifier
                        result.insert_str(adjusted_pos, prefix);
                    }
                }
            }

            result
        }
        Err(_) => {
            // Parse failed - fallback to simple identifier check
            if is_simple_identifier(&js_content) {
                if should_prefix_identifier(&js_content, ctx) {
                    ["_ctx.", &js_content].concat()
                } else if is_ref_binding_simple(&js_content, ctx) {
                    // Add .value for refs in inline mode
                    [&js_content, ".value"].concat()
                } else {
                    js_content
                }
            } else {
                js_content
            }
        }
    }
}

/// Check if expression contains TypeScript syntax that needs stripping
fn needs_typescript_stripping(content: &str) -> bool {
    // Quick check for common TypeScript patterns
    // - " as " is TypeScript type assertion
    // - We avoid checking ": " as it's also used in object literals
    // - Generic types like "Array<string>" - but we need to be careful not to match comparison operators
    content.contains(" as ")
}

/// Strip TypeScript type annotations from an expression
pub fn strip_typescript_from_expression(content: &str) -> std::string::String {
    // Only process if TypeScript syntax is detected
    if !needs_typescript_stripping(content) {
        return content.to_string();
    }

    let allocator = OxcAllocator::default();
    let source_type = SourceType::ts();

    // Wrap in a dummy statement to make it parseable
    let wrapped = format!("const _expr_ = ({});", content);
    let parser = Parser::new(&allocator, &wrapped, source_type);
    let parse_result = parser.parse();

    if !parse_result.errors.is_empty() {
        // If parsing fails, return original content
        return content.to_string();
    }

    let mut program = parse_result.program;

    // Run semantic analysis
    let semantic_ret = SemanticBuilder::new()
        .with_excess_capacity(2.0)
        .build(&program);

    if !semantic_ret.errors.is_empty() {
        return content.to_string();
    }

    let (symbols, scopes) = semantic_ret.semantic.into_symbol_table_and_scope_tree();

    // Transform TypeScript to JavaScript
    let transform_options = TransformOptions::default();
    let ret = Transformer::new(&allocator, std::path::Path::new(""), &transform_options)
        .build_with_symbols_and_scopes(symbols, scopes, &mut program);

    if !ret.errors.is_empty() {
        return content.to_string();
    }

    // Generate JavaScript code
    let js_code = Codegen::new().build(&program).code;

    // Extract the expression from the generated code
    // The output can be: "const _expr_ = (...);\n" or "const _expr_ = ...;\n"
    // (codegen may remove unnecessary parentheses)
    let prefix = "const _expr_ = ";
    if let Some(start) = js_code.find(prefix) {
        let expr_start = start + prefix.len();
        // Find the semicolon at the end
        if let Some(end) = js_code[expr_start..].find(';') {
            let expr = &js_code[expr_start..expr_start + end];
            // Remove surrounding parentheses if present
            let expr = expr.trim();
            if expr.starts_with('(') && expr.ends_with(')') {
                return expr[1..expr.len() - 1].to_string();
            }
            return expr.to_string();
        }
    }

    // Fallback: return original content
    content.to_string()
}

/// Prefix identifiers in expression with _ctx. for codegen
/// This is a simpler version that doesn't require TransformContext
pub fn prefix_identifiers_in_expression(content: &str) -> std::string::String {
    let allocator = OxcAllocator::default();
    let source_type = SourceType::default().with_module(true);

    // Wrap in parentheses to make it a valid expression statement
    let wrapped = format!("({})", content);
    let parser = Parser::new(&allocator, &wrapped, source_type);
    let parse_result = parser.parse_expression();

    match parse_result {
        Ok(expr) => {
            // Collect identifiers and their positions
            let mut rewrites: Vec<(usize, usize, std::string::String)> = Vec::new();
            let mut local_vars: FxHashSet<std::string::String> = FxHashSet::default();

            collect_identifiers_for_prefix(&expr, &mut rewrites, &mut local_vars, content);

            if rewrites.is_empty() {
                return content.to_string();
            }

            // Sort by position (descending) to apply replacements from end to start
            rewrites.sort_by(|a, b| b.0.cmp(&a.0));

            let mut result = content.to_string();
            for (start, end, replacement) in rewrites {
                if start < result.len() && end <= result.len() {
                    result.replace_range(start..end, &replacement);
                }
            }

            result
        }
        Err(_) => content.to_string(),
    }
}

/// Collect identifiers that need _ctx. prefix
fn collect_identifiers_for_prefix(
    expr: &oxc_ast::ast::Expression<'_>,
    rewrites: &mut Vec<(usize, usize, std::string::String)>,
    local_vars: &mut FxHashSet<std::string::String>,
    _original: &str,
) {
    use oxc_ast::ast::Expression;

    match expr {
        Expression::Identifier(id) => {
            let name = id.name.as_str();
            // Skip JS globals and local variables
            if !JS_GLOBALS.contains(name) && !local_vars.contains(name) {
                // Adjust position: subtract 1 for the opening parenthesis we added
                let start = id.span.start as usize - 1;
                let end = id.span.end as usize - 1;
                rewrites.push((start, end, ["_ctx.", name].concat()));
            }
        }
        Expression::ArrowFunctionExpression(arrow) => {
            // Add arrow function params to local scope
            for param in &arrow.params.items {
                collect_binding_names(&param.pattern, local_vars);
            }
            // Process body statements
            for stmt in arrow.body.statements.iter() {
                if let oxc_ast::ast::Statement::ExpressionStatement(expr_stmt) = stmt {
                    collect_identifiers_for_prefix(
                        &expr_stmt.expression,
                        rewrites,
                        local_vars,
                        _original,
                    );
                }
            }
        }
        Expression::CallExpression(call) => {
            collect_identifiers_for_prefix(&call.callee, rewrites, local_vars, _original);
            for arg in &call.arguments {
                if let oxc_ast::ast::Argument::SpreadElement(spread) = arg {
                    collect_identifiers_for_prefix(
                        &spread.argument,
                        rewrites,
                        local_vars,
                        _original,
                    );
                } else if let Some(expr) = arg.as_expression() {
                    collect_identifiers_for_prefix(expr, rewrites, local_vars, _original);
                }
            }
        }
        Expression::ComputedMemberExpression(computed) => {
            collect_identifiers_for_prefix(&computed.object, rewrites, local_vars, _original);
            collect_identifiers_for_prefix(&computed.expression, rewrites, local_vars, _original);
        }
        Expression::StaticMemberExpression(static_member) => {
            collect_identifiers_for_prefix(&static_member.object, rewrites, local_vars, _original);
            // Don't prefix the property name
        }
        Expression::PrivateFieldExpression(private) => {
            collect_identifiers_for_prefix(&private.object, rewrites, local_vars, _original);
        }
        Expression::ParenthesizedExpression(paren) => {
            collect_identifiers_for_prefix(&paren.expression, rewrites, local_vars, _original);
        }
        Expression::BinaryExpression(binary) => {
            collect_identifiers_for_prefix(&binary.left, rewrites, local_vars, _original);
            collect_identifiers_for_prefix(&binary.right, rewrites, local_vars, _original);
        }
        Expression::ConditionalExpression(cond) => {
            collect_identifiers_for_prefix(&cond.test, rewrites, local_vars, _original);
            collect_identifiers_for_prefix(&cond.consequent, rewrites, local_vars, _original);
            collect_identifiers_for_prefix(&cond.alternate, rewrites, local_vars, _original);
        }
        Expression::LogicalExpression(logical) => {
            collect_identifiers_for_prefix(&logical.left, rewrites, local_vars, _original);
            collect_identifiers_for_prefix(&logical.right, rewrites, local_vars, _original);
        }
        Expression::UnaryExpression(unary) => {
            collect_identifiers_for_prefix(&unary.argument, rewrites, local_vars, _original);
        }
        Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                match prop {
                    oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) => {
                        collect_identifiers_for_prefix(&p.value, rewrites, local_vars, _original);
                    }
                    oxc_ast::ast::ObjectPropertyKind::SpreadProperty(spread) => {
                        collect_identifiers_for_prefix(
                            &spread.argument,
                            rewrites,
                            local_vars,
                            _original,
                        );
                    }
                }
            }
        }
        Expression::ArrayExpression(arr) => {
            for elem in &arr.elements {
                match elem {
                    oxc_ast::ast::ArrayExpressionElement::SpreadElement(spread) => {
                        collect_identifiers_for_prefix(
                            &spread.argument,
                            rewrites,
                            local_vars,
                            _original,
                        );
                    }
                    oxc_ast::ast::ArrayExpressionElement::Elision(_) => {}
                    _ => {
                        if let Some(expr) = elem.as_expression() {
                            collect_identifiers_for_prefix(expr, rewrites, local_vars, _original);
                        }
                    }
                }
            }
        }
        Expression::TemplateLiteral(template) => {
            for expr in &template.expressions {
                collect_identifiers_for_prefix(expr, rewrites, local_vars, _original);
            }
        }
        _ => {}
    }
}

/// Collect binding names from a pattern
fn collect_binding_names(
    pattern: &oxc_ast::ast::BindingPattern<'_>,
    local_vars: &mut FxHashSet<std::string::String>,
) {
    match &pattern.kind {
        oxc_ast::ast::BindingPatternKind::BindingIdentifier(id) => {
            local_vars.insert(id.name.to_string());
        }
        oxc_ast::ast::BindingPatternKind::ObjectPattern(obj) => {
            for prop in &obj.properties {
                collect_binding_names(&prop.value, local_vars);
            }
        }
        oxc_ast::ast::BindingPatternKind::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                collect_binding_names(elem, local_vars);
            }
        }
        oxc_ast::ast::BindingPatternKind::AssignmentPattern(assign) => {
            collect_binding_names(&assign.left, local_vars);
        }
    }
}

/// Visitor to collect identifiers that need prefixing
struct IdentifierCollector<'a, 'ctx> {
    ctx: &'a TransformContext<'ctx>,
    /// Identifiers that are being declared (e.g., in arrow function params)
    local_scope: FxHashSet<String>,
    /// (position, prefix) pairs for rewrites
    rewrites: FxHashSet<(usize, &'static str)>,
    /// (position, suffix) pairs for suffix rewrites (e.g., .value for refs)
    suffix_rewrites: Vec<(usize, &'static str)>,
}

impl<'a, 'ctx> IdentifierCollector<'a, 'ctx> {
    fn new(ctx: &'a TransformContext<'ctx>) -> Self {
        Self {
            ctx,
            local_scope: FxHashSet::default(),
            rewrites: FxHashSet::default(),
            suffix_rewrites: Vec::new(),
        }
    }

    fn should_prefix(&self, name: &str) -> bool {
        // Skip if in local scope (function params, etc.)
        if self.local_scope.contains(name) {
            return false;
        }

        should_prefix_identifier(name, self.ctx)
    }

    /// Check if an identifier is a ref that needs .value suffix
    fn is_ref_binding(&self, name: &str) -> bool {
        // Skip if in local scope
        if self.local_scope.contains(name) {
            return false;
        }

        // Check if this is an inline mode ref binding
        if self.ctx.options.inline {
            if let Some(bindings) = &self.ctx.options.binding_metadata {
                if let Some(binding_type) = bindings.bindings.get(name) {
                    // SetupRef needs .value access
                    return matches!(binding_type, crate::options::BindingType::SetupRef);
                }
            }
        }
        false
    }
}

impl<'a, 'ctx> Visit<'_> for IdentifierCollector<'a, 'ctx> {
    fn visit_identifier_reference(&mut self, ident: &oxc_ast_types::IdentifierReference<'_>) {
        let name = ident.name.as_str();
        if self.should_prefix(name) {
            let prefix = get_identifier_prefix(self.ctx, name);
            self.rewrites.insert((ident.span.start as usize, prefix));
        } else if self.is_ref_binding(name) {
            // Add .value suffix for refs in inline mode
            self.suffix_rewrites
                .push((ident.span.end as usize, ".value"));
        }
    }

    fn visit_member_expression(&mut self, expr: &oxc_ast_types::MemberExpression<'_>) {
        // Visit the object part normally
        match expr {
            oxc_ast_types::MemberExpression::ComputedMemberExpression(computed) => {
                self.visit_expression(&computed.object);
                // For computed access [expr], visit the expression normally
                self.visit_expression(&computed.expression);
            }
            oxc_ast_types::MemberExpression::StaticMemberExpression(static_expr) => {
                self.visit_expression(&static_expr.object);
                // Don't visit the property - it's a static name, not a reference
            }
            oxc_ast_types::MemberExpression::PrivateFieldExpression(private) => {
                self.visit_expression(&private.object);
                // Private field name shouldn't be prefixed
            }
        }
    }

    fn visit_arrow_function_expression(
        &mut self,
        arrow: &oxc_ast_types::ArrowFunctionExpression<'_>,
    ) {
        // Add params to local scope
        for param in &arrow.params.items {
            self.collect_binding_pattern(&param.pattern);
        }

        // Visit body
        self.visit_function_body(&arrow.body);
    }
}

impl<'a, 'ctx> IdentifierCollector<'a, 'ctx> {
    fn collect_binding_pattern(&mut self, pattern: &oxc_ast_types::BindingPattern<'_>) {
        match &pattern.kind {
            oxc_ast_types::BindingPatternKind::BindingIdentifier(id) => {
                self.local_scope.insert(id.name.to_string().into());
            }
            oxc_ast_types::BindingPatternKind::ObjectPattern(obj) => {
                for prop in &obj.properties {
                    self.collect_binding_pattern(&prop.value);
                }
                if let Some(rest) = &obj.rest {
                    self.collect_binding_pattern(&rest.argument);
                }
            }
            oxc_ast_types::BindingPatternKind::ArrayPattern(arr) => {
                for elem in arr.elements.iter().flatten() {
                    self.collect_binding_pattern(elem);
                }
                if let Some(rest) = &arr.rest {
                    self.collect_binding_pattern(&rest.argument);
                }
            }
            oxc_ast_types::BindingPatternKind::AssignmentPattern(assign) => {
                self.collect_binding_pattern(&assign.left);
            }
        }
    }
}

/// Check if identifier should be prefixed
fn should_prefix_identifier(name: &str, ctx: &TransformContext<'_>) -> bool {
    // Don't prefix globals
    if JS_GLOBALS.contains(name) {
        return false;
    }

    // Don't prefix if in scope (local variable from v-for, v-slot, etc.)
    if ctx.is_in_scope(name) {
        return false;
    }

    // In inline mode, setup bindings don't need _ctx prefix
    // In non-inline mode (default), ALL bindings need _ctx prefix
    if ctx.options.inline {
        // Inline mode: setup bindings are accessed directly
        if let Some(bindings) = &ctx.options.binding_metadata {
            if bindings.bindings.contains_key(name) {
                // Known binding - don't prefix in inline mode
                return false;
            }
        }
    }

    // Default: prefix with _ctx.
    true
}

/// Get prefix for identifier based on binding metadata
fn get_identifier_prefix(_ctx: &TransformContext<'_>, _id: &str) -> &'static str {
    // In module mode with prefix_identifiers, use _ctx. prefix
    "_ctx."
}

/// Check if a simple identifier is a ref binding in inline mode
fn is_ref_binding_simple(name: &str, ctx: &TransformContext<'_>) -> bool {
    if ctx.options.inline {
        if let Some(bindings) = &ctx.options.binding_metadata {
            if let Some(binding_type) = bindings.bindings.get(name) {
                return matches!(binding_type, crate::options::BindingType::SetupRef);
            }
        }
    }
    false
}

/// Check if string is a simple identifier
pub fn is_simple_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    if !first.is_alphabetic() && first != '_' && first != '$' {
        return false;
    }

    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

/// Clone an expression node
fn clone_expression<'a>(exp: &ExpressionNode<'a>, allocator: &'a Bump) -> ExpressionNode<'a> {
    match exp {
        ExpressionNode::Simple(simple) => ExpressionNode::Simple(Box::new_in(
            SimpleExpressionNode {
                content: simple.content.clone(),
                is_static: simple.is_static,
                const_type: simple.const_type,
                loc: simple.loc.clone(),
                js_ast: None,
                hoisted: None,
                identifiers: None,
                is_handler_key: simple.is_handler_key,
            },
            allocator,
        )),
        ExpressionNode::Compound(compound) => {
            // TODO: proper compound expression cloning
            ExpressionNode::Compound(Box::new_in(
                CompoundExpressionNode {
                    children: bumpalo::collections::Vec::new_in(allocator),
                    loc: compound.loc.clone(),
                    identifiers: None,
                    is_handler_key: compound.is_handler_key,
                },
                allocator,
            ))
        }
    }
}

/// Process inline handler expression
pub fn process_inline_handler<'a>(
    ctx: &mut TransformContext<'a>,
    exp: &ExpressionNode<'a>,
) -> ExpressionNode<'a> {
    let allocator = ctx.allocator;

    match exp {
        ExpressionNode::Simple(simple) => {
            if simple.is_static {
                return clone_expression(exp, allocator);
            }

            let content = &simple.content;

            // Check if it's an inline function expression
            if content.contains("=>") || content.starts_with("function") {
                // Process identifiers in the handler
                if ctx.options.prefix_identifiers {
                    let rewritten = rewrite_expression(content, ctx, false);
                    return ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode {
                            content: String::new(&rewritten),
                            is_static: false,
                            const_type: ConstantType::NotConstant,
                            loc: simple.loc.clone(),
                            js_ast: None,
                            hoisted: None,
                            identifiers: None,
                            is_handler_key: true,
                        },
                        allocator,
                    ));
                } else if ctx.options.is_ts {
                    // Strip TypeScript type annotations even without prefix_identifiers
                    let stripped = strip_typescript_from_expression(content);
                    return ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode {
                            content: String::new(&stripped),
                            is_static: false,
                            const_type: ConstantType::NotConstant,
                            loc: simple.loc.clone(),
                            js_ast: None,
                            hoisted: None,
                            identifiers: None,
                            is_handler_key: true,
                        },
                        allocator,
                    ));
                }
                return clone_expression(exp, allocator);
            }

            // Check if it's a simple identifier (method name)
            // Vue passes method references directly, no wrapping needed
            if is_simple_identifier(content) {
                let new_content =
                    if ctx.options.prefix_identifiers && should_prefix_identifier(content, ctx) {
                        ["_ctx.", content].concat()
                    } else {
                        content.to_string()
                    };

                return ExpressionNode::Simple(Box::new_in(
                    SimpleExpressionNode {
                        content: String::new(&new_content),
                        is_static: false,
                        const_type: ConstantType::NotConstant,
                        loc: simple.loc.clone(),
                        js_ast: None,
                        hoisted: None,
                        identifiers: None,
                        is_handler_key: true,
                    },
                    allocator,
                ));
            }

            // Compound expression - rewrite and wrap in arrow function
            let rewritten = if ctx.options.prefix_identifiers {
                rewrite_expression(content, ctx, false)
            } else if ctx.options.is_ts {
                // Strip TypeScript type annotations even without prefix_identifiers
                strip_typescript_from_expression(content)
            } else {
                content.to_string()
            };
            let new_content = ["$event => (", &rewritten, ")"].concat();

            ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode {
                    content: String::new(&new_content),
                    is_static: false,
                    const_type: ConstantType::NotConstant,
                    loc: simple.loc.clone(),
                    js_ast: None,
                    hoisted: None,
                    identifiers: None,
                    is_handler_key: true,
                },
                allocator,
            ))
        }
        _ => clone_expression(exp, allocator),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_simple_identifier() {
        assert!(is_simple_identifier("foo"));
        assert!(is_simple_identifier("_bar"));
        assert!(is_simple_identifier("$baz"));
        assert!(is_simple_identifier("foo123"));
        assert!(!is_simple_identifier("123foo"));
        assert!(!is_simple_identifier("foo-bar"));
        assert!(!is_simple_identifier("foo.bar"));
        assert!(!is_simple_identifier(""));
    }

    #[test]
    fn test_js_globals() {
        assert!(JS_GLOBALS.contains("Array"));
        assert!(JS_GLOBALS.contains("Object"));
        assert!(JS_GLOBALS.contains("console"));
        assert!(JS_GLOBALS.contains("Math"));
        assert!(JS_GLOBALS.contains("$event"));
        assert!(!JS_GLOBALS.contains("myVar"));
    }

    #[test]
    fn test_strip_typescript_from_expression() {
        // Simple as assertion
        let result = strip_typescript_from_expression("$event.target as HTMLSelectElement");
        assert!(
            !result.contains(" as "),
            "Expected no 'as' keyword, got: {}",
            result
        );
        assert!(result.contains("$event.target"));

        // Chained as assertions
        let result =
            strip_typescript_from_expression("($event.target as HTMLInputElement).value as string");
        assert!(
            !result.contains(" as "),
            "Expected no 'as' keyword, got: {}",
            result
        );

        // No TypeScript - should return as-is
        let result = strip_typescript_from_expression("foo.bar.baz");
        assert_eq!(result.trim(), "foo.bar.baz");

        // Complex nested expression with multiple as assertions (from App.vue)
        let result = strip_typescript_from_expression(
            "handlePresetChange(($event.target as HTMLSelectElement).value as PresetKey)",
        );
        eprintln!("Complex expression result: {}", result);
        assert!(
            !result.contains(" as "),
            "Expected no 'as' keyword, got: {}",
            result
        );
        assert!(
            result.contains("handlePresetChange"),
            "Should contain function call"
        );
        assert!(
            result.contains("$event.target"),
            "Should contain event target"
        );
    }
}
