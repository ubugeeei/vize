//! Type-aware lint rules.
//!
//! These rules require semantic analysis (Croquis) and/or type
//! information from external type checkers (tsgo) to function properly.
//!
//! ## Rule Categories
//!
//! ### Script Rules
//! - `type/require-typed-props` - Require type definition for defineProps
//! - `type/require-typed-emits` - Require type definition for defineEmits
//! - `type/no-floating-promises` - Disallow unhandled Promise results
//!
//! ### Template Rules
//! - `type/no-unsafe-template-binding` - Disallow type-unsafe template bindings

mod no_floating_promises;
mod require_typed_emits;
mod require_typed_props;

pub use no_floating_promises::NoFloatingPromises;
pub use require_typed_emits::RequireTypedEmits;
pub use require_typed_props::RequireTypedProps;
