//! Fast hashing utilities using xxHash3.
//!
//! Provides high-performance hashing for HMR change detection
//! and content-based cache invalidation.

use xxhash_rust::xxh3::xxh3_64;

/// Compute a 64-bit hash of the given bytes using xxHash3.
///
/// This is the fastest hash algorithm available, suitable for
/// non-cryptographic purposes like change detection.
#[inline]
pub fn hash_bytes(data: &[u8]) -> u64 {
    xxh3_64(data)
}

/// Compute a 64-bit hash of the given string using xxHash3.
#[inline]
pub fn hash_str(data: &str) -> u64 {
    xxh3_64(data.as_bytes())
}

/// Convert a hash to a hex string (16 characters).
#[inline]
pub fn hash_to_hex(hash: u64) -> String {
    format!("{:016x}", hash)
}

/// Compute hash of a string and return as hex.
#[inline]
pub fn content_hash(content: &str) -> String {
    hash_to_hex(hash_str(content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_consistency() {
        let content = "Hello, World!";
        let hash1 = hash_str(content);
        let hash2 = hash_str(content);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_difference() {
        let hash1 = hash_str("Hello");
        let hash2 = hash_str("World");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hex_format() {
        let hash = hash_str("test");
        let hex = hash_to_hex(hash);
        assert_eq!(hex.len(), 16);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_content_hash() {
        let hash = content_hash("template content");
        assert_eq!(hash.len(), 16);
    }
}
