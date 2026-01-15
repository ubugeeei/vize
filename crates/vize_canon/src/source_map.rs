//! High-performance bidirectional source map for SFC ↔ Virtual TypeScript mapping.
//!
//! Design principles:
//! - Zero allocation in hot paths (lookups)
//! - Compact representation (u32 offsets only, no strings)
//! - O(log n) lookups via binary search in both directions
//! - Cache-friendly memory layout

use std::cmp::Ordering;

/// Compact source range using u32 offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    #[inline]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    #[inline]
    pub const fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    #[inline]
    pub const fn contains(&self, offset: u32) -> bool {
        offset >= self.start && offset < self.end
    }
}

/// Mapping feature flags packed into a single byte.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct MappingFlags(u8);

impl MappingFlags {
    pub const HOVER: u8 = 1 << 0;
    pub const COMPLETION: u8 = 1 << 1;
    pub const DEFINITION: u8 = 1 << 2;
    pub const REFERENCES: u8 = 1 << 3;
    pub const RENAME: u8 = 1 << 4;
    pub const DIAGNOSTICS: u8 = 1 << 5;
    pub const SEMANTIC_TOKENS: u8 = 1 << 6;

    pub const ALL: u8 = Self::HOVER
        | Self::COMPLETION
        | Self::DEFINITION
        | Self::REFERENCES
        | Self::RENAME
        | Self::DIAGNOSTICS
        | Self::SEMANTIC_TOKENS;

    #[inline]
    pub const fn all() -> Self {
        Self(Self::ALL)
    }

    #[inline]
    pub const fn none() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn has(&self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }

    #[inline]
    pub const fn hover_only() -> Self {
        Self(Self::HOVER | Self::DIAGNOSTICS)
    }

    #[inline]
    pub const fn navigation() -> Self {
        Self(Self::COMPLETION | Self::DEFINITION | Self::REFERENCES)
    }
}

/// Mapping context type (what kind of code this maps to).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MappingKind {
    #[default]
    Unknown = 0,
    /// Script content (1:1 mapping)
    Script = 1,
    /// Template interpolation {{ expr }}
    Interpolation = 2,
    /// Directive expression v-if="expr"
    DirectiveExpr = 3,
    /// Directive argument :prop or v-bind:prop
    DirectiveArg = 4,
    /// Event handler @click="handler"
    EventHandler = 5,
    /// v-for variable
    VForVar = 6,
    /// v-slot binding
    SlotBinding = 7,
    /// Component tag reference
    ComponentRef = 8,
}

/// Single mapping entry - compact representation.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Mapping {
    /// Range in source SFC
    pub source: Span,
    /// Range in generated virtual TS
    pub generated: Span,
    /// Feature flags
    pub flags: MappingFlags,
    /// Context type
    pub kind: MappingKind,
}

impl Mapping {
    #[inline]
    pub const fn new(source: Span, generated: Span) -> Self {
        Self {
            source,
            generated,
            flags: MappingFlags::all(),
            kind: MappingKind::Unknown,
        }
    }

    #[inline]
    pub const fn with_kind(source: Span, generated: Span, kind: MappingKind) -> Self {
        Self {
            source,
            generated,
            flags: MappingFlags::all(),
            kind,
        }
    }

    #[inline]
    pub const fn with_flags(
        source: Span,
        generated: Span,
        kind: MappingKind,
        flags: MappingFlags,
    ) -> Self {
        Self {
            source,
            generated,
            flags,
            kind,
        }
    }

    /// Map source offset to generated offset (relative within mapping).
    #[inline]
    pub fn source_to_generated(&self, src_offset: u32) -> Option<u32> {
        if !self.source.contains(src_offset) {
            return None;
        }
        let relative = src_offset - self.source.start;
        let gen_len = self.generated.len();
        // Clamp relative offset to generated range
        let clamped = if gen_len > 0 {
            relative.min(gen_len - 1)
        } else {
            0
        };
        Some(self.generated.start + clamped)
    }

    /// Map generated offset to source offset (relative within mapping).
    #[inline]
    pub fn generated_to_source(&self, gen_offset: u32) -> Option<u32> {
        if !self.generated.contains(gen_offset) {
            return None;
        }
        let relative = gen_offset - self.generated.start;
        let src_len = self.source.len();
        let clamped = if src_len > 0 {
            relative.min(src_len - 1)
        } else {
            0
        };
        Some(self.source.start + clamped)
    }
}

/// High-performance bidirectional source map.
///
/// Maintains two sorted indices for O(log n) lookups in both directions.
#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    /// Mappings sorted by source offset
    by_source: Vec<Mapping>,
    /// Indices into by_source, sorted by generated offset
    by_generated: Vec<u16>,
    /// Block offset in original SFC (e.g., template start offset)
    pub block_offset: u32,
}

