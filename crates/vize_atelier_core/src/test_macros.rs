//! Test helper macros for Vue compiler.
//!
//! Provides concise macros for testing parsing and transformation.

/// Parse a template and run assertions
///
/// # Example
/// ```ignore
/// parse_test!("<div>hello</div>" => {
///     root.children.len() == 1,
///     matches!(root.children[0], TemplateChildNode::Element(_)),
/// });
/// ```
#[macro_export]
macro_rules! parse_test {
    ($input:expr => { $($assertion:expr),* $(,)? }) => {{
        let allocator = bumpalo::Bump::new();
        let (root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        $(
            assert!($assertion, "Assertion failed: {}", stringify!($assertion));
        )*
        (root, errors)
    }};
}

/// Parse and match against expected structure
///
/// # Example
/// ```ignore
/// assert_parse!("<div></div>" => element("div"));
/// assert_parse!("{{ msg }}" => interpolation("msg"));
/// ```
#[macro_export]
macro_rules! assert_parse {
    ($input:expr => element($tag:expr)) => {{
        let allocator = bumpalo::Bump::new();
        let (root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        assert_eq!(root.children.len(), 1, "Expected 1 child");
        match &root.children[0] {
            $crate::ast::TemplateChildNode::Element(el) => {
                assert_eq!(el.tag.as_str(), $tag, "Tag mismatch");
            }
            other => panic!("Expected Element, got {:?}", other.node_type()),
        }
    }};

    ($input:expr => element($tag:expr, children: $count:expr)) => {{
        let allocator = bumpalo::Bump::new();
        let (root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        assert_eq!(root.children.len(), 1, "Expected 1 root child");
        match &root.children[0] {
            $crate::ast::TemplateChildNode::Element(el) => {
                assert_eq!(el.tag.as_str(), $tag, "Tag mismatch");
                assert_eq!(el.children.len(), $count, "Children count mismatch");
            }
            other => panic!("Expected Element, got {:?}", other.node_type()),
        }
    }};

    ($input:expr => element($tag:expr, props: [$($prop:tt),*])) => {{
        let allocator = bumpalo::Bump::new();
        let (root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        match &root.children[0] {
            $crate::ast::TemplateChildNode::Element(el) => {
                assert_eq!(el.tag.as_str(), $tag);
                let mut _i = 0;
                $(
                    assert_prop!(&el.props[_i], $prop);
                    _i += 1;
                )*
            }
            other => panic!("Expected Element, got {:?}", other.node_type()),
        }
    }};

    ($input:expr => text($content:expr)) => {{
        let allocator = bumpalo::Bump::new();
        let (root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        assert_eq!(root.children.len(), 1, "Expected 1 child");
        match &root.children[0] {
            $crate::ast::TemplateChildNode::Text(text) => {
                assert_eq!(text.content.as_str(), $content, "Text content mismatch");
            }
            other => panic!("Expected Text, got {:?}", other.node_type()),
        }
    }};

    ($input:expr => interpolation($content:expr)) => {{
        let allocator = bumpalo::Bump::new();
        let (root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        assert_eq!(root.children.len(), 1, "Expected 1 child");
        match &root.children[0] {
            $crate::ast::TemplateChildNode::Interpolation(interp) => {
                match &interp.content {
                    $crate::ast::ExpressionNode::Simple(exp) => {
                        assert_eq!(exp.content.as_str(), $content, "Expression content mismatch");
                    }
                    _ => panic!("Expected SimpleExpression"),
                }
            }
            other => panic!("Expected Interpolation, got {:?}", other.node_type()),
        }
    }};

    ($input:expr => children($count:expr)) => {{
        let allocator = bumpalo::Bump::new();
        let (root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        assert_eq!(root.children.len(), $count, "Children count mismatch");
    }};
}

/// Assert prop matches expected type
#[macro_export]
macro_rules! assert_prop {
    ($prop:expr, attr($name:expr)) => {
        match $prop {
            $crate::ast::PropNode::Attribute(attr) => {
                assert_eq!(attr.name.as_str(), $name, "Attribute name mismatch");
            }
            _ => panic!("Expected Attribute"),
        }
    };

    ($prop:expr, attr($name:expr, $value:expr)) => {
        match $prop {
            $crate::ast::PropNode::Attribute(attr) => {
                assert_eq!(attr.name.as_str(), $name, "Attribute name mismatch");
                match &attr.value {
                    Some(v) => assert_eq!(v.content.as_str(), $value, "Attribute value mismatch"),
                    None => panic!("Expected attribute value"),
                }
            }
            _ => panic!("Expected Attribute"),
        }
    };

    ($prop:expr, dir($name:expr)) => {
        match $prop {
            $crate::ast::PropNode::Directive(dir) => {
                assert_eq!(dir.name.as_str(), $name, "Directive name mismatch");
            }
            _ => panic!("Expected Directive"),
        }
    };

    ($prop:expr, dir($name:expr, arg: $arg:expr)) => {
        match $prop {
            $crate::ast::PropNode::Directive(dir) => {
                assert_eq!(dir.name.as_str(), $name, "Directive name mismatch");
                match &dir.arg {
                    Some($crate::ast::ExpressionNode::Simple(exp)) => {
                        assert_eq!(exp.content.as_str(), $arg, "Directive arg mismatch");
                    }
                    _ => panic!("Expected directive argument"),
                }
            }
            _ => panic!("Expected Directive"),
        }
    };

    ($prop:expr, dir($name:expr, exp: $exp:expr)) => {
        match $prop {
            $crate::ast::PropNode::Directive(dir) => {
                assert_eq!(dir.name.as_str(), $name, "Directive name mismatch");
                match &dir.exp {
                    Some($crate::ast::ExpressionNode::Simple(exp)) => {
                        assert_eq!(exp.content.as_str(), $exp, "Directive exp mismatch");
                    }
                    _ => panic!("Expected directive expression"),
                }
            }
            _ => panic!("Expected Directive"),
        }
    };

    ($prop:expr, dir($name:expr, arg: $arg:expr, exp: $exp:expr)) => {
        match $prop {
            $crate::ast::PropNode::Directive(dir) => {
                assert_eq!(dir.name.as_str(), $name, "Directive name mismatch");
                match &dir.arg {
                    Some($crate::ast::ExpressionNode::Simple(a)) => {
                        assert_eq!(a.content.as_str(), $arg, "Directive arg mismatch");
                    }
                    _ => panic!("Expected directive argument"),
                }
                match &dir.exp {
                    Some($crate::ast::ExpressionNode::Simple(e)) => {
                        assert_eq!(e.content.as_str(), $exp, "Directive exp mismatch");
                    }
                    _ => panic!("Expected directive expression"),
                }
            }
            _ => panic!("Expected Directive"),
        }
    };
}

/// Assert transform results
#[macro_export]
macro_rules! assert_transform {
    ($input:expr => helpers: [$($helper:ident),* $(,)?]) => {{
        let allocator = bumpalo::Bump::new();
        let (mut root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        $crate::transform::transform(&allocator, &mut root, $crate::options::TransformOptions::default(), None);
        assert!(root.transformed, "Expected root to be transformed");
        $(
            assert!(
                root.helpers.iter().any(|h| matches!(h, $crate::ast::RuntimeHelper::$helper)),
                concat!("Expected helper: ", stringify!($helper))
            );
        )*
    }};

    ($input:expr => components: [$($comp:expr),* $(,)?]) => {{
        let allocator = bumpalo::Bump::new();
        let (mut root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        $crate::transform::transform(&allocator, &mut root, $crate::options::TransformOptions::default(), None);
        $(
            assert!(
                root.components.iter().any(|c| c.as_str() == $comp),
                concat!("Expected component: ", $comp)
            );
        )*
    }};
}

/// Quick element extraction
#[macro_export]
macro_rules! get_element {
    ($root:expr) => {
        match &$root.children[0] {
            $crate::ast::TemplateChildNode::Element(el) => el,
            other => panic!("Expected Element, got {:?}", other.node_type()),
        }
    };
    ($root:expr, $index:expr) => {
        match &$root.children[$index] {
            $crate::ast::TemplateChildNode::Element(el) => el,
            other => panic!("Expected Element, got {:?}", other.node_type()),
        }
    };
}

/// Quick directive extraction from element
#[macro_export]
macro_rules! get_directive {
    ($el:expr, $name:expr) => {
        $el.props
            .iter()
            .find_map(|p| match p {
                $crate::ast::PropNode::Directive(d) if d.name.as_str() == $name => Some(d),
                _ => None,
            })
            .expect(concat!("Directive not found: ", $name))
    };
}

/// Assert codegen output contains expected content
#[macro_export]
macro_rules! assert_codegen {
    ($input:expr => contains: [$($expected:expr),* $(,)?]) => {{
        let allocator = bumpalo::Bump::new();
        let (mut root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        $crate::transform::transform(&allocator, &mut root, $crate::options::TransformOptions::default(), None);
        let result = $crate::codegen::generate(&root, $crate::options::CodegenOptions::default());
        $(
            assert!(
                result.code.contains($expected),
                "Expected codegen to contain '{}', got:\n{}", $expected, result.code
            );
        )*
        result
    }};

    ($input:expr => code_matches: $pattern:expr) => {{
        let allocator = bumpalo::Bump::new();
        let (mut root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        $crate::transform::transform(&allocator, &mut root, $crate::options::TransformOptions::default(), None);
        let result = $crate::codegen::generate(&root, $crate::options::CodegenOptions::default());
        assert!(
            result.code.contains($pattern),
            "Expected codegen to match pattern '{}', got:\n{}", $pattern, result.code
        );
        result
    }};
}

/// Compile template and return result (parse + transform + codegen)
#[macro_export]
macro_rules! compile {
    ($input:expr) => {{
        let allocator = bumpalo::Bump::new();
        let (mut root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        $crate::transform::transform(
            &allocator,
            &mut root,
            $crate::options::TransformOptions::default(),
            None,
        );
        $crate::codegen::generate(&root, $crate::options::CodegenOptions::default())
    }};

    ($input:expr, $options:expr) => {{
        let allocator = bumpalo::Bump::new();
        let (mut root, errors) = $crate::parser::parse(&allocator, $input);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        $crate::transform::transform(
            &allocator,
            &mut root,
            $crate::options::TransformOptions::default(),
            None,
        );
        $crate::codegen::generate(&root, $options)
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_assert_parse_element() {
        assert_parse!("<div></div>" => element("div"));
    }

    #[test]
    fn test_assert_parse_text() {
        assert_parse!("hello world" => text("hello world"));
    }

    #[test]
    fn test_assert_parse_interpolation() {
        assert_parse!("{{ msg }}" => interpolation("msg"));
    }

    #[test]
    fn test_assert_parse_element_with_children() {
        assert_parse!("<div><span></span></div>" => element("div", children: 1));
    }

    #[test]
    fn test_assert_transform_helpers() {
        assert_transform!("<div>hello</div>" => helpers: [CreateElementVNode]);
    }

    #[test]
    fn test_assert_transform_components() {
        assert_transform!("<MyComponent></MyComponent>" => components: ["MyComponent"]);
    }

    #[test]
    fn test_assert_codegen_element() {
        assert_codegen!("<div>hello</div>" => contains: ["_createElementBlock", "\"div\"", "\"hello\""]);
    }

    #[test]
    fn test_assert_codegen_interpolation() {
        // When prefix_identifiers is false (default), expressions are not prefixed with _ctx.
        assert_codegen!("<div>{{ msg }}</div>" => contains: ["_toDisplayString", "msg"]);
    }

    #[test]
    fn test_compile_macro() {
        let result = compile!("<div>test</div>");
        assert!(result.code.contains("_createElementBlock"));
    }
}
