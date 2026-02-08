//! Structural directive transforms (v-if, v-for).

use vize_carton::{Box, Bump, String, Vec};

use crate::ast::*;
use crate::errors::ErrorCode;

use super::context::clone_expression;
use super::traverse::traverse_children;
use super::{ExitFn, ParentNode, TransformContext};

/// Simple expression content for passing between functions
pub struct SimpleExpressionContent {
    pub content: String,
    pub is_static: bool,
    pub loc: SourceLocation,
}

/// Check if element has a structural directive
pub fn check_structural_directive<'a>(
    el: &ElementNode<'a>,
) -> Option<(
    String,
    Option<SimpleExpressionContent>,
    Option<SourceLocation>,
)> {
    for prop in el.props.iter() {
        if let PropNode::Directive(dir) = prop {
            match dir.name.as_str() {
                "if" | "else-if" | "else" | "for" => {
                    let exp_content = dir.exp.as_ref().map(|e| match e {
                        ExpressionNode::Simple(s) => SimpleExpressionContent {
                            content: s.content.clone(),
                            is_static: s.is_static,
                            loc: s.loc.clone(),
                        },
                        ExpressionNode::Compound(c) => SimpleExpressionContent {
                            content: c.loc.source.clone(),
                            is_static: false,
                            loc: c.loc.clone(),
                        },
                    });
                    let exp_loc = dir.exp.as_ref().map(|e| e.loc().clone());
                    return Some((dir.name.clone(), exp_content, exp_loc));
                }
                _ => {}
            }
        }
    }
    None
}

