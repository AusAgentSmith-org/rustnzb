//! Archive extraction: RAR, 7z, ZIP.
//!
//! - RAR: Shell out to `unrar` binary
//! - 7z: Shell out to `7z`/`7zz`/`7za` binary
//! - ZIP: Uses std::fs + zip crate

use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;
use tracing::{info, warn};

/// Result of an unpack operation.
#[derive(Debug)]
pub struct UnpackResult {
    pub success: bool,
    pub files_extracted: Vec<String>,
    pub output: String,
}

/// Extract RAR archives in a directory.
pub async fn extract_rar(rar_file: &Path, output_dir: &Path) -> anyhow::Result<UnpackResult> {
    let unrar_bin = find_unrar().ok_or_else(|| anyhow::anyhow!("unrar binary not found on PATH"))?;

    info!(file = %rar_file.display(), dest = %output_dir.display(), "Extracting RAR");

    std::fs::create_dir_all(output_dir)?;

    let output = Command::new(&unrar_bin)
        .args(["x", "-o+", "-y"])
        .arg(rar_file)
        .arg(output_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let success = output.status.success();

    if !success {
        warn!(
            file = %rar_file.display(),
            exit_code = ?output.status.code(),
            "RAR extraction failed"
        );
    }

    Ok(UnpackResult {
        success,
        files_extracted: Vec::new(), // TODO: parse from output
        output: stdout,
    })
}

/// Extract 7z archives by shelling out to the 7z binary.
pub async fn extract_7z(archive_file: &Path, output_dir: &Path) -> anyhow::Result<UnpackResult> {
    let sevenz_bin =
        find_7z().ok_or_else(|| anyhow::anyhow!("7z/7zz/7za binary not found on PATH"))?;

    info!(file = %archive_file.display(), dest = %output_dir.display(), "Extracting 7z");

    std::fs::create_dir_all(output_dir)?;

    let output = Command::new(&sevenz_bin)
        .arg("x")
        .arg("-y") // assume yes on all queries
        .arg(format!("-o{}", output_dir.display()))
        .arg(archive_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let success = output.status.success();

    if !success {
        warn!(
            file = %archive_file.display(),
            exit_code = ?output.status.code(),
            "7z extraction failed"
        );
    }

    Ok(UnpackResult {
        success,
        files_extracted: Vec::new(), // TODO: parse from output
        output: stdout,
    })
}

/// Extract ZIP archives.
pub async fn extract_zip(zip_file: &Path, output_dir: &Path) -> anyhow::Result<UnpackResult> {
    info!(file = %zip_file.display(), dest = %output_dir.display(), "Extracting ZIP");

    // Use tokio spawn_blocking since zip extraction is CPU-bound
    let zip_path = zip_file.to_path_buf();
    let out_path = output_dir.to_path_buf();

    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<UnpackResult> {
        let file = std::fs::File::open(&zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut extracted = Vec::new();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let outpath = out_path.join(entry.mangled_name());

            if entry.is_dir() {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut entry, &mut outfile)?;
                extracted.push(outpath.to_string_lossy().to_string());
            }
        }

        Ok(UnpackResult {
            success: true,
            files_extracted: extracted,
            output: String::new(),
        })
    })
    .await??;

    Ok(result)
}

fn find_unrar() -> Option<String> {
    for name in &["unrar", "rar"] {
        if which_exists(name) {
            return Some(name.to_string());
        }
    }
    None
}

/// Find the 7z binary on the system. Checks `7z`, `7zz` (7-Zip standalone),
/// and `7za` (7-Zip standalone, older naming).
pub fn find_7z() -> Option<String> {
    for name in &["7z", "7zz", "7za"] {
        if which_exists(name) {
            return Some(name.to_string());
        }
    }
    None
}

fn which_exists(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
