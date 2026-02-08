//! Vue template parser.
//!
//! This parser uses the tokenizer to convert Vue templates into an AST.

use vize_carton::{Box, Bump, String, Vec};
use vize_relief::ast::*;
use vize_relief::errors::{CompilerError, ErrorCode};
use vize_relief::options::{ParserOptions, WhitespaceStrategy};

use crate::tokenizer::{Callbacks, QuoteType, Tokenizer};

/// Parser context for building AST
pub struct Parser<'a> {
    /// Arena allocator
    allocator: &'a Bump,
    /// Source code
    source: &'a str,
    /// Parser options
    options: ParserOptions,
    /// Current node stack
    stack: Vec<'a, ParserStackEntry<'a>>,
    /// Root node
    root: Option<RootNode<'a>>,
    /// Current element being parsed
    current_element: Option<CurrentElement<'a>>,
    /// Current attribute being parsed
    current_attr: Option<CurrentAttribute<'a>>,
    /// Current directive being parsed
    current_dir: Option<CurrentDirective<'a>>,
    /// Errors collected during parsing
    errors: Vec<'a, CompilerError>,
    /// Newline positions for calculating line/column
    newlines: Vec<'a, usize>,
    /// Whether in pre block
    in_pre: bool,
    /// Whether in v-pre block
    in_v_pre: bool,
}

/// Stack entry for tracking parent elements
#[derive(Debug)]
struct ParserStackEntry<'a> {
    element: ElementNode<'a>,
    in_pre: bool,
    in_v_pre: bool,
}

/// Current element being parsed
struct CurrentElement<'a> {
    tag: String,
    tag_start: usize,
    #[allow(dead_code)]
    tag_end: usize,
    ns: Namespace,
    is_self_closing: bool,
    props: Vec<'a, PropNode<'a>>,
}

/// Current attribute being parsed
struct CurrentAttribute<'a> {
    name: String,
    name_start: usize,
    name_end: usize,
    value_start: Option<usize>,
    value_end: Option<usize>,
    _marker: std::marker::PhantomData<&'a ()>,
}