/// Extract and remove key prop from element
pub fn extract_key_prop<'a>(el: &mut ElementNode<'a>) -> Option<PropNode<'a>> {
    let mut key_index = None;
    for (i, prop) in el.props.iter().enumerate() {
        match prop {
            PropNode::Attribute(attr) if attr.name == "key" => {
                key_index = Some(i);
                break;
            }
            PropNode::Directive(dir) if dir.name == "bind" => {
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    if arg.content == "key" {
                        key_index = Some(i);
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    key_index.map(|i| el.props.remove(i))
}

/// Remove structural directive from element props
pub fn remove_structural_directive<'a>(el: &mut Box<'a, ElementNode<'a>>, dir_name: &str) {
    let mut i = 0;
    while i < el.props.len() {
        if let PropNode::Directive(dir) = &el.props[i] {
            if dir.name.as_str() == dir_name {
                el.props.remove(i);
                return;
            }
        }
        i += 1;
    }
}

/// Transform v-if directive
pub fn transform_v_if<'a>(
    ctx: &mut TransformContext<'a>,
    exp: Option<&SimpleExpressionContent>,
    _exp_loc: Option<SourceLocation>,
    is_root: bool,
) -> Option<std::vec::Vec<ExitFn<'a>>> {
    let allocator = ctx.allocator;

    if is_root {
        // Take the current element from parent
        let taken = ctx.take_current_node();
        let taken_node = taken?;

        // Get element info before moving
        let (element_loc, is_template_if) = match &taken_node {
            TemplateChildNode::Element(el) => {
                (el.loc.clone(), el.tag_type == ElementType::Template)
            }
            _ => return None,
        };

        // Create condition expression and process it for identifier prefixing
        let condition = exp.map(|e| {
            let raw_exp = ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode {
                    content: e.content.clone(),
                    is_static: e.is_static,
                    const_type: if e.is_static {
                        ConstantType::CanStringify
                    } else {
                        ConstantType::NotConstant
                    },
                    loc: e.loc.clone(),
                    js_ast: None,
                    hoisted: None,
                    identifiers: None,
                    is_handler_key: false,
                    is_ref_transformed: false,
                },
                allocator,
            ));
            // Process expression to add $setup. prefix
            if ctx.options.prefix_identifiers || ctx.options.is_ts {
                crate::transforms::transform_expression::process_expression(ctx, &raw_exp, false)
            } else {
                raw_exp
            }
        });

        // Extract user key from the element if present
        let mut user_key = None;
        let taken_node = match taken_node {
            TemplateChildNode::Element(mut el) => {
                user_key = extract_key_prop(&mut el);
                TemplateChildNode::Element(el)
            }
            other => other,
        };

        // Process user_key expression for identifier prefixing (e.g., keyA -> _ctx.keyA)
        if let Some(PropNode::Directive(ref mut dir)) = user_key {
            if ctx.options.prefix_identifiers || ctx.options.is_ts {
                if let Some(ref exp) = dir.exp {
                    let processed = crate::transforms::transform_expression::process_expression(
                        ctx, exp, false,
                    );
                    dir.exp = Some(processed);
                }
            }
        }

        // Create branch with the taken element
        let mut branch_children = Vec::new_in(allocator);
        branch_children.push(taken_node);

        let branch = IfBranchNode {
            condition,
            children: branch_children,
            user_key,
            is_template_if,
            loc: element_loc.clone(),
        };

        let mut branches = Vec::new_in(allocator);
        branches.push(branch);

        let if_node = IfNode {
            branches,
            codegen_node: None,
            loc: element_loc,
        };

        // Replace placeholder with IfNode
        ctx.replace_node(TemplateChildNode::If(Box::new_in(if_node, allocator)));

        // Add helpers
        ctx.helper(RuntimeHelper::OpenBlock);
        ctx.helper(RuntimeHelper::CreateBlock);
        ctx.helper(RuntimeHelper::Fragment);
        ctx.helper(RuntimeHelper::CreateComment);

        None
    } else {
        // Find previous v-if node and add branch to it
        let child_index = ctx.child_index;

        // First, find the if node index
        let found_if_idx = if let Some(parent) = &ctx.parent {
            let children = parent.children_mut();
            let mut found = None;

            // Look backwards for v-if node
            for j in (0..child_index).rev() {
                match &children[j] {
                    TemplateChildNode::If(_) => {
                        found = Some(j);
                        break;
                    }
                    TemplateChildNode::Comment(_) => continue,
                    TemplateChildNode::Text(t) if t.content.trim().is_empty() => continue,
                    _ => break,
                }
            }
            found
        } else {
            None
        };

        if let Some(if_idx) = found_if_idx {
            // Take current element
            let taken = ctx.take_current_node();
            let taken_node = taken?;

            let (element_loc, is_template_if) = match &taken_node {
                TemplateChildNode::Element(el) => {
                    (el.loc.clone(), el.tag_type == ElementType::Template)
                }
                _ => return None,
            };

            // Create condition for else-if, None for else
            let condition = exp.map(|e| {
                let raw_exp = ExpressionNode::Simple(Box::new_in(
                    SimpleExpressionNode {
                        content: e.content.clone(),
                        is_static: e.is_static,
                        const_type: if e.is_static {
                            ConstantType::CanStringify
                        } else {
                            ConstantType::NotConstant
                        },
                        loc: e.loc.clone(),
                        js_ast: None,
                        hoisted: None,
                        identifiers: None,
                        is_handler_key: false,
                        is_ref_transformed: false,
                    },
                    allocator,
                ));
                // Process expression to add $setup. prefix
                if ctx.options.prefix_identifiers || ctx.options.is_ts {
                    crate::transforms::transform_expression::process_expression(
                        ctx, &raw_exp, false,
                    )
                } else {
                    raw_exp
                }
            });

            // Extract user key from the element if present
            let mut user_key = None;
            let taken_node = match taken_node {
                TemplateChildNode::Element(mut el) => {
                    user_key = extract_key_prop(&mut el);
                    TemplateChildNode::Element(el)
                }
                other => other,
            };

            // Process user_key expression for identifier prefixing
            if let Some(PropNode::Directive(ref mut dir)) = user_key {
                if ctx.options.prefix_identifiers || ctx.options.is_ts {
                    if let Some(ref exp) = dir.exp {
                        let processed = crate::transforms::transform_expression::process_expression(
                            ctx, exp, false,
                        );
                        dir.exp = Some(processed);
                    }
                }
            }

            // Check for key collision with existing branches (vuejs/core #13881)
            let has_key_collision = if let Some(ref new_key) = user_key {
                let new_key_str = extract_key_value_str(new_key);
                if let Some(parent) = &ctx.parent {
                    let children = parent.children_mut();
                    if let TemplateChildNode::If(if_node) = &children[if_idx] {
                        if_node.branches.iter().any(|existing_branch| {
                            if let Some(ref existing_key) = existing_branch.user_key {
                                let existing_key_str = extract_key_value_str(existing_key);
                                matches!((&new_key_str, &existing_key_str), (Some(nk), Some(ek)) if nk == ek)
                            } else {
                                false
                            }
                        })
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if has_key_collision {
                ctx.on_error(ErrorCode::VIfSameKey, None);
            }

            // Create new branch
            let mut branch_children = Vec::new_in(allocator);
            branch_children.push(taken_node);

            let branch = IfBranchNode {
                condition,
                children: branch_children,
                user_key,
                is_template_if,
                loc: element_loc,
            };

            // Add branch to if node and traverse its children
            // Save context state before traversing (traverse_children modifies parent)
            let saved_parent = ctx.parent;
            let saved_grandparent = ctx.grandparent;
            let saved_child_index = ctx.child_index;

            if let Some(parent) = &ctx.parent {
                let children = parent.children_mut();
                if let TemplateChildNode::If(if_node) = &mut children[if_idx] {
                    if_node.branches.push(branch);
                    // Traverse the newly added branch to process components in it
                    let branch_idx = if_node.branches.len() - 1;
                    let branch_ptr = &mut if_node.branches[branch_idx] as *mut IfBranchNode<'a>;
                    traverse_children(ctx, ParentNode::IfBranch(branch_ptr));
                }
            }

            // Restore context state before removing node
            ctx.parent = saved_parent;
            ctx.grandparent = saved_grandparent;
            ctx.child_index = saved_child_index;

            // Remove the placeholder we left
            ctx.remove_node();
        } else {
            ctx.on_error(ErrorCode::VElseNoAdjacentIf, None);
        }

        None
    }
}

/// Transform v-for directive
pub fn transform_v_for<'a>(
    ctx: &mut TransformContext<'a>,
    exp: Option<&SimpleExpressionContent>,
    _exp_loc: Option<SourceLocation>,
) -> Option<std::vec::Vec<ExitFn<'a>>> {
    let allocator = ctx.allocator;

    let Some(exp) = exp else {
        ctx.on_error(ErrorCode::VForNoExpression, None);
        return None;
    };

    // Take the current element from parent
    let taken = ctx.take_current_node();
    let taken_node = taken?;

    let element_loc = match &taken_node {
        TemplateChildNode::Element(el) => el.loc.clone(),
        _ => return None,
    };

    // Parse v-for expression: "item in items" or "(item, index) in items"
    let (mut source, value_alias, key_alias, index_alias) =
        parse_v_for_expression(allocator, &exp.content, &exp.loc);

    // Process source expression with binding-aware identifier prefixing
    // This ensures imports and refs are correctly handled (e.g., _unref(PRESETS) instead of _ctx.PRESETS)
    if ctx.options.prefix_identifiers || ctx.options.is_ts {
        use crate::transforms::process_expression;
        // Process the source expression through the binding-aware transform
        let processed = process_expression(ctx, &source, false);
        source = processed;
    }

    // Create ForNode children with taken element
    let mut for_children = Vec::new_in(allocator);
    for_children.push(taken_node);

    // Create parse result (clone expressions for parse_result)
    let parse_result = ForParseResult {
        source: clone_expression(allocator, &source),
        value: value_alias.as_ref().map(|e| clone_expression(allocator, e)),
        key: key_alias.as_ref().map(|e| clone_expression(allocator, e)),
        index: index_alias.as_ref().map(|e| clone_expression(allocator, e)),
        finalized: false,
    };

    let for_node = ForNode {
        source,
        value_alias,
        key_alias,
        object_index_alias: index_alias,
        parse_result,
        children: for_children,
        codegen_node: None,
        loc: element_loc,
    };

    // Replace placeholder with ForNode
    ctx.replace_node(TemplateChildNode::For(Box::new_in(for_node, allocator)));

    // Add helpers
    ctx.helper(RuntimeHelper::RenderList);
    ctx.helper(RuntimeHelper::OpenBlock);
    ctx.helper(RuntimeHelper::CreateBlock);
    ctx.helper(RuntimeHelper::CreateElementBlock);
    ctx.helper(RuntimeHelper::Fragment);

    None
}

