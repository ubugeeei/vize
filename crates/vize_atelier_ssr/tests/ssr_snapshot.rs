//! SSR compiler snapshot tests.
//!
//! These tests compare the SSR compiler output against expected snapshots.
//! The snapshots are based on Vue's official compiler-ssr test cases.

use vize_atelier_ssr::compile_ssr;
use vize_carton::Bump;

/// Helper to get the compiled string content (the template literal part)
fn get_compiled_string(src: &str) -> String {
    let allocator = Bump::new();
    // Wrap in a div to avoid root-level attr injection
    let wrapped = format!("<div>{}</div>", src);
    let (_, errors, result) = compile_ssr(&allocator, &wrapped);

    if !errors.is_empty() {
        panic!("Compilation errors: {:?}", errors);
    }

    result.code
}

/// Helper to compile full template
fn compile_full(src: &str) -> String {
    let allocator = Bump::new();
    let (_, errors, result) = compile_ssr(&allocator, src);

    if !errors.is_empty() {
        panic!("Compilation errors: {:?}", errors);
    }

    result.code
}

// =============================================================================
// Text Tests
// =============================================================================

mod text {
    use super::*;

    #[test]
    fn static_text() {
        insta::assert_snapshot!(get_compiled_string("foo"));
    }

    #[test]
    fn static_text_with_template_string_special_chars() {
        insta::assert_snapshot!(get_compiled_string("`${foo}`"));
    }

    #[test]
    fn comments() {
        insta::assert_snapshot!(get_compiled_string("<!--bar-->"));
    }

    #[test]
    fn static_text_escape() {
        insta::assert_snapshot!(get_compiled_string("&lt;foo&gt;"));
    }

    #[test]
    fn nested_elements_with_static_text() {
        insta::assert_snapshot!(get_compiled_string("<span>hello</span><span>bye</span>"));
    }

    #[test]
    fn interpolation() {
        insta::assert_snapshot!(compile_full("foo {{ bar }} baz"));
    }

    #[test]
    fn nested_elements_with_interpolation() {
        insta::assert_snapshot!(compile_full(
            "<div><span>{{ foo }} bar</span><span>baz {{ qux }}</span></div>"
        ));
    }
}

// =============================================================================
// Element Tests
// =============================================================================

mod element {
    use super::*;

    #[test]
    fn basic_elements() {
        insta::assert_snapshot!(get_compiled_string("<div></div>"));
    }

    #[test]
    fn self_closing_div() {
        insta::assert_snapshot!(get_compiled_string("<div/>"));
    }

    #[test]
    fn nested_elements() {
        insta::assert_snapshot!(get_compiled_string("<span></span><span></span>"));
    }

    #[test]
    fn void_element() {
        insta::assert_snapshot!(get_compiled_string("<input>"));
    }

