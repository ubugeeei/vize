//! Module registry for caching analyzed files.
//!
//! The registry stores analyzed file results and provides efficient lookup
//! and incremental update capabilities.
//!
//! ## Performance Optimizations
//!
//! - Uses `FxHashMap` for O(1) lookup with fast hashing
//! - Uses `CompactString` for filename storage (SSO for short strings)
//! - Lazy file metadata loading to avoid unnecessary I/O
//! - Source hashing for change detection without file I/O

use crate::Croquis;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use vize_carton::{CompactString, FxHashMap};

/// Unique identifier for a file in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FileId(u32);

impl FileId {
    /// Invalid file ID (sentinel value).
    pub const INVALID: Self = Self(u32::MAX);

    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[inline(always)]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.0 != u32::MAX
    }
}

/// Entry for an analyzed module in the registry.
#[derive(Debug)]
pub struct ModuleEntry {
    /// Unique file ID.
    pub id: FileId,
    /// Absolute file path.
    pub path: PathBuf,
    /// File name for display.
    pub filename: CompactString,
    /// Last modification time (for cache invalidation).
    pub mtime: Option<SystemTime>,
    /// Analysis result.
    pub analysis: Croquis,
    /// Source code hash for change detection.
    pub source_hash: u64,
    /// Whether this is a Vue SFC.
    pub is_vue_sfc: bool,
    /// Component name (extracted from filename or defineComponent).
    pub component_name: Option<CompactString>,
}

/// Registry for tracking all analyzed files in a project.
#[derive(Debug, Default)]
pub struct ModuleRegistry {
    /// Map from file path to file ID.
    path_to_id: FxHashMap<PathBuf, FileId>,
    /// Map from file ID to module entry.
    entries: FxHashMap<FileId, ModuleEntry>,
    /// Next available file ID.
    next_id: u32,
    /// Project root path.
    project_root: Option<PathBuf>,
}

impl ModuleRegistry {
    /// Create a new empty registry.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry with a project root.
    pub fn with_project_root(root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: Some(root.into()),
            ..Default::default()
        }
    }

    /// Set the project root.
    pub fn set_project_root(&mut self, root: impl Into<PathBuf>) {
        self.project_root = Some(root.into());
    }

    /// Get the project root.
    #[inline]
    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    /// Register a new file or update an existing one.
    ///
    /// Returns the file ID and whether this was a new entry.
    pub fn register(
        &mut self,
        path: impl AsRef<Path>,
        source: &str,
        analysis: Croquis,
    ) -> (FileId, bool) {
        let path = path.as_ref();
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(ref root) = self.project_root {
            root.join(path)
        } else {
            path.to_path_buf()
        };

        let source_hash = hash_source(source);

        if let Some(&existing_id) = self.path_to_id.get(&abs_path) {
            // Update existing entry
            if let Some(entry) = self.entries.get_mut(&existing_id) {
                entry.source_hash = source_hash;
                entry.analysis = analysis;
                entry.mtime = std::fs::metadata(&abs_path)
                    .ok()
                    .and_then(|m| m.modified().ok());
            }
            return (existing_id, false);
        }

        // Create new entry
        let id = FileId::new(self.next_id);
        self.next_id += 1;

        let filename = abs_path
            .file_name()
            .map(|s| CompactString::new(s.to_string_lossy().as_ref()))
            .unwrap_or_else(|| CompactString::new("unknown"));

        let is_vue_sfc = abs_path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("vue"));

        let component_name = if is_vue_sfc {
            extract_component_name(&abs_path)
        } else {
            None
        };

        let entry = ModuleEntry {
            id,
            path: abs_path.clone(),
            filename,
            mtime: std::fs::metadata(&abs_path)
                .ok()
                .and_then(|m| m.modified().ok()),
            analysis,
            source_hash,
            is_vue_sfc,
            component_name,
        };

        self.path_to_id.insert(abs_path, id);
        self.entries.insert(id, entry);

        (id, true)
    }

    /// Get a module entry by file ID.
    #[inline]
    pub fn get(&self, id: FileId) -> Option<&ModuleEntry> {
        self.entries.get(&id)
    }

    /// Get a module entry by file path.
    pub fn get_by_path(&self, path: impl AsRef<Path>) -> Option<&ModuleEntry> {
        let path = path.as_ref();
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(ref root) = self.project_root {
            root.join(path)
        } else {
            path.to_path_buf()
        };

        self.path_to_id
            .get(&abs_path)
            .and_then(|id| self.entries.get(id))
    }

    /// Get the file ID for a path.
    pub fn get_id(&self, path: impl AsRef<Path>) -> Option<FileId> {
        let path = path.as_ref();
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(ref root) = self.project_root {
            root.join(path)
        } else {
            path.to_path_buf()
        };

        self.path_to_id.get(&abs_path).copied()
    }

    /// Check if a file needs re-analysis (based on mtime).
    pub fn needs_update(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        let Some(entry) = self.get_by_path(path) else {
            return true; // Not in registry
        };

        let Some(cached_mtime) = entry.mtime else {
            return true; // No cached mtime
        };

        let Ok(meta) = std::fs::metadata(path) else {
            return true; // Can't read metadata
        };

        let Ok(current_mtime) = meta.modified() else {
            return true; // Can't get mtime
        };

        current_mtime > cached_mtime
    }

    /// Remove a file from the registry.
    pub fn remove(&mut self, path: impl AsRef<Path>) -> Option<ModuleEntry> {
        let path = path.as_ref();
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(ref root) = self.project_root {
            root.join(path)
        } else {
            path.to_path_buf()
        };

        if let Some(id) = self.path_to_id.remove(&abs_path) {
            return self.entries.remove(&id);
        }
        None
    }

    /// Clear all entries from the registry.
    pub fn clear(&mut self) {
        self.path_to_id.clear();
        self.entries.clear();
        self.next_id = 0;
    }

    /// Get the number of registered files.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the registry is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries.
    pub fn iter(&self) -> impl Iterator<Item = &ModuleEntry> {
        self.entries.values()
    }

    /// Get all Vue SFC entries.
    pub fn vue_components(&self) -> impl Iterator<Item = &ModuleEntry> {
        self.entries.values().filter(|e| e.is_vue_sfc)
    }

    /// Find entries by component name.
    pub fn find_by_component_name(&self, name: &str) -> Option<&ModuleEntry> {
        self.entries
            .values()
            .find(|e| e.component_name.as_deref() == Some(name))
    }
}

/// Hash source code for change detection.
#[inline]
fn hash_source(source: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = rustc_hash::FxHasher::default();
    source.hash(&mut hasher);
    hasher.finish()
}

/// Extract component name from file path.
///
/// For `MyComponent.vue`, returns `Some("MyComponent")`.
fn extract_component_name(path: &Path) -> Option<CompactString> {
    path.file_stem()
        .map(|s| CompactString::new(s.to_string_lossy().as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_basic() {
        let mut registry = ModuleRegistry::new();

        let (id1, is_new) = registry.register("test.vue", "<template></template>", Croquis::new());
        assert!(is_new);

        let (id2, is_new) = registry.register("test.vue", "<template></template>", Croquis::new());
        assert!(!is_new);
        assert_eq!(id1, id2);

        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_component_name_extraction() {
        let path = Path::new("/src/components/MyButton.vue");
        let name = extract_component_name(path);
        assert_eq!(name, Some(CompactString::new("MyButton")));
    }
}
