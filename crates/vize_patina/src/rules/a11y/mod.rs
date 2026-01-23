//! Accessibility (a11y) lint rules.
//!
//! These rules help ensure Vue templates are accessible to all users,
//! including those using assistive technologies.
//!
//! Based on [eslint-plugin-vuejs-accessibility](https://github.com/vue-a11y/eslint-plugin-vuejs-accessibility).

mod anchor_has_content;
mod aria_props;
mod click_events_have_key_events;
mod form_control_has_label;
mod heading_has_content;
mod iframe_has_title;
mod img_alt;
mod no_distracting_elements;
mod tabindex_no_positive;

pub use anchor_has_content::AnchorHasContent;
pub use aria_props::AriaProps;
pub use click_events_have_key_events::ClickEventsHaveKeyEvents;
pub use form_control_has_label::FormControlHasLabel;
pub use heading_has_content::HeadingHasContent;
pub use iframe_has_title::IframeHasTitle;
pub use img_alt::ImgAlt;
pub use no_distracting_elements::NoDistractingElements;
pub use tabindex_no_positive::TabindexNoPositive;