/// Current directive being parsed
struct CurrentDirective<'a> {
    name: String,
    raw_name: String,
    name_start: usize,
    #[allow(dead_code)]
    name_end: usize,
    arg: Option<(String, usize, usize, bool)>, // (content, start, end, is_dynamic)
    modifiers: Vec<'a, (String, usize, usize)>,
    value_start: Option<usize>,
    value_end: Option<usize>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Parser<'a> {
    /// Create a new parser
    pub fn new(allocator: &'a Bump, source: &'a str) -> Self {
        Self::with_options(allocator, source, ParserOptions::default())
    }

    /// Create a new parser with options
    pub fn with_options(allocator: &'a Bump, source: &'a str, options: ParserOptions) -> Self {
        Self {
            allocator,
            source,
            options,
            stack: Vec::new_in(allocator),
            root: None,
            current_element: None,
            current_attr: None,
            current_dir: None,
            errors: Vec::new_in(allocator),
            newlines: Vec::new_in(allocator),
            in_pre: false,
            in_v_pre: false,
        }
    }

    /// Parse the source and return the AST
    pub fn parse(mut self) -> (RootNode<'a>, Vec<'a, CompilerError>) {
        // Initialize root node
        let root = RootNode::new(self.allocator, self.source);
        self.root = Some(root);

        // Copy delimiters to avoid borrow issue
        let delimiter_open: Vec<'a, u8> =
            Vec::from_iter_in(self.options.delimiters.0.bytes(), self.allocator);
        let delimiter_close: Vec<'a, u8> =
            Vec::from_iter_in(self.options.delimiters.1.bytes(), self.allocator);

        // We need to use a struct that implements Callbacks
        // Create a wrapper that can capture the parser
        let mut tokenizer = Tokenizer::with_delimiters(
            self.source,
            ParserCallbacks { parser: &mut self },
            &delimiter_open,
            &delimiter_close,
        );
        tokenizer.tokenize();

        // Handle any unclosed elements
        self.handle_unclosed_elements();

        // Condense whitespace if needed
        if let Some(ref mut root) = self.root {
            if self.options.whitespace == WhitespaceStrategy::Condense {
                condense_whitespace(&mut root.children);
            }
        }

        let root = self.root.take().unwrap();
        (root, self.errors)
    }

    /// Get source slice
    fn get_source(&self, start: usize, end: usize) -> &str {
        &self.source[start..end]
    }

    /// Calculate position from byte offset
    fn get_pos(&self, offset: usize) -> Position {
        let line = match self.newlines.binary_search(&offset) {
            Ok(i) => i + 1,
            Err(i) => i + 1,
        };

        let column = if line == 1 {
            offset + 1
        } else if line > 1 && line - 2 < self.newlines.len() {
            offset - self.newlines[line - 2]
        } else {
            offset + 1
        };

        Position::new(offset as u32, line as u32, column as u32)
    }

    /// Create a source location
    fn create_loc(&self, start: usize, end: usize) -> SourceLocation {
        SourceLocation::new(
            self.get_pos(start),
            self.get_pos(end),
            self.get_source(start, end),
        )
    }

    /// Add child to current context (stack top or root)
    fn add_child(&mut self, child: TemplateChildNode<'a>) {
        if let Some(entry) = self.stack.last_mut() {
            entry.element.children.push(child);
        } else if let Some(ref mut root) = self.root {
            root.children.push(child);
        }
    }

    /// Handle unclosed elements at end of parsing
    fn handle_unclosed_elements(&mut self) {
        while let Some(entry) = self.stack.pop() {
            let loc = entry.element.loc.clone();
            self.errors
                .push(CompilerError::new(ErrorCode::MissingEndTag, Some(loc)));

            // Add the unclosed element to parent
            let boxed = Box::new_in(entry.element, self.allocator);
            self.add_child(TemplateChildNode::Element(boxed));
        }
    }

    /// Process text content
    fn on_text_impl(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }

        let content = self.get_source(start, end);
        let loc = self.create_loc(start, end);

        let text_node = TextNode::new(content, loc);
        let boxed = Box::new_in(text_node, self.allocator);
        self.add_child(TemplateChildNode::Text(boxed));
    }

    /// Process interpolation
    fn on_interpolation_impl(&mut self, start: usize, end: usize) {
        let raw_content = self.get_source(start, end);
        let content = raw_content.trim();

        // Calculate trimmed positions for accurate source mapping
        let leading_ws = raw_content.len() - raw_content.trim_start().len();
        let trimmed_start = start + leading_ws;
        let trimmed_end = trimmed_start + content.len();

        let delim_len = self.options.delimiters.0.len();
        let full_start = start - delim_len;
        let full_end = end + self.options.delimiters.1.len();
        let loc = self.create_loc(full_start, full_end);
        let inner_loc = self.create_loc(trimmed_start, trimmed_end);

        // Create expression node
        let expr = SimpleExpressionNode::new(content, false, inner_loc);
        let expr_boxed = Box::new_in(expr, self.allocator);

        let interp = InterpolationNode {
            content: ExpressionNode::Simple(expr_boxed),
            loc,
        };
        let boxed = Box::new_in(interp, self.allocator);
        self.add_child(TemplateChildNode::Interpolation(boxed));
    }

    /// Process open tag name
    fn on_open_tag_name_impl(&mut self, start: usize, end: usize) {
        let tag = self.get_source(start, end);
        let ns =
            (self.options.get_namespace)(tag, self.stack.last().map(|e| e.element.tag.as_str()));

        self.current_element = Some(CurrentElement {
            tag: tag.into(),
            tag_start: start,
            tag_end: end,
            ns,
            is_self_closing: false,
            props: Vec::new_in(self.allocator),
        });
    }

    /// Process open tag end
    fn on_open_tag_end_impl(&mut self, end: usize) {
        if let Some(current) = self.current_element.take() {
            let tag_start = current.tag_start;
            let loc = self.create_loc(tag_start - 1, end + 1); // Include < and >

            let mut element = ElementNode::new(self.allocator, current.tag.clone(), loc);
            element.ns = current.ns;
            element.is_self_closing = current.is_self_closing;
            element.props = current.props;

            // Determine element type
            element.tag_type = self.determine_element_type(&element);

            // Check for pre tags
            let is_pre = (self.options.is_pre_tag)(element.tag.as_str());
            let has_v_pre = element
                .props
                .iter()
                .any(|p| matches!(p, PropNode::Directive(d) if d.name == "pre"));

            if current.is_self_closing || (self.options.is_void_tag)(element.tag.as_str()) {
                // Self-closing or void tag, add directly
                let boxed = Box::new_in(element, self.allocator);
                self.add_child(TemplateChildNode::Element(boxed));
            } else {
                // Push to stack
                self.stack.push(ParserStackEntry {
                    element,
                    in_pre: self.in_pre,
                    in_v_pre: self.in_v_pre,
                });
                self.in_pre = is_pre || self.in_pre;
                self.in_v_pre = has_v_pre || self.in_v_pre;
            }
        }
    }

    /// Process self-closing tag
    fn on_self_closing_tag_impl(&mut self, _end: usize) {
        if let Some(ref mut current) = self.current_element {
            current.is_self_closing = true;
        }
    }

    /// Process close tag
    fn on_close_tag_impl(&mut self, start: usize, end: usize) {
        let tag = self.get_source(start, end);

        // Find matching open tag
        let mut found = false;
        for i in (0..self.stack.len()).rev() {
            if self.stack[i].element.tag.eq_ignore_ascii_case(tag) {
                found = true;

                // Pop all elements up to and including the match
                let mut elements: Vec<'a, ParserStackEntry<'a>> = Vec::new_in(self.allocator);
                while self.stack.len() > i {
                    elements.push(self.stack.pop().unwrap());
                }

                // Report errors for unclosed elements (except the matching one)
                for entry in elements.iter().skip(1) {
                    let loc = entry.element.loc.clone();
                    self.errors
                        .push(CompilerError::new(ErrorCode::MissingEndTag, Some(loc)));
                }

                // Add all popped elements back as children
                for entry in elements.into_iter().rev() {
                    let in_pre = entry.in_pre;
                    let in_v_pre = entry.in_v_pre;

                    let boxed = Box::new_in(entry.element, self.allocator);
                    self.add_child(TemplateChildNode::Element(boxed));

                    self.in_pre = in_pre;
                    self.in_v_pre = in_v_pre;
                }

                break;
            }
        }

        if !found {
            let loc = self.create_loc(start - 2, end + 1); // Include </ and >
            self.errors
                .push(CompilerError::new(ErrorCode::InvalidEndTag, Some(loc)));
        }
    }

    /// Determine element type (element, component, slot, template)
    fn determine_element_type(&self, element: &ElementNode<'a>) -> ElementType {
        let tag = element.tag.as_str();

        // Check for slot
        if tag == "slot" {
            return ElementType::Slot;
        }

        // Check for template
        if tag == "template" {
            // Template with v-if, v-for, or v-slot is a template element
            let has_structural_directive = element.props.iter().any(|p| {
                matches!(p, PropNode::Directive(d) if matches!(d.name.as_str(), "if" | "else-if" | "else" | "for" | "slot"))
            });
            if has_structural_directive {
                return ElementType::Template;
            }
        }

        // Check if it's a component
        if self.is_component(tag) {
            return ElementType::Component;
        }

        ElementType::Element
    }

    /// Check if tag is a component
    fn is_component(&self, tag: &str) -> bool {
        // Core built-in components
        if matches!(
            tag,
            "Teleport"
                | "Suspense"
                | "KeepAlive"
                | "BaseTransition"
                | "Transition"
                | "TransitionGroup"
        ) {
            return true;
        }

        // Custom element check
        if let Some(is_custom) = self.options.is_custom_element {
            if is_custom(tag) {
                return false;
            }
        }

        // Native tag check
        if let Some(is_native) = self.options.is_native_tag {
            if !is_native(tag) {
                return true;
            }
        } else {
            // Default: check if starts with uppercase
            if tag.chars().next().is_some_and(|c| c.is_uppercase()) {
                return true;
            }
        }

        false
    }

    /// Process attribute name
    fn on_attrib_name_impl(&mut self, start: usize, end: usize) {
        let name = self.get_source(start, end);
        self.current_attr = Some(CurrentAttribute {
            name: name.into(),
            name_start: start,
            name_end: end,
            value_start: None,
            value_end: None,
            _marker: std::marker::PhantomData,
        });
    }

    /// Process directive name
    fn on_dir_name_impl(&mut self, start: usize, end: usize) {
        let raw_name = self.get_source(start, end);
        let name = parse_directive_name(raw_name);

        self.current_dir = Some(CurrentDirective {
            name: name.into(),
            raw_name: raw_name.into(),
            name_start: start,
            name_end: end,
            arg: None,
            modifiers: Vec::new_in(self.allocator),
            value_start: None,
            value_end: None,
            _marker: std::marker::PhantomData,
        });
    }

    /// Process directive argument
    fn on_dir_arg_impl(&mut self, start: usize, end: usize) {
        let arg: String = self.get_source(start, end).into();
        // Check if dynamic arg (was inside [ ])
        let is_dynamic = start > 0 && self.source.as_bytes().get(start - 1) == Some(&b'[');
        if let Some(ref mut dir) = self.current_dir {
            dir.arg = Some((arg, start, end, is_dynamic));
        }
    }

    /// Process directive modifier
    fn on_dir_modifier_impl(&mut self, start: usize, end: usize) {
        let modifier: String = self.get_source(start, end).into();
        if let Some(ref mut dir) = self.current_dir {
            dir.modifiers.push((modifier, start, end));
        }
    }

    /// Process attribute data (value content)
    fn on_attrib_data_impl(&mut self, start: usize, end: usize) {
        if let Some(ref mut attr) = self.current_attr {
            if attr.value_start.is_none() {
                attr.value_start = Some(start);
            }
            attr.value_end = Some(end);
        }
        if let Some(ref mut dir) = self.current_dir {
            if dir.value_start.is_none() {
                dir.value_start = Some(start);
            }
            dir.value_end = Some(end);
        }
    }

    /// Process attribute end
    fn on_attrib_end_impl(&mut self, quote: QuoteType, end: usize) {
        // Handle regular attribute
        if let Some(attr) = self.current_attr.take() {
            self.finish_attribute(attr, quote, end);
        }

        // Handle directive
        if let Some(dir) = self.current_dir.take() {
            self.finish_directive(dir, quote, end);
        }
    }

    /// Finish building an attribute node
    fn finish_attribute(&mut self, attr: CurrentAttribute<'a>, _quote: QuoteType, end: usize) {
        let loc = self.create_loc(attr.name_start, end);
        let name_loc = self.create_loc(attr.name_start, attr.name_end);

        let mut attr_node = AttributeNode::new(attr.name.clone(), loc);
        attr_node.name_loc = name_loc;

        // Add value if present
        if let (Some(v_start), Some(v_end)) = (attr.value_start, attr.value_end) {
            let value_content = self.get_source(v_start, v_end);
            let value_loc = self.create_loc(v_start, v_end);
            attr_node.value = Some(TextNode::new(value_content, value_loc));
        }

        if let Some(ref mut current) = self.current_element {
            let boxed = Box::new_in(attr_node, self.allocator);
            current.props.push(PropNode::Attribute(boxed));
        }
    }

    /// Finish building a directive node
    fn finish_directive(&mut self, dir: CurrentDirective<'a>, _quote: QuoteType, end: usize) {
        let loc = self.create_loc(dir.name_start, end);

        let mut dir_node = DirectiveNode::new(self.allocator, dir.name.clone(), loc);
        dir_node.raw_name = Some(dir.raw_name);

        // Add argument if present
        if let Some((arg_content, arg_start, arg_end, is_dynamic)) = dir.arg {
            let arg_loc = self.create_loc(arg_start, arg_end);
            let mut arg_expr = SimpleExpressionNode::new(arg_content, !is_dynamic, arg_loc);
            if is_dynamic {
                arg_expr.const_type = ConstantType::NotConstant;
            }
            let arg_boxed = Box::new_in(arg_expr, self.allocator);
            dir_node.arg = Some(ExpressionNode::Simple(arg_boxed));
        }

        // Add modifiers
        for (mod_content, mod_start, mod_end) in dir.modifiers {
            let mod_loc = self.create_loc(mod_start, mod_end);
            let mod_expr = SimpleExpressionNode::new(mod_content, true, mod_loc);
            dir_node.modifiers.push(mod_expr);
        }

        // Add expression if present
        if let (Some(v_start), Some(v_end)) = (dir.value_start, dir.value_end) {
            let exp_content = self.get_source(v_start, v_end);
            let exp_loc = self.create_loc(v_start, v_end);
            let exp_node = SimpleExpressionNode::new(exp_content, false, exp_loc);
            let exp_boxed = Box::new_in(exp_node, self.allocator);
            dir_node.exp = Some(ExpressionNode::Simple(exp_boxed));
        }

        if let Some(ref mut current) = self.current_element {
            let boxed = Box::new_in(dir_node, self.allocator);
            current.props.push(PropNode::Directive(boxed));
        }
    }

    /// Process comment
    fn on_comment_impl(&mut self, start: usize, end: usize) {
        if !self.options.comments {
            return;
        }

        let content = self.get_source(start, end);
        let loc = self.create_loc(start - 4, end + 3); // Include <!-- and -->

        let comment = CommentNode::new(content, loc);
        let boxed = Box::new_in(comment, self.allocator);
        self.add_child(TemplateChildNode::Comment(boxed));
    }

    /// Handle error
    fn on_error_impl(&mut self, code: ErrorCode, index: usize) {
        let loc = self.create_loc(index, index + 1);
        self.errors.push(CompilerError::new(code, Some(loc)));
    }
}

