//! Helper functions for Vue template analysis.
//!
//! Provides utilities for:
//! - Component and directive detection
//! - Identifier extraction from expressions
//! - v-for/v-slot expression parsing
//! - Inline callback parameter extraction

use oxc_allocator::Allocator;
use oxc_ast::ast::BindingPattern;
use oxc_parser::Parser;
use oxc_span::SourceType;
use vize_carton::{smallvec, CompactString, SmallVec};

/// Check if a tag is a component (PascalCase or contains hyphen)
#[inline]
pub fn is_component_tag(tag: &str) -> bool {
    tag.contains('-') || tag.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

/// Check if a directive is built-in
#[inline]
pub fn is_builtin_directive(name: &str) -> bool {
    matches!(
        name,
        "if" | "else"
            | "else-if"
            | "for"
            | "show"
            | "bind"
            | "on"
            | "model"
            | "slot"
            | "text"
            | "html"
            | "cloak"
            | "once"
            | "pre"
            | "memo"
    )
}

/// Check if a string is a JS keyword
#[inline]
pub fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "true"
            | "false"
            | "null"
            | "undefined"
            | "this"
            | "arguments"
            | "if"
            | "else"
            | "for"
            | "while"
            | "do"
            | "switch"
            | "case"
            | "break"
            | "continue"
            | "return"
            | "throw"
            | "try"
            | "catch"
            | "finally"
            | "new"
            | "delete"
            | "typeof"
            | "void"
            | "in"
            | "of"
            | "instanceof"
            | "function"
            | "class"
            | "const"
            | "let"
            | "var"
            | "async"
            | "await"
            | "yield"
            | "import"
            | "export"
            | "default"
            | "from"
            | "as"
    )
}

/// Hybrid identifier extraction - fast path for simple expressions, OXC for complex ones.
/// Only extracts "root" identifiers - identifiers that are references, not:
/// - Property accesses (item.name -> only "item" extracted)
/// - Object literal keys ({ active: value } -> only "value" extracted)
/// - String literals, computed property names, etc.
#[inline]
pub fn extract_identifiers_oxc(expr: &str) -> Vec<CompactString> {
    // Use OXC parser for complex expressions:
    // - Object literals: { }
    // - Type assertions: as Type
    // - Arrow functions: () =>
    if expr.contains('{') || expr.contains(" as ") || expr.contains("=>") {
        return extract_identifiers_oxc_slow(expr);
    }

    // Fast path: simple expressions without complex constructs
    extract_identifiers_fast(expr)
}

