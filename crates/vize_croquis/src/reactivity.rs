//! Reactivity tracking for Vue templates.
//!
//! Tracks reactive sources (ref, reactive, computed) and their dependencies.
//! Also detects reactivity loss patterns.

use vize_carton::{CompactString, FxHashMap, FxHashSet};

/// Unique identifier for a reactive source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ReactiveId(u32);

impl ReactiveId {
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[inline(always)]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Kind of reactive source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReactiveKind {
    /// ref()
    Ref = 0,
    /// shallowRef()
    ShallowRef = 1,
    /// reactive()
    Reactive = 2,
    /// shallowReactive()
    ShallowReactive = 3,
    /// computed()
    Computed = 4,
    /// readonly()
    Readonly = 5,
    /// shallowReadonly()
    ShallowReadonly = 6,
    /// toRef()
    ToRef = 7,
    /// toRefs()
    ToRefs = 8,
}

impl ReactiveKind {
    /// Check if this kind requires .value access
    #[inline]
    pub const fn needs_value_access(self) -> bool {
        matches!(
            self,
            Self::Ref | Self::ShallowRef | Self::Computed | Self::ToRef
        )
    }

    /// Get the kind from a function name
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "ref" => Some(Self::Ref),
            "shallowRef" => Some(Self::ShallowRef),
            "reactive" => Some(Self::Reactive),
            "shallowReactive" => Some(Self::ShallowReactive),
            "computed" => Some(Self::Computed),
            "readonly" => Some(Self::Readonly),
            "shallowReadonly" => Some(Self::ShallowReadonly),
            "toRef" => Some(Self::ToRef),
            "toRefs" => Some(Self::ToRefs),
            _ => None,
        }
    }

    /// Get display abbreviation for VIR output
    /// - st = state (ref)
    /// - ist = implicit state (reactive - no .value needed)
    /// - drv = derived (computed)
    #[inline]
    pub const fn to_display(self) -> &'static str {
        match self {
            Self::Ref | Self::ShallowRef | Self::ToRef | Self::ToRefs => "st",
            Self::Reactive | Self::ShallowReactive => "ist",
            Self::Computed => "drv",
            Self::Readonly | Self::ShallowReadonly => "ro",
        }
    }
}

/// A reactive source in the code
#[derive(Debug, Clone)]
pub struct ReactiveSource {
    pub id: ReactiveId,
    pub name: CompactString,
    pub kind: ReactiveKind,
    pub declaration_offset: u32,
}

/// Kind of reactivity loss
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactivityLossKind {
    /// Destructuring reactive object: `const { x } = reactive({...})`
    ReactiveDestructure {
        source_name: CompactString,
        destructured_props: Vec<CompactString>,
    },
    /// Destructuring ref.value: `const { x } = ref({...}).value`
    RefValueDestructure {
        source_name: CompactString,
        destructured_props: Vec<CompactString>,
    },
    /// Extracting ref.value to plain variable: `const x = ref(0); const y = x.value`
    RefValueExtract {
        source_name: CompactString,
        target_name: CompactString,
    },
    /// Spreading reactive object: `{ ...state }`
    ReactiveSpread { source_name: CompactString },
    /// Reassigning reactive variable: `let state = reactive({}); state = {}`
    ReactiveReassign { source_name: CompactString },
}

/// A detected reactivity loss
#[derive(Debug, Clone)]
pub struct ReactivityLoss {
    pub kind: ReactivityLossKind,
    pub start: u32,
    pub end: u32,
}

/// Tracks reactive sources during analysis
#[derive(Debug, Default)]
pub struct ReactivityTracker {
    sources: Vec<ReactiveSource>,
    by_name: FxHashMap<CompactString, ReactiveId>,
    /// Set of reactive variable names for quick lookup
    reactive_names: FxHashSet<CompactString>,
    /// Detected reactivity losses
    losses: Vec<ReactivityLoss>,
    next_id: u32,
}

impl ReactivityTracker {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a reactive source
    pub fn register(
        &mut self,
        name: CompactString,
        kind: ReactiveKind,
        declaration_offset: u32,
    ) -> ReactiveId {
        let id = ReactiveId::new(self.next_id);
        self.next_id += 1;

        self.reactive_names.insert(name.clone());
        self.by_name.insert(name.clone(), id);
        self.sources.push(ReactiveSource {
            id,
            name,
            kind,
            declaration_offset,
        });

        id
    }

    /// Look up a reactive source by name
    #[inline]
    pub fn lookup(&self, name: &str) -> Option<&ReactiveSource> {
        self.by_name
            .get(name)
            .and_then(|id| self.sources.get(id.as_u32() as usize))
    }

    /// Check if a name is a reactive source
    #[inline]
    pub fn is_reactive(&self, name: &str) -> bool {
        self.reactive_names.contains(name)
    }

    /// Check if a name needs .value access
    #[inline]
    pub fn needs_value_access(&self, name: &str) -> bool {
        self.lookup(name)
            .is_some_and(|s| s.kind.needs_value_access())
    }

    /// Get all reactive sources
    #[inline]
    pub fn sources(&self) -> &[ReactiveSource] {
        &self.sources
    }

    /// Get source count
    #[inline]
    pub fn count(&self) -> usize {
        self.sources.len()
    }