    #[test]
    fn static_attrs() {
        insta::assert_snapshot!(get_compiled_string(r#"<div id="foo" class="bar"></div>"#));
    }

    #[test]
    fn v_bind_class() {
        insta::assert_snapshot!(get_compiled_string(r#"<div id="foo" :class="bar"></div>"#));
    }

    #[test]
    fn static_class_plus_v_bind_class() {
        insta::assert_snapshot!(get_compiled_string(
            r#"<div class="foo" :class="bar"></div>"#
        ));
    }

    #[test]
    fn v_bind_style() {
        insta::assert_snapshot!(get_compiled_string(r#"<div id="foo" :style="bar"></div>"#));
    }

    #[test]
    fn v_bind_boolean() {
        insta::assert_snapshot!(get_compiled_string(
            r#"<input type="checkbox" :checked="checked">"#
        ));
    }

    #[test]
    fn v_bind_non_boolean() {
        insta::assert_snapshot!(get_compiled_string(r#"<div :id="id" class="bar"></div>"#));
    }

    #[test]
    fn v_bind_object() {
        insta::assert_snapshot!(get_compiled_string(r#"<div v-bind="obj"></div>"#));
    }

    #[test]
    fn should_ignore_v_on() {
        insta::assert_snapshot!(get_compiled_string(r#"<div id="foo" @click="bar"/>"#));
    }
}

// =============================================================================
// v-if Tests
// =============================================================================

mod v_if {
    use super::*;

    #[test]
    fn basic_v_if() {
        insta::assert_snapshot!(compile_full(r#"<div v-if="foo">hello</div>"#));
    }

    #[test]
    fn v_if_else() {
        insta::assert_snapshot!(compile_full(
            r#"<div v-if="foo">foo</div><div v-else>bar</div>"#
        ));
    }

    #[test]
    fn v_if_else_if_else() {
        insta::assert_snapshot!(compile_full(
            r#"<div v-if="foo">foo</div><div v-else-if="bar">bar</div><div v-else>baz</div>"#
        ));
    }

    #[test]
    fn v_if_on_template() {
        insta::assert_snapshot!(compile_full(
            r#"<template v-if="foo"><div>hello</div></template>"#
        ));
    }

    #[test]
    fn v_if_with_text() {
        insta::assert_snapshot!(compile_full(r#"<div v-if="foo">{{ msg }}</div>"#));
    }
}

// =============================================================================
// v-for Tests
// =============================================================================

mod v_for {
    use super::*;

    #[test]
    fn basic_v_for() {
        insta::assert_snapshot!(compile_full(
            r#"<div v-for="item in list">{{ item }}</div>"#
        ));
    }

    #[test]
    fn v_for_with_key() {
        insta::assert_snapshot!(compile_full(
            r#"<div v-for="(item, key) in list">{{ item }} - {{ key }}</div>"#
        ));
    }

    #[test]
    fn v_for_with_index() {
        insta::assert_snapshot!(compile_full(
            r#"<div v-for="(item, key, index) in list">{{ index }}</div>"#
        ));
    }

    #[test]
    fn v_for_on_template() {
        insta::assert_snapshot!(compile_full(
            r#"<template v-for="item in list"><div>{{ item }}</div></template>"#
        ));
    }

    #[test]
    fn nested_v_for() {
        insta::assert_snapshot!(compile_full(
            r#"<div v-for="row in rows"><span v-for="col in row">{{ col }}</span></div>"#
        ));
    }
}

// =============================================================================
// v-model Tests
// =============================================================================

mod v_model {
    use super::*;

    #[test]
    fn v_model_text_input() {
        insta::assert_snapshot!(get_compiled_string(r#"<input v-model="msg">"#));
    }

    #[test]
    fn v_model_checkbox() {
        insta::assert_snapshot!(get_compiled_string(
            r#"<input type="checkbox" v-model="checked">"#
        ));
    }

    #[test]
    fn v_model_radio() {
        insta::assert_snapshot!(get_compiled_string(
            r#"<input type="radio" v-model="picked" value="a">"#
        ));
    }

    #[test]
    fn v_model_textarea() {
        insta::assert_snapshot!(get_compiled_string(
            r#"<textarea v-model="msg"></textarea>"#
        ));
    }
}

// =============================================================================
// v-show Tests
// =============================================================================

mod v_show {
    use super::*;

    #[test]
    fn basic_v_show() {
        insta::assert_snapshot!(get_compiled_string(r#"<div v-show="foo">hello</div>"#));
    }

    #[test]
    fn v_show_with_other_attrs() {
        insta::assert_snapshot!(get_compiled_string(
            r#"<div id="foo" v-show="bar">hello</div>"#
        ));
    }
}

// =============================================================================
// Component Tests
// =============================================================================

mod component {
    use super::*;

    #[test]
    fn basic_component() {
        insta::assert_snapshot!(compile_full(r#"<Foo></Foo>"#));
    }

    #[test]
    fn component_with_children() {
        insta::assert_snapshot!(compile_full(r#"<Foo>hello</Foo>"#));
    }

    #[test]
    fn component_with_slot_content() {
        insta::assert_snapshot!(compile_full(r#"<Foo><div>slot content</div></Foo>"#));
    }
}

// =============================================================================
// Slot Tests
// =============================================================================

mod slot {
    use super::*;

    #[test]
    fn basic_slot() {
        insta::assert_snapshot!(get_compiled_string(r#"<slot></slot>"#));
    }

    #[test]
    fn named_slot() {
        insta::assert_snapshot!(get_compiled_string(r#"<slot name="header"></slot>"#));
    }

    #[test]
    fn slot_with_fallback() {
        insta::assert_snapshot!(get_compiled_string(r#"<slot>fallback content</slot>"#));
    }
}

// =============================================================================
// v-html Tests
// =============================================================================

mod v_html {
    use super::*;

    #[test]
    fn basic_v_html() {
        insta::assert_snapshot!(get_compiled_string(r#"<div v-html="content"></div>"#));
    }

    #[test]
    fn v_html_with_other_attrs() {
        insta::assert_snapshot!(get_compiled_string(
            r#"<div id="foo" v-html="content"></div>"#
        ));
    }
}

// =============================================================================
// v-text Tests
// =============================================================================

mod v_text {
    use super::*;

    #[test]
    fn basic_v_text() {
        insta::assert_snapshot!(get_compiled_string(r#"<div v-text="msg"></div>"#));
    }

    #[test]
    fn v_text_with_other_attrs() {
        insta::assert_snapshot!(get_compiled_string(r#"<div id="foo" v-text="msg"></div>"#));
    }
}

// =============================================================================
// Scope ID Tests
// =============================================================================

mod scope_id {
    use vize_atelier_ssr::{compile_ssr_with_options, SsrCompilerOptions};
    use vize_carton::Bump;

    fn compile_with_scope_id(src: &str) -> String {
        let allocator = Bump::new();
        let options = SsrCompilerOptions {
            scope_id: Some("data-v-abc123".into()),
            ..Default::default()
        };
        let (_, errors, result) = compile_ssr_with_options(&allocator, src, options);

        if !errors.is_empty() {
            panic!("Compilation errors: {:?}", errors);
        }

        result.code
    }

    #[test]
    fn element_with_scope_id() {
        insta::assert_snapshot!(compile_with_scope_id("<div>hello</div>"));
    }

    #[test]
    fn nested_with_scope_id() {
        insta::assert_snapshot!(compile_with_scope_id("<div><span>nested</span></div>"));
    }
}

// =============================================================================
// CSS Variables Tests
// =============================================================================

mod css_vars {
    use vize_atelier_ssr::{compile_ssr_with_options, SsrCompilerOptions};
    use vize_carton::Bump;

    fn compile_with_css_vars(src: &str) -> String {
        let allocator = Bump::new();
        let options = SsrCompilerOptions {
            ssr_css_vars: Some(r#"{ "color": _ctx.color }"#.into()),
            ..Default::default()
        };
        let (_, errors, result) = compile_ssr_with_options(&allocator, src, options);

        if !errors.is_empty() {
            panic!("Compilation errors: {:?}", errors);
        }

        result.code
    }

    #[test]
    fn element_with_css_vars() {
        insta::assert_snapshot!(compile_with_css_vars("<div>hello</div>"));
    }
}

// =============================================================================
// Fragment Tests
// =============================================================================

mod fragment {
    use super::*;

    #[test]
    fn multiple_root_elements() {
        insta::assert_snapshot!(compile_full("<div>a</div><div>b</div>"));
    }

    #[test]
    fn multiple_root_with_interpolation() {
        insta::assert_snapshot!(compile_full("<div>{{ a }}</div><div>{{ b }}</div>"));
    }
}

// =============================================================================
// Nested Structures Tests
// =============================================================================

mod nested {
    use super::*;

    #[test]
    fn v_if_inside_v_for() {
        insta::assert_snapshot!(compile_full(
            r#"<div v-for="item in items"><span v-if="item.ok">{{ item.text }}</span></div>"#
        ));
    }

    #[test]
    fn v_for_inside_v_if() {
        insta::assert_snapshot!(compile_full(
            r#"<div v-if="show"><span v-for="item in items">{{ item }}</span></div>"#
        ));
    }
}
