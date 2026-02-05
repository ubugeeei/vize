//! Element transform.
//!
//! Transforms element nodes and their props.

use vize_carton::{capitalize, String};

use crate::ast::*;
use crate::transform::TransformContext;

/// Resolve element type
pub fn resolve_element_type<'a>(
    ctx: &mut TransformContext<'a>,
    el: &ElementNode<'a>,
) -> ElementType {
    let tag = &el.tag;

    // Check if it's a component
    if is_component(tag, el) {
        ctx.helper(RuntimeHelper::ResolveComponent);
        ctx.add_component(tag.clone());
        ElementType::Component
    } else if tag == "slot" {
        ElementType::Slot
    } else if tag == "template" {
        ElementType::Template
    } else {
        ElementType::Element
    }
}

/// Check if tag is a component
fn is_component(tag: &str, el: &ElementNode<'_>) -> bool {
    // Components start with uppercase or contain -
    let first_char = tag.chars().next().unwrap_or('a');
    if first_char.is_uppercase() {
        return true;
    }
    if tag.contains('-') {
        return true;
    }
    // Check for is attribute
    for prop in el.props.iter() {
        if let PropNode::Directive(dir) = prop {
            if dir.name == "is" {
                return true;
            }
        }
        if let PropNode::Attribute(attr) = prop {
            if attr.name == "is" {
                return true;
            }
        }
    }
    false
}

/// Build element props for codegen
pub fn build_props<'a>(
    _ctx: &mut TransformContext<'a>,
    el: &ElementNode<'a>,
) -> Option<TransformPropsExpression<'a>> {
    if el.props.is_empty() {
        return None;
    }

    let mut properties: Vec<PropItem<'a>> = Vec::new();
    let mut dynamic_prop_names: Vec<String> = Vec::new();
    let mut has_runtime_props = false;

    for prop in el.props.iter() {
        match prop {
            PropNode::Attribute(attr) => {
                // Static attribute
                let key = attr.name.clone();
                let value = attr.value.as_ref().map(|v| v.content.clone());
                properties.push(PropItem::Static { key, value });
            }
            PropNode::Directive(dir) => {
                match dir.name.as_str() {
                    "bind" => {
                        has_runtime_props = true;
                        if let Some(ExpressionNode::Simple(exp)) = &dir.arg {
                            if !exp.is_static {
                                dynamic_prop_names.push(exp.content.clone());
                            }
                        }
                    }
                    "on" => {
                        has_runtime_props = true;
                        if let Some(ExpressionNode::Simple(exp)) = &dir.arg {
                            if !exp.is_static {
                                let cap = capitalize(&exp.content);
                                let mut name = String::with_capacity(2 + cap.len());
                                name.push_str("on");
                                name.push_str(&cap);
                                dynamic_prop_names.push(name);
                            }
                        }
                    }
                    _ => {
                        // Other directives
                    }
                }
            }
        }
    }

    if properties.is_empty() && !has_runtime_props {
        return None;
    }

    Some(TransformPropsExpression {
        properties,
        dynamic_prop_names,
        has_runtime_props,
    })
}

/// Props expression for codegen (transform-specific)
#[derive(Debug)]
pub struct TransformPropsExpression<'a> {
    pub properties: Vec<PropItem<'a>>,
    pub dynamic_prop_names: Vec<String>,
    pub has_runtime_props: bool,
}

/// Individual prop item
#[derive(Debug)]
pub enum PropItem<'a> {
    Static {
        key: String,
        value: Option<String>,
    },
    Dynamic {
        key: ExpressionNode<'a>,
        value: ExpressionNode<'a>,
    },
}

/// Build element codegen node
pub fn build_element_codegen<'a>(
    ctx: &mut TransformContext<'a>,
    el: &ElementNode<'a>,
) -> Option<TransformVNodeCall<'a>> {
    let tag: String = match el.tag_type {
        ElementType::Element => {
            ctx.helper(RuntimeHelper::CreateElementVNode);
            {
                let mut name = String::with_capacity(el.tag.len() + 2);
                name.push('"');
                name.push_str(&el.tag);
                name.push('"');
                name
            }
        }
        ElementType::Component => {
            ctx.helper(RuntimeHelper::CreateVNode);
            ctx.helper(RuntimeHelper::ResolveComponent);
            {
                let mut name = String::with_capacity(11 + el.tag.len());
                name.push_str("_component_");
                name.push_str(&el.tag);
                name
            }
        }
        _ => return None,
    };

    let props = build_props(ctx, el);
    let has_children = !el.children.is_empty();

    Some(TransformVNodeCall {
        tag,
        props,
        children: if has_children {
            Some(ChildrenType::Element)
        } else {
            None
        },
        patch_flag: calculate_patch_flag(el),
        dynamic_props: None,
        is_block: false,
        disable_tracking: false,
        is_component: el.tag_type == ElementType::Component,
    })
}

/// Calculate patch flag for element
fn calculate_patch_flag(el: &ElementNode<'_>) -> Option<i32> {
    let mut flag = 0;

    for prop in el.props.iter() {
        if let PropNode::Directive(dir) = prop {
            match dir.name.as_str() {
                "bind" => match &dir.arg {
                    Some(ExpressionNode::Simple(exp)) => {
                        match exp.content.as_str() {
                            "class" => flag |= 2, // CLASS
                            "style" => flag |= 4, // STYLE
                            _ => flag |= 8,       // PROPS
                        }
                    }
                    Some(_) => flag |= 16, // Compound expression - FULL_PROPS
                    None => flag |= 16,    // No arg - FULL_PROPS
                },
                "on" => {}
                _ => {}
            }
        }
    }

    // Check for text children with interpolation
    for child in el.children.iter() {
        if let TemplateChildNode::Interpolation(_) = child {
            flag |= 1; // TEXT
        }
    }

    if flag > 0 {
        Some(flag)
    } else {
        None
    }
}

/// VNode call for codegen (transform-specific)
#[derive(Debug)]
pub struct TransformVNodeCall<'a> {
    pub tag: String,
    pub props: Option<TransformPropsExpression<'a>>,
    pub children: Option<ChildrenType>,
    pub patch_flag: Option<i32>,
    pub dynamic_props: Option<Vec<String>>,
    pub is_block: bool,
    pub disable_tracking: bool,
    pub is_component: bool,
}

/// Children type for codegen
#[derive(Debug)]
pub enum ChildrenType {
    Element,
    Component,
    Slot,
    Text,
    RawSlots,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use bumpalo::Bump;

    #[test]
    fn test_resolve_element_type() {
        let allocator = Bump::new();
        let mut ctx = TransformContext::new(&allocator, "".into(), Default::default());

        let (root, _) = parse(&allocator, r#"<div>test</div>"#);
        if let TemplateChildNode::Element(el) = &root.children[0] {
            assert_eq!(resolve_element_type(&mut ctx, el), ElementType::Element);
        }
    }

    #[test]
    fn test_resolve_component_type() {
        let allocator = Bump::new();
        let mut ctx = TransformContext::new(&allocator, "".into(), Default::default());

        let (root, _) = parse(&allocator, r#"<MyComponent></MyComponent>"#);
        if let TemplateChildNode::Element(el) = &root.children[0] {
            assert_eq!(resolve_element_type(&mut ctx, el), ElementType::Component);
        }
    }
}