/// Fast string-based identifier extraction for simple expressions.
#[inline]
fn extract_identifiers_fast(expr: &str) -> Vec<CompactString> {
    let mut identifiers = Vec::with_capacity(4);
    let bytes = expr.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let c = bytes[i];

        // Skip single-quoted strings
        if c == b'\'' {
            i += 1;
            while i < len && bytes[i] != b'\'' {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        // Skip double-quoted strings
        if c == b'"' {
            i += 1;
            while i < len && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        // Handle template literals
        if c == b'`' {
            i += 1;
            while i < len {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'`' {
                    i += 1;
                    break;
                }
                if bytes[i] == b'$' && i + 1 < len && bytes[i + 1] == b'{' {
                    i += 2;
                    let interp_start = i;
                    let mut brace_depth = 1;
                    while i < len && brace_depth > 0 {
                        match bytes[i] {
                            b'{' => brace_depth += 1,
                            b'}' => brace_depth -= 1,
                            _ => {}
                        }
                        if brace_depth > 0 {
                            i += 1;
                        }
                    }
                    if interp_start < i {
                        let interp_content = &expr[interp_start..i];
                        for ident in extract_identifiers_fast(interp_content) {
                            identifiers.push(ident);
                        }
                    }
                    if i < len {
                        i += 1;
                    }
                    continue;
                }
                i += 1;
            }
            continue;
        }

        // Start of identifier
        if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
            let start = i;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
            {
                i += 1;
            }

            // Check if preceded by '.' (property access)
            let is_property_access = if start > 0 {
                let mut j = start - 1;
                loop {
                    let prev = bytes[j];
                    if prev == b' ' || prev == b'\t' || prev == b'\n' || prev == b'\r' {
                        if j == 0 {
                            break false;
                        }
                        j -= 1;
                    } else {
                        break prev == b'.';
                    }
                }
            } else {
                false
            };

            if !is_property_access {
                identifiers.push(CompactString::new(&expr[start..i]));
            }
        } else {
            i += 1;
        }
    }

    identifiers
}

/// OXC-based identifier extraction for expressions with object literals.
#[inline]
fn extract_identifiers_oxc_slow(expr: &str) -> Vec<CompactString> {
    use oxc_ast::ast::{
        ArrayExpressionElement, BindingPattern, Expression, ObjectPropertyKind, PropertyKey,
    };

    let allocator = Allocator::default();
    let source_type = SourceType::from_path("expr.ts").unwrap_or_default();

    let ret = Parser::new(&allocator, expr, source_type).parse_expression();
    let parsed_expr = match ret {
        Ok(expr) => expr,
        Err(_) => return Vec::new(),
    };

    let mut identifiers = Vec::with_capacity(4);

    // Collect binding names from a pattern (for arrow function parameters)
    fn collect_binding_names<'a>(pattern: &'a BindingPattern<'a>, names: &mut Vec<&'a str>) {
        match pattern {
            BindingPattern::BindingIdentifier(id) => {
                names.push(id.name.as_str());
            }
            BindingPattern::ObjectPattern(obj) => {
                for prop in obj.properties.iter() {
                    collect_binding_names(&prop.value, names);
                }
                if let Some(rest) = &obj.rest {
                    collect_binding_names(&rest.argument, names);
                }
            }
            BindingPattern::ArrayPattern(arr) => {
                for elem in arr.elements.iter().flatten() {
                    collect_binding_names(elem, names);
                }
                if let Some(rest) = &arr.rest {
                    collect_binding_names(&rest.argument, names);
                }
            }
            BindingPattern::AssignmentPattern(assign) => {
                collect_binding_names(&assign.left, names);
            }
        }
    }

    // Recursive AST walker to collect identifier references
    fn walk_expr(expr: &Expression<'_>, identifiers: &mut Vec<CompactString>) {
        match expr {
            // Direct identifier reference - this is what we want
            Expression::Identifier(id) => {
                identifiers.push(CompactString::new(id.name.as_str()));
            }

            // Member expressions - only extract the object, not the property
            Expression::StaticMemberExpression(member) => {
                walk_expr(&member.object, identifiers);
            }
            Expression::ComputedMemberExpression(member) => {
                walk_expr(&member.object, identifiers);
                walk_expr(&member.expression, identifiers);
            }
            Expression::PrivateFieldExpression(field) => {
                walk_expr(&field.object, identifiers);
            }

            // Object expressions - skip keys, only process values
            Expression::ObjectExpression(obj) => {
                for prop in obj.properties.iter() {
                    match prop {
                        ObjectPropertyKind::ObjectProperty(p) => {
                            if p.computed {
                                if let Some(key_expr) = p.key.as_expression() {
                                    walk_expr(key_expr, identifiers);
                                }
                            }
                            if p.shorthand {
                                if let PropertyKey::StaticIdentifier(id) = &p.key {
                                    identifiers.push(CompactString::new(id.name.as_str()));
                                }
                            } else {
                                walk_expr(&p.value, identifiers);
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            walk_expr(&spread.argument, identifiers);
                        }
                    }
                }
            }

            // Array expressions
            Expression::ArrayExpression(arr) => {
                for elem in arr.elements.iter() {
                    match elem {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            walk_expr(&spread.argument, identifiers);
                        }
                        ArrayExpressionElement::Elision(_) => {}
                        _ => {
                            if let Some(e) = elem.as_expression() {
                                walk_expr(e, identifiers);
                            }
                        }
                    }
                }
            }

            // Binary/Logical/Conditional expressions
            Expression::BinaryExpression(binary) => {
                walk_expr(&binary.left, identifiers);
                walk_expr(&binary.right, identifiers);
            }
            Expression::LogicalExpression(logical) => {
                walk_expr(&logical.left, identifiers);
                walk_expr(&logical.right, identifiers);
            }
            Expression::ConditionalExpression(cond) => {
                walk_expr(&cond.test, identifiers);
                walk_expr(&cond.consequent, identifiers);
                walk_expr(&cond.alternate, identifiers);
            }

            // Unary expressions
            Expression::UnaryExpression(unary) => {
                walk_expr(&unary.argument, identifiers);
            }
            Expression::UpdateExpression(update) => match &update.argument {
                oxc_ast::ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                    identifiers.push(CompactString::new(id.name.as_str()));
                }
                oxc_ast::ast::SimpleAssignmentTarget::StaticMemberExpression(member) => {
                    walk_expr(&member.object, identifiers);
                }
                oxc_ast::ast::SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                    walk_expr(&member.object, identifiers);
                    walk_expr(&member.expression, identifiers);
                }
                oxc_ast::ast::SimpleAssignmentTarget::PrivateFieldExpression(field) => {
                    walk_expr(&field.object, identifiers);
                }
                _ => {}
            },

            // Call expressions
            Expression::CallExpression(call) => {
                walk_expr(&call.callee, identifiers);
                for arg in call.arguments.iter() {
                    if let Some(e) = arg.as_expression() {
                        walk_expr(e, identifiers);
                    }
                }
            }
            Expression::NewExpression(new_expr) => {
                walk_expr(&new_expr.callee, identifiers);
                for arg in new_expr.arguments.iter() {
                    if let Some(e) = arg.as_expression() {
                        walk_expr(e, identifiers);
                    }
                }
            }

            // Arrow/Function expressions - parameters are local scope, don't extract them
            Expression::ArrowFunctionExpression(arrow) => {
                // Collect parameter names to exclude from identifiers
                let mut param_names: Vec<&str> = Vec::new();
                for param in arrow.params.items.iter() {
                    collect_binding_names(&param.pattern, &mut param_names);
                }

                if arrow.expression {
                    if let Some(oxc_ast::ast::Statement::ExpressionStatement(expr_stmt)) =
                        arrow.body.statements.first()
                    {
                        // Walk body but filter out parameter references
                        let mut body_idents = Vec::new();
                        walk_expr(&expr_stmt.expression, &mut body_idents);
                        for ident in body_idents {
                            if !param_names.contains(&ident.as_str()) {
                                identifiers.push(ident);
                            }
                        }
                    }
                }
            }

            // Sequence expressions
            Expression::SequenceExpression(seq) => {
                for e in seq.expressions.iter() {
                    walk_expr(e, identifiers);
                }
            }

            // Assignment expressions
            Expression::AssignmentExpression(assign) => {
                walk_expr(&assign.right, identifiers);
            }

            // Template literals
            Expression::TemplateLiteral(template) => {
                for expr in template.expressions.iter() {
                    walk_expr(expr, identifiers);
                }
            }
            Expression::TaggedTemplateExpression(tagged) => {
                walk_expr(&tagged.tag, identifiers);
                for expr in tagged.quasi.expressions.iter() {
                    walk_expr(expr, identifiers);
                }
            }

            // Parenthesized/Await/Yield
            Expression::ParenthesizedExpression(paren) => {
                walk_expr(&paren.expression, identifiers);
            }
            Expression::AwaitExpression(await_expr) => {
                walk_expr(&await_expr.argument, identifiers);
            }
            Expression::YieldExpression(yield_expr) => {
                if let Some(arg) = &yield_expr.argument {
                    walk_expr(arg, identifiers);
                }
            }

            // Chained expressions
            Expression::ChainExpression(chain) => match &chain.expression {
                oxc_ast::ast::ChainElement::CallExpression(call) => {
                    walk_expr(&call.callee, identifiers);
                    for arg in call.arguments.iter() {
                        if let Some(e) = arg.as_expression() {
                            walk_expr(e, identifiers);
                        }
                    }
                }
                oxc_ast::ast::ChainElement::TSNonNullExpression(non_null) => {
                    walk_expr(&non_null.expression, identifiers);
                }
                oxc_ast::ast::ChainElement::StaticMemberExpression(member) => {
                    walk_expr(&member.object, identifiers);
                }
                oxc_ast::ast::ChainElement::ComputedMemberExpression(member) => {
                    walk_expr(&member.object, identifiers);
                    walk_expr(&member.expression, identifiers);
                }
                oxc_ast::ast::ChainElement::PrivateFieldExpression(field) => {
                    walk_expr(&field.object, identifiers);
                }
            },

            // TypeScript specific
            Expression::TSAsExpression(as_expr) => {
                walk_expr(&as_expr.expression, identifiers);
            }
            Expression::TSSatisfiesExpression(satisfies) => {
                walk_expr(&satisfies.expression, identifiers);
            }
            Expression::TSNonNullExpression(non_null) => {
                walk_expr(&non_null.expression, identifiers);
            }
            Expression::TSTypeAssertion(assertion) => {
                walk_expr(&assertion.expression, identifiers);
            }
            Expression::TSInstantiationExpression(inst) => {
                walk_expr(&inst.expression, identifiers);
            }

            // Literals - no identifiers
            Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::RegExpLiteral(_) => {}

            _ => {}
        }
    }

    walk_expr(&parsed_expr, &mut identifiers);
    identifiers
}