/// Parse v-for expression
fn parse_v_for_expression<'a>(
    allocator: &'a Bump,
    content: &str,
    loc: &SourceLocation,
) -> (
    ExpressionNode<'a>,
    Option<ExpressionNode<'a>>,
    Option<ExpressionNode<'a>>,
    Option<ExpressionNode<'a>>,
) {
    // Match patterns like "item in items" or "(item, index) in items"
    let (alias_part, source_part) = if let Some(idx) = content.find(" in ") {
        (&content[..idx], &content[idx + 4..])
    } else if let Some(idx) = content.find(" of ") {
        (&content[..idx], &content[idx + 4..])
    } else {
        // Return source as-is
        let source = ExpressionNode::Simple(Box::new_in(
            SimpleExpressionNode {
                content: String::new(content),
                is_static: false,
                const_type: ConstantType::NotConstant,
                loc: loc.clone(),
                js_ast: None,
                hoisted: None,
                identifiers: None,
                is_handler_key: false,
                is_ref_transformed: false,
            },
            allocator,
        ));
        return (source, None, None, None);
    };

    let source_str = source_part.trim();
    let alias_str = alias_part.trim();

    // Parse source expression
    let source = ExpressionNode::Simple(Box::new_in(
        SimpleExpressionNode {
            content: String::new(source_str),
            is_static: false,
            const_type: ConstantType::NotConstant,
            loc: SourceLocation::default(),
            js_ast: None,
            hoisted: None,
            identifiers: None,
            is_handler_key: false,
            is_ref_transformed: false,
        },
        allocator,
    ));

    // Parse aliases
    let (value, key, index) = if alias_str.starts_with('(') && alias_str.ends_with(')') {
        let inner = &alias_str[1..alias_str.len() - 1];
        let aliases: std::vec::Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

        let value = if !aliases.is_empty() && !aliases[0].is_empty() {
            Some(ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode {
                    content: String::new(aliases[0]),
                    is_static: false,
                    const_type: ConstantType::NotConstant,
                    loc: SourceLocation::default(),
                    js_ast: None,
                    hoisted: None,
                    identifiers: None,
                    is_handler_key: false,
                    is_ref_transformed: false,
                },
                allocator,
            )))
        } else {
            None
        };

        let key = if aliases.len() > 1 && !aliases[1].is_empty() {
            Some(ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode {
                    content: String::new(aliases[1]),
                    is_static: false,
                    const_type: ConstantType::NotConstant,
                    loc: SourceLocation::default(),
                    js_ast: None,
                    hoisted: None,
                    identifiers: None,
                    is_handler_key: false,
                    is_ref_transformed: false,
                },
                allocator,
            )))
        } else {
            None
        };

        let index = if aliases.len() > 2 && !aliases[2].is_empty() {
            Some(ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode {
                    content: String::new(aliases[2]),
                    is_static: false,
                    const_type: ConstantType::NotConstant,
                    loc: SourceLocation::default(),
                    js_ast: None,
                    hoisted: None,
                    identifiers: None,
                    is_handler_key: false,
                    is_ref_transformed: false,
                },
                allocator,
            )))
        } else {
            None
        };

        (value, key, index)
    } else {
        // Simple alias
        let value = Some(ExpressionNode::Simple(Box::new_in(
            SimpleExpressionNode {
                content: String::new(alias_str),
                is_static: false,
                const_type: ConstantType::NotConstant,
                loc: SourceLocation::default(),
                js_ast: None,
                hoisted: None,
                identifiers: None,
                is_handler_key: false,
                is_ref_transformed: false,
            },
            allocator,
        )));
        (value, None, None)
    };

    (source, value, key, index)
}

/// Extract key value string from a PropNode for comparison
fn extract_key_value_str(prop: &PropNode<'_>) -> Option<std::string::String> {
    match prop {
        PropNode::Attribute(attr) => attr.value.as_ref().map(|v| v.content.to_string()),
        PropNode::Directive(dir) => dir.exp.as_ref().map(|exp| match exp {
            ExpressionNode::Simple(s) => s.content.to_string(),
            ExpressionNode::Compound(c) => c.loc.source.to_string(),
        }),
    }
}
