//! Server state management.

use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::OnceCell;
use tower_lsp::lsp_types::Url;

#[cfg(feature = "native")]
use std::sync::OnceLock;

#[cfg(feature = "native")]
use vize_canon::{BatchTypeChecker, BatchTypeCheckerTrait, TsgoBridge, TsgoBridgeConfig};

use crate::document::DocumentStore;
use crate::virtual_code::{VirtualCodeGenerator, VirtualDocuments};

/// Batch type check result cache.
#[cfg(feature = "native")]
pub struct BatchTypeCheckCache {
    /// Diagnostics per file.
    pub diagnostics: DashMap<PathBuf, Vec<vize_canon::BatchDiagnostic>>,
    /// Whether the cache is valid.
    pub valid: std::sync::atomic::AtomicBool,
}

#[cfg(feature = "native")]
impl BatchTypeCheckCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            diagnostics: DashMap::new(),
            valid: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Invalidate the cache.
    pub fn invalidate(&self) {
        self.valid.store(false, std::sync::atomic::Ordering::SeqCst);
        self.diagnostics.clear();
    }

    /// Check if the cache is valid.
    pub fn is_valid(&self) -> bool {
        self.valid.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Mark the cache as valid.
    pub fn mark_valid(&self) {
        self.valid.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Get diagnostics for a file.
    pub fn get_diagnostics(&self, path: &PathBuf) -> Vec<vize_canon::BatchDiagnostic> {
        self.diagnostics
            .get(path)
            .map(|d| d.clone())
            .unwrap_or_default()
    }

    /// Set diagnostics for a file.
    pub fn set_diagnostics(&self, path: PathBuf, diagnostics: Vec<vize_canon::BatchDiagnostic>) {
        self.diagnostics.insert(path, diagnostics);
    }
}

#[cfg(feature = "native")]
impl Default for BatchTypeCheckCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Server state containing all runtime data.
pub struct ServerState {
    /// Document store for managing open documents
    pub documents: DocumentStore,
    /// Virtual code generator (reusable)
    virtual_gen: RwLock<VirtualCodeGenerator>,
    /// Cached virtual documents per file
    virtual_docs_cache: DashMap<Url, VirtualDocuments>,
    /// Formatting options (loaded from vize.config.json)
    #[cfg(feature = "glyph")]
    format_options: RwLock<vize_glyph::FormatOptions>,
    /// tsgo bridge for TypeScript language features (lazy initialized)
    #[cfg(feature = "native")]
    tsgo_bridge: OnceCell<Arc<TsgoBridge>>,
    /// Flag to track if tsgo initialization has been attempted and failed
    #[cfg(feature = "native")]
    tsgo_init_failed: std::sync::atomic::AtomicBool,
    /// Workspace root path
    #[cfg(feature = "native")]
    workspace_root: RwLock<Option<PathBuf>>,
    /// Batch type checker (lazy initialized, sync)
    #[cfg(feature = "native")]
    batch_checker: OnceLock<Arc<RwLock<BatchTypeChecker>>>,
    /// Batch type check result cache
    #[cfg(feature = "native")]
    batch_cache: BatchTypeCheckCache,
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerState {
    /// Create a new server state.
    pub fn new() -> Self {
        Self {
            documents: DocumentStore::new(),
            virtual_gen: RwLock::new(VirtualCodeGenerator::new()),
            virtual_docs_cache: DashMap::new(),
            #[cfg(feature = "glyph")]
            format_options: RwLock::new(vize_glyph::FormatOptions::default()),
            #[cfg(feature = "native")]
            tsgo_bridge: OnceCell::new(),
            #[cfg(feature = "native")]
            tsgo_init_failed: std::sync::atomic::AtomicBool::new(false),
            #[cfg(feature = "native")]
            workspace_root: RwLock::new(None),
            #[cfg(feature = "native")]
            batch_checker: OnceLock::new(),
            #[cfg(feature = "native")]
            batch_cache: BatchTypeCheckCache::new(),
        }
    }

    /// Set the workspace root path.
    #[cfg(feature = "native")]
    pub fn set_workspace_root(&self, path: PathBuf) {
        *self.workspace_root.write() = Some(path);
        // Invalidate batch cache when workspace changes
        self.batch_cache.invalidate();
    }

    /// Get the workspace root path.
    #[cfg(feature = "native")]
    pub fn get_workspace_root(&self) -> Option<PathBuf> {
        self.workspace_root.read().clone()
    }

    /// Get or initialize the batch type checker.
    #[cfg(feature = "native")]
    pub fn get_batch_checker(&self) -> Option<Arc<RwLock<BatchTypeChecker>>> {
        let workspace_root = self.get_workspace_root()?;

        // Try to get existing value first
        if let Some(checker) = self.batch_checker.get() {
            return Some(checker.clone());
        }

        // Try to initialize
        match BatchTypeChecker::new(&workspace_root) {
            Ok(checker) => {
                let arc = Arc::new(RwLock::new(checker));
                // get_or_init to handle race condition
                Some(self.batch_checker.get_or_init(|| arc.clone()).clone())
            }
            Err(_) => None,
        }
    }

    /// Check if batch type checker is available.
    #[cfg(feature = "native")]
    pub fn has_batch_checker(&self) -> bool {
        self.batch_checker.get().is_some()
    }

    /// Get the batch type check cache.
    #[cfg(feature = "native")]
    pub fn get_batch_cache(&self) -> &BatchTypeCheckCache {
        &self.batch_cache
    }

    /// Run batch type checking and update the cache.
    #[cfg(feature = "native")]
    pub fn run_batch_type_check(&self) -> Option<vize_canon::BatchTypeCheckResult> {
        let checker = self.get_batch_checker()?;
        let mut checker_guard = checker.write();

        // Scan project if not already scanned
        if checker_guard.file_count() == 0 && checker_guard.scan_project().is_err() {
            return None;
        }

        // Run type check
        let result = checker_guard.check_project().ok()?;

        // Update cache
        self.batch_cache.diagnostics.clear();
        for diag in &result.diagnostics {
            self.batch_cache
                .diagnostics
                .entry(diag.file.clone())
                .or_default()
                .push(diag.clone());
        }
        self.batch_cache.mark_valid();

        Some(result)
    }

    /// Invalidate batch type check cache (e.g., when a file changes).
    #[cfg(feature = "native")]
    pub fn invalidate_batch_cache(&self) {
        self.batch_cache.invalidate();
    }

    /// Get or initialize the tsgo bridge.
    ///
    /// Returns None if tsgo is not available or failed to initialize.
    #[cfg(feature = "native")]
    pub async fn get_tsgo_bridge(&self) -> Option<Arc<TsgoBridge>> {
        use std::sync::atomic::Ordering;

        // If already initialized successfully, return it
        if let Some(bridge) = self.tsgo_bridge.get() {
            return Some(bridge.clone());
        }

        // If initialization already failed, don't retry
        if self.tsgo_init_failed.load(Ordering::SeqCst) {
            return None;
        }

        // Get workspace root for tsgo configuration
        let workspace_root = self.get_workspace_root();

        let result = self
            .tsgo_bridge
            .get_or_try_init(|| async {
                let config = TsgoBridgeConfig {
                    working_dir: workspace_root,
                    timeout_ms: 30000, // 30 second timeout for requests (tsgo needs time to analyze)
                    ..Default::default()
                };
                let bridge = TsgoBridge::with_config(config);

                // Add timeout for spawning tsgo (5 seconds)
                match tokio::time::timeout(std::time::Duration::from_secs(5), bridge.spawn()).await
                {
                    Ok(Ok(())) => {
                        tracing::info!("tsgo bridge initialized successfully");
                        Ok(Arc::new(bridge))
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("tsgo bridge spawn failed: {}", e);
                        Err(())
                    }
                    Err(_) => {
                        tracing::warn!("tsgo bridge spawn timed out");
                        Err(())
                    }
                }
            })
            .await;

        match result {
            Ok(bridge) => Some(bridge.clone()),
            Err(()) => {
                // Mark as failed so we don't retry
                self.tsgo_init_failed.store(true, Ordering::SeqCst);
                None
            }
        }
    }

    /// Check if tsgo bridge is available (without initializing).
    #[cfg(feature = "native")]
    pub fn has_tsgo_bridge(&self) -> bool {
        self.tsgo_bridge.initialized()
    }

    /// Generate and cache virtual documents for a document.
    pub fn update_virtual_docs(&self, uri: &Url, content: &str) {
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        if let Ok(descriptor) = vize_atelier_sfc::parse_sfc(content, options) {
            let base_uri = uri.path();
            let virtual_docs = self.virtual_gen.write().generate(&descriptor, base_uri);
            self.virtual_docs_cache.insert(uri.clone(), virtual_docs);
        }
    }

    /// Get cached virtual documents for a document.
    pub fn get_virtual_docs(
        &self,
        uri: &Url,
    ) -> Option<dashmap::mapref::one::Ref<'_, Url, VirtualDocuments>> {
        self.virtual_docs_cache.get(uri)
    }

    /// Remove cached virtual documents when a document is closed.
    pub fn remove_virtual_docs(&self, uri: &Url) {
        self.virtual_docs_cache.remove(uri);
    }

    /// Clear all cached virtual documents.
    pub fn clear_virtual_docs(&self) {
        self.virtual_docs_cache.clear();
    }

    /// Get a clone of the current format options.
    #[cfg(feature = "glyph")]
    #[inline]
    pub fn get_format_options(&self) -> vize_glyph::FormatOptions {
        self.format_options.read().clone()
    }

    /// Load format options from `vize.config.json` in the given directory.
    #[cfg(feature = "glyph")]
    pub fn load_format_config(&self, dir: &std::path::Path) {
        let config_path = dir.join("vize.config.json");
        if !config_path.exists() {
            return;
        }
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            // Parse only the "fmt" field to avoid pulling in the full VizeConfig type
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(fmt_value) = value.get("fmt") {
                    if let Ok(opts) =
                        serde_json::from_value::<vize_glyph::FormatOptions>(fmt_value.clone())
                    {
                        *self.format_options.write() = opts;
                        tracing::info!("Loaded format config from {}", config_path.display());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_format_options() {
        let state = ServerState::new();
        let opts = state.get_format_options();
        assert_eq!(opts.print_width, 100);
        assert_eq!(opts.tab_width, 2);
        assert!(!opts.use_tabs);
        assert!(opts.semi);
        assert!(!opts.single_quote);
        assert!(opts.sort_attributes);
        assert!(opts.normalize_directive_shorthands);
    }

    #[test]
    fn load_format_config_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let state = ServerState::new();
        state.load_format_config(dir.path());
        // options remain default
        let opts = state.get_format_options();
        assert_eq!(opts.print_width, 100);
    }

    #[test]
    fn load_format_config_from_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vize.config.json"),
            r#"{
                "fmt": {
                    "printWidth": 80,
                    "tabWidth": 4,
                    "useTabs": true,
                    "semi": false,
                    "singleQuote": true
                }
            }"#,
        )
        .unwrap();

        let state = ServerState::new();
        state.load_format_config(dir.path());
        let opts = state.get_format_options();
        assert_eq!(opts.print_width, 80);
        assert_eq!(opts.tab_width, 4);
        assert!(opts.use_tabs);
        assert!(!opts.semi);
        assert!(opts.single_quote);
    }

    #[test]
    fn load_format_config_partial() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vize.config.json"),
            r#"{ "fmt": { "printWidth": 120 } }"#,
        )
        .unwrap();

        let state = ServerState::new();
        state.load_format_config(dir.path());
        let opts = state.get_format_options();
        assert_eq!(opts.print_width, 120);
        // defaults preserved
        assert_eq!(opts.tab_width, 2);
        assert!(opts.semi);
    }

    #[test]
    fn load_format_config_no_fmt_section() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vize.config.json"),
            r#"{ "check": { "globals": ["$t"] } }"#,
        )
        .unwrap();

        let state = ServerState::new();
        state.load_format_config(dir.path());
        // no fmt section → options remain default
        let opts = state.get_format_options();
        assert_eq!(opts.print_width, 100);
    }

    #[test]
    fn load_format_config_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("vize.config.json"), "not valid json").unwrap();

        let state = ServerState::new();
        state.load_format_config(dir.path());
        // options remain default
        let opts = state.get_format_options();
        assert_eq!(opts.print_width, 100);
    }
}
