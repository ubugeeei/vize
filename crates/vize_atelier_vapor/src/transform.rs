//! Vapor IR transformation.
//!
//! Transforms the template AST into Vapor IR for code generation.

use vize_carton::{Box, Bump, FxHashMap, String, Vec};

use crate::ir::*;
use vize_atelier_core::{
    DirectiveNode, ElementNode, ElementType, ExpressionNode, ForNode, IfNode, InterpolationNode,
    PropNode, RootNode, SimpleExpressionNode, SourceLocation, TemplateChildNode, TextNode,
};

/// Transform AST to Vapor IR
pub fn transform_to_ir<'a>(allocator: &'a Bump, root: &RootNode<'a>) -> RootIRNode<'a> {
    let mut ctx = TransformContext::new(allocator);

    // Create block for root
    let block = transform_children(&mut ctx, &root.children);

    RootIRNode {
        node: RootNode::new(allocator, ""),
        source: String::from(""),
        template: Default::default(),
        template_index_map: Default::default(),
        root_template_indexes: Vec::new_in(allocator),
        component: Vec::new_in(allocator),
        directive: Vec::new_in(allocator),
        block,
        has_template_ref: false,
        has_deferred_v_show: false,
        templates: ctx.templates,
        element_template_map: ctx.element_template_map,
    }
}

/// Transform context
struct TransformContext<'a> {
    allocator: &'a Bump,
    temp_id: usize,
    templates: Vec<'a, String>,
    element_template_map: FxHashMap<usize, usize>,
}

impl<'a> TransformContext<'a> {
    fn new(allocator: &'a Bump) -> Self {
        Self {
            allocator,
            temp_id: 0,
            templates: Vec::new_in(allocator),
            element_template_map: FxHashMap::default(),
        }
    }

    fn next_id(&mut self) -> usize {
        let id = self.temp_id;
        self.temp_id += 1;
        id
    }

    fn add_template(&mut self, element_id: usize, template: String) -> usize {
        let template_index = self.templates.len();
        self.templates.push(template);
        self.element_template_map.insert(element_id, template_index);
        template_index
    }
}

/// Transform children nodes
fn transform_children<'a>(
    ctx: &mut TransformContext<'a>,
    children: &[TemplateChildNode<'a>],
) -> BlockIRNode<'a> {
    let mut block = BlockIRNode::new(ctx.allocator);
    // Note: Don't consume an ID for the block itself - element IDs should start from 0

    for child in children {
        match child {
            TemplateChildNode::Element(el) => {
                transform_element(ctx, el, &mut block);
            }
            TemplateChildNode::Text(text) => {
                transform_text(ctx, text, &mut block);
            }
            TemplateChildNode::Interpolation(interp) => {
                transform_interpolation(ctx, interp, &mut block);
            }
            TemplateChildNode::If(if_node) => {
                transform_if_node(ctx, if_node, &mut block);
            }
            TemplateChildNode::For(for_node) => {
                transform_for_node(ctx, for_node, &mut block);
            }
            TemplateChildNode::Comment(_) => {
                // Comments are ignored in Vapor mode
            }
            _ => {}
        }
    }

    block
}