/// Parse directive name from raw attribute name
fn parse_directive_name(raw: &str) -> &str {
    // Handle shorthand
    match raw.chars().next() {
        Some(':') => return "bind",
        Some('@') => return "on",
        Some('#') => return "slot",
        Some('.') => return "bind", // .prop shorthand
        _ => {}
    }

    // Handle v-directive
    if let Some(rest) = raw.strip_prefix("v-") {
        // Find end of directive name (before : or .)
        let end = rest.find([':', '.']).unwrap_or(rest.len());
        return &rest[..end];
    }

    raw
}

/// Wrapper struct for implementing Callbacks
struct ParserCallbacks<'a, 'p> {
    parser: &'p mut Parser<'a>,
}

impl<'a, 'p> Callbacks for ParserCallbacks<'a, 'p> {
    fn on_text(&mut self, start: usize, end: usize) {
        self.parser.on_text_impl(start, end);
    }

    fn on_text_entity(&mut self, char: char, start: usize, end: usize) {
        // For now, treat entities as regular text
        let _ = (char, start, end);
    }

    fn on_interpolation(&mut self, start: usize, end: usize) {
        self.parser.on_interpolation_impl(start, end);
    }

    fn on_open_tag_name(&mut self, start: usize, end: usize) {
        self.parser.on_open_tag_name_impl(start, end);
    }

