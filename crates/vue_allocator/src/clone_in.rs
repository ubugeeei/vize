//! CloneIn trait for cloning values into an arena.

use crate::{Allocator, Box, Vec};

/// Trait for cloning a value into an arena allocator.
///
/// This is similar to `Clone`, but allocates the cloned value in the given arena.
///
/// # Example
///
/// ```
/// use vue_allocator::{Allocator, Box, CloneIn};
///
/// let allocator = Allocator::default();
/// let original = Box::new_in(42, allocator.as_bump());
/// let cloned = original.clone_in(&allocator);
/// assert_eq!(*original, *cloned);
/// ```
pub trait CloneIn<'a> {
    /// The type of the cloned value.
    type Cloned;

    /// Clones the value into the given allocator.
    fn clone_in(&self, allocator: &'a Allocator) -> Self::Cloned;
}

// Implement CloneIn for primitive types that are Copy
macro_rules! impl_clone_in_for_copy {
    ($($ty:ty),*) => {
        $(
            impl<'a> CloneIn<'a> for $ty {
                type Cloned = $ty;

                #[inline]
                fn clone_in(&self, _allocator: &'a Allocator) -> Self::Cloned {
                    *self
                }
            }
        )*
    };
}

impl_clone_in_for_copy!(
    bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

// Implement CloneIn for &str -> &'a str
impl<'a> CloneIn<'a> for &str {
    type Cloned = &'a str;

    #[inline]
    fn clone_in(&self, allocator: &'a Allocator) -> Self::Cloned {
        allocator.alloc_str(self)
    }
}

// Implement CloneIn for String -> &'a str
impl<'a> CloneIn<'a> for String {
    type Cloned = &'a str;

    #[inline]
    fn clone_in(&self, allocator: &'a Allocator) -> Self::Cloned {
        allocator.alloc_str(self)
    }
}

// Implement CloneIn for Option<T>
impl<'a, T: CloneIn<'a>> CloneIn<'a> for Option<T> {
    type Cloned = Option<T::Cloned>;

    #[inline]
    fn clone_in(&self, allocator: &'a Allocator) -> Self::Cloned {
        self.as_ref().map(|v| v.clone_in(allocator))
    }
}

// Implement CloneIn for Box
impl<'old, 'new, T: CloneIn<'new>> CloneIn<'new> for Box<'old, T>
where
    T::Cloned: 'new,
{
    type Cloned = Box<'new, T::Cloned>;

    #[inline]
    fn clone_in(&self, allocator: &'new Allocator) -> Self::Cloned {
        Box::new_in((**self).clone_in(allocator), allocator.as_bump())
    }
}

// Implement CloneIn for Vec
impl<'old, 'new, T: CloneIn<'new>> CloneIn<'new> for Vec<'old, T>
where
    T::Cloned: 'new,
{
    type Cloned = Vec<'new, T::Cloned>;

    #[inline]
    fn clone_in(&self, allocator: &'new Allocator) -> Self::Cloned {
        let mut new_vec = Vec::with_capacity_in(self.len(), allocator.as_bump());
        for item in self.iter() {
            new_vec.push(item.clone_in(allocator));
        }
        new_vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone_in_primitives() {
        let allocator = Allocator::default();

        assert_eq!(42i32.clone_in(&allocator), 42);
        assert!(true.clone_in(&allocator));
        assert_eq!(2.5f64.clone_in(&allocator), 2.5);
    }

    #[test]
    fn test_clone_in_str() {
        let allocator = Allocator::default();
        let s = "hello";
        let cloned = s.clone_in(&allocator);
        assert_eq!(cloned, "hello");
    }

    #[test]
    fn test_clone_in_string() {
        let allocator = Allocator::default();
        let s = String::from("hello");
        let cloned = s.clone_in(&allocator);
        assert_eq!(cloned, "hello");
    }

    #[test]
    fn test_clone_in_option() {
        let allocator = Allocator::default();

        let some_val: Option<i32> = Some(42);
        let cloned = some_val.clone_in(&allocator);
        assert_eq!(cloned, Some(42));

        let none_val: Option<i32> = None;
        let cloned = none_val.clone_in(&allocator);
        assert_eq!(cloned, None);
    }

    #[test]
    fn test_clone_in_box() {
        let allocator = Allocator::default();
        let boxed = Box::new_in(42, allocator.as_bump());
        let cloned = boxed.clone_in(&allocator);
        assert_eq!(*cloned, 42);
    }

    #[test]
    fn test_clone_in_vec() {
        let allocator = Allocator::default();
        let mut vec = Vec::new_in(allocator.as_bump());
        vec.push(1);
        vec.push(2);
        vec.push(3);

        let cloned = vec.clone_in(&allocator);
        assert_eq!(cloned.len(), 3);
        assert_eq!(cloned[0], 1);
        assert_eq!(cloned[1], 2);
        assert_eq!(cloned[2], 3);
    }
}