/// Transform element node
fn transform_element<'a>(
    ctx: &mut TransformContext<'a>,
    el: &ElementNode<'a>,
    block: &mut BlockIRNode<'a>,
) {
    let element_id = ctx.next_id();

    match el.tag_type {
        ElementType::Element => {
            // Generate template string and register it
            let template = generate_element_template(el);
            ctx.add_template(element_id, template);

            // Process props and events
            for prop in el.props.iter() {
                match prop {
                    PropNode::Directive(dir) => {
                        transform_directive(ctx, dir, element_id, el, block);
                    }
                    PropNode::Attribute(_attr) => {
                        // Static attributes are included in the template
                    }
                }
            }

            // Check if we have mixed text and interpolation children
            let has_text_or_interpolation = el.children.iter().any(|c| {
                matches!(
                    c,
                    TemplateChildNode::Text(_) | TemplateChildNode::Interpolation(_)
                )
            });
            let has_interpolation = el
                .children
                .iter()
                .any(|c| matches!(c, TemplateChildNode::Interpolation(_)));

            if has_interpolation && has_text_or_interpolation {
                // Collect all text parts and interpolations together
                transform_text_children(ctx, &el.children, element_id, block);
            }

            // Process other dynamic children
            for child in el.children.iter() {
                match child {
                    TemplateChildNode::Interpolation(_) | TemplateChildNode::Text(_) => {
                        // Already handled above
                    }
                    TemplateChildNode::Element(child_el) => {
                        // Only process dynamic child elements
                        if !is_static_element(child_el) {
                            transform_element(ctx, child_el, block);
                        }
                    }
                    TemplateChildNode::If(if_node) => {
                        transform_if_node(ctx, if_node, block);
                    }
                    TemplateChildNode::For(for_node) => {
                        transform_for_node(ctx, for_node, block);
                    }
                    _ => {}
                }
            }
        }
        ElementType::Component => {
            // Component handling - process props and events
            let mut props = Vec::new_in(ctx.allocator);
            let slots = Vec::new_in(ctx.allocator);

            // Process props (v-bind and v-on directives, and static attributes)
            for prop in el.props.iter() {
                match prop {
                    PropNode::Directive(dir) => {
                        if dir.name.as_str() == "bind" {
                            // v-bind -> prop
                            if let Some(ref arg) = dir.arg {
                                if let ExpressionNode::Simple(key_exp) = arg {
                                    let key_node = SimpleExpressionNode::new(
                                        key_exp.content.clone(),
                                        key_exp.is_static,
                                        key_exp.loc.clone(),
                                    );
                                    let key = Box::new_in(key_node, ctx.allocator);

                                    let mut values = Vec::new_in(ctx.allocator);
                                    if let Some(ref exp) = dir.exp {
                                        if let ExpressionNode::Simple(val_exp) = exp {
                                            let val_node = SimpleExpressionNode::new(
                                                val_exp.content.clone(),
                                                val_exp.is_static,
                                                val_exp.loc.clone(),
                                            );
                                            values.push(Box::new_in(val_node, ctx.allocator));
                                        }
                                    }

                                    props.push(IRProp {
                                        key,
                                        values,
                                        is_component: true,
                                    });
                                }
                            }
                        } else if dir.name.as_str() == "on" {
                            // v-on -> onXxx prop
                            if let Some(ref arg) = dir.arg {
                                if let ExpressionNode::Simple(event_exp) = arg {
                                    // Convert event name to onXxx format
                                    let event_name = event_exp.content.as_str();
                                    let on_name = if event_name.is_empty() {
                                        String::from("on")
                                    } else {
                                        let mut s = String::from("on");
                                        let mut chars = event_name.chars();
                                        if let Some(c) = chars.next() {
                                            s.push(c.to_ascii_uppercase());
                                        }
                                        for c in chars {
                                            s.push(c);
                                        }
                                        s
                                    };

                                    let key_node = SimpleExpressionNode::new(
                                        on_name,
                                        true,
                                        event_exp.loc.clone(),
                                    );
                                    let key = Box::new_in(key_node, ctx.allocator);

                                    let mut values = Vec::new_in(ctx.allocator);
                                    if let Some(ref exp) = dir.exp {
                                        if let ExpressionNode::Simple(val_exp) = exp {
                                            let val_node = SimpleExpressionNode::new(
                                                val_exp.content.clone(),
                                                val_exp.is_static,
                                                val_exp.loc.clone(),
                                            );
                                            values.push(Box::new_in(val_node, ctx.allocator));
                                        }
                                    }

                                    props.push(IRProp {
                                        key,
                                        values,
                                        is_component: true,
                                    });
                                }
                            }
                        }
                    }
                    PropNode::Attribute(attr) => {
                        // Static attribute -> prop
                        let key_node = SimpleExpressionNode::new(
                            attr.name.clone(),
                            true,
                            SourceLocation::STUB,
                        );
                        let key = Box::new_in(key_node, ctx.allocator);

                        let mut values = Vec::new_in(ctx.allocator);
                        if let Some(ref value) = attr.value {
                            let val_node = SimpleExpressionNode::new(
                                value.content.clone(),
                                true,
                                SourceLocation::STUB,
                            );
                            values.push(Box::new_in(val_node, ctx.allocator));
                        }

                        props.push(IRProp {
                            key,
                            values,
                            is_component: true,
                        });
                    }
                }
            }

            let create_component = CreateComponentIRNode {
                id: element_id,
                tag: el.tag.clone(),
                props,
                slots,
                asset: true,
                once: false,
                dynamic_slots: false,
            };

            block
                .operation
                .push(OperationNode::CreateComponent(create_component));
        }
        ElementType::Slot => {
            // Slot outlet handling
            let name_exp = SimpleExpressionNode::new("default", true, SourceLocation::STUB);
            let slot_outlet = SlotOutletIRNode {
                id: element_id,
                name: Box::new_in(name_exp, ctx.allocator),
                props: Vec::new_in(ctx.allocator),
                fallback: None,
            };

            block.operation.push(OperationNode::SlotOutlet(slot_outlet));
        }
        ElementType::Template => {
            // Template element - process children directly
            for child in el.children.iter() {
                match child {
                    TemplateChildNode::Element(child_el) => {
                        transform_element(ctx, child_el, block);
                    }
                    TemplateChildNode::Text(text) => {
                        transform_text(ctx, text, block);
                    }
                    TemplateChildNode::Interpolation(interp) => {
                        transform_interpolation(ctx, interp, block);
                    }
                    _ => {}
                }
            }
        }
    }

    block.returns.push(element_id);
}