    fn on_open_tag_end(&mut self, end: usize) {
        self.parser.on_open_tag_end_impl(end);
    }

    fn on_self_closing_tag(&mut self, end: usize) {
        self.parser.on_self_closing_tag_impl(end);
        self.parser.on_open_tag_end_impl(end);
    }

    fn on_close_tag(&mut self, start: usize, end: usize) {
        self.parser.on_close_tag_impl(start, end);
    }

    fn on_attrib_data(&mut self, start: usize, end: usize) {
        self.parser.on_attrib_data_impl(start, end);
    }

    fn on_attrib_entity(&mut self, _char: char, _start: usize, _end: usize) {
        // For now, ignore entity in attributes
    }

    fn on_attrib_end(&mut self, quote: QuoteType, end: usize) {
        self.parser.on_attrib_end_impl(quote, end);
    }

    fn on_attrib_name(&mut self, start: usize, end: usize) {
        self.parser.on_attrib_name_impl(start, end);
    }

    fn on_attrib_name_end(&mut self, _end: usize) {
        // No-op for now
    }

    fn on_dir_name(&mut self, start: usize, end: usize) {
        self.parser.on_dir_name_impl(start, end);
    }

    fn on_dir_arg(&mut self, start: usize, end: usize) {
        self.parser.on_dir_arg_impl(start, end);
    }

