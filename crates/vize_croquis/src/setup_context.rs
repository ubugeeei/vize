//! Setup context violation tracking.
//!
//! Detects Vue APIs called outside of setup context, which can cause:
//! - CSRP (Cross-request State Pollution) in SSR
//! - Memory leaks from watchers/effects not being cleaned up

use vize_carton::CompactString;

/// Kind of setup context violation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SetupContextViolationKind {
    /// Module-level ref/reactive - CSRP risk
    ModuleLevelState = 0,
    /// Module-level watch/watchEffect - memory leak
    ModuleLevelWatch = 1,
    /// Module-level computed - memory leak
    ModuleLevelComputed = 2,
    /// Module-level provide - invalid, will throw
    ModuleLevelProvide = 3,
    /// Module-level inject - invalid, will throw
    ModuleLevelInject = 4,
    /// Module-level lifecycle hook - invalid, will throw
    ModuleLevelLifecycle = 5,
}

impl SetupContextViolationKind {
    /// Get display string for the violation kind
    #[inline]
    pub const fn to_display(self) -> &'static str {
        match self {
            Self::ModuleLevelState => "module-level-state",
            Self::ModuleLevelWatch => "module-level-watch",
            Self::ModuleLevelComputed => "module-level-computed",
            Self::ModuleLevelProvide => "module-level-provide",
            Self::ModuleLevelInject => "module-level-inject",
            Self::ModuleLevelLifecycle => "module-level-lifecycle",
        }
    }

    /// Get severity level (error > warning > info)
    #[inline]
    pub const fn severity(self) -> ViolationSeverity {
        match self {
            // These will throw at runtime - error
            Self::ModuleLevelProvide | Self::ModuleLevelInject | Self::ModuleLevelLifecycle => {
                ViolationSeverity::Error
            }
            // CSRP risk - warning
            Self::ModuleLevelState => ViolationSeverity::Warning,
            // Memory leak risk - warning
            Self::ModuleLevelWatch | Self::ModuleLevelComputed => ViolationSeverity::Warning,
        }
    }

    /// Get description of the issue
    pub fn description(self) -> &'static str {
        match self {
            Self::ModuleLevelState => {
                "Module-level reactive state causes Cross-request State Pollution (CSRP) in SSR"
            }
            Self::ModuleLevelWatch => {
                "Module-level watch is never cleaned up, causing memory leaks"
            }
            Self::ModuleLevelComputed => {
                "Module-level computed is never cleaned up, causing memory leaks"
            }
            Self::ModuleLevelProvide => "provide() must be called inside setup() or <script setup>",
            Self::ModuleLevelInject => "inject() must be called inside setup() or <script setup>",
            Self::ModuleLevelLifecycle => {
                "Lifecycle hooks must be called inside setup() or <script setup>"
            }
        }
    }

    /// Determine violation kind from function name
    pub fn from_api_name(name: &str) -> Option<Self> {
        match name {
            // State APIs - CSRP risk
            "ref" | "shallowRef" | "reactive" | "shallowReactive" | "customRef" | "toRef"
            | "toRefs" => Some(Self::ModuleLevelState),

            // Watch APIs - memory leak risk
            "watch" | "watchEffect" | "watchPostEffect" | "watchSyncEffect" => {
                Some(Self::ModuleLevelWatch)
            }

            // Computed - memory leak risk
            "computed" => Some(Self::ModuleLevelComputed),

            // Provide - invalid outside setup
            "provide" => Some(Self::ModuleLevelProvide),

            // Inject - invalid outside setup
            "inject" => Some(Self::ModuleLevelInject),

            // Lifecycle hooks - invalid outside setup
            "onMounted" | "onUnmounted" | "onBeforeMount" | "onBeforeUnmount" | "onUpdated"
            | "onBeforeUpdate" | "onActivated" | "onDeactivated" | "onErrorCaptured"
            | "onRenderTracked" | "onRenderTriggered" | "onServerPrefetch" => {
                Some(Self::ModuleLevelLifecycle)
            }

            _ => None,
        }
    }
}

/// Severity level for violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ViolationSeverity {
    Info = 0,
    Warning = 1,
    Error = 2,
}

impl ViolationSeverity {
    #[inline]
    pub const fn to_display(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// A detected setup context violation
#[derive(Debug, Clone)]
pub struct SetupContextViolation {
    pub kind: SetupContextViolationKind,
    pub api_name: CompactString,
    pub start: u32,
    pub end: u32,
}

/// Tracks setup context violations during analysis
#[derive(Debug, Default)]
pub struct SetupContextTracker {
    violations: Vec<SetupContextViolation>,
}

impl SetupContextTracker {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a setup context violation
    #[inline]
    pub fn record_violation(
        &mut self,
        kind: SetupContextViolationKind,
        api_name: CompactString,
        start: u32,
        end: u32,
    ) {
        self.violations.push(SetupContextViolation {
            kind,
            api_name,
            start,
            end,
        });
    }

    /// Get all violations
    #[inline]
    pub fn violations(&self) -> &[SetupContextViolation] {
        &self.violations
    }

    /// Check if there are any violations
    #[inline]
    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    /// Get violations count
    #[inline]
    pub fn count(&self) -> usize {
        self.violations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_violation_kind_from_api() {
        assert_eq!(
            SetupContextViolationKind::from_api_name("ref"),
            Some(SetupContextViolationKind::ModuleLevelState)
        );
        assert_eq!(
            SetupContextViolationKind::from_api_name("watch"),
            Some(SetupContextViolationKind::ModuleLevelWatch)
        );
        assert_eq!(
            SetupContextViolationKind::from_api_name("computed"),
            Some(SetupContextViolationKind::ModuleLevelComputed)
        );
        assert_eq!(
            SetupContextViolationKind::from_api_name("provide"),
            Some(SetupContextViolationKind::ModuleLevelProvide)
        );
        assert_eq!(
            SetupContextViolationKind::from_api_name("inject"),
            Some(SetupContextViolationKind::ModuleLevelInject)
        );
        assert_eq!(
            SetupContextViolationKind::from_api_name("onMounted"),
            Some(SetupContextViolationKind::ModuleLevelLifecycle)
        );
        assert_eq!(SetupContextViolationKind::from_api_name("unknown"), None);
    }

    #[test]
    fn test_tracker() {
        let mut tracker = SetupContextTracker::new();

        tracker.record_violation(
            SetupContextViolationKind::ModuleLevelState,
            CompactString::new("ref"),
            0,
            10,
        );

        assert!(tracker.has_violations());
        assert_eq!(tracker.count(), 1);
        assert_eq!(
            tracker.violations()[0].kind,
            SetupContextViolationKind::ModuleLevelState
        );
    }
}