/// Transform IfNode (from compiler-core v-if transform)
fn transform_if_node<'a>(
    ctx: &mut TransformContext<'a>,
    if_node: &IfNode<'a>,
    block: &mut BlockIRNode<'a>,
) {
    if if_node.branches.is_empty() {
        return;
    }

    // Allocate ID for the if node itself
    let if_id = ctx.next_id();

    // First branch is the v-if condition
    let first_branch = &if_node.branches[0];

    // Get condition from first branch
    let condition = if let Some(ref cond) = first_branch.condition {
        match cond {
            ExpressionNode::Simple(simple) => {
                let cond_node = SimpleExpressionNode::new(
                    simple.content.clone(),
                    simple.is_static,
                    simple.loc.clone(),
                );
                Box::new_in(cond_node, ctx.allocator)
            }
            ExpressionNode::Compound(compound) => {
                let cond_node = SimpleExpressionNode::new(
                    compound.loc.source.clone(),
                    false,
                    compound.loc.clone(),
                );
                Box::new_in(cond_node, ctx.allocator)
            }
        }
    } else {
        // No condition means v-else, which shouldn't be the first branch
        let cond_node = SimpleExpressionNode::new("true", false, SourceLocation::STUB);
        Box::new_in(cond_node, ctx.allocator)
    };

    // Consume an ID for the positive branch block
    let _positive_branch_id = ctx.next_id();

    // Transform first branch children
    let positive = transform_children(ctx, &first_branch.children);

    // Handle remaining branches (v-else-if, v-else)
    let negative = if if_node.branches.len() > 1 {
        Some(transform_remaining_branches(ctx, &if_node.branches[1..]))
    } else {
        None
    };

    let ir_if = IfIRNode {
        id: if_id,
        condition,
        positive,
        negative,
        once: false,
        parent: None,
        anchor: None,
    };

    block
        .operation
        .push(OperationNode::If(Box::new_in(ir_if, ctx.allocator)));
    block.returns.push(if_id);
}

