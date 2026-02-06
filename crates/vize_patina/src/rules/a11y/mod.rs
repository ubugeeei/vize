//! Accessibility (a11y) lint rules.
//!
//! These rules help ensure Vue templates are accessible to all users,
//! including those using assistive technologies.
//!
//! Based on [eslint-plugin-vuejs-accessibility](https://github.com/vue-a11y/eslint-plugin-vuejs-accessibility).

mod alt_text;
mod anchor_has_content;
mod anchor_is_valid;
mod aria_props;
mod aria_role;
mod aria_unsupported_elements;
mod click_events_have_key_events;
mod form_control_has_label;
mod heading_has_content;
mod helpers;
mod iframe_has_title;
mod img_alt;
mod interactive_supports_focus;
mod label_has_for;
mod media_has_caption;
mod mouse_events_have_key_events;
mod no_access_key;
mod no_aria_hidden_on_focusable;
mod no_autofocus;
mod no_distracting_elements;
mod no_redundant_roles;
mod no_role_presentation_on_focusable;
mod no_static_element_interactions;
mod role_has_required_aria_props;
mod tabindex_no_positive;

pub use alt_text::AltText;
pub use anchor_has_content::AnchorHasContent;
pub use anchor_is_valid::AnchorIsValid;
pub use aria_props::AriaProps;
pub use aria_role::AriaRole;
pub use aria_unsupported_elements::AriaUnsupportedElements;
pub use click_events_have_key_events::ClickEventsHaveKeyEvents;
pub use form_control_has_label::FormControlHasLabel;
pub use heading_has_content::HeadingHasContent;
pub use iframe_has_title::IframeHasTitle;
pub use img_alt::ImgAlt;
pub use interactive_supports_focus::InteractiveSupportsFocus;
pub use label_has_for::LabelHasFor;
pub use media_has_caption::MediaHasCaption;
pub use mouse_events_have_key_events::MouseEventsHaveKeyEvents;
pub use no_access_key::NoAccessKey;
pub use no_aria_hidden_on_focusable::NoAriaHiddenOnFocusable;
pub use no_autofocus::NoAutofocus;
pub use no_distracting_elements::NoDistractingElements;
pub use no_redundant_roles::NoRedundantRoles;
pub use no_role_presentation_on_focusable::NoRolePresentationOnFocusable;
pub use no_static_element_interactions::NoStaticElementInteractions;
pub use role_has_required_aria_props::RoleHasRequiredAriaProps;
pub use tabindex_no_positive::TabindexNoPositive;
