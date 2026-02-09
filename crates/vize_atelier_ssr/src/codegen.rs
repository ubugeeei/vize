//! SSR code generation.
//!
//! SSR code generation produces JavaScript that uses template literals and `_push()` calls
//! to build HTML strings on the server side.

use crate::options::SsrCompilerOptions;
use vize_atelier_core::ast::{
    CommentNode, ElementNode, ElementType, ForNode, IfNode, InterpolationNode, RootNode,
    RuntimeHelper, TemplateChildNode, TextNode,
};
use vize_carton::{Bump, FxHashSet};

/// SSR codegen result
#[derive(Debug, Default)]
pub struct SsrCodegenResult {
    /// Generated render function code
    pub code: String,
    /// Import preamble
    pub preamble: String,
}

/// SSR codegen context
pub struct SsrCodegenContext<'a> {
    #[allow(dead_code)]
    allocator: &'a Bump,
    options: &'a SsrCompilerOptions,
    /// Output buffer
    code: Vec<u8>,
    /// Indent level
    indent_level: u32,
    /// Used SSR helpers
    ssr_helpers: FxHashSet<RuntimeHelper>,
    /// Used core helpers (from vue)
    core_helpers: FxHashSet<RuntimeHelper>,
    /// Current template literal parts being accumulated
    current_template_parts: Vec<TemplatePart>,
    /// Whether we have an open _push call
    #[allow(dead_code)]
    has_open_push: bool,
    /// Whether currently within a slot scope
    #[allow(dead_code)]
    with_slot_scope_id: bool,
}

/// A part of a template literal
#[derive(Debug)]
enum TemplatePart {
    /// Static string content
    Static(String),
    /// Dynamic expression
    Dynamic(String),
}

impl<'a> SsrCodegenContext<'a> {
    pub fn new(allocator: &'a Bump, options: &'a SsrCompilerOptions) -> Self {
        Self {
            allocator,
            options,
            code: Vec::with_capacity(1024),
            indent_level: 0,
            ssr_helpers: FxHashSet::default(),
            core_helpers: FxHashSet::default(),
            current_template_parts: Vec::new(),
            has_open_push: false,
            with_slot_scope_id: false,
        }
    }

    /// Generate SSR code from the AST
    pub fn generate(mut self, root: &RootNode) -> SsrCodegenResult {
        // Check if this is a fragment (multiple non-text children)
        let is_fragment = root.children.len() > 1
            && root
                .children
                .iter()
                .any(|c| !matches!(c, TemplateChildNode::Text(_)));

        // Generate function signature
        self.push("function ssrRender(_ctx, _push, _parent, _attrs");
        if self.options.scope_id.is_some() {
            self.push(", _scopeId");
        }
        self.push(") {\n");
        self.indent_level += 1;

        // Inject CSS vars if present
        if let Some(css_vars) = &self.options.ssr_css_vars {
            self.push_indent();
            self.push("const _cssVars = { style: ");
            self.push(css_vars);
            self.push(" }\n");
        }

        // Process children
        self.process_children(&root.children, is_fragment, false, false);

        // Flush any remaining template literal
        self.flush_push();

        self.indent_level -= 1;
        self.push("}\n");

        // Build preamble with imports
        let preamble = self.build_preamble();

        SsrCodegenResult {
            code: String::from_utf8(self.code).unwrap_or_default(),
            preamble,
        }
    }

    /// Process a list of children nodes
    fn process_children(
        &mut self,
        children: &[TemplateChildNode],
        as_fragment: bool,
        disable_nested_fragments: bool,
        disable_comment: bool,
    ) {
        if as_fragment {
            self.push_string_part_static("<!--[-->");
        }

        for child in children {
            self.process_child(child, disable_nested_fragments, disable_comment);
        }

        if as_fragment {
            self.push_string_part_static("<!--]-->");
        }
    }

