//! Post-processing pipeline orchestrator.
//!
//! Stages:
//! - If articles_failed > 0: par2 repair (verifies + repairs in one pass)
//! - If articles_failed == 0: par2 skipped entirely (files are known-good)
//! - Extract archives (RAR, 7z, ZIP)
//! - Cleanup temporary files

use std::path::{Path, PathBuf};
use std::time::Instant;

use nzb_core::models::{StageResult, StageStatus};
use tracing::{error, info, warn};

use crate::detect::{find_archives, find_cleanup_files, find_par2_files, ArchiveType};
use crate::par2::{par2_repair, parse_par2_output};
use crate::unpack::{extract_7z, extract_rar, extract_zip};

/// Final result of the complete post-processing pipeline.
#[derive(Debug)]
pub struct PostProcResult {
    /// Whether all stages completed successfully.
    pub success: bool,
    /// Results from each stage that was attempted.
    pub stages: Vec<StageResult>,
    /// Error message if the pipeline failed.
    pub error: Option<String>,
}

/// Configuration for the post-processing pipeline.
#[derive(Debug, Clone)]
pub struct PostProcConfig {
    /// Remove par2 and archive files after successful extraction.
    pub cleanup_after_extract: bool,
    /// Directory where extracted files should be placed.
    /// If None, extracts into the job directory itself.
    pub output_dir: Option<PathBuf>,
    /// Number of articles that failed during download.
    /// When 0, par2 verification is skipped (files are known-good).
    /// When > 0, `par2 repair` is run directly (which verifies + repairs
    /// in a single pass), avoiding the redundant verify-then-repair double-scan.
    pub articles_failed: usize,
}

impl Default for PostProcConfig {
    fn default() -> Self {
        Self {
            cleanup_after_extract: true,
            output_dir: None,
            articles_failed: 0,
        }
    }
}

/// Run the full post-processing pipeline on a completed job directory.
///
/// Stages executed in order:
/// 1. **Verify** — par2 verification
/// 2. **Repair** — par2 repair (only if verify found issues)
/// 3. **Extract** — unpack RAR, 7z, ZIP archives
/// 4. **Cleanup** — remove archive/par2 files (if configured)
pub async fn run_pipeline(job_dir: &Path, config: &PostProcConfig) -> PostProcResult {
    let mut stages: Vec<StageResult> = Vec::new();
    let mut pipeline_ok = true;

    info!(dir = %job_dir.display(), "Starting post-processing pipeline");

    // ------------------------------------------------------------------
    // Stage 1 + 2: Par2 verify / repair
    // ------------------------------------------------------------------
    // When articles_failed == 0, we skip par2 entirely — the download
    // completed without errors so the files are known-good.
    //
    // When articles_failed > 0, we run `par2 repair` directly. The
    // repair command verifies first and then repairs in a single pass,
    // avoiding the redundant double-scan of a separate verify + repair.
    let par2_files = find_par2_files(job_dir);

    if par2_files.is_empty() {
        // No par2 files — skip both stages
        stages.push(StageResult {
            name: "Verify".to_string(),
            status: StageStatus::Skipped,
            message: Some("No par2 files found".to_string()),
            duration_secs: 0.0,
        });
    } else if config.articles_failed == 0 {
        // No failures — skip par2 verification entirely
        info!("Download had zero article failures — skipping par2 verification");
        stages.push(StageResult {
            name: "Verify".to_string(),
            status: StageStatus::Skipped,
            message: Some("Skipped — download had zero article failures".to_string()),
            duration_secs: 0.0,
        });
    } else {
        // Articles failed — run par2 repair directly (verifies + repairs in one pass)
        info!(
            articles_failed = config.articles_failed,
            "Running par2 repair directly (verify + repair in single pass)"
        );
        let result = run_repair_stage(job_dir).await;
        if result.status == StageStatus::Failed {
            pipeline_ok = false;
        }
        stages.push(result);
    }

    // ------------------------------------------------------------------
    // Stage 3: Extract
    // ------------------------------------------------------------------
    if pipeline_ok {
        let output_dir = config.output_dir.as_deref().unwrap_or(job_dir);
        let result = run_extract_stage(job_dir, output_dir).await;
        if result.status == StageStatus::Failed {
            pipeline_ok = false;
        }
        stages.push(result);
    }

    // ------------------------------------------------------------------
    // Stage 4: Cleanup
    // ------------------------------------------------------------------
    if pipeline_ok && config.cleanup_after_extract {
        let result = run_cleanup_stage(job_dir);
        stages.push(result);
    }

    let error = if pipeline_ok {
        None
    } else {
        // Collect failure messages from stages
        let msgs: Vec<String> = stages
            .iter()
            .filter(|s| s.status == StageStatus::Failed)
            .filter_map(|s| s.message.clone())
            .collect();
        Some(msgs.join("; "))
    };

    info!(
        success = pipeline_ok,
        stages = stages.len(),
        "Post-processing pipeline finished"
    );

    PostProcResult {
        success: pipeline_ok,
        stages,
        error,
    }
}

// ---------------------------------------------------------------------------
// Internal stage runners
// ---------------------------------------------------------------------------

