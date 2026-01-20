//! AST traversal functions for template transformation.

use crate::ast::*;

use super::element::{transform_element, transform_interpolation};
use super::structural::{
    check_structural_directive, remove_structural_directive, transform_v_for, transform_v_if,
};
use super::{ExitFn, ParentNode, TransformContext};

/// Traverse children of a parent node
pub fn traverse_children<'a>(ctx: &mut TransformContext<'a>, parent: ParentNode<'a>) {
    let children = parent.children_mut();
    let mut i = 0;

    while i < children.len() {
        ctx.grandparent = ctx.parent;
        ctx.parent = Some(parent);
        ctx.child_index = i;
        ctx.reset_node_removed();

        traverse_node(ctx, &mut children[i]);

        if ctx.was_node_removed() {
            // Node was removed, don't increment i
        } else {
            i += 1;
        }
    }
}

/// Traverse a single node
pub fn traverse_node<'a>(ctx: &mut TransformContext<'a>, node: &mut TemplateChildNode<'a>) {
    ctx.current_node = Some(node as *mut _);

    // Collect exit functions from transforms
    let mut exit_fns: std::vec::Vec<ExitFn<'a>> = std::vec::Vec::new();

    // Apply node transforms based on node type
    match node {
        TemplateChildNode::Element(el) => {
            // Check for structural directives first
            let structural_result = check_structural_directive(el);

            if let Some((dir_name, exp, exp_loc)) = structural_result {
                // Remove the directive from props
                remove_structural_directive(el, &dir_name);

                // Handle the structural directive
                match dir_name.as_str() {
                    "if" => {
                        if let Some(exits) = transform_v_if(ctx, exp.as_ref(), exp_loc, true) {
                            exit_fns.extend(exits);
                        }
                    }
                    "else-if" | "else" => {
                        if let Some(exits) = transform_v_if(ctx, exp.as_ref(), exp_loc, false) {
                            exit_fns.extend(exits);
                        }
                    }
                    "for" => {
                        if let Some(exits) = transform_v_for(ctx, exp.as_ref(), exp_loc) {
                            exit_fns.extend(exits);
                        }
                    }
                    _ => {}
                }

                // If node was replaced (e.g., by v-if transform), we need to traverse the new node
                if let Some(current_ptr) = ctx.current_node {
                    let current = unsafe { &mut *current_ptr };
                    match current {
                        TemplateChildNode::If(if_node) => {
                            // Traverse if branches that were just created
                            for i in 0..if_node.branches.len() {
                                let branch_ptr = &mut if_node.branches[i] as *mut IfBranchNode<'a>;
                                traverse_children(ctx, ParentNode::IfBranch(branch_ptr));
                            }
                            // Run exit functions and return early
                            for exit_fn in exit_fns.into_iter().rev() {
                                exit_fn(ctx);
                            }
                            return;
                        }
                        TemplateChildNode::For(for_node) => {
                            // Enter v-for scope with aliases
                            let value = for_node.value_alias.as_ref().and_then(|e| {
                                if let ExpressionNode::Simple(exp) = e {
                                    Some(exp.content.as_str())
                                } else {
                                    None
                                }
                            });
                            let key = for_node.key_alias.as_ref().and_then(|e| {
                                if let ExpressionNode::Simple(exp) = e {
                                    Some(exp.content.as_str())
                                } else {
                                    None
                                }
                            });
                            let index = for_node.object_index_alias.as_ref().and_then(|e| {
                                if let ExpressionNode::Simple(exp) = e {
                                    Some(exp.content.as_str())
                                } else {
                                    None
                                }
                            });
                            let source = match &for_node.source {
                                ExpressionNode::Simple(exp) => exp.content.as_str(),
                                ExpressionNode::Compound(c) => c.loc.source.as_str(),
                            };
                            ctx.enter_v_for_scope(value, key, index, source);

                            // Traverse for children
                            let for_ptr = for_node.as_mut() as *mut ForNode<'a>;
                            traverse_children(ctx, ParentNode::For(for_ptr));

                            // Exit v-for scope
                            ctx.exit_scope();

                            // Add helpers
                            ctx.helper(RuntimeHelper::RenderList);
                            ctx.helper(RuntimeHelper::Fragment);

                            // Run exit functions and return early
                            for exit_fn in exit_fns.into_iter().rev() {
                                exit_fn(ctx);
                            }
                            return;
                        }
                        TemplateChildNode::Element(el) => {
                            // Still an element, process it
                            if let Some(exits) = transform_element(ctx, el) {
                                exit_fns.extend(exits);
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Node was removed, return early
                    return;
                }
            } else {
                // No structural directive, process element normally
                if let Some(exits) = transform_element(ctx, el) {
                    exit_fns.extend(exits);
                }
            }
        }
        TemplateChildNode::Interpolation(interp) => {
            transform_interpolation(ctx, interp);
        }
        TemplateChildNode::Text(_) => {
            ctx.helper(RuntimeHelper::CreateText);
        }
        TemplateChildNode::Comment(_) => {
            ctx.helper(RuntimeHelper::CreateComment);
        }
        TemplateChildNode::If(if_node) => {
            // Traverse if branches
            for i in 0..if_node.branches.len() {
                let branch_ptr = &mut if_node.branches[i] as *mut IfBranchNode<'a>;
                traverse_children(ctx, ParentNode::IfBranch(branch_ptr));
            }
        }
        TemplateChildNode::For(for_node) => {
            // Enter v-for scope with aliases
            let value = for_node.value_alias.as_ref().and_then(|e| {
                if let ExpressionNode::Simple(exp) = e {
                    Some(exp.content.as_str())
                } else {
                    None
                }
            });
            let key = for_node.key_alias.as_ref().and_then(|e| {
                if let ExpressionNode::Simple(exp) = e {
                    Some(exp.content.as_str())
                } else {
                    None
                }
            });
            let index = for_node.object_index_alias.as_ref().and_then(|e| {
                if let ExpressionNode::Simple(exp) = e {
                    Some(exp.content.as_str())
                } else {
                    None
                }
            });
            let source = match &for_node.source {
                ExpressionNode::Simple(exp) => exp.content.as_str(),
                ExpressionNode::Compound(c) => c.loc.source.as_str(),
            };
            ctx.enter_v_for_scope(value, key, index, source);

            // Traverse for children
            let for_ptr = for_node.as_mut() as *mut ForNode<'a>;
            traverse_children(ctx, ParentNode::For(for_ptr));

            // Exit v-for scope
            ctx.exit_scope();

            // Add helpers
            ctx.helper(RuntimeHelper::RenderList);
            ctx.helper(RuntimeHelper::Fragment);
        }
        _ => {}
    }

    // Traverse children for element nodes
    if let TemplateChildNode::Element(el) = node {
        let el_ptr = el.as_mut() as *mut ElementNode<'a>;
        traverse_children(ctx, ParentNode::Element(el_ptr));
    }

    // Call exit functions in reverse order
    ctx.current_node = Some(node as *mut _);
    for exit_fn in exit_fns.into_iter().rev() {
        exit_fn(ctx);
    }
}