    /// Process a single child node
    fn process_child(
        &mut self,
        child: &TemplateChildNode,
        disable_nested_fragments: bool,
        disable_comment: bool,
    ) {
        match child {
            TemplateChildNode::Element(el) => {
                self.process_element(el, disable_nested_fragments);
            }
            TemplateChildNode::Text(text) => {
                self.process_text(text);
            }
            TemplateChildNode::Comment(comment) => {
                if !disable_comment {
                    self.process_comment(comment);
                }
            }
            TemplateChildNode::Interpolation(interp) => {
                self.process_interpolation(interp);
            }
            TemplateChildNode::If(if_node) => {
                self.process_if(if_node, disable_nested_fragments, disable_comment);
            }
            TemplateChildNode::For(for_node) => {
                self.process_for(for_node, disable_nested_fragments);
            }
            TemplateChildNode::IfBranch(_) => {
                // Handled by process_if
            }
            TemplateChildNode::TextCall(_) | TemplateChildNode::CompoundExpression(_) => {
                // These don't appear in SSR since transformText is not used
            }
            TemplateChildNode::Hoisted(_) => {
                // Hoisting is not used in SSR
            }
        }
    }

    /// Process an element node
    fn process_element(&mut self, el: &ElementNode, disable_nested_fragments: bool) {
        match el.tag_type {
            ElementType::Element => {
                self.process_plain_element(el);
            }
            ElementType::Component => {
                self.process_component(el, disable_nested_fragments);
            }
            ElementType::Slot => {
                self.process_slot_outlet(el);
            }
            ElementType::Template => {
                // Process template children directly
                self.process_children(&el.children, false, disable_nested_fragments, false);
            }
        }
    }

    /// Process a plain HTML element
    fn process_plain_element(&mut self, el: &ElementNode) {
        let tag = &el.tag;

        // Start tag
        self.push_string_part_static("<");
        self.push_string_part_static(tag);

        // Process attributes
        self.process_element_attrs(el);

        // Scope ID
        if let Some(scope_id) = &self.options.scope_id {
            self.push_string_part_static(" ");
            self.push_string_part_static(scope_id);
        }

        // Check if void element
        if vize_carton::is_void_tag(tag) {
            self.push_string_part_static(">");
            return;
        }

        self.push_string_part_static(">");

        // Process children
        self.process_children(&el.children, false, false, false);

        // End tag
        self.push_string_part_static("</");
        self.push_string_part_static(tag);
        self.push_string_part_static(">");
    }

    /// Process element attributes
    fn process_element_attrs(&mut self, el: &ElementNode) {
        use vize_atelier_core::ast::PropNode;

        for prop in &el.props {
            match prop {
                PropNode::Attribute(attr) => {
                    self.push_string_part_static(" ");
                    self.push_string_part_static(&attr.name);
                    if let Some(value) = &attr.value {
                        self.push_string_part_static("=\"");
                        // Escape HTML attribute value
                        self.push_string_part_static(&escape_html_attr(&value.content));
                        self.push_string_part_static("\"");
                    }
                }
                PropNode::Directive(dir) => {
                    self.process_directive_on_element(el, dir);
                }
            }
        }
    }

    /// Process a directive on an element
    fn process_directive_on_element(
        &mut self,
        el: &ElementNode,
        dir: &vize_atelier_core::ast::DirectiveNode,
    ) {
        match dir.name.as_str() {
            "bind" => {
                self.process_v_bind_on_element(el, dir);
            }
            "on" => {
                // Event handlers are ignored in SSR
            }
            "model" => {
                self.process_v_model_on_element(el, dir);
            }
            "show" => {
                self.process_v_show_on_element(el, dir);
            }
            "html" => {
                // v-html is processed when generating children
            }
            "text" => {
                // v-text is processed when generating children
            }
            _ => {
                // Custom directives: use ssrGetDirectiveProps
                self.process_custom_directive(el, dir);
            }
        }
    }

