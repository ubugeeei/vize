//! HTML conformance lint rules.
//!
//! These rules check whether HTML in Vue templates conforms to
//! the WHATWG HTML Living Standard. Based on markuplint.

pub mod helpers;

mod deprecated_attr;
mod deprecated_element;
mod id_duplication;
mod no_consecutive_br;
mod no_duplicate_dt;
mod no_empty_palpable_content;
mod require_datetime;

pub use deprecated_attr::DeprecatedAttr;
pub use deprecated_element::DeprecatedElement;
pub use id_duplication::IdDuplication;
pub use no_consecutive_br::NoConsecutiveBr;
pub use no_duplicate_dt::NoDuplicateDt;
pub use no_empty_palpable_content::NoEmptyPalpableContent;
pub use require_datetime::RequireDatetime;