    fn on_dir_modifier(&mut self, start: usize, end: usize) {
        self.parser.on_dir_modifier_impl(start, end);
    }

    fn on_comment(&mut self, start: usize, end: usize) {
        self.parser.on_comment_impl(start, end);
    }

    fn on_cdata(&mut self, _start: usize, _end: usize) {
        // CDATA handling
    }

    fn on_processing_instruction(&mut self, _start: usize, _end: usize) {
        // Processing instruction handling
    }

    fn on_end(&mut self) {
        // End of input
    }

    fn on_error(&mut self, code: ErrorCode, index: usize) {
        self.parser.on_error_impl(code, index);
    }

    fn is_in_v_pre(&self) -> bool {
        self.parser.in_v_pre
    }
}

/// Condense whitespace in children
fn condense_whitespace<'a>(children: &mut Vec<'a, TemplateChildNode<'a>>) {
    let mut i = 0;
    while i < children.len() {
        // Determine what action to take for whitespace-only text nodes
        let action = if let TemplateChildNode::Text(ref text) = children[i] {
            let content = text.content.as_str();
            if content.chars().all(char::is_whitespace) {
                let prev_is_text = i > 0
                    && matches!(
                        children[i - 1],
                        TemplateChildNode::Text(_) | TemplateChildNode::Interpolation(_)
                    );
                let next_is_text = i + 1 < children.len()
                    && matches!(
                        children[i + 1],
                        TemplateChildNode::Text(_) | TemplateChildNode::Interpolation(_)
                    );

                if !prev_is_text && !next_is_text {
                    // Between non-text nodes (e.g. two elements):
                    // If whitespace contains a newline, remove it entirely
                    // (this handles indentation between block-level elements).
                    // If it's just spaces (no newline), condense to single space
                    // to preserve inline spacing (vuejs/core #7542).
                    let has_newline = content.contains('\n');
                    if has_newline {
                        WhitespaceAction::Remove
                    } else {
                        WhitespaceAction::Condense
                    }
                } else {
                    WhitespaceAction::Keep
                }
            } else {
                WhitespaceAction::Keep
            }
        } else {
            WhitespaceAction::Keep
        };

        match action {
            WhitespaceAction::Remove => {
                children.remove(i);
                continue;
            }
            WhitespaceAction::Condense => {
                // Condense whitespace between two elements to a single space
                if let TemplateChildNode::Text(ref mut text) = children[i] {
                    text.content = " ".into();
                }
            }
            WhitespaceAction::Keep => {}
        }

        // Recurse into elements
        if let TemplateChildNode::Element(ref mut el) = children[i] {
            condense_whitespace(&mut el.children);
        }

        i += 1;
    }
}