/// Transform remaining if branches (v-else-if, v-else)
fn transform_remaining_branches<'a>(
    ctx: &mut TransformContext<'a>,
    branches: &[vize_atelier_core::IfBranchNode<'a>],
) -> NegativeBranch<'a> {
    if branches.is_empty() {
        // This shouldn't happen, but return an empty block just in case
        return NegativeBranch::Block(BlockIRNode::new(ctx.allocator));
    }

    let branch = &branches[0];

    if let Some(ref cond) = branch.condition {
        // v-else-if: create nested IfIRNode
        // Note: v-else-if is inline, so it doesn't consume its own ID

        let condition = match cond {
            ExpressionNode::Simple(simple) => {
                let cond_node = SimpleExpressionNode::new(
                    simple.content.clone(),
                    simple.is_static,
                    simple.loc.clone(),
                );
                Box::new_in(cond_node, ctx.allocator)
            }
            ExpressionNode::Compound(compound) => {
                let cond_node = SimpleExpressionNode::new(
                    compound.loc.source.clone(),
                    false,
                    compound.loc.clone(),
                );
                Box::new_in(cond_node, ctx.allocator)
            }
        };

        // Consume ID for positive branch block
        let _positive_branch_id = ctx.next_id();

        let positive = transform_children(ctx, &branch.children);

        let negative = if branches.len() > 1 {
            // Consume ID for negative branch callback block
            let _negative_block_id = ctx.next_id();
            Some(transform_remaining_branches(ctx, &branches[1..]))
        } else {
            None
        };

        let nested_if = IfIRNode {
            id: 0, // Not used for inline v-else-if
            condition,
            positive,
            negative,
            once: false,
            parent: None,
            anchor: None,
        };

        NegativeBranch::If(Box::new_in(nested_if, ctx.allocator))
    } else {
        // v-else: consume ID for the else branch block
        let _else_branch_id = ctx.next_id();
        NegativeBranch::Block(transform_children(ctx, &branch.children))
    }
}

/// Transform ForNode (from compiler-core v-for transform)
fn transform_for_node<'a>(
    ctx: &mut TransformContext<'a>,
    for_node: &ForNode<'a>,
    block: &mut BlockIRNode<'a>,
) {
    // Get source expression
    let source = match &for_node.source {
        ExpressionNode::Simple(simple) => {
            let source_node = SimpleExpressionNode::new(
                simple.content.clone(),
                simple.is_static,
                simple.loc.clone(),
            );
            Box::new_in(source_node, ctx.allocator)
        }
        ExpressionNode::Compound(compound) => {
            let source_node =
                SimpleExpressionNode::new(compound.loc.source.clone(), false, compound.loc.clone());
            Box::new_in(source_node, ctx.allocator)
        }
    };

    // Get value alias
    let value = for_node.value_alias.as_ref().map(|v| match v {
        ExpressionNode::Simple(simple) => {
            let val_node = SimpleExpressionNode::new(
                simple.content.clone(),
                simple.is_static,
                simple.loc.clone(),
            );
            Box::new_in(val_node, ctx.allocator)
        }
        ExpressionNode::Compound(compound) => {
            let val_node =
                SimpleExpressionNode::new(compound.loc.source.clone(), false, compound.loc.clone());
            Box::new_in(val_node, ctx.allocator)
        }
    });

    // Get key alias
    let key = for_node.key_alias.as_ref().map(|k| match k {
        ExpressionNode::Simple(simple) => {
            let key_node = SimpleExpressionNode::new(
                simple.content.clone(),
                simple.is_static,
                simple.loc.clone(),
            );
            Box::new_in(key_node, ctx.allocator)
        }
        ExpressionNode::Compound(compound) => {
            let key_node =
                SimpleExpressionNode::new(compound.loc.source.clone(), false, compound.loc.clone());
            Box::new_in(key_node, ctx.allocator)
        }
    });

    // Get index alias
    let index = for_node.object_index_alias.as_ref().map(|i| match i {
        ExpressionNode::Simple(simple) => {
            let idx_node = SimpleExpressionNode::new(
                simple.content.clone(),
                simple.is_static,
                simple.loc.clone(),
            );
            Box::new_in(idx_node, ctx.allocator)
        }
        ExpressionNode::Compound(compound) => {
            let idx_node =
                SimpleExpressionNode::new(compound.loc.source.clone(), false, compound.loc.clone());
            Box::new_in(idx_node, ctx.allocator)
        }
    });

    // Transform children as render block
    let render = transform_children(ctx, &for_node.children);

    let ir_for = ForIRNode {
        id: ctx.next_id(),
        source,
        value,
        key,
        index,
        key_prop: None, // TODO: Handle key prop from element
        render,
        once: false,
        component: false,
        only_child: for_node.children.len() == 1,
    };

    block
        .operation
        .push(OperationNode::For(Box::new_in(ir_for, ctx.allocator)));
}

