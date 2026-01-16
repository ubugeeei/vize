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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Type of the provided value (if available from TypeScript).
    pub value_type: Option<CompactString>,
    /// Whether this provide is inside an imported composable function.
    pub from_composable: Option<CompactString>,
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
    /// Expected type of the injected value (if available).
    pub expected_type: Option<CompactString>,
    /// The destructure pattern used (if any)
    pub pattern: InjectPattern,
    /// Whether this inject is inside an imported composable function.
    pub from_composable: Option<CompactString>,
    pub start: u32,
    pub end: u32,
}

/// A composable function call (use* function) at top-level of setup.
#[derive(Debug, Clone)]
pub struct ComposableCall {
    /// Name of the composable function.
    pub name: CompactString,
    /// Source module path.
    pub source: CompactString,
    /// Local binding name (if assigned).
    pub local_name: Option<CompactString>,
    /// Whether this composable uses provide().
    pub uses_provide: bool,
    /// Whether this composable uses inject().
    pub uses_inject: bool,
    /// Whether this composable uses reactive APIs.
    pub uses_reactivity: bool,
    pub start: u32,
    pub end: u32,
}

/// Tracks provide/inject during analysis
#[derive(Debug, Default)]
pub struct ProvideInjectTracker {
    provides: Vec<ProvideEntry>,
    injects: Vec<InjectEntry>,
    composables: Vec<ComposableCall>,
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
        value_type: Option<CompactString>,
        from_composable: Option<CompactString>,
        start: u32,
        end: u32,
    ) -> ProviderId {
        let id = ProviderId::new(self.next_id);
        self.next_id += 1;

        self.provides.push(ProvideEntry {
            id,
            key,
            value,
            value_type,
            from_composable,
            start,
            end,
        });

        id
    }

    /// Add an inject() call
    #[allow(clippy::too_many_arguments)]
    pub fn add_inject(
        &mut self,
        key: ProvideKey,
        local_name: CompactString,
        default_value: Option<CompactString>,
        expected_type: Option<CompactString>,
        pattern: InjectPattern,
        from_composable: Option<CompactString>,
        start: u32,
        end: u32,
    ) {
        self.injects.push(InjectEntry {
            key,
            local_name,
            default_value,
            expected_type,
            pattern,
            from_composable,
            start,
            end,
        });
    }

    /// Add a composable function call
    #[allow(clippy::too_many_arguments)]
    pub fn add_composable(
        &mut self,
        name: CompactString,
        source: CompactString,
        local_name: Option<CompactString>,
        uses_provide: bool,
        uses_inject: bool,
        uses_reactivity: bool,
        start: u32,
        end: u32,
    ) {
        self.composables.push(ComposableCall {
            name,
            source,
            local_name,
            uses_provide,
            uses_inject,
            uses_reactivity,
            start,
            end,
        });
    }

    /// Get all composable calls
    #[inline]
    pub fn composables(&self) -> &[ComposableCall] {
        &self.composables
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
            Some(CompactString::new("string")),
            None,
            0,
            20,
        );

        tracker.add_inject(
            ProvideKey::String(CompactString::new("theme")),
            CompactString::new("theme"),
            Some(CompactString::new("'light'")),
            Some(CompactString::new("string")),
            InjectPattern::Simple,
            None,
            30,
            50,
        );

        assert_eq!(tracker.provides().len(), 1);
        assert_eq!(tracker.injects().len(), 1);
    }

    #[test]
    fn test_composable_tracking() {
        let mut tracker = ProvideInjectTracker::new();

        tracker.add_composable(
            CompactString::new("useTheme"),
            CompactString::new("./composables/theme"),
            Some(CompactString::new("theme")),
            true,
            false,
            true,
            0,
            20,
        );

        assert_eq!(tracker.composables().len(), 1);
        assert!(tracker.composables()[0].uses_provide);
        assert!(tracker.composables()[0].uses_reactivity);
    }
}
