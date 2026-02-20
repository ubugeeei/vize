//! AST visitor for lint rule execution.
//!
//! High-performance visitor with minimal allocations.

use crate::context::{ElementContext, LintContext};
use crate::rule::Rule;
use vize_carton::directive::{parse_level_severity, parse_vize_directive, DirectiveKind};
use vize_carton::CompactString;
use vize_relief::ast::{
    CommentNode, ElementNode, ExpressionNode, PropNode, RootNode, TemplateChildNode,
};

/// Visit the AST and run all rules
pub struct LintVisitor<'a, 'ctx, 'rules> {
    ctx: &'ctx mut LintContext<'a>,
    rules: &'rules [Box<dyn Rule>],
}

impl<'a, 'ctx, 'rules> LintVisitor<'a, 'ctx, 'rules> {
    /// Create a new visitor
    #[inline]
    pub fn new(ctx: &'ctx mut LintContext<'a>, rules: &'rules [Box<dyn Rule>]) -> Self {
        Self { ctx, rules }
    }

    /// Visit the root node and traverse the AST
    #[inline]
    pub fn visit_root(&mut self, root: &RootNode<'a>) {
        // Run template-level checks
        for rule in self.rules.iter() {
            self.ctx.current_rule = rule.meta().name;
            rule.run_on_template(self.ctx, root);
        }

        // Visit children
        for child in root.children.iter() {
            self.visit_child(child);
        }
    }

    #[inline]
    fn visit_child(&mut self, node: &TemplateChildNode<'a>) {
        match node {
            TemplateChildNode::Element(el) => self.visit_element(el),
            TemplateChildNode::Interpolation(interp) => {
                for rule in self.rules.iter() {
                    self.ctx.current_rule = rule.meta().name;
                    rule.check_interpolation(self.ctx, interp);
                }
            }
            TemplateChildNode::If(if_node) => self.visit_if(if_node),
            TemplateChildNode::For(for_node) => self.visit_for(for_node),
            TemplateChildNode::Comment(comment) => {
                self.process_disable_comment(&comment.content, comment.loc.start.line);
                if let Some(kind) = comment.directive {
                    self.process_vize_directive(comment, kind);
                }
            }
            TemplateChildNode::Text(_) => {}
            _ => {}
        }
    }

    /// Process disable comments like `vize-disable` or `vize-disable-next-line`
    fn process_disable_comment(&mut self, content: &str, line: u32) {
        let content = content.trim();

        // vize-disable-next-line [rule1, rule2, ...]
        if let Some(rest) = content.strip_prefix("vize-disable-next-line") {
            let rest = rest.trim();
            if rest.is_empty() {
                self.ctx.disable_next_line(line);
            } else {
                let rules: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
                self.ctx.disable_rules_next_line(&rules, line);
            }
            return;
        }

        // vize-disable [rule1, rule2, ...]
        if let Some(rest) = content.strip_prefix("vize-disable") {
            let rest = rest.trim();
            if rest.is_empty() {
                self.ctx.disable_all(line, None);
            } else {
                let rules: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
                self.ctx.disable_rules(&rules, line, None);
            }
        }
    }

    /// Process `@vize:` directives on comment nodes.
    fn process_vize_directive(&mut self, comment: &CommentNode, kind: DirectiveKind) {
        let line = comment.loc.start.line;
        let loc = &comment.loc;

        match kind {
            DirectiveKind::Todo => {
                // Parse the payload from the comment content
                if let Some(d) = parse_vize_directive(&comment.content, line, loc.start.offset) {
                    let msg = if d.payload.is_empty() {
                        CompactString::from("TODO")
                    } else {
                        CompactString::from(format!("TODO: {}", d.payload))
                    };
                    self.ctx.current_rule = "vize/todo";
                    self.ctx.warn(msg, loc);
                }
            }
            DirectiveKind::Fixme => {
                if let Some(d) = parse_vize_directive(&comment.content, line, loc.start.offset) {
                    let msg = if d.payload.is_empty() {
                        CompactString::from("FIXME")
                    } else {
                        CompactString::from(format!("FIXME: {}", d.payload))
                    };
                    self.ctx.current_rule = "vize/fixme";
                    self.ctx.error(msg, loc);
                }
            }
            DirectiveKind::Expected => {
                self.ctx.expect_error_next_line(line);
            }
            DirectiveKind::IgnoreStart => {
                self.ctx.push_ignore_region(line);
            }
            DirectiveKind::IgnoreEnd => {
                self.ctx.pop_ignore_region(line);
            }
            DirectiveKind::Level => {
                if let Some(d) = parse_vize_directive(&comment.content, line, loc.start.offset) {
                    if let Some(severity) = parse_level_severity(&d.payload) {
                        self.ctx.set_severity_override_next_line(line, severity);
                    }
                }
            }
            DirectiveKind::Deprecated => {
                if let Some(d) = parse_vize_directive(&comment.content, line, loc.start.offset) {
                    let msg = if d.payload.is_empty() {
                        CompactString::from("Deprecated")
                    } else {
                        CompactString::from(format!("Deprecated: {}", d.payload))
                    };
                    self.ctx.current_rule = "vize/deprecated";
                    self.ctx.warn(msg, loc);
                }
            }
            // Docs, DevOnly, Unknown: no lint action needed
            _ => {}
        }
    }

