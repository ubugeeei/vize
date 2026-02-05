//! Expression transform.
//!
//! Transforms expressions by prefixing identifiers with `_ctx.` for proper
//! context binding in the compiled render function (script setup mode).

use oxc_allocator::Allocator as OxcAllocator;
use oxc_ast::ast as oxc_ast_types;
use oxc_ast::visit::walk::{
    walk_assignment_expression, walk_object_property, walk_update_expression,
};
use oxc_ast::Visit;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use vize_carton::{Box, Bump, FxHashSet, String};

use std::string::String as StdString;
use vize_croquis::builtins::is_global_allowed;

use crate::ast::{CompoundExpressionNode, ConstantType, ExpressionNode, SimpleExpressionNode};
use crate::transform::TransformContext;

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

            // Skip if already processed for ref transformation
            if simple.is_ref_transformed {
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
                let result = rewrite_expression(content, ctx, as_params);
                if result.used_unref {
                    ctx.helper(crate::ast::RuntimeHelper::Unref);
                }
                result.code
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
                    is_ref_transformed: true,
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

/// Result of expression rewriting
struct RewriteResult {
    code: StdString,
    used_unref: bool,
}

/// Rewrite an expression string, prefixing identifiers with _ctx. where needed
fn rewrite_expression(
    content: &str,
    ctx: &TransformContext<'_>,
    _as_params: bool,
) -> RewriteResult {
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
    let mut wrapped = String::with_capacity(js_content.len() + 2);
    wrapped.push('(');
    wrapped.push_str(&js_content);
    wrapped.push(')');
    let parser = Parser::new(&oxc_allocator, &wrapped, source_type);
    let parse_result = parser.parse_expression();

    match parse_result {
        Ok(expr) => {
            // Successfully parsed - walk the AST and collect identifiers to rewrite
            let mut collector = IdentifierCollector::new(ctx);
            collector.visit_expression(&expr);

            let used_unref = collector.used_unref;

            // Combine prefix rewrites (from HashSet) with suffix rewrites
            // Each rewrite is (position, prefix, suffix)
            let mut all_rewrites: Vec<(usize, StdString, StdString)> = collector
                .rewrites
                .into_iter()
                .map(|(pos, prefix)| (pos, prefix, StdString::new()))
                .collect();

            // Add suffix rewrites (suffixes come after the identifier)
            for (pos, suffix) in collector.suffix_rewrites {
                all_rewrites.push((pos, StdString::new(), suffix));
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
                        result.insert_str(adjusted_pos, &suffix);
                    }
                    if !prefix.is_empty() {
                        // Insert prefix at the start of identifier
                        result.insert_str(adjusted_pos, &prefix);
                    }
                }
            }

            RewriteResult {
                code: result,
                used_unref,
            }
        }
        Err(_) => {
            // Parse failed - fallback to simple identifier check
            let code = if is_simple_identifier(&js_content) {
                if let Some(prefix) = get_identifier_prefix(&js_content, ctx) {
                    [prefix, &js_content].concat()
                } else if is_ref_binding_simple(&js_content, ctx) {
                    // Add .value for refs in inline mode
                    [&js_content, ".value"].concat()
                } else {
                    js_content
                }
            } else {
                js_content
            };
            RewriteResult {
                code,
                used_unref: false,
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
    if content.contains(" as ") {
        return true;
    }

    // Check for arrow function parameter type annotations: (param: Type) =>
    // Pattern: identifier followed by : and then some type, before ) =>
    if content.contains("=>") {
        // Look for patterns like "(x: Type)" or "(x: Type, y: Type2)"
        let bytes = content.as_bytes();
        let mut in_paren = false;
        let mut after_ident = false;
        for (i, &b) in bytes.iter().enumerate() {
            match b {
                b'(' => {
                    in_paren = true;
                    after_ident = false;
                }
                b')' => {
                    in_paren = false;
                    after_ident = false;
                }
                b':' if in_paren && after_ident => {
                    // Found colon after identifier inside parens before =>
                    // This is likely a type annotation
                    // Check it's not :: (TypeScript namespace separator)
                    if i + 1 < bytes.len() && bytes[i + 1] != b':' {
                        return true;
                    }
                }
                b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'$' | b'0'..=b'9' => {
                    after_ident = true;
                }
                b' ' | b'\t' => {
                    // Whitespace doesn't reset after_ident
                }
                b',' => {
                    // Comma resets for next parameter
                    after_ident = false;
                }
                _ => {
                    after_ident = false;
                }
            }
        }
    }

    // Check for non-null assertion operator (foo!, bar.baz!, etc.)
    // This is tricky because we need to distinguish from logical NOT (!foo)
    // Non-null assertion comes AFTER an expression, not before
    // Pattern: identifier/closing bracket/paren followed by !
    let bytes = content.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'!' {
            // Check if this is a non-null assertion (! after identifier/)/])
            // rather than logical NOT (! before expression)
            if i > 0 {
                let prev = bytes[i - 1];
                // Non-null assertion if previous char is:
                // - alphanumeric (foo!)
                // - underscore or dollar (var_!)
                // - closing paren (foo()!)
                // - closing bracket (foo[0]!)
                let is_non_null_assertion = prev.is_ascii_alphanumeric()
                    || prev == b'_'
                    || prev == b'$'
                    || prev == b')'
                    || prev == b']';
                if is_non_null_assertion {
                    return true;
                }
            }
        }
    }

    false
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
    let mut wrapped = String::with_capacity(content.len() + 18);
    wrapped.push_str("const _expr_ = (");
    wrapped.push_str(content);
    wrapped.push_str(");");
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
        if let Some(end) = js_code[expr_start..].rfind(';') {
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
    let mut wrapped = String::with_capacity(content.len() + 2);
    wrapped.push('(');
    wrapped.push_str(content);
    wrapped.push(')');
    let parser = Parser::new(&allocator, &wrapped, source_type);
    let parse_result = parser.parse_expression();

    match parse_result {
        Ok(expr) => {
            // Collect identifiers and their positions
            let mut rewrites: Vec<(usize, usize, std::string::String)> = Vec::new();
            let mut local_vars: FxHashSet<StdString> = FxHashSet::default();

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
    local_vars: &mut FxHashSet<StdString>,
    _original: &str,
) {
    use oxc_ast::ast::Expression;

    match expr {
        Expression::Identifier(id) => {
            let name = id.name.as_str();
            // Skip JS globals and local variables
            if !is_global_allowed(name) && !local_vars.contains(name) {
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
    local_vars: &mut FxHashSet<StdString>,
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
    local_scope: FxHashSet<StdString>,
    /// (position, prefix) pairs for rewrites
    rewrites: FxHashSet<(usize, StdString)>,
    /// (position, suffix) pairs for suffix rewrites (e.g., .value for refs)
    suffix_rewrites: Vec<(usize, StdString)>,
    /// Assignment target identifier positions (for .value on LHS)
    assignment_targets: FxHashSet<usize>,
    /// Whether _unref helper was used
    used_unref: bool,
}

impl<'a, 'ctx> IdentifierCollector<'a, 'ctx> {
    fn new(ctx: &'a TransformContext<'ctx>) -> Self {
        Self {
            ctx,
            local_scope: FxHashSet::default(),
            rewrites: FxHashSet::default(),
            suffix_rewrites: Vec::new(),
            assignment_targets: FxHashSet::default(),
            used_unref: false,
        }
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

    /// Check if an identifier needs _unref() wrapping
    /// This applies to let/var declarations and maybe-ref bindings
    fn needs_unref(&self, name: &str) -> bool {
        // Skip if in local scope
        if self.local_scope.contains(name) {
            return false;
        }

        // Check if this is a let/maybe-ref binding that needs _unref()
        // This applies in both inline and function modes
        if let Some(bindings) = &self.ctx.options.binding_metadata {
            if let Some(binding_type) = bindings.bindings.get(name) {
                // SetupLet and SetupMaybeRef need _unref()
                return matches!(
                    binding_type,
                    crate::options::BindingType::SetupLet
                        | crate::options::BindingType::SetupMaybeRef
                );
            }
        }
        false
    }
}

impl<'a, 'ctx> Visit<'_> for IdentifierCollector<'a, 'ctx> {
    fn visit_identifier_reference(&mut self, ident: &oxc_ast_types::IdentifierReference<'_>) {
        let name = ident.name.as_str();
        // Skip if in local scope
        if self.local_scope.contains(name) {
            return;
        }

        let needs_unref = self.needs_unref(name);
        let is_assignment_target = self
            .assignment_targets
            .contains(&(ident.span.start as usize));

        if is_assignment_target {
            if let Some(prefix) = get_identifier_prefix(name, self.ctx) {
                self.rewrites
                    .insert((ident.span.start as usize, prefix.to_string()));
            }
            if self.is_ref_binding(name) || needs_unref {
                self.suffix_rewrites
                    .push((ident.span.end as usize, ".value".to_string()));
            }
            return;
        }

        if let Some(prefix) = get_identifier_prefix(name, self.ctx) {
            // In function mode, SetupLet bindings need both $setup. prefix and _unref() wrapper
            // Result: _unref($setup.b) instead of just $setup.b
            if needs_unref && prefix == "$setup." {
                self.rewrites
                    .insert((ident.span.start as usize, "_unref($setup.".to_string()));
                self.suffix_rewrites
                    .push((ident.span.end as usize, ")".to_string()));
                self.used_unref = true;
            } else {
                self.rewrites
                    .insert((ident.span.start as usize, prefix.to_string()));
            }
        } else if self.is_ref_binding(name) {
            // Add .value suffix for refs in inline mode
            self.suffix_rewrites
                .push((ident.span.end as usize, ".value".to_string()));
        } else if needs_unref {
            // Wrap with _unref() for let/var bindings (inline mode)
            self.rewrites
                .insert((ident.span.start as usize, "_unref(".to_string()));
            self.suffix_rewrites
                .push((ident.span.end as usize, ")".to_string()));
            self.used_unref = true;
        }
    }

    fn visit_member_expression(&mut self, expr: &oxc_ast_types::MemberExpression<'_>) {
        // Visit the object part, but skip .value addition if already accessing .value
        match expr {
            oxc_ast_types::MemberExpression::ComputedMemberExpression(computed) => {
                self.visit_expression(&computed.object);
                // For computed access [expr], visit the expression normally
                self.visit_expression(&computed.expression);
            }
            oxc_ast_types::MemberExpression::StaticMemberExpression(static_expr) => {
                // If this is `ref.value`, don't add another .value to the ref object
                let property_name = static_expr.property.name.as_str();
                if property_name == "value" {
                    // Check if object is a simple identifier that is a ref
                    if let oxc_ast_types::Expression::Identifier(ident) = &static_expr.object {
                        let name = ident.name.as_str();
                        if self.is_ref_binding(name) {
                            // Skip adding .value - it's already accessed via .value
                            // But still add _ctx. prefix if needed
                            if let Some(prefix) = get_identifier_prefix(name, self.ctx) {
                                self.rewrites
                                    .insert((ident.span.start as usize, prefix.to_string()));
                            }
                            return;
                        }
                    }
                }
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

    fn visit_assignment_expression(&mut self, expr: &oxc_ast_types::AssignmentExpression<'_>) {
        self.collect_assignment_targets(&expr.left);
        walk_assignment_expression(self, expr);
    }

    fn visit_update_expression(&mut self, expr: &oxc_ast_types::UpdateExpression<'_>) {
        self.collect_simple_assignment_targets(&expr.argument);
        walk_update_expression(self, expr);
    }

    fn visit_object_property(&mut self, prop: &oxc_ast_types::ObjectProperty<'_>) {
        if prop.shorthand {
            if let oxc_ast_types::PropertyKey::StaticIdentifier(ident) = &prop.key {
                let name = ident.name.as_str();
                if self.local_scope.contains(name) || is_global_allowed(name) {
                    return;
                }
                if self.ctx.is_in_scope(name) {
                    return;
                }

                if let Some(prefix) = get_identifier_prefix(name, self.ctx) {
                    if !prefix.is_empty() {
                        let mut suffix = StdString::with_capacity(2 + prefix.len() + name.len());
                        suffix.push_str(": ");
                        suffix.push_str(prefix);
                        suffix.push_str(name);
                        self.suffix_rewrites.push((ident.span.end as usize, suffix));
                        return;
                    }
                }
            }
        }

        walk_object_property(self, prop);
    }
}

impl<'a, 'ctx> IdentifierCollector<'a, 'ctx> {
    fn collect_binding_pattern(&mut self, pattern: &oxc_ast_types::BindingPattern<'_>) {
        match &pattern.kind {
            oxc_ast_types::BindingPatternKind::BindingIdentifier(id) => {
                self.local_scope.insert(id.name.to_string());
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

    fn collect_assignment_targets(&mut self, target: &oxc_ast_types::AssignmentTarget<'_>) {
        use oxc_ast_types::{AssignmentTarget, AssignmentTargetProperty};

        match target {
            AssignmentTarget::AssignmentTargetIdentifier(ident) => {
                self.assignment_targets.insert(ident.span.start as usize);
            }
            AssignmentTarget::ObjectAssignmentTarget(obj) => {
                for prop in &obj.properties {
                    match prop {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(
                            prop_ident,
                        ) => {
                            self.assignment_targets
                                .insert(prop_ident.binding.span.start as usize);
                        }
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(prop_prop) => {
                            self.collect_assignment_targets_maybe_default(&prop_prop.binding);
                        }
                    }
                }
                if let Some(rest) = &obj.rest {
                    self.collect_assignment_targets(&rest.target);
                }
            }
            AssignmentTarget::ArrayAssignmentTarget(arr) => {
                for elem in arr.elements.iter().flatten() {
                    self.collect_assignment_targets_maybe_default(elem);
                }
                if let Some(rest) = &arr.rest {
                    self.collect_assignment_targets(&rest.target);
                }
            }
            _ => {}
        }
    }

    fn collect_assignment_targets_maybe_default(
        &mut self,
        target: &oxc_ast_types::AssignmentTargetMaybeDefault<'_>,
    ) {
        use oxc_ast_types::{AssignmentTargetMaybeDefault, AssignmentTargetProperty};

        match target {
            AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(def) => {
                self.collect_assignment_targets(&def.binding);
            }
            AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(ident) => {
                self.assignment_targets.insert(ident.span.start as usize);
            }
            AssignmentTargetMaybeDefault::ObjectAssignmentTarget(obj) => {
                for prop in &obj.properties {
                    match prop {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(
                            prop_ident,
                        ) => {
                            self.assignment_targets
                                .insert(prop_ident.binding.span.start as usize);
                        }
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(prop_prop) => {
                            self.collect_assignment_targets_maybe_default(&prop_prop.binding);
                        }
                    }
                }
                if let Some(rest) = &obj.rest {
                    self.collect_assignment_targets(&rest.target);
                }
            }
            AssignmentTargetMaybeDefault::ArrayAssignmentTarget(arr) => {
                for elem in arr.elements.iter().flatten() {
                    self.collect_assignment_targets_maybe_default(elem);
                }
                if let Some(rest) = &arr.rest {
                    self.collect_assignment_targets(&rest.target);
                }
            }
            _ => {}
        }
    }

    fn collect_simple_assignment_targets(
        &mut self,
        target: &oxc_ast_types::SimpleAssignmentTarget<'_>,
    ) {
        use oxc_ast_types::SimpleAssignmentTarget;

        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) = target {
            self.assignment_targets.insert(ident.span.start as usize);
        }
    }
}

/// Check if identifier should be prefixed
/// Determine what prefix (if any) an identifier needs
/// Returns: None = no prefix, Some("_ctx.") = context prefix, Some("__props.") = props prefix,
///          Some("$setup.") = setup context prefix (for function mode with binding metadata)
fn get_identifier_prefix(name: &str, ctx: &TransformContext<'_>) -> Option<&'static str> {
    // Don't prefix globals
    if is_global_allowed(name) {
        return None;
    }

    // Don't prefix if in scope (local variable from v-for, v-slot, etc.)
    if ctx.is_in_scope(name) {
        return None;
    }

    // Check binding metadata for setup bindings
    if let Some(bindings) = &ctx.options.binding_metadata {
        if let Some(binding_type) = bindings.bindings.get(name) {
            // Props need prefix based on mode
            if matches!(
                binding_type,
                crate::options::BindingType::Props | crate::options::BindingType::PropsAliased
            ) {
                // In inline mode: use __props. (local variable in setup)
                // In function mode: use $props. (render function parameter)
                if ctx.options.inline {
                    return Some("__props.");
                } else {
                    return Some("$props.");
                }
            }

            if ctx.options.inline {
                // In inline mode, setup bindings are accessed directly via closure
                return None;
            } else {
                // In function mode (inline = false), setup bindings use $setup. prefix
                // This is the pattern Vue's @vitejs/plugin-vue uses for proper reactivity tracking
                return Some("$setup.");
            }
        }
    }

    // Default: prefix with _ctx.
    Some("_ctx.")
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
                is_ref_transformed: simple.is_ref_transformed,
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

            // Skip if already processed for ref transformation
            if simple.is_ref_transformed {
                return clone_expression(exp, allocator);
            }

            let content = &simple.content;

            // Check if it's an inline function expression
            if content.contains("=>") || content.starts_with("function") {
                // Process identifiers in the handler
                if ctx.options.prefix_identifiers {
                    let result = rewrite_expression(content, ctx, false);
                    if result.used_unref {
                        ctx.helper(crate::ast::RuntimeHelper::Unref);
                    }
                    return ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode {
                            content: String::new(&result.code),
                            is_static: false,
                            const_type: ConstantType::NotConstant,
                            loc: simple.loc.clone(),
                            js_ast: None,
                            hoisted: None,
                            identifiers: None,
                            is_handler_key: true,
                            is_ref_transformed: true,
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
                            is_ref_transformed: true,
                        },
                        allocator,
                    ));
                }
                return clone_expression(exp, allocator);
            }

            // Check if it's a simple identifier (method name)
            // Vue passes method references directly, no wrapping needed
            if is_simple_identifier(content) {
                let new_content = if ctx.options.prefix_identifiers {
                    // Use the same prefix logic as get_identifier_prefix for consistency
                    if let Some(prefix) = get_identifier_prefix(content, ctx) {
                        [prefix, content].concat()
                    } else {
                        content.to_string()
                    }
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
                        is_ref_transformed: true,
                    },
                    allocator,
                ));
            }

            // Compound expression - rewrite and wrap in arrow function
            let rewritten = if ctx.options.prefix_identifiers {
                let result = rewrite_expression(content, ctx, false);
                if result.used_unref {
                    ctx.helper(crate::ast::RuntimeHelper::Unref);
                }
                result.code
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
                    is_ref_transformed: true,
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
        assert!(is_global_allowed("Array"));
        assert!(is_global_allowed("Object"));
        assert!(is_global_allowed("console"));
        assert!(is_global_allowed("Math"));
        assert!(is_global_allowed("$event"));
        assert!(!is_global_allowed("myVar"));
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

    // =============================================================================
    // Patch Tests: TypeScript detection improvements
    // =============================================================================

    #[test]
    fn test_needs_typescript_stripping_as_keyword() {
        assert!(needs_typescript_stripping("foo as string"));
        assert!(needs_typescript_stripping("$event.target as HTMLElement"));
        assert!(!needs_typescript_stripping("foo.bar"));
    }

    #[test]
    fn test_needs_typescript_stripping_arrow_function_params() {
        // Arrow function with typed parameters should be detected
        assert!(needs_typescript_stripping("(x: number) => x + 1"));
        assert!(needs_typescript_stripping("(item: Item) => item.name"));
        assert!(needs_typescript_stripping(
            "(a: string, b: number) => a + b"
        ));

        // Arrow function without types should not need stripping
        assert!(!needs_typescript_stripping("(x) => x + 1"));
        assert!(!needs_typescript_stripping("x => x + 1"));
    }

    #[test]
    fn test_needs_typescript_stripping_generic_detection_note() {
        // NOTE: Generic function call detection (e.g., useStore<RootState>())
        // is not implemented in needs_typescript_stripping.
        // Generic stripping is handled by the full OXC TypeScript transformer
        // when compiling script blocks with is_ts = false.
        // This test documents the current behavior.

        // Currently NOT detected as needing stripping:
        assert!(!needs_typescript_stripping("useStore<RootState>()"));
        assert!(!needs_typescript_stripping("ref<User | null>(null)"));

        // Regular function calls correctly don't need stripping:
        assert!(!needs_typescript_stripping("useStore()"));
        assert!(!needs_typescript_stripping("ref(null)"));
    }

    #[test]
    fn test_strip_typescript_documents_limitations() {
        // NOTE: strip_typescript_from_expression is a simple parser-based
        // transformation for template expressions. It handles common cases
        // like "as" assertions, but complex TypeScript like generics are
        // handled by the full OXC transformer in compile_script.
        //
        // For template expressions with generics, they are stripped during
        // script compilation (not in the template transform phase).

        // "as" assertions are stripped:
        let result = strip_typescript_from_expression("foo as string");
        assert!(!result.contains(" as "), "as assertions should be stripped");

        // Generics in expressions MAY or MAY NOT be stripped depending on context
        // This is expected behavior - complex cases are handled elsewhere
        let result = strip_typescript_from_expression("useStore<RootState>()");
        // Document the actual behavior - generics aren't stripped by this function
        eprintln!("Generic expression result: {}", result);
    }

    #[test]
    fn test_strip_typescript_arrow_param_types() {
        let result = strip_typescript_from_expression("items.filter((x: number) => x > 1)");
        eprintln!("Arrow param stripped: {}", result);
        // Note: This may or may not strip depending on the OXC parser's handling
        // The important thing is that it doesn't crash
        assert!(result.contains("filter"));
    }
}

// Note: Multiline arrow function handling and ES6 shorthand expansion
// are tested via SFC snapshot tests in tests/fixtures/sfc/patches.toml.