/// Action to take for a whitespace-only text node during condensing
enum WhitespaceAction {
    /// Keep the node as-is
    Keep,
    /// Remove the node entirely
    Remove,
    /// Condense to a single space
    Condense,
}

/// Parse a Vue template
pub fn parse<'a>(allocator: &'a Bump, source: &'a str) -> (RootNode<'a>, Vec<'a, CompilerError>) {
    Parser::new(allocator, source).parse()
}

/// Parse a Vue template with options
pub fn parse_with_options<'a>(
    allocator: &'a Bump,
    source: &'a str,
    options: ParserOptions,
) -> (RootNode<'a>, Vec<'a, CompilerError>) {
    Parser::with_options(allocator, source, options).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_element() {
        let allocator = Bump::new();
        let (root, errors) = parse(&allocator, "<div></div>");

        assert!(errors.is_empty());
        assert_eq!(root.children.len(), 1);

        if let TemplateChildNode::Element(el) = &root.children[0] {
            assert_eq!(el.tag.as_str(), "div");
            assert!(!el.is_self_closing);
        } else {
            panic!("Expected element node");
        }
    }

    #[test]
    fn test_parse_text() {
        let allocator = Bump::new();
        let (root, errors) = parse(&allocator, "hello");

        assert!(errors.is_empty());
        assert_eq!(root.children.len(), 1);

        if let TemplateChildNode::Text(text) = &root.children[0] {
            assert_eq!(text.content.as_str(), "hello");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn test_parse_interpolation() {
        let allocator = Bump::new();
        let (root, errors) = parse(&allocator, "{{ msg }}");

        assert!(errors.is_empty());
        assert_eq!(root.children.len(), 1);

        if let TemplateChildNode::Interpolation(interp) = &root.children[0] {
            if let ExpressionNode::Simple(expr) = &interp.content {
                assert_eq!(expr.content.as_str(), "msg");
            } else {
                panic!("Expected simple expression");
            }
        } else {
            panic!("Expected interpolation node");
        }
    }

    #[test]
    fn test_parse_directive() {
        let allocator = Bump::new();
        let (root, errors) = parse(&allocator, r#"<div v-if="ok"></div>"#);

        assert!(errors.is_empty());
        assert_eq!(root.children.len(), 1);

        if let TemplateChildNode::Element(el) = &root.children[0] {
            assert_eq!(el.props.len(), 1);
            if let PropNode::Directive(dir) = &el.props[0] {
                assert_eq!(dir.name.as_str(), "if");
                if let Some(ExpressionNode::Simple(exp)) = &dir.exp {
                    assert_eq!(exp.content.as_str(), "ok");
                }
            } else {
                panic!("Expected directive");
            }
        } else {
            panic!("Expected element node");
        }
    }

    #[test]
    fn test_parse_shorthand_bind() {
        let allocator = Bump::new();
        let (root, errors) = parse(&allocator, r#"<div :class="cls"></div>"#);

        assert!(errors.is_empty());

        if let TemplateChildNode::Element(el) = &root.children[0] {
            if let PropNode::Directive(dir) = &el.props[0] {
                assert_eq!(dir.name.as_str(), "bind");
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    assert_eq!(arg.content.as_str(), "class");
                }
            } else {
                panic!("Expected directive");
            }
        }
    }

    #[test]
    fn test_parse_shorthand_on() {
        let allocator = Bump::new();
        let (root, errors) = parse(&allocator, r#"<button @click="handler"></button>"#);

        assert!(errors.is_empty());

        if let TemplateChildNode::Element(el) = &root.children[0] {
            if let PropNode::Directive(dir) = &el.props[0] {
                assert_eq!(dir.name.as_str(), "on");
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    assert_eq!(arg.content.as_str(), "click");
                }
            } else {
                panic!("Expected directive");
            }
        }
    }

    #[test]
    fn test_parse_nested_elements() {
        let allocator = Bump::new();
        let (root, errors) = parse(&allocator, "<div><span>text</span></div>");

        assert!(errors.is_empty());
        assert_eq!(root.children.len(), 1);

        if let TemplateChildNode::Element(el) = &root.children[0] {
            assert_eq!(el.tag.as_str(), "div");
            assert_eq!(el.children.len(), 1);

            if let TemplateChildNode::Element(span) = &el.children[0] {
                assert_eq!(span.tag.as_str(), "span");
            }
        }
    }

    #[test]
    fn test_parse_self_closing() {
        let allocator = Bump::new();
        let (root, errors) = parse(&allocator, "<input />");

        assert!(errors.is_empty());
        assert_eq!(root.children.len(), 1);

        if let TemplateChildNode::Element(el) = &root.children[0] {
            assert_eq!(el.tag.as_str(), "input");
            assert!(el.is_self_closing);
        }
    }
}
