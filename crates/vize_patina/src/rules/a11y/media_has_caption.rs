//! a11y/media-has-caption
//!
//! Require `<video>` and `<audio>` elements to have captions.
//!
//! Media elements should have a `<track kind="captions">` child for
//! accessibility. Alternatively, `muted` attribute or `aria-label` can
//! satisfy the requirement for some cases.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <video src="movie.mp4"></video>
//! ```
//!
//! ### Valid
//! ```vue
//! <video src="movie.mp4">
//!   <track kind="captions" src="captions.vtt" />
//! </video>
//! <video src="movie.mp4" muted></video>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, TemplateChildNode};

use super::helpers::get_static_attribute_value;
use vize_relief::ast::PropNode;

static META: RuleMeta = RuleMeta {
    name: "a11y/media-has-caption",
    description: "Require media elements to have captions",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Require media elements to have captions
#[derive(Default)]
pub struct MediaHasCaption;

fn has_caption_track(children: &[TemplateChildNode]) -> bool {
    for child in children {
        if let TemplateChildNode::Element(el) = child {
            if el.tag == "track" {
                if let Some(kind) = get_static_attribute_value(el, "kind") {
                    if kind == "captions" || kind == "descriptions" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

impl Rule for MediaHasCaption {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        if element.tag != "video" && element.tag != "audio" {
            return;
        }

        // Muted media doesn't need captions (boolean attribute - may have no value)
        let has_muted = element.props.iter().any(|prop| {
            if let PropNode::Attribute(attr) = prop {
                attr.name == "muted"
            } else {
                false
            }
        });
        if has_muted {
            return;
        }

        // aria-label satisfies the requirement
        if get_static_attribute_value(element, "aria-label").is_some() {
            return;
        }

        // aria-labelledby satisfies the requirement
        if get_static_attribute_value(element, "aria-labelledby").is_some() {
            return;
        }

        // Check for <track kind="captions"> child
        if has_caption_track(&element.children) {
            return;
        }

        ctx.warn_with_help(
            ctx.t_fmt(
                "a11y/media-has-caption.message",
                &[("tag", element.tag.as_str())],
            ),
            &element.loc,
            ctx.t("a11y/media-has-caption.help"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;

    fn create_linter() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(MediaHasCaption));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_video_with_track() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<video src="movie.mp4"><track kind="captions" src="captions.vtt" /></video>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_video_muted() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<video src="movie.mp4" muted></video>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_video_with_aria_label() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<video src="movie.mp4" aria-label="Movie clip"></video>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_video_no_captions() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<video src="movie.mp4"></video>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_audio_no_captions() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<audio src="podcast.mp3"></audio>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_component_skipped() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<VideoPlayer src="movie.mp4"></VideoPlayer>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }
}
