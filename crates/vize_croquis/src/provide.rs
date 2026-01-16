//! Provide/Inject tracking for Vue components.

use vize_carton::CompactString;

/// Provider identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProviderId(u32);

impl ProviderId {
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[inline(always)]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Key used for provide/inject
#[derive(Debug, Clone)]
pub enum ProvideKey {
    /// String key
    String(CompactString),
    /// Symbol key
    Symbol(CompactString),
}

/// A provide() call
#[derive(Debug, Clone)]
pub struct ProvideEntry {
    pub id: ProviderId,
    pub key: ProvideKey,
    pub value: CompactString,
    pub start: u32,
    pub end: u32,
}

/// Destructure pattern for inject results
#[derive(Debug, Clone, Default)]
pub enum InjectPattern {
    /// Simple assignment: `const foo = inject('foo')`
    #[default]
    Simple,
    /// Object destructuring: `const { a, b } = inject('foo')`
    ObjectDestructure(Vec<CompactString>),
    /// Array destructuring: `const [a, b] = inject('foo')`
    ArrayDestructure(Vec<CompactString>),
}

/// An inject() call
#[derive(Debug, Clone)]
pub struct InjectEntry {
    pub key: ProvideKey,
    pub local_name: CompactString,
    pub default_value: Option<CompactString>,
    /// The destructure pattern used (if any)
    pub pattern: InjectPattern,
    pub start: u32,
    pub end: u32,
}

/// Tracks provide/inject during analysis
#[derive(Debug, Default)]
pub struct ProvideInjectTracker {
    provides: Vec<ProvideEntry>,
    injects: Vec<InjectEntry>,
    next_id: u32,
}

impl ProvideInjectTracker {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a provide() call
    pub fn add_provide(
        &mut self,
        key: ProvideKey,
        value: CompactString,
        start: u32,
        end: u32,
    ) -> ProviderId {
        let id = ProviderId::new(self.next_id);
        self.next_id += 1;

        self.provides.push(ProvideEntry {
            id,
            key,
            value,
            start,
            end,
        });

        id
    }

    /// Add an inject() call
    pub fn add_inject(
        &mut self,
        key: ProvideKey,
        local_name: CompactString,
        default_value: Option<CompactString>,
        pattern: InjectPattern,
        start: u32,
        end: u32,
    ) {
        self.injects.push(InjectEntry {
            key,
            local_name,
            default_value,
            pattern,
            start,
            end,
        });
    }

    /// Check if any inject uses destructuring (reactivity loss risk)
    #[inline]
    pub fn has_destructured_injects(&self) -> bool {
        self.injects
            .iter()
            .any(|i| !matches!(i.pattern, InjectPattern::Simple))
    }

    /// Get injects that use destructuring
    pub fn destructured_injects(&self) -> impl Iterator<Item = &InjectEntry> {
        self.injects
            .iter()
            .filter(|i| !matches!(i.pattern, InjectPattern::Simple))
    }

    /// Get all provides
    #[inline]
    pub fn provides(&self) -> &[ProvideEntry] {
        &self.provides
    }

    /// Get all injects
    #[inline]
    pub fn injects(&self) -> &[InjectEntry] {
        &self.injects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provide_inject() {
        let mut tracker = ProvideInjectTracker::new();

        tracker.add_provide(
            ProvideKey::String(CompactString::new("theme")),
            CompactString::new("dark"),
            0,
            20,
        );

        tracker.add_inject(
            ProvideKey::String(CompactString::new("theme")),
            CompactString::new("theme"),
            Some(CompactString::new("'light'")),
            InjectPattern::Simple,
            30,
            50,
        );

        assert_eq!(tracker.provides().len(), 1);
        assert_eq!(tracker.injects().len(), 1);
    }
}
