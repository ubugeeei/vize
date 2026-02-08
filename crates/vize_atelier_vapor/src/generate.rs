//! Vapor code generation.
//!
//! Generates JavaScript code from Vapor IR.

use std::fmt::Write;

use crate::ir::*;
use vize_atelier_core::ExpressionNode;
use vize_carton::FxHashMap;

/// Vapor code generation result
pub struct VaporGenerateResult {
    /// Generated code
    pub code: std::string::String,
    /// Static templates
    pub templates: std::vec::Vec<vize_carton::String>,
}

/// Generate Vapor code from IR
pub fn generate_vapor(ir: &RootIRNode<'_>) -> VaporGenerateResult {
    let mut ctx = GenerateContext::new(&ir.element_template_map);

    // Template helper is always used if we have templates
    if !ir.templates.is_empty() {
        ctx.use_helper("template");
    }

    // Generate template declarations (to separate string, we'll prepend imports later)
    let mut template_code = String::new();
    for (i, template) in ir.templates.iter().enumerate() {
        writeln!(
            template_code,
            "const t{} = _template(\"{}\", true)",
            i,
            escape_template(template)
        )
        .ok();
    }

    // First pass: collect delegate events
    collect_delegate_events(&mut ctx, &ir.block);

    // Generate component function body first to collect used helpers
    ctx.push_line("export function render(_ctx) {");
    ctx.indent();

    // Generate block content (includes template instantiation, text nodes, operations, effects, return)
    generate_block(&mut ctx, &ir.block, &ir.element_template_map);

    ctx.deindent();
    ctx.push_line("}");

    // Generate delegate events code (after templates, before function)
    let mut delegate_code = String::new();
    if !ctx.delegate_events.is_empty() {
        ctx.use_helper("delegateEvents");
        let mut events: Vec<_> = ctx.delegate_events.iter().collect();
        events.sort();
        for event in events {
            writeln!(delegate_code, "_delegateEvents(\"{}\")", event).ok();
        }
    }

    // Now generate imports at the front with only used helpers
    let imports = generate_imports(&ctx);

    // Combine: imports + templates + delegate events + function body
    let mut final_code = imports;
    if !template_code.is_empty() {
        final_code.push_str(&template_code);
    }
    if !delegate_code.is_empty() {
        final_code.push_str(&delegate_code);
    }
    // Add blank line before function
    if !final_code.is_empty() {
        final_code.push('\n');
    }
    final_code.push_str(&ctx.code);

    VaporGenerateResult {
        code: final_code,
        templates: ir.templates.iter().cloned().collect(),
    }
}

/// Collect delegate events from block
fn collect_delegate_events(ctx: &mut GenerateContext, block: &BlockIRNode<'_>) {
    for op in block.operation.iter() {
        if let OperationNode::SetEvent(set_event) = op {
            if set_event.delegate {
                ctx.add_delegate_event(&set_event.key.content);
            }
        }
    }
}

/// Generate context
struct GenerateContext<'a> {
    code: String,
    indent_level: u32,
    #[allow(dead_code)]
    element_template_map: &'a FxHashMap<usize, usize>,
    temp_count: usize,
    /// Used helpers for import generation
    used_helpers: std::collections::HashSet<&'static str>,
    /// Events that need delegation (event names)
    delegate_events: std::collections::HashSet<std::string::String>,
    /// Text node references (element_id -> text_node_var)
    text_nodes: FxHashMap<usize, std::string::String>,
}

impl<'a> GenerateContext<'a> {
    fn new(element_template_map: &'a FxHashMap<usize, usize>) -> Self {
        Self {
            code: String::with_capacity(4096),
            indent_level: 0,
            element_template_map,
            temp_count: 0,
            used_helpers: std::collections::HashSet::new(),
            delegate_events: std::collections::HashSet::new(),
            text_nodes: FxHashMap::default(),
        }
    }