/// Parse v-for expression into variables and source
#[inline]
pub fn parse_v_for_expression(expr: &str) -> (SmallVec<[CompactString; 3]>, CompactString) {
    let bytes = expr.as_bytes();
    let len = bytes.len();

    // Find " in " or " of " separator
    let mut split_pos = None;
    let mut i = 0;
    while i + 4 <= len {
        if bytes[i] == b' '
            && ((bytes[i + 1] == b'i' && bytes[i + 2] == b'n')
                || (bytes[i + 1] == b'o' && bytes[i + 2] == b'f'))
            && bytes[i + 3] == b' '
        {
            split_pos = Some(i);
            break;
        }
        i += 1;
    }

    let Some(pos) = split_pos else {
        return (smallvec![], CompactString::new(expr.trim()));
    };

    let alias_part = expr[..pos].trim();
    let source_part = expr[pos + 4..].trim();
    let source = CompactString::new(source_part);

    // Fast path: simple identifier
    if !alias_part.starts_with('(')
        && !alias_part.contains('{')
        && is_valid_identifier_fast(alias_part.as_bytes())
    {
        return (smallvec![CompactString::new(alias_part)], source);
    }

    // Fast path: simple tuple (item, index)
    if alias_part.starts_with('(') && alias_part.ends_with(')') && !alias_part.contains('{') {
        let inner = &alias_part[1..alias_part.len() - 1];
        let mut vars = SmallVec::new();
        for part in inner.split(',') {
            let part = part.trim();
            if !part.is_empty() && is_valid_identifier_fast(part.as_bytes()) {
                vars.push(CompactString::new(part));
            }
        }
        if !vars.is_empty() {
            return (vars, source);
        }
    }

    // Complex case: use OXC parser
    parse_v_for_with_oxc(alias_part, source)
}

