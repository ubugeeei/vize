//! Carton - The artist's toolbox for Vize.
//!
//! This crate provides the foundational utilities and data structures for the Vize compiler,
//! much like a carton (artist's portfolio case) holds all the essential tools and materials
//! an artist needs for their work.
//!
//! # Modules
//!
//! - **Allocator**: Arena-based memory allocation for efficient AST construction
//! - **Shared utilities**: DOM configuration, optimization flags, and helper functions
//!
//! # Example
//!
//! ```
//! use vize_carton::{Allocator, Box, Vec};
//!
//! let allocator = Allocator::default();
//!
//! // Allocate a boxed value
//! let boxed = Box::new_in(42, allocator.as_bump());
//! assert_eq!(*boxed, 42);
//!
//! // Create a vector
//! let mut vec = Vec::new_in(allocator.as_bump());
//! vec.push(1);
//! vec.push(2);
//! vec.push(3);
//! assert_eq!(vec.len(), 3);
//! ```

// Allocator modules
mod allocator;
mod boxed;
mod clone_in;
mod vec;

// Shared modules
pub mod dom_tag_config;
pub mod flags;
pub mod general;
pub mod hash;
pub mod i18n;
pub mod lsp;
pub mod profiler;
pub mod source_range;
pub mod string_builder;

// Re-export allocator types
pub use allocator::Allocator;
pub use boxed::Box;
pub use clone_in::CloneIn;
pub use vec::Vec;

// Re-export bumpalo types for convenience
pub use bumpalo::collections::String as BumpString;
pub use bumpalo::collections::Vec as BumpVec;
pub use bumpalo::Bump;

// Re-export compact_str::CompactString for convenience
pub use compact_str::CompactString;
pub use compact_str::CompactString as String;

// Re-export smallvec for stack-optimized collections
pub use smallvec::{smallvec, SmallVec};

// Re-export bitflags for flag types
pub use bitflags::bitflags;

// Re-export rustc-hash for fast hash maps/sets
pub use rustc_hash::{FxHashMap, FxHashSet};

// Re-export phf for compile-time perfect hash functions
pub use phf::{phf_map, phf_set, Map as PhfMap, Set as PhfSet};

// Re-export shared utilities
pub use dom_tag_config::*;
pub use flags::*;
pub use general::*;