/// Transform text node
fn transform_text<'a>(
    ctx: &mut TransformContext<'a>,
    text: &TextNode,
    block: &mut BlockIRNode<'a>,
) {
    let element_id = ctx.next_id();
    let template: String = text.content.clone();
    ctx.templates.push(template);
    block.returns.push(element_id);
}

/// Transform interpolation node (standalone, not inside element)
fn transform_interpolation<'a>(
    ctx: &mut TransformContext<'a>,
    interp: &InterpolationNode<'a>,
    block: &mut BlockIRNode<'a>,
) {
    let element_id = ctx.next_id();

    // Create SetText operation
    let values = match &interp.content {
        ExpressionNode::Simple(simple) => {
            let mut v = Vec::new_in(ctx.allocator);
            let exp = SimpleExpressionNode::new(
                simple.content.clone(),
                simple.is_static,
                simple.loc.clone(),
            );
            v.push(Box::new_in(exp, ctx.allocator));
            v
        }
        _ => Vec::new_in(ctx.allocator),
    };

    let set_text = SetTextIRNode {
        element: element_id,
        values,
    };

    // Add to effects (reactive)
    let mut effect_ops = Vec::new_in(ctx.allocator);
    effect_ops.push(OperationNode::SetText(set_text));

    block.effect.push(IREffect {
        operations: effect_ops,
    });

    block.returns.push(element_id);
}

/// Transform text children (combined text and interpolations)
fn transform_text_children<'a>(
    ctx: &mut TransformContext<'a>,
    children: &[TemplateChildNode<'a>],
    parent_element_id: usize,
    block: &mut BlockIRNode<'a>,
) {
    let mut values = Vec::new_in(ctx.allocator);

    // Collect all text parts and interpolations
    for child in children.iter() {
        match child {
            TemplateChildNode::Text(text) => {
                // Static text part
                let exp = SimpleExpressionNode::new(
                    text.content.clone(),
                    true, // is_static = true
                    SourceLocation::STUB,
                );
                values.push(Box::new_in(exp, ctx.allocator));
            }
            TemplateChildNode::Interpolation(interp) => {
                // Dynamic interpolation
                if let ExpressionNode::Simple(simple) = &interp.content {
                    let exp = SimpleExpressionNode::new(
                        simple.content.clone(),
                        simple.is_static,
                        simple.loc.clone(),
                    );
                    values.push(Box::new_in(exp, ctx.allocator));
                }
            }
            _ => {}
        }
    }

    if !values.is_empty() {
        let set_text = SetTextIRNode {
            element: parent_element_id,
            values,
        };

        let mut effect_ops = Vec::new_in(ctx.allocator);
        effect_ops.push(OperationNode::SetText(set_text));

        block.effect.push(IREffect {
            operations: effect_ops,
        });
    }
}