    fn visit_element(&mut self, el: &ElementNode<'a>) {
        // Check for v-for and v-if directives using iterators (no allocation)
        let has_v_for = el
            .props
            .iter()
            .any(|p| matches!(p, PropNode::Directive(d) if d.name.as_str() == "for"));
        let has_v_if = el
            .props
            .iter()
            .any(|p| matches!(p, PropNode::Directive(d) if d.name.as_str() == "if" || d.name.as_str() == "else-if"));

        // Extract v-for variables (only allocates if v-for exists)
        let v_for_vars = if has_v_for {
            self.extract_v_for_vars(el)
        } else {
            Vec::new()
        };

        // Build element context with CompactString tag (efficient for small strings)
        let elem_ctx = ElementContext {
            tag: CompactString::from(el.tag.as_str()),
            has_v_for,
            has_v_if,
            v_for_vars,
        };

        self.ctx.push_element(elem_ctx);

        // Enter element - run rules
        for rule in self.rules.iter() {
            self.ctx.current_rule = rule.meta().name;
            rule.enter_element(self.ctx, el);
        }

        // Check directives
        for prop in el.props.iter() {
            if let PropNode::Directive(dir) = prop {
                for rule in self.rules.iter() {
                    self.ctx.current_rule = rule.meta().name;
                    rule.check_directive(self.ctx, el, dir);
                }
            }
        }

        // Visit children
        for child in el.children.iter() {
            self.visit_child(child);
        }

        // Exit element - run rules
        for rule in self.rules.iter() {
            self.ctx.current_rule = rule.meta().name;
            rule.exit_element(self.ctx, el);
        }

        self.ctx.pop_element();
    }

    #[inline]
    fn visit_if(&mut self, if_node: &vize_relief::ast::IfNode<'a>) {
        // Run if checks
        for rule in self.rules.iter() {
            self.ctx.current_rule = rule.meta().name;
            rule.check_if(self.ctx, if_node);
        }

        // Visit branches
        for branch in if_node.branches.iter() {
            for child in branch.children.iter() {
                self.visit_child(child);
            }
        }
    }

    #[inline]
    fn visit_for(&mut self, for_node: &vize_relief::ast::ForNode<'a>) {
        // Run for checks
        for rule in self.rules.iter() {
            self.ctx.current_rule = rule.meta().name;
            rule.check_for(self.ctx, for_node);
        }

        // Visit children
        for child in for_node.children.iter() {
            self.visit_child(child);
        }
    }

    /// Extract variable names from v-for directive on an element
    #[inline]
    fn extract_v_for_vars(&self, el: &ElementNode<'a>) -> Vec<CompactString> {
        for prop in el.props.iter() {
            if let PropNode::Directive(dir) = prop {
                if dir.name.as_str() == "for" {
                    if let Some(exp) = &dir.exp {
                        return parse_v_for_variables(exp);
                    }
                }
            }
        }
        Vec::new()
    }
}