/// Parse complex v-for alias using OXC
#[cold]
fn parse_v_for_with_oxc(
    alias: &str,
    source: CompactString,
) -> (SmallVec<[CompactString; 3]>, CompactString) {
    let mut buffer = [0u8; 256];
    let prefix = b"let [";
    let suffix = b"] = x";

    let inner = if alias.starts_with('(') && alias.ends_with(')') {
        &alias[1..alias.len() - 1]
    } else {
        alias
    };

    let total_len = prefix.len() + inner.len() + suffix.len();
    if total_len > buffer.len() {
        let pattern_str = format!("let [{}] = x", inner);
        return parse_v_for_pattern(&pattern_str, source);
    }

    buffer[..prefix.len()].copy_from_slice(prefix);
    buffer[prefix.len()..prefix.len() + inner.len()].copy_from_slice(inner.as_bytes());
    buffer[prefix.len() + inner.len()..total_len].copy_from_slice(suffix);

    // SAFETY: we only copy ASCII bytes
    let pattern_str = unsafe { std::str::from_utf8_unchecked(&buffer[..total_len]) };
    parse_v_for_pattern(pattern_str, source)
}

/// Parse v-for pattern using OXC
fn parse_v_for_pattern(
    pattern_str: &str,
    source: CompactString,
) -> (SmallVec<[CompactString; 3]>, CompactString) {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true);
    let ret = Parser::new(&allocator, pattern_str, source_type).parse();

    let mut vars = SmallVec::new();

    if let Some(oxc_ast::ast::Statement::VariableDeclaration(var_decl)) = ret.program.body.first() {
        if let Some(declarator) = var_decl.declarations.first() {
            extract_binding_names(&declarator.id, &mut vars);
        }
    }

    (vars, source)
}

/// Extract binding names from a binding pattern
fn extract_binding_names(
    pattern: &oxc_ast::ast::BindingPattern<'_>,
    names: &mut SmallVec<[CompactString; 3]>,
) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            names.push(CompactString::new(id.name.as_str()));
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in obj.properties.iter() {
                extract_binding_names(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                extract_binding_names(&rest.argument, names);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                extract_binding_names(elem, names);
            }
            if let Some(rest) = &arr.rest {
                extract_binding_names(&rest.argument, names);
            }
        }
        BindingPattern::AssignmentPattern(assign) => {
            extract_binding_names(&assign.left, names);
        }
    }
}

