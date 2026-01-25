//! Text segmentation using grapheme clusters.

use compact_str::CompactString;
use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;

use super::width::TextWidth;

/// A text segment with its display width.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSegment {
    /// The grapheme cluster
    pub grapheme: CompactString,
    /// Display width in columns
    pub width: usize,
}

impl TextSegment {
    /// Create a new text segment.
    pub fn new(grapheme: impl Into<CompactString>) -> Self {
        let g: CompactString = grapheme.into();
        let width = TextWidth::width(g.as_str());
        Self { grapheme: g, width }
    }

    /// Create a segment from a single character.
    pub fn from_char(c: char) -> Self {
        Self::new(c.to_string())
    }

    /// Check if this segment is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.grapheme.is_empty()
    }

    /// Check if this is a wide character.
    #[inline]
    pub fn is_wide(&self) -> bool {
        self.width > 1
    }

    /// Check if this is a zero-width character.
    #[inline]
    pub fn is_zero_width(&self) -> bool {
        self.width == 0
    }
}

/// Iterator over text segments (grapheme clusters).
pub struct TextSegmentIter<'a> {
    inner: unicode_segmentation::Graphemes<'a>,
}

impl<'a> TextSegmentIter<'a> {
    /// Create a new iterator from a string.
    pub fn new(s: &'a str) -> Self {
        Self {
            inner: s.graphemes(true),
        }
    }
}

impl<'a> Iterator for TextSegmentIter<'a> {
    type Item = TextSegment;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(TextSegment::new)
    }
}

/// Segment a string into grapheme clusters.
pub fn segment(s: &str) -> TextSegmentIter<'_> {
    TextSegmentIter::new(s)
}

/// Segment and collect into a SmallVec.
pub fn segment_vec(s: &str) -> SmallVec<[TextSegment; 16]> {
    segment(s).collect()
}

/// Calculate display properties of segmented text.
pub struct SegmentedText {
    /// The segments
    pub segments: SmallVec<[TextSegment; 16]>,
    /// Total display width
    pub total_width: usize,
    /// Number of grapheme clusters
    pub grapheme_count: usize,
}

impl SegmentedText {
    /// Create from a string.
    pub fn new(s: &str) -> Self {
        let segments: SmallVec<[TextSegment; 16]> = segment(s).collect();
        let total_width = segments.iter().map(|s| s.width).sum();
        let grapheme_count = segments.len();

        Self {
            segments,
            total_width,
            grapheme_count,
        }
    }

    /// Get segment at grapheme index.
    pub fn get(&self, index: usize) -> Option<&TextSegment> {
        self.segments.get(index)
    }

    /// Get the grapheme index at a given column position.
    pub fn index_at_column(&self, column: usize) -> Option<usize> {
        let mut col = 0;
        for (i, seg) in self.segments.iter().enumerate() {
            if col + seg.width > column {
                return Some(i);
            }
            col += seg.width;
        }
        None
    }

    /// Get the column position of a grapheme index.
    pub fn column_at_index(&self, index: usize) -> usize {
        self.segments.iter().take(index).map(|s| s.width).sum()
    }

    /// Slice the segmented text by grapheme indices.
    pub fn slice(&self, start: usize, end: usize) -> String {
        self.segments
            .iter()
            .skip(start)
            .take(end - start)
            .map(|s| s.grapheme.as_str())
            .collect()
    }

    /// Slice by column positions.
    pub fn slice_columns(&self, start_col: usize, end_col: usize) -> String {
        let mut result = String::new();
        let mut col = 0;

        for seg in &self.segments {
            if col >= end_col {
                break;
            }
            if col + seg.width > start_col {
                result.push_str(&seg.grapheme);
            }
            col += seg.width;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_ascii() {
        let segments: Vec<_> = segment("Hello").collect();
        assert_eq!(segments.len(), 5);
        assert_eq!(segments[0].grapheme.as_str(), "H");
        assert_eq!(segments[0].width, 1);
    }

    #[test]
    fn test_segment_cjk() {
        let segments: Vec<_> = segment("あいう").collect();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].grapheme.as_str(), "あ");
        assert_eq!(segments[0].width, 2);
    }