/// Parse v-for expression to extract variable names.
///
/// Uses CompactString for efficient small string storage.
///
/// Handles formats like:
/// - `item in items`
/// - `(item, index) in items`
/// - `(value, key, index) in object`
#[inline]
pub fn parse_v_for_variables(exp: &ExpressionNode) -> Vec<CompactString> {
    let content = match exp {
        ExpressionNode::Simple(s) => s.content.as_str(),
        ExpressionNode::Compound(_) => return Vec::new(),
    };

    // Split on " in " or " of " - use byte search for speed
    let bytes = content.as_bytes();
    let (alias_part, _) = if let Some(idx) = find_pattern(bytes, b" in ") {
        (&content[..idx], &content[idx + 4..])
    } else if let Some(idx) = find_pattern(bytes, b" of ") {
        (&content[..idx], &content[idx + 4..])
    } else {
        return Vec::new();
    };

    let alias_str = alias_part.trim();

    // Handle destructuring: (item, index), { id, name }, or [first, second]
    let is_tuple = alias_str.starts_with('(') && alias_str.ends_with(')');
    let is_object = alias_str.starts_with('{') && alias_str.ends_with('}');
    let is_array = alias_str.starts_with('[') && alias_str.ends_with(']');

    if is_tuple || is_object || is_array {
        let inner = &alias_str[1..alias_str.len() - 1];
        // Pre-allocate with estimated capacity
        let mut vars = Vec::with_capacity(3);
        for s in inner.split(',') {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Handle object shorthand: { id } -> id, { id: itemId } -> itemId
            if is_object {
                if let Some(colon_idx) = trimmed.find(':') {
                    // { id: itemId } -> itemId
                    let value_part = trimmed[colon_idx + 1..].trim();
                    if !value_part.is_empty() {
                        vars.push(CompactString::from(value_part));
                    }
                } else {
                    // { id } -> id (shorthand)
                    vars.push(CompactString::from(trimmed));
                }
            } else {
                vars.push(CompactString::from(trimmed));
            }
        }
        vars
    } else {
        // Single variable - avoid allocation if possible
        vec![CompactString::from(alias_str)]
    }
}

/// Fast byte pattern search
#[inline]
fn find_pattern(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }

    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vize_carton::Bump;
    use vize_relief::ast::SimpleExpressionNode;

    fn make_simple_exp<'a>(allocator: &'a Bump, content: &str) -> ExpressionNode<'a> {
        ExpressionNode::Simple(vize_carton::Box::new_in(
            SimpleExpressionNode::new(
                vize_carton::String::from(content),
                false,
                vize_relief::ast::SourceLocation::STUB,
            ),
            allocator,
        ))
    }

    #[test]
    fn test_parse_v_for_simple() {
        let allocator = Bump::new();
        let exp = make_simple_exp(&allocator, "item in items");
        let vars = parse_v_for_variables(&exp);
        assert_eq!(vars, vec![CompactString::from("item")]);
    }

    #[test]
    fn test_parse_v_for_with_index() {
        let allocator = Bump::new();
        let exp = make_simple_exp(&allocator, "(item, index) in items");
        let vars = parse_v_for_variables(&exp);
        assert_eq!(
            vars,
            vec![CompactString::from("item"), CompactString::from("index")]
        );
    }

    #[test]
    fn test_parse_v_for_object() {
        let allocator = Bump::new();
        let exp = make_simple_exp(&allocator, "(value, key, index) in object");
        let vars = parse_v_for_variables(&exp);
        assert_eq!(
            vars,
            vec![
                CompactString::from("value"),
                CompactString::from("key"),
                CompactString::from("index"),
            ]
        );
    }

    #[test]
    fn test_parse_v_for_object_destructuring() {
        let allocator = Bump::new();
        let exp = make_simple_exp(&allocator, "{ id } in items");
        let vars = parse_v_for_variables(&exp);
        assert_eq!(vars, vec![CompactString::from("id")]);
    }

    #[test]
    fn test_parse_v_for_object_destructuring_multiple() {
        let allocator = Bump::new();
        let exp = make_simple_exp(&allocator, "{ id, name } in items");
        let vars = parse_v_for_variables(&exp);
        assert_eq!(
            vars,
            vec![CompactString::from("id"), CompactString::from("name")]
        );
    }

    #[test]
    fn test_parse_v_for_object_destructuring_with_rename() {
        let allocator = Bump::new();
        let exp = make_simple_exp(&allocator, "{ id: itemId, name: itemName } in items");
        let vars = parse_v_for_variables(&exp);
        assert_eq!(
            vars,
            vec![
                CompactString::from("itemId"),
                CompactString::from("itemName")
            ]
        );
    }

    #[test]
    fn test_parse_v_for_array_destructuring() {
        let allocator = Bump::new();
        let exp = make_simple_exp(&allocator, "[first, second] in items");
        let vars = parse_v_for_variables(&exp);
        assert_eq!(
            vars,
            vec![CompactString::from("first"), CompactString::from("second")]
        );
    }
}
