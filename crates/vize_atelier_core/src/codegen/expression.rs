//! Expression generation functions.

use crate::ast::*;
use crate::options::BindingType;
use vize_croquis::builtins::is_global_allowed;

use super::context::CodegenContext;

/// Prefix identifiers in expression with appropriate prefix based on binding metadata
/// This is a context-aware version that uses $setup. for setup bindings in function mode
fn prefix_identifiers_with_context(content: &str, ctx: &CodegenContext) -> String {
    use oxc_allocator::Allocator as OxcAllocator;
    use oxc_ast::visit::Visit;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use vize_carton::FxHashSet;

    let allocator = OxcAllocator::default();
    let source_type = SourceType::default().with_module(true);

    // Wrap in parentheses to make it a valid expression statement
    let wrapped = format!("({})", content);
    let parser = Parser::new(&allocator, &wrapped, source_type);
    let parse_result = parser.parse_expression();

    match parse_result {
        Ok(expr) => {
            // Collect identifiers and their positions
            let mut rewrites: Vec<(usize, usize, String)> = Vec::new();
            let mut local_vars: FxHashSet<String> = FxHashSet::default();

            // Visitor to collect identifiers
            struct IdentifierVisitor<'a, 'b> {
                rewrites: &'a mut Vec<(usize, usize, String)>,
                local_vars: &'a mut FxHashSet<String>,
                ctx: &'b CodegenContext,
                offset: u32,
            }

            impl<'a, 'b> Visit<'_> for IdentifierVisitor<'a, 'b> {
                fn visit_identifier_reference(
                    &mut self,
                    ident: &oxc_ast::ast::IdentifierReference<'_>,
                ) {
                    let name = ident.name.as_str();

                    // Skip if local variable
                    if self.local_vars.contains(name) {
                        return;
                    }

                    // Skip globals
                    if is_global_allowed(name) {
                        return;
                    }

                    // Skip slot params
                    if self.ctx.is_slot_param(name) {
                        return;
                    }

                    // Determine prefix based on binding metadata
                    let prefix = if let Some(ref metadata) = self.ctx.options.binding_metadata {
                        if let Some(binding_type) = metadata.bindings.get(name) {
                            match binding_type {
                                BindingType::Props | BindingType::PropsAliased => "$props.",
                                _ => {
                                    // In inline mode, no prefix
                                    // In function mode, use $setup.
                                    if self.ctx.options.inline {
                                        ""
                                    } else {
                                        "$setup."
                                    }
                                }
                            }
                        } else {
                            "_ctx."
                        }
                    } else {
                        "_ctx."
                    };

                    if !prefix.is_empty() {
                        let start = (ident.span.start - self.offset) as usize;
                        let end = (ident.span.end - self.offset) as usize;
                        self.rewrites
                            .push((start, end, format!("{}{}", prefix, name)));
                    }
                }

                fn visit_variable_declarator(
                    &mut self,
                    declarator: &oxc_ast::ast::VariableDeclarator<'_>,
                ) {
                    // Add local var names to skip list
                    if let oxc_ast::ast::BindingPatternKind::BindingIdentifier(ident) =
                        &declarator.id.kind
                    {
                        self.local_vars.insert(ident.name.to_string());
                    }
                    // Visit init expression
                    if let Some(init) = &declarator.init {
                        self.visit_expression(init);
                    }
                }

                fn visit_arrow_function_expression(
                    &mut self,
                    arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
                ) {
                    // Add arrow function params to local vars
                    for param in &arrow.params.items {
                        if let oxc_ast::ast::BindingPatternKind::BindingIdentifier(ident) =
                            &param.pattern.kind
                        {
                            self.local_vars.insert(ident.name.to_string());
                        }
                    }
                    // Visit body
                    self.visit_function_body(&arrow.body);
                }
            }

            let mut visitor = IdentifierVisitor {
                rewrites: &mut rewrites,
                local_vars: &mut local_vars,
                ctx,
                offset: 1, // Account for the '(' we added
            };
            visitor.visit_expression(&expr);

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

/// Generate a simple expression (like an identifier) with appropriate prefix
/// Used for ref attribute values that need $setup. prefix in function mode
#[allow(dead_code)]
pub fn generate_simple_expression_with_prefix(ctx: &CodegenContext, content: &str) -> String {
    prefix_identifiers_with_context(content, ctx)
}

/// Generate expression
pub fn generate_expression(ctx: &mut CodegenContext, expr: &ExpressionNode<'_>) {
    match expr {
        ExpressionNode::Simple(exp) => {
            generate_simple_expression(ctx, exp);
        }
        ExpressionNode::Compound(comp) => {
            for child in comp.children.iter() {
                match child {
                    CompoundExpressionChild::Simple(exp) => {
                        generate_simple_expression(ctx, exp);
                    }
                    CompoundExpressionChild::String(s) => {
                        ctx.push(s);
                    }
                    CompoundExpressionChild::Symbol(helper) => {
                        ctx.push(ctx.helper(*helper));
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Generate simple expression
pub fn generate_simple_expression(ctx: &mut CodegenContext, exp: &SimpleExpressionNode<'_>) {
    if exp.is_static {
        ctx.push("\"");
        ctx.push(&exp.content);
        ctx.push("\"");
    } else {
        // Strip TypeScript if needed
        if ctx.options.is_ts && exp.content.contains(" as ") {
            let stripped = crate::transforms::strip_typescript_from_expression(&exp.content);
            ctx.push(&stripped);
        } else {
            // Expression content should already be processed by transform phase
            // (e.g., "msg" -> "_ctx.msg" if prefix_identifiers is enabled)
            ctx.push(&exp.content);
        }
    }
}

/// Check if a string is a simple member expression like _ctx.foo or $setup.bar
/// This is used to determine if an event handler needs wrapping
pub fn is_simple_member_expression(s: &str) -> bool {
    // Check for pattern like _ctx.identifier or $setup.identifier
    if let Some(dot_pos) = s.find('.') {
        let prefix = &s[..dot_pos];
        let suffix = &s[dot_pos + 1..];
        // Prefix should be _ctx, $setup, or similar
        let valid_prefix = prefix == "_ctx" || prefix == "$setup" || prefix == "$props";
        // Suffix should be a simple identifier (no dots, no parens, etc.)
        let valid_suffix = !suffix.is_empty()
            && !suffix.contains('.')
            && !suffix.contains('(')
            && !suffix.contains('[');
        return valid_prefix && valid_suffix;
    }
    false
}

/// Check if an event handler expression is an inline handler
/// Inline handlers are expressions that are NOT simple identifiers or member expressions
/// Note: This is kept for potential future use (e.g., optimizations)
#[allow(dead_code)]
pub fn is_inline_handler(exp: &ExpressionNode<'_>) -> bool {
    match exp {
        ExpressionNode::Simple(simple) => {
            if simple.is_static {
                return false;
            }

            // Use the ORIGINAL source expression, not the transformed content
            // During transform phase, inline handlers like "count++" get wrapped as
            // "$event => (count.value++)" which would incorrectly be detected as "already arrow function"
            let content = simple.loc.source.as_str();

            // Already an arrow function or function expression - not inline
            if content.contains("=>") || content.trim().starts_with("function") {
                return false;
            }

            // Simple identifier or member expression - not inline (method reference)
            if crate::transforms::is_simple_identifier(content)
                || is_simple_member_expression(content)
            {
                return false;
            }

            // Everything else is an inline handler (needs caching)
            true
        }
        ExpressionNode::Compound(_) => {
            // Compound expressions are typically inline
            true
        }
    }
}

/// Generate event handler expression
/// Wraps inline expressions in arrow functions, strips TypeScript, and prefixes identifiers
/// When `for_caching` is true, simple identifiers are wrapped with safety check
pub fn generate_event_handler(
    ctx: &mut CodegenContext,
    exp: &ExpressionNode<'_>,
    for_caching: bool,
) {
    match exp {
        ExpressionNode::Simple(simple) => {
            if simple.is_static {
                ctx.push("\"");
                ctx.push(&simple.content);
                ctx.push("\"");
                return;
            }

            let content = &simple.content;

            // Step 1: Strip TypeScript if needed
            let ts_stripped = if ctx.options.is_ts && content.contains(" as ") {
                crate::transforms::strip_typescript_from_expression(content)
            } else {
                content.to_string()
            };

            // Step 2: Prefix identifiers if needed
            // Use context-aware prefixing to handle binding metadata and inline/function mode
            let processed = if ctx.options.prefix_identifiers {
                prefix_identifiers_with_context(&ts_stripped, ctx)
            } else {
                ts_stripped
            };

            // Check if it's already an arrow function or function expression
            if processed.contains("=>") || processed.trim().starts_with("function") {
                ctx.push(&processed);
                return;
            }

            // Check if it's a simple identifier or member expression (method name/reference)
            // _ctx.handler, handler, $setup.handler
            if crate::transforms::is_simple_identifier(&processed)
                || is_simple_member_expression(&processed)
            {
                if for_caching {
                    // When caching, wrap simple identifiers with safety check:
                    // (...args) => (_ctx.handler && _ctx.handler(...args))
                    ctx.push("(...args) => (");
                    ctx.push(&processed);
                    ctx.push(" && ");
                    ctx.push(&processed);
                    ctx.push("(...args))");
                } else {
                    // Not caching: use directly
                    ctx.push(&processed);
                }
                return;
            }

            // Compound expression (function call, etc.): wrap as $event => (expression)
            ctx.push("$event => (");
            ctx.push(&processed);
            ctx.push(")");
        }
        ExpressionNode::Compound(comp) => {
            // For compound expressions, generate normally
            for child in comp.children.iter() {
                match child {
                    CompoundExpressionChild::Simple(exp) => {
                        generate_simple_expression(ctx, exp);
                    }
                    CompoundExpressionChild::String(s) => {
                        ctx.push(s);
                    }
                    CompoundExpressionChild::Symbol(helper) => {
                        ctx.push(ctx.helper(*helper));
                    }
                    _ => {}
                }
            }
        }
    }
}