impl SourceMap {
    /// Create an empty source map.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            by_source: Vec::with_capacity(cap),
            by_generated: Vec::with_capacity(cap),
            block_offset: 0,
        }
    }

    /// Set block offset (for template position in SFC).
    #[inline]
    pub fn set_block_offset(&mut self, offset: u32) {
        self.block_offset = offset;
    }

    /// Add a mapping. Call `build()` after adding all mappings.
    #[inline]
    pub fn push(&mut self, mapping: Mapping) {
        self.by_source.push(mapping);
    }

    /// Add a simple mapping with spans.
    #[inline]
    pub fn push_simple(&mut self, src_start: u32, src_end: u32, gen_start: u32, gen_end: u32) {
        self.push(Mapping::new(
            Span::new(src_start, src_end),
            Span::new(gen_start, gen_end),
        ));
    }

    /// Build indices after adding all mappings.
    /// Must be called before lookups.
    pub fn build(&mut self) {
        // Sort by source offset
        self.by_source.sort_unstable_by_key(|m| m.source.start);

        // Build generated index (indices sorted by generated offset)
        let len = self.by_source.len().min(u16::MAX as usize);
        self.by_generated = (0..len as u16).collect();
        self.by_generated.sort_unstable_by_key(|&idx| {
            self.by_source
                .get(idx as usize)
                .map(|m| m.generated.start)
                .unwrap_or(u32::MAX)
        });
    }

    /// Number of mappings.
    #[inline]
    pub fn len(&self) -> usize {
        self.by_source.len()
    }

    /// Check if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.by_source.is_empty()
    }

    /// Get all mappings (sorted by source offset).
    #[inline]
    pub fn mappings(&self) -> &[Mapping] {
        &self.by_source
    }

    /// Find mapping containing source offset via binary search.
    /// O(log n) complexity.
    #[inline]
    pub fn find_by_source(&self, offset: u32) -> Option<&Mapping> {
        let idx = self
            .by_source
            .binary_search_by(|m| {
                if m.source.end <= offset {
                    Ordering::Less
                } else if m.source.start > offset {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            })
            .ok()?;
        self.by_source.get(idx)
    }

    /// Find mapping containing generated offset via binary search.
    /// O(log n) complexity.
    #[inline]
    pub fn find_by_generated(&self, offset: u32) -> Option<&Mapping> {
        let idx = self
            .by_generated
            .binary_search_by(|&i| {
                let m = &self.by_source[i as usize];
                if m.generated.end <= offset {
                    Ordering::Less
                } else if m.generated.start > offset {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            })
            .ok()?;
        let mapping_idx = self.by_generated.get(idx)?;
        self.by_source.get(*mapping_idx as usize)
    }

    /// Map source offset to generated offset.
    /// O(log n) complexity.
    #[inline]
    pub fn to_generated(&self, src_offset: u32) -> Option<u32> {
        self.find_by_source(src_offset)?
            .source_to_generated(src_offset)
    }

    /// Map generated offset to source offset (with block_offset applied).
    /// O(log n) complexity.
    #[inline]
    pub fn to_source(&self, gen_offset: u32) -> Option<u32> {
        self.find_by_generated(gen_offset)?
            .generated_to_source(gen_offset)
            .map(|o| o + self.block_offset)
    }

    /// Map source offset to generated, with feature check.
    #[inline]
    pub fn to_generated_if(&self, src_offset: u32, flag: u8) -> Option<u32> {
        let mapping = self.find_by_source(src_offset)?;
        if mapping.flags.has(flag) {
            mapping.source_to_generated(src_offset)
        } else {
            None
        }
    }

    /// Map generated offset to source, with feature check.
    #[inline]
    pub fn to_source_if(&self, gen_offset: u32, flag: u8) -> Option<u32> {
        let mapping = self.find_by_generated(gen_offset)?;
        if mapping.flags.has(flag) {
            mapping
                .generated_to_source(gen_offset)
                .map(|o| o + self.block_offset)
        } else {
            None
        }
    }

    /// Map source range to generated range.
    #[inline]
    pub fn source_range_to_generated(&self, source: Span) -> Option<Span> {
        let start = self.to_generated(source.start)?;
        let end = self
            .to_generated(source.end.saturating_sub(1))
            .map(|e| e + 1)?;
        Some(Span::new(start, end))
    }

    /// Map generated range to source range.
    #[inline]
    pub fn generated_range_to_source(&self, generated: Span) -> Option<Span> {
        let start = self.to_source(generated.start)?;
        let end = self
            .to_source(generated.end.saturating_sub(1))
            .map(|e| e + 1)?;
        Some(Span::new(start, end))
    }

    /// Iterator over all mappings of a specific kind.
    #[inline]
    pub fn iter_kind(&self, kind: MappingKind) -> impl Iterator<Item = &Mapping> {
        self.by_source.iter().filter(move |m| m.kind == kind)
    }
}

/// Position in source (line/column, 0-indexed).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

impl Position {
    #[inline]
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

/// Convert byte offset to line/column position.
/// O(n) but uses byte iteration for speed.
#[inline]
pub fn offset_to_position(source: &str, offset: u32) -> Position {
    let offset = offset as usize;
    let bytes = source.as_bytes();
    let len = bytes.len().min(offset);

    let mut line = 0u32;
    let mut last_newline = 0usize;

    for (i, &byte) in bytes[..len].iter().enumerate() {
        if byte == b'\n' {
            line += 1;
            last_newline = i + 1;
        }
    }

    Position {
        line,
        column: (len - last_newline) as u32,
    }
}

/// Convert line/column position to byte offset.
/// O(n) but uses byte iteration for speed.
#[inline]
pub fn position_to_offset(source: &str, pos: Position) -> Option<u32> {
    let bytes = source.as_bytes();
    let mut current_line = 0u32;
    let mut line_start = 0usize;

    for (i, &byte) in bytes.iter().enumerate() {
        if current_line == pos.line {
            let offset = line_start + pos.column as usize;
            return if offset <= bytes.len() {
                Some(offset as u32)
            } else {
                None
            };
        }
        if byte == b'\n' {
            current_line += 1;
            line_start = i + 1;
        }
    }

    // Handle last line (no trailing newline)
    if current_line == pos.line {
        let offset = line_start + pos.column as usize;
        if offset <= bytes.len() {
            return Some(offset as u32);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_contains() {
        let span = Span::new(10, 20);
        assert!(!span.contains(9));
        assert!(span.contains(10));
        assert!(span.contains(15));
        assert!(span.contains(19));
        assert!(!span.contains(20));
    }

    #[test]
    fn test_mapping_source_to_generated() {
        let mapping = Mapping::new(Span::new(10, 20), Span::new(100, 110));
        assert_eq!(mapping.source_to_generated(10), Some(100));
        assert_eq!(mapping.source_to_generated(15), Some(105));
        assert_eq!(mapping.source_to_generated(19), Some(109));
        assert_eq!(mapping.source_to_generated(9), None);
        assert_eq!(mapping.source_to_generated(20), None);
    }

    #[test]
    fn test_source_map_lookup() {
        let mut map = SourceMap::with_capacity(2);
        map.push_simple(10, 20, 100, 110);
        map.push_simple(30, 40, 200, 210);
        map.build();

        assert_eq!(map.to_generated(15), Some(105));
        assert_eq!(map.to_generated(35), Some(205));
        assert_eq!(map.to_generated(25), None);
    }

    #[test]
    fn test_source_map_reverse_lookup() {
        let mut map = SourceMap::with_capacity(2);
        map.push_simple(10, 20, 100, 110);
        map.push_simple(30, 40, 200, 210);
        map.build();

        assert_eq!(map.to_source(105), Some(15));
        assert_eq!(map.to_source(205), Some(35));
        assert_eq!(map.to_source(150), None);
    }

    #[test]
    fn test_source_map_with_block_offset() {
        let mut map = SourceMap::with_capacity(1);
        map.set_block_offset(50);
        map.push_simple(10, 20, 100, 110);
        map.build();

        assert_eq!(map.to_source(105), Some(65)); // 15 + 50
    }

    #[test]
    fn test_offset_to_position() {
        let source = "line1\nline2\nline3";
        assert_eq!(offset_to_position(source, 0), Position::new(0, 0));
        assert_eq!(offset_to_position(source, 5), Position::new(0, 5));
        assert_eq!(offset_to_position(source, 6), Position::new(1, 0));
        assert_eq!(offset_to_position(source, 8), Position::new(1, 2));
        assert_eq!(offset_to_position(source, 12), Position::new(2, 0));
    }

    #[test]
    fn test_position_to_offset() {
        let source = "line1\nline2\nline3";
        assert_eq!(position_to_offset(source, Position::new(0, 0)), Some(0));
        assert_eq!(position_to_offset(source, Position::new(0, 5)), Some(5));
        assert_eq!(position_to_offset(source, Position::new(1, 0)), Some(6));
        assert_eq!(position_to_offset(source, Position::new(1, 2)), Some(8));
        assert_eq!(position_to_offset(source, Position::new(2, 0)), Some(12));
    }

    #[test]
    fn test_mapping_flags() {
        let flags = MappingFlags::all();
        assert!(flags.has(MappingFlags::HOVER));
        assert!(flags.has(MappingFlags::COMPLETION));

        let hover_only = MappingFlags::hover_only();
        assert!(hover_only.has(MappingFlags::HOVER));
        assert!(hover_only.has(MappingFlags::DIAGNOSTICS));
        assert!(!hover_only.has(MappingFlags::COMPLETION));
    }
}