    /// Process v-bind directive
    fn process_v_bind_on_element(
        &mut self,
        _el: &ElementNode,
        dir: &vize_atelier_core::ast::DirectiveNode,
    ) {
        use vize_atelier_core::ast::ExpressionNode;

        // Get the argument (attribute name)
        let arg_name = match &dir.arg {
            Some(ExpressionNode::Simple(simple)) if simple.is_static => {
                Some(simple.content.clone())
            }
            _ => None,
        };

        // Get the expression
        let exp = match &dir.exp {
            Some(ExpressionNode::Simple(simple)) => simple.content.as_str(),
            Some(ExpressionNode::Compound(_)) => {
                // For compound expressions, we'd need to flatten - for now use placeholder
                "_ctx.value"
            }
            None => return,
        };

        match arg_name.as_deref() {
            Some("class") => {
                self.use_ssr_helper(RuntimeHelper::SsrRenderClass);
                self.push_string_part_dynamic(&format!("_ssrRenderClass({})", exp));
            }
            Some("style") => {
                self.use_ssr_helper(RuntimeHelper::SsrRenderStyle);
                self.push_string_part_static(" style=\"");
                self.push_string_part_dynamic(&format!("_ssrRenderStyle({})", exp));
                self.push_string_part_static("\"");
            }
            Some(name) => {
                self.use_ssr_helper(RuntimeHelper::SsrRenderAttr);
                self.push_string_part_dynamic(&format!("_ssrRenderAttr(\"{}\", {})", name, exp));
            }
            None => {
                // v-bind without argument - spread attributes
                self.use_ssr_helper(RuntimeHelper::SsrRenderAttrs);
                self.push_string_part_dynamic(&format!("_ssrRenderAttrs({})", exp));
            }
        }
    }

    /// Process v-model directive
    fn process_v_model_on_element(
        &mut self,
        el: &ElementNode,
        dir: &vize_atelier_core::ast::DirectiveNode,
    ) {
        use vize_atelier_core::ast::ExpressionNode;

        let exp = match &dir.exp {
            Some(ExpressionNode::Simple(simple)) => simple.content.as_str(),
            _ => return,
        };

        let tag = el.tag.as_str();

        match tag {
            "input" => {
                // Check input type from attributes
                let input_type = self.get_element_attr_value(el, "type");
                match input_type.as_deref() {
                    Some("checkbox") => {
                        self.use_ssr_helper(RuntimeHelper::SsrIncludeBooleanAttr);
                        self.use_ssr_helper(RuntimeHelper::SsrLooseContain);
                        self.push_string_part_dynamic(&format!(
                            "(_ssrIncludeBooleanAttr(Array.isArray({}) ? _ssrLooseContain({}, null) : {})) ? \" checked\" : \"\"",
                            exp, exp, exp
                        ));
                    }
                    Some("radio") => {
                        self.use_ssr_helper(RuntimeHelper::SsrIncludeBooleanAttr);
                        self.use_ssr_helper(RuntimeHelper::SsrLooseEqual);
                        let value = self.get_element_attr_value(el, "value");
                        let value_exp = value.as_deref().unwrap_or("null");
                        self.push_string_part_dynamic(&format!(
                            "(_ssrIncludeBooleanAttr(_ssrLooseEqual({}, {}))) ? \" checked\" : \"\"",
                            exp, value_exp
                        ));
                    }
                    _ => {
                        // text input
                        self.use_ssr_helper(RuntimeHelper::SsrRenderAttr);
                        self.push_string_part_dynamic(&format!(
                            "_ssrRenderAttr(\"value\", {})",
                            exp
                        ));
                    }
                }
            }
            "textarea" => {
                // textarea value is set as content
                self.use_ssr_helper(RuntimeHelper::SsrInterpolate);
                // Note: will be handled when processing children
            }
            "select" => {
                // select value is handled on child options
            }
            _ => {}
        }
    }

    /// Process v-show directive
    fn process_v_show_on_element(
        &mut self,
        _el: &ElementNode,
        dir: &vize_atelier_core::ast::DirectiveNode,
    ) {
        use vize_atelier_core::ast::ExpressionNode;

        let exp = match &dir.exp {
            Some(ExpressionNode::Simple(simple)) => simple.content.as_str(),
            _ => return,
        };

        // v-show="expr" => style="display: none" if !expr
        self.push_string_part_dynamic(&format!(
            "(({}) ? \"\" : \" style=\\\"display: none;\\\"\")",
            exp
        ));
    }