    fn add_delegate_event(&mut self, event_name: &str) {
        self.delegate_events.insert(event_name.to_string());
    }

    fn next_text_node(&mut self, element_id: usize) -> std::string::String {
        // Use element ID for text node variable name (x2 matches n2)
        let mut var_name = std::string::String::with_capacity(8);
        var_name.push('x');
        var_name.push_str(&element_id.to_string());
        self.text_nodes.insert(element_id, var_name.clone());
        var_name
    }

    fn use_helper(&mut self, name: &'static str) {
        self.used_helpers.insert(name);
    }

    fn push(&mut self, s: &str) {
        self.code.push_str(s);
    }

    fn push_line(&mut self, s: &str) {
        self.push_indent();
        self.code.push_str(s);
        self.code.push('\n');
    }

    fn push_indent(&mut self) {
        for _ in 0..self.indent_level {
            self.code.push_str("  ");
        }
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn deindent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    fn next_temp(&mut self) -> String {
        let name = format!("_t{}", self.temp_count);
        self.temp_count += 1;
        name
    }
}

/// Generate imports based on used helpers
fn generate_imports(ctx: &GenerateContext) -> String {
    if ctx.used_helpers.is_empty() {
        return String::new();
    }

    // Define priority order for helpers (lower = earlier in import)
    fn helper_priority(name: &str) -> u32 {
        match name {
            "resolveComponent" => 1,
            "createComponentWithFallback" => 2,
            "child" => 10,
            "next" => 11,
            "txt" => 20,
            "toDisplayString" => 21,
            "setText" => 22,
            "setClass" => 30,
            "setProp" => 31,
            "setStyle" => 32,
            "setAttr" => 33,
            "createInvoker" => 40,
            "delegateEvents" => 41,
            "setInsertionState" => 78,
            "renderEffect" => 79,
            "createIf" => 80,
            "createFor" => 81,
            "template" => 100,
            _ => 50,
        }
    }

    let mut helpers: Vec<_> = ctx.used_helpers.iter().copied().collect();
    helpers.sort_by_key(|h| helper_priority(h));

    let imports = helpers
        .iter()
        .map(|h| format!("{} as _{}", h, h))
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {{ {} }} from 'vue';\n", imports)
}

/// Generate block
fn generate_block(
    ctx: &mut GenerateContext,
    block: &BlockIRNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    // Instantiate templates for elements in this block's returns
    for element_id in block.returns.iter() {
        if let Some(&template_index) = element_template_map.get(element_id) {
            let mut line = std::string::String::with_capacity(32);
            line.push_str("const n");
            line.push_str(&element_id.to_string());
            line.push_str(" = t");
            line.push_str(&template_index.to_string());
            line.push_str("()");
            ctx.push_line(&line);
        }
    }

    // Generate text node references for effects in this block
    for effect in block.effect.iter() {
        for op in effect.operations.iter() {
            if let OperationNode::SetText(set_text) = op {
                ctx.use_helper("txt");
                let var_name = ctx.next_text_node(set_text.element);
                let mut line = std::string::String::with_capacity(32);
                line.push_str("const ");
                line.push_str(&var_name);
                line.push_str(" = _txt(n");
                line.push_str(&set_text.element.to_string());
                line.push(')');
                ctx.push_line(&line);
            }
        }
    }

    // Generate operations
    for op in block.operation.iter() {
        generate_operation(ctx, op, element_template_map);
    }

    // Generate effects
    for effect in block.effect.iter() {
        generate_effect(ctx, effect, element_template_map);
    }

    // Generate return
    if !block.returns.is_empty() {
        let returns = block
            .returns
            .iter()
            .map(|r| ["n", &r.to_string()].concat())
            .collect::<Vec<_>>()
            .join(", ");

        if block.returns.len() == 1 {
            ctx.push_line(&["return ", &returns].concat());
        } else {
            ctx.push_line(&["return [", &returns, "]"].concat());
        }
    }
}

/// Generate operation
fn generate_operation(
    ctx: &mut GenerateContext,
    op: &OperationNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    match op {
        OperationNode::SetProp(set_prop) => {
            generate_set_prop(ctx, set_prop);
        }
        OperationNode::SetDynamicProps(set_props) => {
            generate_set_dynamic_props(ctx, set_props);
        }
        OperationNode::SetText(set_text) => {
            generate_set_text(ctx, set_text);
        }
        OperationNode::SetEvent(set_event) => {
            generate_set_event(ctx, set_event);
        }
        OperationNode::SetHtml(set_html) => {
            generate_set_html(ctx, set_html);
        }
        OperationNode::SetTemplateRef(set_ref) => {
            generate_set_template_ref(ctx, set_ref);
        }
        OperationNode::InsertNode(insert) => {
            generate_insert_node(ctx, insert);
        }
        OperationNode::PrependNode(prepend) => {
            generate_prepend_node(ctx, prepend);
        }
        OperationNode::Directive(directive) => {
            generate_directive(ctx, directive);
        }
        OperationNode::If(if_node) => {
            generate_if(ctx, if_node, element_template_map);
        }
        OperationNode::For(for_node) => {
            generate_for(ctx, for_node, element_template_map);
        }
        OperationNode::CreateComponent(component) => {
            generate_create_component(ctx, component);
        }
        OperationNode::SlotOutlet(slot) => {
            generate_slot_outlet(ctx, slot);
        }
        OperationNode::GetTextChild(get_text) => {
            generate_get_text_child(ctx, get_text);
        }
    }
}

/// Generate effect
fn generate_effect(
    ctx: &mut GenerateContext,
    effect: &IREffect<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    ctx.use_helper("renderEffect");

    // If only one operation, use single-line format
    if effect.operations.len() == 1 {
        let op = &effect.operations[0];
        let op_code = generate_operation_inline(ctx, op);
        ctx.push_line(&format!("_renderEffect(() => {})", op_code));
    } else {
        ctx.push_line("_renderEffect(() => {");
        ctx.indent();

        for op in effect.operations.iter() {
            generate_operation(ctx, op, element_template_map);
        }

        ctx.deindent();
        ctx.push_line("})");
    }
}

/// Generate operation inline (returns code string)
fn generate_operation_inline(ctx: &mut GenerateContext, op: &OperationNode<'_>) -> String {
    match op {
        OperationNode::SetProp(set_prop) => {
            let element = format!("n{}", set_prop.element);
            let key = &set_prop.prop.key.content;
            let is_svg = is_svg_tag(set_prop.tag.as_str());
            let value = if let Some(first) = set_prop.prop.values.first() {
                if first.is_static {
                    format!("\"{}\"", first.content)
                } else {
                    format!("_ctx.{}", first.content)
                }
            } else {
                String::from("undefined")
            };

            if key.as_str() == "class" {
                if is_svg {
                    ctx.use_helper("setAttr");
                    format!("_setAttr({}, \"class\", {})", element, value)
                } else {
                    ctx.use_helper("setClass");
                    format!("_setClass({}, {})", element, value)
                }
            } else if key.as_str() == "style" {
                if is_svg {
                    ctx.use_helper("setAttr");
                    format!("_setAttr({}, \"style\", {})", element, value)
                } else {
                    ctx.use_helper("setStyle");
                    format!("_setStyle({}, {})", element, value)
                }
            } else {
                ctx.use_helper("setProp");
                format!("_setProp({}, \"{}\", {})", element, key, value)
            }
        }
        OperationNode::SetText(set_text) => {
            ctx.use_helper("setText");
            let text_ref = if let Some(text_var) = ctx.text_nodes.get(&set_text.element) {
                text_var.clone()
            } else {
                format!("n{}", set_text.element)
            };

            let values: Vec<String> = set_text
                .values
                .iter()
                .map(|v| {
                    ctx.use_helper("toDisplayString");
                    if v.is_static {
                        format!("\"{}\"", v.content)
                    } else {
                        format!("_toDisplayString(_ctx.{})", v.content)
                    }
                })
                .collect();

            if values.len() == 1 {
                format!("_setText({}, {})", text_ref, values[0])
            } else {
                format!("_setText({}, {})", text_ref, values.join(" + "))
            }
        }
        _ => String::from("/* unsupported */"),
    }
}

/// Generate SetProp
fn generate_set_prop(ctx: &mut GenerateContext, set_prop: &SetPropIRNode<'_>) {
    let element = format!("n{}", set_prop.element);
    let key = &set_prop.prop.key.content;
    let is_svg = is_svg_tag(set_prop.tag.as_str());

    let value = if let Some(first) = set_prop.prop.values.first() {
        if first.is_static {
            format!("\"{}\"", first.content)
        } else {
            format!("_ctx.{}", first.content)
        }
    } else {
        String::from("undefined")
    };

    if key.as_str() == "class" {
        if is_svg {
            ctx.use_helper("setAttr");
            ctx.push_line(&format!("_setAttr({}, \"class\", {})", element, value));
        } else {
            ctx.use_helper("setClass");
            ctx.push_line(&format!("_setClass({}, {})", element, value));
        }
    } else if key.as_str() == "style" {
        if is_svg {
            ctx.use_helper("setAttr");
            ctx.push_line(&format!("_setAttr({}, \"style\", {})", element, value));
        } else {
            ctx.use_helper("setStyle");
            ctx.push_line(&format!("_setStyle({}, {})", element, value));
        }
    } else {
        ctx.use_helper("setProp");
        ctx.push_line(&format!("_setProp({}, \"{}\", {})", element, key, value));
    }
}

/// Generate SetDynamicProps
fn generate_set_dynamic_props(ctx: &mut GenerateContext, set_props: &SetDynamicPropsIRNode<'_>) {
    let element = format!("n{}", set_props.element);

    for prop in set_props.props.iter() {
        let expr = if prop.is_static {
            format!("\"{}\"", prop.content)
        } else {
            prop.content.to_string()
        };
        ctx.push_line(&format!("Object.assign({}, {})", element, expr));
    }
}

/// Generate SetText
fn generate_set_text(ctx: &mut GenerateContext, set_text: &SetTextIRNode<'_>) {
    ctx.use_helper("setText");

    // Use text node reference if available, otherwise use element directly
    let text_ref = if let Some(text_var) = ctx.text_nodes.get(&set_text.element) {
        text_var.clone()
    } else {
        format!("n{}", set_text.element)
    };

    let values: Vec<String> = set_text
        .values
        .iter()
        .map(|v| {
            ctx.use_helper("toDisplayString");
            if v.is_static {
                format!("\"{}\"", v.content)
            } else {
                format!("_toDisplayString(_ctx.{})", v.content)
            }
        })
        .collect();

    if values.len() == 1 {
        ctx.push_line(&format!("_setText({}, {})", text_ref, values[0]));
    } else {
        ctx.push_line(&format!("_setText({}, {})", text_ref, values.join(" + ")));
    }
}

/// Generate SetEvent
fn generate_set_event(ctx: &mut GenerateContext, set_event: &SetEventIRNode<'_>) {
    ctx.use_helper("createInvoker");

    let element = format!("n{}", set_event.element);
    let event_name = &set_event.key.content;

    let handler = if let Some(ref value) = set_event.value {
        value.content.to_string()
    } else {
        String::from("() => {}")
    };

    // Determine handler format based on content
    let invoker_body = if handler.contains("$event") {
        // Handler uses $event - pass it as parameter
        format!("$event => (_ctx.{})", handler)
    } else if handler.contains("?.") {
        // Optional call expression like foo?.() or foo?.bar() - cache it
        format!("(...args) => (_ctx.{})", handler)
    } else if is_inline_statement(&handler) {
        // Inline statement like count++ or foo = bar
        format!("() => (_ctx.{})", handler)
    } else if handler.contains("(") {
        // Handler is a call expression like handler()
        format!("e => _ctx.{}(e)", handler)
    } else {
        // Handler is a method reference like handler
        format!("e => _ctx.{}(e)", handler)
    };

    ctx.push_line(&format!(
        "{}.$evt{} = _createInvoker({})",
        element, event_name, invoker_body
    ));
}

/// Check if handler is an inline statement (not a function reference)
fn is_inline_statement(handler: &str) -> bool {
    // Assignment or increment/decrement operators
    handler.contains("++")
        || handler.contains("--")
        || handler.contains("+=")
        || handler.contains("-=")
        || handler.contains("=")
}

/// Generate SetHtml
fn generate_set_html(ctx: &mut GenerateContext, set_html: &SetHtmlIRNode<'_>) {
    let element = format!("n{}", set_html.element);

    let value = if set_html.value.is_static {
        format!("\"{}\"", set_html.value.content)
    } else {
        set_html.value.content.to_string()
    };

    ctx.push_line(&format!("{}.innerHTML = {}", element, value));
}

/// Generate SetTemplateRef
fn generate_set_template_ref(ctx: &mut GenerateContext, set_ref: &SetTemplateRefIRNode<'_>) {
    let element = format!("n{}", set_ref.element);

    let value = if set_ref.value.is_static {
        format!("\"{}\"", set_ref.value.content)
    } else {
        set_ref.value.content.to_string()
    };

    ctx.push_line(&format!("_setRef({}, {})", element, value));
}

/// Generate InsertNode
fn generate_insert_node(ctx: &mut GenerateContext, insert: &InsertNodeIRNode) {
    let parent = format!("n{}", insert.parent);
    let elements = insert
        .elements
        .iter()
        .map(|e| format!("n{}", e))
        .collect::<Vec<_>>()
        .join(", ");

    if let Some(anchor) = insert.anchor {
        ctx.push_line(&format!("_insert({}, [{}], n{})", parent, elements, anchor));
    } else {
        ctx.push_line(&format!("_insert({}, [{}])", parent, elements));
    }
}

/// Generate PrependNode
fn generate_prepend_node(ctx: &mut GenerateContext, prepend: &PrependNodeIRNode) {
    let parent = format!("n{}", prepend.parent);
    let elements = prepend
        .elements
        .iter()
        .map(|e| format!("n{}", e))
        .collect::<Vec<_>>()
        .join(", ");

    ctx.push_line(&format!("_prepend({}, [{}])", parent, elements));
}

/// Generate Directive
fn generate_directive(ctx: &mut GenerateContext, directive: &DirectiveIRNode<'_>) {
    let element = format!("n{}", directive.element);
    let name = &directive.name;

    let arg = if let Some(ref arg) = directive.dir.arg {
        match arg {
            ExpressionNode::Simple(exp) => {
                if exp.is_static {
                    format!("\"{}\"", exp.content)
                } else {
                    exp.content.to_string()
                }
            }
            _ => String::from("undefined"),
        }
    } else {
        String::from("undefined")
    };

    let value = if let Some(ref exp) = directive.dir.exp {
        match exp {
            ExpressionNode::Simple(e) => {
                if e.is_static {
                    format!("\"{}\"", e.content)
                } else {
                    e.content.to_string()
                }
            }
            _ => String::from("undefined"),
        }
    } else {
        String::from("undefined")
    };

    ctx.push_line(&format!(
        "_withDirectives({}, [[_{}, {}, {}]])",
        element, name, value, arg
    ));
}

/// Generate If
fn generate_if(
    ctx: &mut GenerateContext,
    if_node: &IfIRNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    generate_if_inner(ctx, if_node, element_template_map);
}

/// Generate If (inner - for top-level if nodes)
fn generate_if_inner(
    ctx: &mut GenerateContext,
    if_node: &IfIRNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    ctx.use_helper("createIf");

    let condition = if if_node.condition.is_static {
        ["\"", if_node.condition.content.as_str(), "\""].concat()
    } else {
        ["(_ctx.", if_node.condition.content.as_str(), ")"].concat()
    };

    ctx.push_line(
        &[
            "const n",
            &if_node.id.to_string(),
            " = _createIf(() => ",
            &condition,
            ", () => {",
        ]
        .concat(),
    );

    ctx.indent();
    generate_block(ctx, &if_node.positive, element_template_map);
    ctx.deindent();

    if let Some(ref negative) = if_node.negative {
        match negative {
            NegativeBranch::Block(block) => {
                ctx.push_line("}, () => {");
                ctx.indent();
                generate_block(ctx, block, element_template_map);
                ctx.deindent();
                ctx.push_line("})");
            }
            NegativeBranch::If(nested_if) => {
                // v-else-if: inline format without block wrapper
                ctx.push_indent();
                ctx.push("}, () => ");
                generate_nested_if(ctx, nested_if, element_template_map);
                ctx.push(")\n");
            }
        }
    } else {
        ctx.push_line("})");
    }
}

/// Generate nested if (for v-else-if chains - starts inline without leading indent)
fn generate_nested_if(
    ctx: &mut GenerateContext,
    if_node: &IfIRNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    ctx.use_helper("createIf");

    let condition = if if_node.condition.is_static {
        ["\"", if_node.condition.content.as_str(), "\""].concat()
    } else {
        ["(_ctx.", if_node.condition.content.as_str(), ")"].concat()
    };

    // Start inline - no leading indent or newline
    ctx.push(&["_createIf(() => ", &condition, ", () => {\n"].concat());

    ctx.indent();
    generate_block(ctx, &if_node.positive, element_template_map);
    ctx.deindent();

    if let Some(ref negative) = if_node.negative {
        match negative {
            NegativeBranch::Block(block) => {
                ctx.push_line("}, () => {");
                ctx.indent();
                generate_block(ctx, block, element_template_map);
                ctx.deindent();
                ctx.push_indent();
                ctx.push("})");
            }
            NegativeBranch::If(nested_if) => {
                ctx.push_indent();
                ctx.push("}, () => ");
                generate_nested_if(ctx, nested_if, element_template_map);
                ctx.push(")");
            }
        }
    } else {
        ctx.push_indent();
        ctx.push("})");
    }
}

/// Generate For
fn generate_for(
    ctx: &mut GenerateContext,
    for_node: &ForIRNode<'_>,
    element_template_map: &FxHashMap<usize, usize>,
) {
    ctx.use_helper("createFor");

    let source = if for_node.source.is_static {
        ["\"", for_node.source.content.as_str(), "\""].concat()
    } else {
        ["(_ctx.", for_node.source.content.as_str(), " || [])"].concat()
    };

    let value_name = for_node
        .value
        .as_ref()
        .map(|v| v.content.as_str())
        .unwrap_or("_item");
    let key_name = for_node.key.as_ref().map(|k| k.content.as_str());
    let index_name = for_node.index.as_ref().map(|i| i.content.as_str());

    let params = match (key_name, index_name) {
        (Some(k), Some(i)) => [value_name, ", ", k, ", ", i].concat(),
        (Some(k), None) => [value_name, ", ", k].concat(),
        _ => value_name.to_string(),
    };

    ctx.push_line(&["_createFor(() => ", &source, ", (", &params, ") => {"].concat());
    ctx.indent();
    generate_block(ctx, &for_node.render, element_template_map);
    ctx.deindent();
    ctx.push_line("})");
}

/// Generate CreateComponent
fn generate_create_component(ctx: &mut GenerateContext, component: &CreateComponentIRNode<'_>) {
    ctx.use_helper("resolveComponent");
    ctx.use_helper("createComponentWithFallback");

    let tag = &component.tag;
    let component_var = ["_component_", tag.as_str()].concat();

    // Resolve component
    ctx.push_line(
        &[
            "const ",
            &component_var,
            " = _resolveComponent(\"",
            tag.as_str(),
            "\")",
        ]
        .concat(),
    );

    // Props object
    let props = if component.props.is_empty() {
        "null".to_string()
    } else {
        let prop_strs: Vec<String> = component
            .props
            .iter()
            .map(|p| {
                let key = &p.key.content;
                let is_event = key.as_str().starts_with("on") && key.len() > 2;

                let value = if let Some(first) = p.values.first() {
                    if first.is_static {
                        ["() => (\"", first.content.as_str(), "\")"].concat()
                    } else if is_event {
                        // Event handlers: () => _ctx.handler
                        ["() => _ctx.", first.content.as_str()].concat()
                    } else {
                        // Regular props: () => (_ctx.value)
                        ["() => (_ctx.", first.content.as_str(), ")"].concat()
                    }
                } else {
                    "undefined".to_string()
                };
                [key.as_str(), ": ", &value].concat()
            })
            .collect();
        ["{ ", &prop_strs.join(", "), " }"].concat()
    };

    // Generate component creation
    ctx.push_line(
        &[
            "const n",
            &component.id.to_string(),
            " = _createComponentWithFallback(",
            &component_var,
            ", ",
            &props,
            ", null, true)",
        ]
        .concat(),
    );
}

/// Generate SlotOutlet
fn generate_slot_outlet(ctx: &mut GenerateContext, slot: &SlotOutletIRNode<'_>) {
    let name = ctx.next_temp();
    let slot_name = if slot.name.is_static {
        format!("\"{}\"", slot.name.content)
    } else {
        slot.name.content.to_string()
    };

    ctx.push_line(&format!(
        "const {} = _renderSlot($slots, {})",
        name, slot_name
    ));
}

/// Generate GetTextChild
fn generate_get_text_child(ctx: &mut GenerateContext, get_text: &GetTextChildIRNode) {
    let parent = format!("n{}", get_text.parent);
    let child = ctx.next_temp();

    ctx.push_line(&format!("const {} = {}.firstChild", child, parent));
}

/// Escape template string for JavaScript
fn escape_template(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Check if a tag is an SVG element
fn is_svg_tag(tag: &str) -> bool {
    matches!(
        tag,
        "svg"
            | "circle"
            | "ellipse"
            | "line"
            | "path"
            | "polygon"
            | "polyline"
            | "rect"
            | "g"
            | "defs"
            | "symbol"
            | "use"
            | "text"
            | "tspan"
            | "image"
            | "clipPath"
            | "mask"
            | "filter"
            | "linearGradient"
            | "radialGradient"
            | "stop"
            | "foreignObject"
            | "animate"
            | "animateMotion"
            | "animateTransform"
            | "set"
            | "desc"
            | "title"
            | "metadata"
            | "marker"
            | "pattern"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::transform_to_ir;
    use vize_atelier_core::parser::parse;
    use vize_carton::Bump;

    #[test]
    fn test_generate_simple() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, "<div>hello</div>");
        let ir = transform_to_ir(&allocator, &root);
        let result = generate_vapor(&ir);

        assert!(!result.code.is_empty());
        assert!(result.code.contains("export function render"));
    }

    #[test]
    fn test_generate_with_event() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, r#"<button @click="handleClick">Click</button>"#);
        let ir = transform_to_ir(&allocator, &root);
        let result = generate_vapor(&ir);

        assert!(result.code.contains("createInvoker"));
        assert!(result.code.contains("click"));
    }

    #[test]
    fn test_escape_template() {
        assert_eq!(escape_template("hello"), "hello");
        assert_eq!(escape_template("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_template("hello\"world"), "hello\\\"world");
    }
}