async fn run_repair_stage(job_dir: &Path) -> StageResult {
    let start = Instant::now();
    let par2_files = find_par2_files(job_dir);

    if par2_files.is_empty() {
        return StageResult {
            name: "Repair".to_string(),
            status: StageStatus::Skipped,
            message: Some("No par2 files found".to_string()),
            duration_secs: start.elapsed().as_secs_f64(),
        };
    }

    let index_par2 = &par2_files[0];
    info!(file = %index_par2.display(), "Running par2 repair");

    match par2_repair(index_par2).await {
        Ok(result) => {
            let status = if result.repaired {
                StageStatus::Success
            } else {
                StageStatus::Failed
            };

            let (par2_status, _, _) = parse_par2_output(&result.output);
            let message = Some(format!("{par2_status}"));

            StageResult {
                name: "Repair".to_string(),
                status,
                message,
                duration_secs: start.elapsed().as_secs_f64(),
            }
        }
        Err(e) => {
            error!(error = %e, "par2 repair failed with error");
            StageResult {
                name: "Repair".to_string(),
                status: StageStatus::Failed,
                message: Some(format!("par2 repair error: {e}")),
                duration_secs: start.elapsed().as_secs_f64(),
            }
        }
    }
}

async fn run_extract_stage(job_dir: &Path, output_dir: &Path) -> StageResult {
    let start = Instant::now();
    let archives = find_archives(job_dir);

    if archives.is_empty() {
        info!("No archives found — skipping extraction");
        return StageResult {
            name: "Extract".to_string(),
            status: StageStatus::Skipped,
            message: Some("No archives found".to_string()),
            duration_secs: start.elapsed().as_secs_f64(),
        };
    }

    let mut all_ok = true;
    let mut messages: Vec<String> = Vec::new();

    for (archive_type, path) in &archives {
        info!(kind = %archive_type, file = %path.display(), "Extracting archive");

        let result = match archive_type {
            ArchiveType::Rar => extract_rar(path, output_dir).await,
            ArchiveType::SevenZip => extract_7z(path, output_dir).await,
            ArchiveType::Zip => extract_zip(path, output_dir).await,
        };

        match result {
            Ok(unpack_result) => {
                if unpack_result.success {
                    messages.push(format!("{archive_type}: OK"));
                } else {
                    all_ok = false;
                    warn!(kind = %archive_type, file = %path.display(), "Extraction reported failure");
                    messages.push(format!("{archive_type}: failed"));
                }
            }
            Err(e) => {
                all_ok = false;
                error!(kind = %archive_type, file = %path.display(), error = %e, "Extraction error");
                messages.push(format!("{archive_type}: {e}"));
            }
        }
    }

    StageResult {
        name: "Extract".to_string(),
        status: if all_ok {
            StageStatus::Success
        } else {
            StageStatus::Failed
        },
        message: Some(messages.join("; ")),
        duration_secs: start.elapsed().as_secs_f64(),
    }
}