    /// Get reactive names set (for external lookup)
    #[inline]
    pub fn reactive_names(&self) -> &FxHashSet<CompactString> {
        &self.reactive_names
    }

    /// Add a reactivity loss
    #[inline]
    pub fn add_loss(&mut self, loss: ReactivityLoss) {
        self.losses.push(loss);
    }

    /// Record destructuring of a reactive variable
    pub fn record_destructure(
        &mut self,
        source_name: CompactString,
        destructured_props: Vec<CompactString>,
        start: u32,
        end: u32,
    ) {
        if let Some(source) = self.lookup(source_name.as_str()) {
            let kind = if source.kind.needs_value_access() {
                // ref type - destructuring ref.value
                ReactivityLossKind::RefValueDestructure {
                    source_name,
                    destructured_props,
                }
            } else {
                // reactive type
                ReactivityLossKind::ReactiveDestructure {
                    source_name,
                    destructured_props,
                }
            };
            self.losses.push(ReactivityLoss { kind, start, end });
        }
    }

    /// Record spreading of a reactive variable
    pub fn record_spread(&mut self, source_name: CompactString, start: u32, end: u32) {
        if self.is_reactive(source_name.as_str()) {
            self.losses.push(ReactivityLoss {
                kind: ReactivityLossKind::ReactiveSpread { source_name },
                start,
                end,
            });
        }
    }

    /// Record extracting ref.value to a plain variable
    pub fn record_ref_value_extract(
        &mut self,
        source_name: CompactString,
        target_name: CompactString,
        start: u32,
        end: u32,
    ) {
        if let Some(source) = self.lookup(source_name.as_str()) {
            if source.kind.needs_value_access() {
                self.losses.push(ReactivityLoss {
                    kind: ReactivityLossKind::RefValueExtract {
                        source_name,
                        target_name,
                    },
                    start,
                    end,
                });
            }
        }
    }

    /// Get all detected reactivity losses
    #[inline]
    pub fn losses(&self) -> &[ReactivityLoss] {
        &self.losses
    }

    /// Check if there are any reactivity losses
    #[inline]
    pub fn has_losses(&self) -> bool {
        !self.losses.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reactivity_tracker() {
        let mut tracker = ReactivityTracker::new();

        tracker.register(CompactString::new("count"), ReactiveKind::Ref, 0);
        tracker.register(CompactString::new("state"), ReactiveKind::Reactive, 20);

        assert!(tracker.is_reactive("count"));
        assert!(tracker.needs_value_access("count"));

        assert!(tracker.is_reactive("state"));
        assert!(!tracker.needs_value_access("state"));

        assert!(!tracker.is_reactive("unknown"));
    }

    #[test]
    fn test_reactive_destructure_loss() {
        let mut tracker = ReactivityTracker::new();
        tracker.register(CompactString::new("state"), ReactiveKind::Reactive, 0);

        tracker.record_destructure(
            CompactString::new("state"),
            vec![CompactString::new("count"), CompactString::new("name")],
            10,
            20,
        );

        assert!(tracker.has_losses());
        assert_eq!(tracker.losses().len(), 1);
        match &tracker.losses()[0].kind {
            ReactivityLossKind::ReactiveDestructure {
                source_name,
                destructured_props,
            } => {
                assert_eq!(source_name.as_str(), "state");
                assert_eq!(destructured_props.len(), 2);
            }
            _ => panic!("Expected ReactiveDestructure"),
        }
    }

    #[test]
    fn test_reactive_spread_loss() {
        let mut tracker = ReactivityTracker::new();
        tracker.register(CompactString::new("state"), ReactiveKind::Reactive, 0);

        tracker.record_spread(CompactString::new("state"), 10, 20);

        assert!(tracker.has_losses());
        assert_eq!(tracker.losses().len(), 1);
        match &tracker.losses()[0].kind {
            ReactivityLossKind::ReactiveSpread { source_name } => {
                assert_eq!(source_name.as_str(), "state");
            }
            _ => panic!("Expected ReactiveSpread"),
        }
    }

    #[test]
    fn test_ref_value_extract_loss() {
        let mut tracker = ReactivityTracker::new();
        tracker.register(CompactString::new("count"), ReactiveKind::Ref, 0);

        tracker.record_ref_value_extract(
            CompactString::new("count"),
            CompactString::new("value"),
            10,
            20,
        );

        assert!(tracker.has_losses());
        assert_eq!(tracker.losses().len(), 1);
        match &tracker.losses()[0].kind {
            ReactivityLossKind::RefValueExtract {
                source_name,
                target_name,
            } => {
                assert_eq!(source_name.as_str(), "count");
                assert_eq!(target_name.as_str(), "value");
            }
            _ => panic!("Expected RefValueExtract"),
        }
    }

    #[test]
    fn test_non_reactive_no_loss() {
        let mut tracker = ReactivityTracker::new();
        tracker.register(CompactString::new("state"), ReactiveKind::Reactive, 0);

        // Destructuring non-reactive variable should not record a loss
        tracker.record_destructure(
            CompactString::new("other"),
            vec![CompactString::new("count")],
            10,
            20,
        );

        assert!(!tracker.has_losses());
    }
}