/// Extract prop names from v-slot expression pattern
#[inline]
pub fn extract_slot_props(pattern: &str) -> SmallVec<[CompactString; 4]> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return SmallVec::new();
    }

    let bytes = pattern.as_bytes();

    // Fast path: simple identifier
    if bytes[0] != b'{' && bytes[0] != b'[' {
        if is_valid_identifier_fast(bytes) {
            return smallvec![CompactString::new(pattern)];
        }
        return SmallVec::new();
    }

    // Fast path: simple object destructuring
    if bytes[0] == b'{' && !pattern.contains(':') && !pattern.contains('{') {
        let inner = &pattern[1..pattern.len().saturating_sub(1)];
        let mut props = SmallVec::new();
        for part in inner.split(',') {
            let part = part.trim();
            let name = if let Some(eq_pos) = part.find('=') {
                part[..eq_pos].trim()
            } else {
                part
            };
            if !name.is_empty() && is_valid_identifier_fast(name.as_bytes()) {
                props.push(CompactString::new(name));
            }
        }
        if !props.is_empty() {
            return props;
        }
    }

    // Complex case: use OXC parser
    extract_slot_props_with_oxc(pattern)
}

/// Parse complex slot props using OXC
#[cold]
fn extract_slot_props_with_oxc(pattern: &str) -> SmallVec<[CompactString; 4]> {
    let mut buffer = [0u8; 256];
    let prefix = b"let ";
    let suffix = b" = x";

    let total_len = prefix.len() + pattern.len() + suffix.len();
    if total_len > buffer.len() {
        let pattern_str = format!("let {} = x", pattern);
        return parse_slot_pattern(&pattern_str);
    }

    buffer[..prefix.len()].copy_from_slice(prefix);
    buffer[prefix.len()..prefix.len() + pattern.len()].copy_from_slice(pattern.as_bytes());
    buffer[prefix.len() + pattern.len()..total_len].copy_from_slice(suffix);

    // SAFETY: we only copy ASCII bytes
    let pattern_str = unsafe { std::str::from_utf8_unchecked(&buffer[..total_len]) };
    parse_slot_pattern(pattern_str)
}

/// Parse slot pattern using OXC
fn parse_slot_pattern(pattern_str: &str) -> SmallVec<[CompactString; 4]> {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true);
    let ret = Parser::new(&allocator, pattern_str, source_type).parse();

    let mut props = SmallVec::new();

    if let Some(oxc_ast::ast::Statement::VariableDeclaration(var_decl)) = ret.program.body.first() {
        if let Some(declarator) = var_decl.declarations.first() {
            extract_slot_binding_names(&declarator.id, &mut props);
        }
    }

    props
}

/// Extract binding names from slot pattern
fn extract_slot_binding_names(
    pattern: &oxc_ast::ast::BindingPattern<'_>,
    names: &mut SmallVec<[CompactString; 4]>,
) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            names.push(CompactString::new(id.name.as_str()));
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in obj.properties.iter() {
                extract_slot_binding_names(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                extract_slot_binding_names(&rest.argument, names);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                extract_slot_binding_names(elem, names);
            }
            if let Some(rest) = &arr.rest {
                extract_slot_binding_names(&rest.argument, names);
            }
        }
        BindingPattern::AssignmentPattern(assign) => {
            extract_slot_binding_names(&assign.left, names);
        }
    }
}

