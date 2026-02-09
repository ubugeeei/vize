//! DOM compiler snapshot tests.
//!
//! These tests compare the DOM compiler output against expected snapshots.

use vize_atelier_dom::compile_template;
use vize_carton::Bump;

/// Helper to get the compiled code
fn get_compiled(src: &str) -> String {
    let allocator = Bump::new();
    let (_, errors, result) = compile_template(&allocator, src);

    if !errors.is_empty() {
        panic!("Compilation errors: {:?}", errors);
    }

    format!("{}\n{}", result.preamble, result.code)
}

// =============================================================================
// Static Element Tests
// =============================================================================

mod static_element {
    use super::*;

    #[test]
    fn simple_div() {
        insta::assert_snapshot!(get_compiled("<div></div>"));
    }

    #[test]
    fn div_with_text() {
        insta::assert_snapshot!(get_compiled("<div>hello</div>"));
    }

    #[test]
    fn nested_elements() {
        insta::assert_snapshot!(get_compiled("<div><span>hello</span></div>"));
    }
}

// =============================================================================
// Interpolation Tests
// =============================================================================

mod interpolation {
    use super::*;

    #[test]
    fn simple_interpolation() {
        insta::assert_snapshot!(get_compiled("{{ msg }}"));
    }

    #[test]
    fn interpolation_in_element() {
        insta::assert_snapshot!(get_compiled("<div>{{ msg }}</div>"));
    }
}

// =============================================================================
// v-if Tests
// =============================================================================

mod v_if {
    use super::*;

    #[test]
    fn simple_v_if() {
        insta::assert_snapshot!(get_compiled(r#"<div v-if="ok">hello</div>"#));
    }

    #[test]
    fn v_if_v_else() {
        insta::assert_snapshot!(get_compiled(
            r#"<div v-if="ok">yes</div><div v-else>no</div>"#
        ));
    }
}

// =============================================================================
// v-for Tests
// =============================================================================

mod v_for {
    use super::*;

    #[test]
    fn simple_v_for() {
        insta::assert_snapshot!(get_compiled(
            r#"<div v-for="item in items">{{ item }}</div>"#
        ));
    }
}

// =============================================================================
// v-bind Tests
// =============================================================================

mod v_bind {
    use super::*;

    #[test]
    fn dynamic_id() {
        insta::assert_snapshot!(get_compiled(r#"<div :id="foo"></div>"#));
    }

    #[test]
    fn dynamic_class() {
        insta::assert_snapshot!(get_compiled(r#"<div :class="cls"></div>"#));
    }
}

// =============================================================================
// v-on Tests
// =============================================================================

mod v_on {
    use super::*;

    #[test]
    fn click_handler() {
        insta::assert_snapshot!(get_compiled(r#"<div @click="handler"></div>"#));
    }
}

// =============================================================================
// v-model Tests
// =============================================================================

mod v_model {
    use super::*;

    #[test]
    fn input_text() {
        insta::assert_snapshot!(get_compiled(r#"<input v-model="msg" />"#));
    }
}

// =============================================================================
// v-show Tests
// =============================================================================

mod v_show {
    use super::*;

    #[test]
    fn simple_v_show() {
        insta::assert_snapshot!(get_compiled(r#"<div v-show="visible">content</div>"#));
    }
}

// =============================================================================
// Component Tests
// =============================================================================

mod component {
    use super::*;

    #[test]
    fn simple_component() {
        insta::assert_snapshot!(get_compiled("<MyComponent></MyComponent>"));
    }
}
