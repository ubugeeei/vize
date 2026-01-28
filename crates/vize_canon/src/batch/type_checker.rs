//! TypeChecker trait and BatchTypeChecker implementation.

use std::path::{Path, PathBuf};

use super::error::{TsgoError, TsgoResult};
use super::executor::TsgoExecutor;
use super::virtual_project::VirtualProject;
use super::Diagnostic;

/// Result of type checking.
#[derive(Debug, Default)]
pub struct TypeCheckResult {
    /// Diagnostics from type checking.
    pub diagnostics: Vec<Diagnostic>,
    /// Exit code from tsgo.
    pub exit_code: i32,
    /// Whether type checking succeeded.
    pub success: bool,
}

impl TypeCheckResult {
    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == 1)
    }

    /// Get the number of errors.
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == 1).count()
    }

    /// Get the number of warnings.
    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == 2).count()
    }
}

/// Trait for type checking.
pub trait TypeChecker: Send + Sync {
    /// Check the entire project.
    fn check_project(&self) -> TsgoResult<TypeCheckResult>;

    /// Check a single file.
    fn check_file(&self, path: &Path, content: &str) -> TsgoResult<Vec<Diagnostic>>;

    /// Check incrementally (only changed files).
    fn check_incremental(&self, changed: &[PathBuf]) -> TsgoResult<TypeCheckResult>;
}

/// Batch type checker using tsgo CLI.
pub struct BatchTypeChecker {
    /// Virtual project.
    project: VirtualProject,
    /// tsgo executor.
    executor: TsgoExecutor,
    /// Whether the project has been scanned.
    scanned: bool,
}

impl BatchTypeChecker {
    /// Create a new batch type checker.
    pub fn new(project_root: &Path) -> TsgoResult<Self> {
        let project = VirtualProject::new(project_root)?;
        let executor = TsgoExecutor::new(project_root)?;

        Ok(Self {
            project,
            executor,
            scanned: false,
        })
    }

    /// Scan the project for source files.
    pub fn scan_project(&mut self) -> TsgoResult<()> {
        let project_root = self.project.project_root().to_path_buf();

        for entry in walkdir::WalkDir::new(&project_root)
            .into_iter()
            .filter_entry(|e| {
                // Don't filter the root directory itself
                if e.path() == project_root {
                    return true;
                }
                // Skip node_modules and hidden directories
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "node_modules"
            })
        {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            match path.extension().and_then(|e| e.to_str()) {
                Some("vue") => {
                    let content = std::fs::read_to_string(path)?;
                    self.project.register_vue_file(path, &content)?;
                }
                Some("ts" | "tsx") => {
                    // Skip .d.ts files
                    if path.to_string_lossy().ends_with(".d.ts") {
                        continue;
                    }
                    self.project.register_ts_file(path)?;
                }
                _ => {}
            }
        }

        self.scanned = true;
        Ok(())
    }

    /// Get the number of registered files.
    pub fn file_count(&self) -> usize {
        self.project.file_count()
    }
}

impl TypeChecker for BatchTypeChecker {
    fn check_project(&self) -> TsgoResult<TypeCheckResult> {
        if !self.scanned {
            return Err(TsgoError::NotInitialized);
        }

        self.executor.check(&self.project)
    }

    fn check_file(&self, path: &Path, content: &str) -> TsgoResult<Vec<Diagnostic>> {
        // Create a temporary project with just this file
        let project_root = path.parent().unwrap_or(Path::new("."));
        let mut temp_project = VirtualProject::new(project_root)?;

        if path.extension().map(|e| e == "vue").unwrap_or(false) {
            temp_project.register_vue_file(path, content)?;
        } else {
            // For .ts files, we need to write it first
            std::fs::write(path, content)?;
            temp_project.register_ts_file(path)?;
        }

        let result = self.executor.check(&temp_project)?;
        Ok(result.diagnostics)
    }

    fn check_incremental(&self, changed: &[PathBuf]) -> TsgoResult<TypeCheckResult> {
        // For now, just do a full check
        // TODO: Implement proper incremental checking
        let _ = changed;
        self.check_project()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_type_check_result() {
        let mut result = TypeCheckResult::default();
        assert!(!result.has_errors());
        assert_eq!(result.error_count(), 0);

        result.diagnostics.push(Diagnostic {
            file: PathBuf::from("test.vue"),
            line: 0,
            column: 0,
            message: "error".to_string(),
            code: Some(2304),
            severity: 1,
            block_type: None,
        });

        assert!(result.has_errors());
        assert_eq!(result.error_count(), 1);
    }

    #[test]
    fn test_batch_type_checker_scan() {
        let temp_dir = TempDir::new().unwrap();

        // Create a test Vue file
        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        let vue_content = r#"<template>
  <div>{{ message }}</div>
</template>

<script setup lang="ts">
const message = 'Hello'
</script>
"#;
        std::fs::write(src_dir.join("App.vue"), vue_content).unwrap();

        // Create a test TS file
        let ts_content = r#"export const foo = 'bar';"#;
        std::fs::write(src_dir.join("utils.ts"), ts_content).unwrap();

        // Scan the project (skip if tsgo not found)
        let mut checker = match BatchTypeChecker::new(temp_dir.path()) {
            Ok(c) => c,
            Err(_) => {
                // tsgo not found - skip test
                return;
            }
        };

        checker.scan_project().unwrap();
        assert_eq!(checker.file_count(), 2);
    }
}