/// Transform directive
fn transform_directive<'a>(
    ctx: &mut TransformContext<'a>,
    dir: &DirectiveNode<'a>,
    element_id: usize,
    el: &ElementNode<'a>,
    block: &mut BlockIRNode<'a>,
) {
    match dir.name.as_str() {
        "bind" => {
            // v-bind - SetProp
            if let Some(ref arg) = dir.arg {
                if let ExpressionNode::Simple(key_exp) = arg {
                    let key_node = SimpleExpressionNode::new(
                        key_exp.content.clone(),
                        key_exp.is_static,
                        key_exp.loc.clone(),
                    );
                    let key = Box::new_in(key_node, ctx.allocator);

                    let values = if let Some(ref exp) = dir.exp {
                        if let ExpressionNode::Simple(val_exp) = exp {
                            let mut v = Vec::new_in(ctx.allocator);
                            let val_node = SimpleExpressionNode::new(
                                val_exp.content.clone(),
                                val_exp.is_static,
                                val_exp.loc.clone(),
                            );
                            v.push(Box::new_in(val_node, ctx.allocator));
                            v
                        } else {
                            Vec::new_in(ctx.allocator)
                        }
                    } else {
                        Vec::new_in(ctx.allocator)
                    };

                    let set_prop = SetPropIRNode {
                        element: element_id,
                        prop: IRProp {
                            key,
                            values,
                            is_component: el.tag_type == ElementType::Component,
                        },
                        tag: el.tag.clone(),
                    };

                    // Reactive prop - add to effects
                    let mut effect_ops = Vec::new_in(ctx.allocator);
                    effect_ops.push(OperationNode::SetProp(set_prop));
                    block.effect.push(IREffect {
                        operations: effect_ops,
                    });
                }
            }
        }
        "on" => {
            // v-on - SetEvent
            if let Some(ref arg) = dir.arg {
                if let ExpressionNode::Simple(key_exp) = arg {
                    let key_node = SimpleExpressionNode::new(
                        key_exp.content.clone(),
                        key_exp.is_static,
                        key_exp.loc.clone(),
                    );
                    let key = Box::new_in(key_node, ctx.allocator);

                    let value = if let Some(ref exp) = dir.exp {
                        if let ExpressionNode::Simple(val_exp) = exp {
                            let val_node = SimpleExpressionNode::new(
                                val_exp.content.clone(),
                                val_exp.is_static,
                                val_exp.loc.clone(),
                            );
                            Some(Box::new_in(val_node, ctx.allocator))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let set_event = SetEventIRNode {
                        element: element_id,
                        key,
                        value,
                        modifiers: Default::default(),
                        delegate: true,
                        effect: false,
                    };

                    block.operation.push(OperationNode::SetEvent(set_event));
                }
            }
        }
        "if" => {
            // v-if
            if let Some(ref exp) = dir.exp {
                if let ExpressionNode::Simple(cond_exp) = exp {
                    let cond_node = SimpleExpressionNode::new(
                        cond_exp.content.clone(),
                        cond_exp.is_static,
                        cond_exp.loc.clone(),
                    );
                    let condition = Box::new_in(cond_node, ctx.allocator);
                    let positive = transform_children(ctx, &el.children);

                    let if_node = IfIRNode {
                        id: ctx.next_id(),
                        condition,
                        positive,
                        negative: None,
                        once: false,
                        parent: None,
                        anchor: None,
                    };

                    block
                        .operation
                        .push(OperationNode::If(Box::new_in(if_node, ctx.allocator)));
                }
            }
        }
        "for" => {
            // v-for
            if let Some(ref exp) = dir.exp {
                if let ExpressionNode::Simple(source_exp) = exp {
                    let source_node = SimpleExpressionNode::new(
                        source_exp.content.clone(),
                        source_exp.is_static,
                        source_exp.loc.clone(),
                    );
                    let source = Box::new_in(source_node, ctx.allocator);
                    let render = transform_children(ctx, &el.children);

                    let for_node = ForIRNode {
                        id: ctx.next_id(),
                        source,
                        value: None,
                        key: None,
                        index: None,
                        key_prop: None,
                        render,
                        once: false,
                        component: el.tag_type == ElementType::Component,
                        only_child: false,
                    };

                    block
                        .operation
                        .push(OperationNode::For(Box::new_in(for_node, ctx.allocator)));
                }
            }
        }
        "html" => {
            // v-html
            if let Some(ref exp) = dir.exp {
                if let ExpressionNode::Simple(val_exp) = exp {
                    let val_node = SimpleExpressionNode::new(
                        val_exp.content.clone(),
                        val_exp.is_static,
                        val_exp.loc.clone(),
                    );
                    let value = Box::new_in(val_node, ctx.allocator);
                    let set_html = SetHtmlIRNode {
                        element: element_id,
                        value,
                    };

                    let mut effect_ops = Vec::new_in(ctx.allocator);
                    effect_ops.push(OperationNode::SetHtml(set_html));
                    block.effect.push(IREffect {
                        operations: effect_ops,
                    });
                }
            }
        }
        "text" => {
            // v-text
            if let Some(ref exp) = dir.exp {
                if let ExpressionNode::Simple(val_exp) = exp {
                    let mut values = Vec::new_in(ctx.allocator);
                    let val_node = SimpleExpressionNode::new(
                        val_exp.content.clone(),
                        val_exp.is_static,
                        val_exp.loc.clone(),
                    );
                    values.push(Box::new_in(val_node, ctx.allocator));

                    let set_text = SetTextIRNode {
                        element: element_id,
                        values,
                    };

                    let mut effect_ops = Vec::new_in(ctx.allocator);
                    effect_ops.push(OperationNode::SetText(set_text));
                    block.effect.push(IREffect {
                        operations: effect_ops,
                    });
                }
            }
        }
        _ => {
            // Custom directive - create a copy of the directive
            let new_dir = DirectiveNode::new(ctx.allocator, dir.name.clone(), dir.loc.clone());

            let dir_node = DirectiveIRNode {
                element: element_id,
                dir: Box::new_in(new_dir, ctx.allocator),
                name: dir.name.clone(),
                builtin: false,
            };

            block.operation.push(OperationNode::Directive(dir_node));
        }
    }
}

/// Generate element template string (recursively includes static children)
fn generate_element_template(el: &ElementNode<'_>) -> String {
    let mut template = format!("<{}", el.tag);

    // Add static attributes
    for prop in el.props.iter() {
        if let PropNode::Attribute(attr) = prop {
            if let Some(ref value) = attr.value {
                template.push_str(&format!(" {}=\"{}\"", attr.name, value.content));
            } else {
                template.push_str(&format!(" {}", attr.name));
            }
        }
    }

    if el.is_self_closing {
        template.push_str(" />");
    } else {
        template.push('>');

        // Check if there are any interpolations - if so, use a space placeholder
        let has_interpolation = el
            .children
            .iter()
            .any(|c| matches!(c, TemplateChildNode::Interpolation(_)));

        if has_interpolation {
            // Use single space as placeholder for interpolation text content
            template.push(' ');
        } else {
            // Recursively add static children (text and static elements)
            for child in el.children.iter() {
                match child {
                    TemplateChildNode::Text(text) => {
                        template.push_str(&escape_html_text(&text.content));
                    }
                    TemplateChildNode::Element(child_el) => {
                        // Include child elements in template
                        template.push_str(&generate_element_template(child_el));
                    }
                    _ => {
                        // Other dynamic content is handled elsewhere
                    }
                }
            }
        }

        template.push_str(&format!("</{}>", el.tag));
    }

    template.into()
}

/// Escape HTML special characters in text content (vuejs/core #14310)
fn escape_html_text(s: &str) -> std::string::String {
    let mut result = std::string::String::with_capacity(s.len());
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

/// Check if an element is static (no dynamic directives)
fn is_static_element(el: &ElementNode<'_>) -> bool {
    // Check if any prop is a directive (dynamic)
    for prop in el.props.iter() {
        if matches!(prop, PropNode::Directive(_)) {
            return false;
        }
    }

    // Check if any child is dynamic
    for child in el.children.iter() {
        match child {
            TemplateChildNode::Interpolation(_) => return false,
            TemplateChildNode::Element(child_el) => {
                if !is_static_element(child_el) {
                    return false;
                }
            }
            TemplateChildNode::If(_) | TemplateChildNode::For(_) => return false,
            _ => {}
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use vize_atelier_core::parser::parse;

    #[test]
    fn test_transform_simple_element() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, "<div>hello</div>");
        let ir = transform_to_ir(&allocator, &root);

        assert!(!ir.block.returns.is_empty());
    }

    #[test]
    fn test_transform_nested_elements() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, "<div><span>nested</span></div>");
        let ir = transform_to_ir(&allocator, &root);

        assert!(!ir.block.returns.is_empty());
    }
}