fn run_cleanup_stage(job_dir: &Path) -> StageResult {
    let start = Instant::now();
    let files = find_cleanup_files(job_dir);

    if files.is_empty() {
        return StageResult {
            name: "Cleanup".to_string(),
            status: StageStatus::Skipped,
            message: Some("No files to clean up".to_string()),
            duration_secs: start.elapsed().as_secs_f64(),
        };
    }

    let mut removed = 0u32;
    let mut errors = 0u32;

    for path in &files {
        match std::fs::remove_file(path) {
            Ok(()) => {
                removed += 1;
            }
            Err(e) => {
                warn!(file = %path.display(), error = %e, "Failed to remove cleanup file");
                errors += 1;
            }
        }
    }

    let status = if errors == 0 {
        StageStatus::Success
    } else {
        StageStatus::Failed
    };

    StageResult {
        name: "Cleanup".to_string(),
        status,
        message: Some(format!("Removed {removed} files, {errors} errors")),
        duration_secs: start.elapsed().as_secs_f64(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_test_dir(files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for name in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, b"").unwrap();
        }
        dir
    }

    #[test]
    fn test_post_proc_result_default() {
        let result = PostProcResult {
            success: true,
            stages: vec![],
            error: None,
        };
        assert!(result.success);
        assert!(result.stages.is_empty());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_config_default() {
        let config = PostProcConfig::default();
        assert!(config.cleanup_after_extract);
        assert!(config.output_dir.is_none());
        assert_eq!(config.articles_failed, 0);
    }

    #[tokio::test]
    async fn test_pipeline_no_files() {
        // An empty directory should skip all stages
        let dir = make_test_dir(&[]);
        let config = PostProcConfig::default();
        let result = run_pipeline(dir.path(), &config).await;

        assert!(result.success, "Pipeline should succeed for empty dir");

        // Verify should be skipped (no par2), Extract should be skipped (no archives)
        let verify_stage = result.stages.iter().find(|s| s.name == "Verify");
        assert!(verify_stage.is_some(), "Verify stage should be present");
        assert_eq!(verify_stage.unwrap().status, StageStatus::Skipped);

        let extract_stage = result.stages.iter().find(|s| s.name == "Extract");
        assert!(extract_stage.is_some(), "Extract stage should be present");
        assert_eq!(extract_stage.unwrap().status, StageStatus::Skipped);
    }

    #[tokio::test]
    async fn test_pipeline_only_text_files() {
        let dir = make_test_dir(&["readme.txt", "info.nfo"]);
        let config = PostProcConfig::default();
        let result = run_pipeline(dir.path(), &config).await;

        assert!(result.success);
        // All stages should be skipped
        for stage in &result.stages {
            assert_eq!(
                stage.status,
                StageStatus::Skipped,
                "Stage '{}' should be skipped",
                stage.name
            );
        }
    }

    #[test]
    fn test_cleanup_removes_files() {
        let dir = make_test_dir(&[
            "movie.par2",
            "movie.vol00+01.par2",
            "movie.rar",
            "movie.r00",
            "movie.mkv", // should NOT be removed
        ]);

        let result = run_cleanup_stage(dir.path());
        assert_eq!(result.status, StageStatus::Success);

        // movie.mkv should still exist
        assert!(dir.path().join("movie.mkv").exists());
        // par2 and rar files should be gone
        assert!(!dir.path().join("movie.par2").exists());
        assert!(!dir.path().join("movie.vol00+01.par2").exists());
        assert!(!dir.path().join("movie.rar").exists());
        assert!(!dir.path().join("movie.r00").exists());
    }

    #[tokio::test]
    async fn test_pipeline_stage_order() {
        // With an empty dir, we can at least verify the stages that run
        // are in the correct order.
        let dir = make_test_dir(&[]);
        let config = PostProcConfig {
            cleanup_after_extract: false,
            output_dir: None,
            articles_failed: 0,
        };
        let result = run_pipeline(dir.path(), &config).await;

        // Should have Verify and Extract (both skipped). Cleanup is disabled.
        let stage_names: Vec<&str> = result.stages.iter().map(|s| s.name.as_str()).collect();
        assert!(stage_names.contains(&"Verify"), "Should have Verify stage");
        assert!(stage_names.contains(&"Extract"), "Should have Extract stage");

        // Verify should come before Extract
        let verify_idx = stage_names.iter().position(|&n| n == "Verify").unwrap();
        let extract_idx = stage_names.iter().position(|&n| n == "Extract").unwrap();
        assert!(
            verify_idx < extract_idx,
            "Verify ({verify_idx}) should come before Extract ({extract_idx})"
        );
    }

    #[tokio::test]
    async fn test_pipeline_skips_par2_when_zero_failures() {
        // With par2 files present but articles_failed == 0, verify should be skipped
        let dir = make_test_dir(&[
            "movie.par2",
            "movie.vol00+01.par2",
            "movie.mkv",
        ]);
        let config = PostProcConfig {
            cleanup_after_extract: false,
            output_dir: None,
            articles_failed: 0,
        };
        let result = run_pipeline(dir.path(), &config).await;
        assert!(result.success);

        let verify_stage = result.stages.iter().find(|s| s.name == "Verify").unwrap();
        assert_eq!(
            verify_stage.status,
            StageStatus::Skipped,
            "Verify should be skipped when articles_failed == 0"
        );
        assert!(
            verify_stage
                .message
                .as_deref()
                .unwrap_or("")
                .contains("zero article failures"),
            "Skip message should indicate zero failures"
        );
    }

    #[tokio::test]
    async fn test_pipeline_no_par2_files_skips_regardless() {
        // No par2 files — should skip even if articles_failed > 0
        let dir = make_test_dir(&["movie.mkv"]);
        let config = PostProcConfig {
            cleanup_after_extract: false,
            output_dir: None,
            articles_failed: 5,
        };
        let result = run_pipeline(dir.path(), &config).await;
        assert!(result.success);

        let verify_stage = result.stages.iter().find(|s| s.name == "Verify").unwrap();
        assert_eq!(
            verify_stage.status,
            StageStatus::Skipped,
            "Verify should be skipped when no par2 files exist"
        );
        assert!(
            verify_stage
                .message
                .as_deref()
                .unwrap_or("")
                .contains("No par2 files"),
            "Skip message should indicate no par2 files"
        );
    }

    #[tokio::test]
    async fn test_pipeline_runs_repair_when_failures() {
        // With par2 files and articles_failed > 0, should attempt repair
        // (which will fail since these are dummy files, but the stage should be attempted)
        let dir = make_test_dir(&[
            "movie.par2",
            "movie.vol00+01.par2",
            "movie.mkv",
        ]);
        let config = PostProcConfig {
            cleanup_after_extract: false,
            output_dir: None,
            articles_failed: 3,
        };
        let result = run_pipeline(dir.path(), &config).await;

        // Should have a Repair stage (not Verify) since we go straight to repair
        let stage_names: Vec<&str> = result.stages.iter().map(|s| s.name.as_str()).collect();
        assert!(
            stage_names.contains(&"Repair"),
            "Should have Repair stage when articles_failed > 0, got: {stage_names:?}"
        );
        assert!(
            !stage_names.contains(&"Verify"),
            "Should NOT have Verify stage when going direct to repair"
        );
    }
}
