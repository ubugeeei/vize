//! Candidate list for IME conversion.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// A conversion candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// The candidate text
    pub text: CompactString,
    /// Optional annotation (reading, meaning, etc.)
    pub annotation: Option<CompactString>,
    /// Candidate type/category
    pub kind: CandidateKind,
}

impl Candidate {
    /// Create a new candidate.
    pub fn new(text: impl Into<CompactString>) -> Self {
        Self {
            text: text.into(),
            annotation: None,
            kind: CandidateKind::Normal,
        }
    }

    /// Add annotation.
    pub fn with_annotation(mut self, annotation: impl Into<CompactString>) -> Self {
        self.annotation = Some(annotation.into());
        self
    }

    /// Set kind.
    pub fn with_kind(mut self, kind: CandidateKind) -> Self {
        self.kind = kind;
        self
    }
}

/// Types of candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CandidateKind {
    /// Normal conversion
    #[default]
    Normal,
    /// From user dictionary
    UserDictionary,
    /// Learning/history based
    Learned,
    /// Prediction
    Prediction,
    /// Symbol or special character
    Symbol,
    /// Emoji
    Emoji,
}

/// A list of conversion candidates.
#[derive(Debug, Clone, Default)]
pub struct CandidateList {
    /// All candidates
    candidates: SmallVec<[Candidate; 16]>,
    /// Currently selected index
    selected: usize,
    /// Page size for display
    page_size: usize,
    /// Current page start index
    page_start: usize,
}

impl CandidateList {
    /// Create an empty candidate list.
    pub fn new() -> Self {
        Self {
            candidates: SmallVec::new(),
            selected: 0,
            page_size: 9,
            page_start: 0,
        }
    }

    /// Create with candidates.
    pub fn with_candidates(candidates: impl IntoIterator<Item = Candidate>) -> Self {
        Self {
            candidates: candidates.into_iter().collect(),
            selected: 0,
            page_size: 9,
            page_start: 0,
        }
    }

    /// Set page size.
    pub fn with_page_size(mut self, size: usize) -> Self {
        self.page_size = size.max(1);
        self
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Get number of candidates.
    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    /// Get selected index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Get selected candidate.
    pub fn selected_candidate(&self) -> Option<&Candidate> {
        self.candidates.get(self.selected)
    }

    /// Get all candidates.
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    /// Get candidates for current page.
    pub fn current_page(&self) -> &[Candidate] {
        let end = (self.page_start + self.page_size).min(self.candidates.len());
        &self.candidates[self.page_start..end]
    }

    /// Get page info (current_page, total_pages).
    pub fn page_info(&self) -> (usize, usize) {
        let total = self.candidates.len().div_ceil(self.page_size);
        let current = self.page_start / self.page_size;
        (current, total)
    }

    /// Select next candidate.
    pub fn select_next(&mut self) {
        if self.candidates.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.candidates.len();
        self.update_page();
    }

    /// Select previous candidate.
    pub fn select_prev(&mut self) {
        if self.candidates.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.candidates.len() - 1;
        } else {
            self.selected -= 1;
        }
        self.update_page();
    }

    /// Select by index (within current page, 1-9).
    pub fn select_by_number(&mut self, num: usize) -> bool {
        if num == 0 || num > self.page_size {
            return false;
        }
        let index = self.page_start + num - 1;
        if index < self.candidates.len() {
            self.selected = index;
            true
        } else {
            false
        }
    }

    /// Go to next page.
    pub fn next_page(&mut self) {
        let next = self.page_start + self.page_size;
        if next < self.candidates.len() {
            self.page_start = next;
            self.selected = self.page_start;
        }
    }

    /// Go to previous page.
    pub fn prev_page(&mut self) {
        if self.page_start >= self.page_size {
            self.page_start -= self.page_size;
            self.selected = self.page_start;
        } else if self.page_start > 0 {
            self.page_start = 0;
            self.selected = 0;
        }
    }

    /// Update page to show selected item.
    fn update_page(&mut self) {
        if self.selected < self.page_start || self.selected >= self.page_start + self.page_size {
            self.page_start = (self.selected / self.page_size) * self.page_size;
        }
    }

    /// Set candidates.
    pub fn set_candidates(&mut self, candidates: impl IntoIterator<Item = Candidate>) {
        self.candidates = candidates.into_iter().collect();
        self.selected = 0;
        self.page_start = 0;
    }

    /// Clear all candidates.
    pub fn clear(&mut self) {
        self.candidates.clear();
        self.selected = 0;
        self.page_start = 0;
    }

    /// Add a candidate.
    pub fn add(&mut self, candidate: Candidate) {
        self.candidates.push(candidate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_new() {
        let candidate = Candidate::new("日本");
        assert_eq!(candidate.text.as_str(), "日本");
        assert!(candidate.annotation.is_none());
    }

    #[test]
    fn test_candidate_with_annotation() {
        let candidate = Candidate::new("日本").with_annotation("にほん");
        assert_eq!(candidate.annotation.as_deref(), Some("にほん"));
    }

    #[test]
    fn test_candidate_list_new() {
        let list = CandidateList::new();
        assert!(list.is_empty());
        assert_eq!(list.selected(), 0);
    }

    #[test]
    fn test_candidate_list_navigation() {
        let mut list = CandidateList::with_candidates(vec![
            Candidate::new("日本"),
            Candidate::new("二本"),
            Candidate::new("にほん"),
        ]);

        assert_eq!(list.selected(), 0);
        list.select_next();
        assert_eq!(list.selected(), 1);
        list.select_next();
        assert_eq!(list.selected(), 2);
        list.select_next();
        assert_eq!(list.selected(), 0); // wraps

        list.select_prev();
        assert_eq!(list.selected(), 2); // wraps back
    }

    #[test]
    fn test_candidate_list_select_by_number() {
        let mut list = CandidateList::with_candidates(vec![
            Candidate::new("a"),
            Candidate::new("b"),
            Candidate::new("c"),
        ]);

        assert!(list.select_by_number(2));
        assert_eq!(list.selected(), 1);

        assert!(!list.select_by_number(10)); // out of range
    }

    #[test]
    fn test_candidate_list_pagination() {
        let candidates: Vec<_> = (0..25)
            .map(|i| Candidate::new(format!("item{}", i)))
            .collect();
        let mut list = CandidateList::with_candidates(candidates).with_page_size(10);

        assert_eq!(list.page_info(), (0, 3));
        assert_eq!(list.current_page().len(), 10);

        list.next_page();
        assert_eq!(list.page_info(), (1, 3));
        assert_eq!(list.selected(), 10);

        list.next_page();
        assert_eq!(list.page_info(), (2, 3));
        assert_eq!(list.current_page().len(), 5);
    }
}