    #[test]
    fn test_segment_mixed() {
        let segments: Vec<_> = segment("Hi世界").collect();
        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0].width, 1); // H
        assert_eq!(segments[2].width, 2); // 世
    }

    #[test]
    fn test_segment_emoji() {
        // Emoji with skin tone modifier should be one grapheme
        let segments: Vec<_> = segment("👋🏻").collect();
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn test_segmented_text() {
        let st = SegmentedText::new("Hello世界");
        assert_eq!(st.grapheme_count, 7);
        assert_eq!(st.total_width, 9); // 5 + 2*2
    }

    #[test]
    fn test_index_at_column() {
        let st = SegmentedText::new("Hi世界");
        assert_eq!(st.index_at_column(0), Some(0)); // H
        assert_eq!(st.index_at_column(1), Some(1)); // i
        assert_eq!(st.index_at_column(2), Some(2)); // 世
        assert_eq!(st.index_at_column(3), Some(2)); // still 世 (wide char)
        assert_eq!(st.index_at_column(4), Some(3)); // 界
    }

    #[test]
    fn test_column_at_index() {
        let st = SegmentedText::new("Hi世界");
        assert_eq!(st.column_at_index(0), 0);
        assert_eq!(st.column_at_index(1), 1);
        assert_eq!(st.column_at_index(2), 2);
        assert_eq!(st.column_at_index(3), 4); // after 世
    }

    #[test]
    fn test_column_at_index_japanese_only() {
        // 5 Japanese characters, each width 2
        let st = SegmentedText::new("あいうえお");
        assert_eq!(st.grapheme_count, 5);
        assert_eq!(st.total_width, 10);

        // Cursor positions: 0=before あ, 1=after あ, 2=after い, etc.
        assert_eq!(st.column_at_index(0), 0);
        assert_eq!(st.column_at_index(1), 2);
        assert_eq!(st.column_at_index(2), 4);
        assert_eq!(st.column_at_index(3), 6);
        assert_eq!(st.column_at_index(4), 8);
        assert_eq!(st.column_at_index(5), 10); // after last character
    }

    #[test]
    fn test_column_at_index_mixed() {
        // Mixed ASCII and Japanese: "aあb" = 1 + 2 + 1 = 4 columns
        let st = SegmentedText::new("aあb");
        assert_eq!(st.grapheme_count, 3);
        assert_eq!(st.total_width, 4);

        assert_eq!(st.column_at_index(0), 0); // before 'a'
        assert_eq!(st.column_at_index(1), 1); // after 'a', before 'あ'
        assert_eq!(st.column_at_index(2), 3); // after 'あ', before 'b'
        assert_eq!(st.column_at_index(3), 4); // after 'b'
    }

    #[test]
    fn test_slice_columns_japanese() {
        let st = SegmentedText::new("あいうえお");

        // Slice from column 2 (start of い) to column 8 (end of え)
        // Should include い, う, え
        assert_eq!(st.slice_columns(2, 8), "いうえ");

        // Slice from column 0 to 4 should include あ, い
        assert_eq!(st.slice_columns(0, 4), "あい");

        // Slice starting mid-character (column 1) should still include あ
        // because あ spans columns 0-1
        assert_eq!(st.slice_columns(1, 6), "あいう");
    }

    #[test]
    fn test_cursor_wrapping_calculation() {
        // Test cursor position with wrapping
        let st = SegmentedText::new("あいうえおかきくけこ"); // 10 chars, 20 columns
        let area_width = 10;

        // Cursor at end of text (after 10 chars)
        let cursor_idx = 10;
        let cursor_col = st.column_at_index(cursor_idx);
        assert_eq!(cursor_col, 20);

        // Calculate wrapped line and column
        let cursor_line = cursor_col / area_width;
        let cursor_col_in_line = cursor_col % area_width;
        assert_eq!(cursor_line, 2);
        assert_eq!(cursor_col_in_line, 0);

        // Cursor after 5 chars (column 10)
        let cursor_col = st.column_at_index(5);
        assert_eq!(cursor_col, 10);
        let cursor_line = cursor_col / area_width;
        let cursor_col_in_line = cursor_col % area_width;
        assert_eq!(cursor_line, 1);
        assert_eq!(cursor_col_in_line, 0);

        // Cursor after 3 chars (column 6)
        let cursor_col = st.column_at_index(3);
        assert_eq!(cursor_col, 6);
        let cursor_line = cursor_col / area_width;
        let cursor_col_in_line = cursor_col % area_width;
        assert_eq!(cursor_line, 0);
        assert_eq!(cursor_col_in_line, 6);
    }

    #[test]
    fn test_slice() {
        let st = SegmentedText::new("Hello");
        assert_eq!(st.slice(1, 4), "ell");
    }
}