/// Extract parameters from inline arrow function or function expression
#[inline]
pub fn extract_inline_callback_params(
    expr: &str,
) -> Option<vize_carton::SmallVec<[CompactString; 4]>> {
    let bytes = expr.as_bytes();
    let len = bytes.len();
    if len == 0 {
        return None;
    }

    // Skip leading whitespace
    let mut i = 0;
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= len {
        return None;
    }

    // Fast path: check for arrow "=>"
    let arrow_pos = find_arrow(bytes, i);

    if let Some(arrow_idx) = arrow_pos {
        let mut end = arrow_idx;
        while end > i && bytes[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        if end <= i {
            return None;
        }

        let before_bytes = &bytes[i..end];

        // Check for async prefix
        let (param_start, param_end) = if before_bytes.starts_with(b"async")
            && before_bytes.len() > 5
            && before_bytes[5].is_ascii_whitespace()
        {
            let mut s = 5;
            while s < before_bytes.len() && before_bytes[s].is_ascii_whitespace() {
                s += 1;
            }
            (i + s, end)
        } else {
            (i, end)
        };

        let param_bytes = &bytes[param_start..param_end];

        // (params) => pattern
        if param_bytes.first() == Some(&b'(') && param_bytes.last() == Some(&b')') {
            let inner = &expr[param_start + 1..param_end - 1];
            let inner_trimmed = inner.trim();
            if inner_trimmed.is_empty() {
                return Some(vize_carton::SmallVec::new());
            }
            return Some(extract_param_list_fast(inner_trimmed));
        }

        // Single param: e =>
        let param = &expr[param_start..param_end];
        if is_valid_identifier_fast(param.as_bytes()) {
            let mut result = vize_carton::SmallVec::new();
            result.push(CompactString::new(param));
            return Some(result);
        }
    }

    // Check for function expression
    if bytes[i..].starts_with(b"function") {
        let fn_end = i + 8;
        let mut paren_start = fn_end;
        while paren_start < len && bytes[paren_start] != b'(' {
            paren_start += 1;
        }
        if paren_start >= len {
            return None;
        }
        let mut paren_end = paren_start + 1;
        let mut depth = 1;
        while paren_end < len && depth > 0 {
            match bytes[paren_end] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            paren_end += 1;
        }
        if depth == 0 {
            let inner = &expr[paren_start + 1..paren_end - 1];
            let inner_trimmed = inner.trim();
            if inner_trimmed.is_empty() {
                return Some(vize_carton::SmallVec::new());
            }
            return Some(extract_param_list_fast(inner_trimmed));
        }
    }

    None
}

/// Find arrow "=>" position in bytes
#[inline]
fn find_arrow(bytes: &[u8], start: usize) -> Option<usize> {
    let len = bytes.len();
    if len < start + 2 {
        return None;
    }
    let mut i = start;
    while i < len - 1 {
        if bytes[i] == b'=' && bytes[i + 1] == b'>' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Fast identifier validation using bytes
#[inline]
pub fn is_valid_identifier_fast(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let first = bytes[0];
    if !first.is_ascii_alphabetic() && first != b'_' && first != b'$' {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
}

/// Extract parameter list from comma-separated string
#[inline]
fn extract_param_list_fast(params: &str) -> vize_carton::SmallVec<[CompactString; 4]> {
    let bytes = params.as_bytes();
    let len = bytes.len();
    let mut result = vize_carton::SmallVec::new();
    let mut i = 0;

    while i < len {
        // Skip whitespace
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        // Skip rest parameter prefix (...)
        if i + 2 < len && bytes[i] == b'.' && bytes[i + 1] == b'.' && bytes[i + 2] == b'.' {
            i += 3;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
        }

        // Skip destructuring patterns
        if i < len && (bytes[i] == b'{' || bytes[i] == b'[') {
            let open = bytes[i];
            let close = if open == b'{' { b'}' } else { b']' };
            let mut depth = 1;
            i += 1;
            while i < len && depth > 0 {
                if bytes[i] == open {
                    depth += 1;
                } else if bytes[i] == close {
                    depth -= 1;
                }
                i += 1;
            }
            while i < len && bytes[i] != b',' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        // Extract identifier
        let ident_start = i;
        while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
        {
            i += 1;
        }

        if i > ident_start {
            result.push(CompactString::new(&params[ident_start..i]));
        }

        // Skip to next comma
        while i < len && bytes[i] != b',' {
            i += 1;
        }
        if i < len {
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_identifiers_oxc() {
        fn to_strings(ids: Vec<CompactString>) -> Vec<String> {
            ids.into_iter().map(|s| s.to_string()).collect()
        }

        let ids = to_strings(extract_identifiers_oxc("count + 1"));
        assert_eq!(ids, vec!["count"]);

        let ids = to_strings(extract_identifiers_oxc("user.name + item.value"));
        assert_eq!(ids, vec!["user", "item"]);

        let ids = to_strings(extract_identifiers_oxc("{ active: isActive }"));
        assert_eq!(ids, vec!["isActive"]);

        let ids = to_strings(extract_identifiers_oxc("{ foo }"));
        assert_eq!(ids, vec!["foo"]);

        let ids = to_strings(extract_identifiers_oxc("cond ? a : b"));
        assert_eq!(ids, vec!["cond", "a", "b"]);
    }

    #[test]
    fn test_is_component_tag() {
        assert!(is_component_tag("MyComponent"));
        assert!(is_component_tag("my-component"));
        assert!(!is_component_tag("div"));
        assert!(!is_component_tag("span"));
    }

    #[test]
    fn test_is_builtin_directive() {
        assert!(is_builtin_directive("if"));
        assert!(is_builtin_directive("for"));
        assert!(is_builtin_directive("model"));
        assert!(!is_builtin_directive("custom"));
    }
}