    /// Process a custom directive
    fn process_custom_directive(
        &mut self,
        _el: &ElementNode,
        dir: &vize_atelier_core::ast::DirectiveNode,
    ) {
        self.use_ssr_helper(RuntimeHelper::SsrGetDirectiveProps);
        // Custom directives use ssrGetDirectiveProps to merge props
        self.push_string_part_dynamic(&format!(
            "_ssrRenderAttrs(_ssrGetDirectiveProps(_ctx, _directives, \"{}\"))",
            dir.name
        ));
    }

    /// Get an attribute value from an element
    fn get_element_attr_value(&self, el: &ElementNode, name: &str) -> Option<String> {
        use vize_atelier_core::ast::PropNode;

        for prop in &el.props {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == name {
                    return attr.value.as_ref().map(|v| v.content.to_string());
                }
            }
        }
        None
    }

    /// Process a component
    fn process_component(&mut self, el: &ElementNode, _disable_nested_fragments: bool) {
        self.flush_push();
        self.use_ssr_helper(RuntimeHelper::SsrRenderComponent);
        self.use_core_helper(RuntimeHelper::ResolveComponent);

        let tag = &el.tag;

        self.push_indent();
        self.push("_push(_ssrRenderComponent(_component_");
        self.push(tag);
        self.push(", _attrs, ");

        // Process slots
        if el.children.is_empty() {
            self.push("null");
        } else {
            self.push("{\n");
            self.indent_level += 1;
            self.push_indent();
            self.push("default: _withCtx(() => [\n");
            self.indent_level += 1;

            // Flush and start fresh for slot content
            let old_parts = std::mem::take(&mut self.current_template_parts);
            self.process_children(&el.children, false, false, false);
            self.flush_push();
            self.current_template_parts = old_parts;

            self.indent_level -= 1;
            self.push_indent();
            self.push("]),\n");
            self.indent_level -= 1;
            self.push_indent();
            self.push("_: 1\n");
            self.push_indent();
            self.push("}");
        }

        self.push(", _parent))\n");
    }

    /// Process a slot outlet (<slot>)
    fn process_slot_outlet(&mut self, el: &ElementNode) {
        self.flush_push();
        self.use_ssr_helper(RuntimeHelper::SsrRenderSlot);

        self.push_indent();
        self.push("_ssrRenderSlot(_ctx.$slots, ");

        // Get slot name
        let slot_name = self.get_slot_name(el);
        self.push("\"");
        self.push(&slot_name);
        self.push("\", ");

        // Slot props
        self.push("{}, ");

        // Fallback content
        if el.children.is_empty() {
            self.push("null");
        } else {
            self.push("() => {\n");
            self.indent_level += 1;

            let old_parts = std::mem::take(&mut self.current_template_parts);
            self.process_children(&el.children, false, false, false);
            self.flush_push();
            self.current_template_parts = old_parts;

            self.indent_level -= 1;
            self.push_indent();
            self.push("}");
        }

        self.push(", _push, _parent");

        // Scope ID
        if self.options.scope_id.is_some() {
            self.push(", _scopeId");
        }

        self.push(")\n");
    }

    /// Get the name of a slot
    fn get_slot_name(&self, el: &ElementNode) -> String {
        use vize_atelier_core::ast::{ExpressionNode, PropNode};

        for prop in &el.props {
            if let PropNode::Directive(dir) = prop {
                if dir.name == "bind" {
                    if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                        if arg.content == "name" {
                            if let Some(ExpressionNode::Simple(exp)) = &dir.exp {
                                return exp.content.to_string();
                            }
                        }
                    }
                }
            } else if let PropNode::Attribute(attr) = prop {
                if attr.name == "name" {
                    if let Some(value) = &attr.value {
                        return value.content.to_string();
                    }
                }
            }
        }
        "default".to_string()
    }

    /// Process a text node
    fn process_text(&mut self, text: &TextNode) {
        self.push_string_part_static(&escape_html(&text.content));
    }

    /// Process a comment node
    fn process_comment(&mut self, comment: &CommentNode) {
        self.push_string_part_static("<!--");
        self.push_string_part_static(&comment.content);
        self.push_string_part_static("-->");
    }

    /// Process an interpolation node ({{ expr }})
    fn process_interpolation(&mut self, interp: &InterpolationNode) {
        use vize_atelier_core::ast::ExpressionNode;

        self.use_ssr_helper(RuntimeHelper::SsrInterpolate);

        let exp = match &interp.content {
            ExpressionNode::Simple(simple) => simple.content.as_str(),
            ExpressionNode::Compound(_) => "_ctx.value", // placeholder
        };

        self.push_string_part_dynamic(&format!("_ssrInterpolate({})", exp));
    }

    /// Process an if node
    fn process_if(
        &mut self,
        if_node: &IfNode,
        disable_nested_fragments: bool,
        disable_comment: bool,
    ) {
        // Flush current push before if statement
        self.flush_push();

        for (i, branch) in if_node.branches.iter().enumerate() {
            self.push_indent();

            if i == 0 {
                // First branch: if
                self.push("if (");
                if let Some(condition) = &branch.condition {
                    self.push_expression(condition);
                }
                self.push(") {\n");
            } else if branch.condition.is_some() {
                // else-if
                self.push("} else if (");
                if let Some(condition) = &branch.condition {
                    self.push_expression(condition);
                }
                self.push(") {\n");
            } else {
                // else
                self.push("} else {\n");
            }

            self.indent_level += 1;

            // Check if branch needs fragment
            let needs_fragment = !disable_nested_fragments && branch.children.len() > 1;

            self.process_children(
                &branch.children,
                needs_fragment,
                disable_nested_fragments,
                disable_comment,
            );
            self.flush_push();

            self.indent_level -= 1;
        }

        // If no else branch, emit empty comment
        if if_node.branches.iter().all(|b| b.condition.is_some()) {
            self.push_indent();
            self.push("} else {\n");
            self.indent_level += 1;
            self.push_string_part_static("<!---->");
            self.flush_push();
            self.indent_level -= 1;
        }

        self.push_indent();
        self.push("}\n");
    }

    /// Process a for node
    fn process_for(&mut self, for_node: &ForNode, disable_nested_fragments: bool) {
        // Flush current push before for statement
        self.flush_push();

        self.use_ssr_helper(RuntimeHelper::SsrRenderList);

        // Fragment markers for v-for
        if !disable_nested_fragments {
            self.push_indent();
            self.push("_push(`<!--[-->`)\n");
        }

        self.push_indent();
        self.push("_ssrRenderList(");
        self.push_expression(&for_node.source);
        self.push(", (");

        // Value alias
        if let Some(value) = &for_node.value_alias {
            self.push_expression(value);
        }
        // Key alias
        if let Some(key) = &for_node.key_alias {
            self.push(", ");
            self.push_expression(key);
        }
        // Index alias
        if let Some(index) = &for_node.object_index_alias {
            self.push(", ");
            self.push_expression(index);
        }

        self.push(") => {\n");
        self.indent_level += 1;

        // Process for body
        let needs_fragment = !disable_nested_fragments && for_node.children.len() > 1;
        self.process_children(&for_node.children, needs_fragment, true, false);
        self.flush_push();

        self.indent_level -= 1;
        self.push_indent();
        self.push("})\n");

        // Closing fragment marker
        if !disable_nested_fragments {
            self.push_indent();
            self.push("_push(`<!--]-->`)\n");
        }
    }

    /// Push an expression node
    fn push_expression(&mut self, expr: &vize_atelier_core::ast::ExpressionNode) {
        use vize_atelier_core::ast::ExpressionNode;

        match expr {
            ExpressionNode::Simple(simple) => {
                self.push(&simple.content);
            }
            ExpressionNode::Compound(compound) => {
                // Flatten compound expression
                for child in &compound.children {
                    use vize_atelier_core::ast::CompoundExpressionChild;
                    match child {
                        CompoundExpressionChild::Simple(s) => self.push(&s.content),
                        CompoundExpressionChild::String(s) => self.push(s),
                        CompoundExpressionChild::Symbol(helper) => {
                            self.push("_");
                            self.push(helper.name());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Push static string content to the current template literal
    fn push_string_part_static(&mut self, s: &str) {
        if let Some(TemplatePart::Static(last)) = self.current_template_parts.last_mut() {
            last.push_str(s);
        } else {
            self.current_template_parts
                .push(TemplatePart::Static(s.to_string()));
        }
    }

    /// Push dynamic expression to the current template literal
    fn push_string_part_dynamic(&mut self, expr: &str) {
        self.current_template_parts
            .push(TemplatePart::Dynamic(expr.to_string()));
    }

    /// Flush the current template literal as a _push() call
    fn flush_push(&mut self) {
        if self.current_template_parts.is_empty() {
            return;
        }

        // Take ownership of parts to avoid borrow issues
        let parts = std::mem::take(&mut self.current_template_parts);

        self.push_indent();
        self.push("_push(`");

        for part in &parts {
            match part {
                TemplatePart::Static(s) => {
                    // Escape backticks and ${
                    let escaped = s.replace('`', "\\`").replace("${", "\\${");
                    self.push(&escaped);
                }
                TemplatePart::Dynamic(expr) => {
                    self.push("${");
                    self.push(expr);
                    self.push("}");
                }
            }
        }

        self.push("`)\n");
    }

    /// Use an SSR helper
    fn use_ssr_helper(&mut self, helper: RuntimeHelper) {
        self.ssr_helpers.insert(helper);
    }

    /// Use a core helper (from vue)
    fn use_core_helper(&mut self, helper: RuntimeHelper) {
        self.core_helpers.insert(helper);
    }

    /// Push raw code to the buffer
    fn push(&mut self, s: &str) {
        self.code.extend_from_slice(s.as_bytes());
    }

    /// Push indentation
    fn push_indent(&mut self) {
        for _ in 0..self.indent_level {
            self.code.extend_from_slice(b"  ");
        }
    }

    /// Build the preamble with imports
    fn build_preamble(&self) -> String {
        let mut preamble = String::new();

        // SSR helpers from @vue/server-renderer
        if !self.ssr_helpers.is_empty() {
            preamble.push_str("import { ");
            let helpers: Vec<_> = self
                .ssr_helpers
                .iter()
                .map(|h| format!("{} as _{}", h.name(), h.name()))
                .collect();
            preamble.push_str(&helpers.join(", "));
            preamble.push_str(" } from \"@vue/server-renderer\"\n");
        }

        // Core helpers from vue
        if !self.core_helpers.is_empty() {
            preamble.push_str("import { ");
            let helpers: Vec<_> = self
                .core_helpers
                .iter()
                .map(|h| format!("{} as _{}", h.name(), h.name()))
                .collect();
            preamble.push_str(&helpers.join(", "));
            preamble.push_str(" } from \"vue\"\n");
        }

        preamble
    }
}

/// Escape HTML special characters
fn escape_html(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(c),
        }
    }
    result
}

/// Escape HTML attribute value
fn escape_html_attr(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '"' => result.push_str("&quot;"),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<div>"), "&lt;div&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("\"hello\""), "&quot;hello&quot;");
    }

    #[test]
    fn test_escape_html_all_special_chars() {
        assert_eq!(escape_html("&"), "&amp;");
        assert_eq!(escape_html("<"), "&lt;");
        assert_eq!(escape_html(">"), "&gt;");
        assert_eq!(escape_html("\""), "&quot;");
        assert_eq!(escape_html("'"), "&#39;");
    }

    #[test]
    fn test_escape_html_no_special() {
        assert_eq!(escape_html("hello world"), "hello world");
        assert_eq!(escape_html("abc123"), "abc123");
    }

    #[test]
    fn test_escape_html_attr() {
        assert_eq!(escape_html_attr("hello\"world"), "hello&quot;world");
        assert_eq!(escape_html_attr("a & b"), "a &amp; b");
    }

    #[test]
    fn test_escape_html_attr_preserves_angle_brackets() {
        // In attribute context, < and > do not need escaping
        assert_eq!(escape_html_attr("<foo>"), "<foo>");
        assert_eq!(escape_html_attr("a > b"), "a > b");
    }

    #[test]
    fn test_escape_html_attr_no_special() {
        assert_eq!(escape_html_attr("hello"), "hello");
    }

    #[test]
    fn test_ssr_codegen_result_default() {
        let result = SsrCodegenResult::default();
        assert!(result.code.is_empty());
        assert!(result.preamble.is_empty());
    }
}
