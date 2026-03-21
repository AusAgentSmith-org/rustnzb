//! Bundled par2cmdline-turbo binary.
//!
//! This crate downloads the pre-built par2cmdline-turbo binary at build time
//! and provides a path to it at runtime. The binary is statically linked and
//! has no external dependencies.
//!
//! # Usage
//! ```rust,no_run
//! let par2_path = par2_sys::par2_bin_path();
//! std::process::Command::new(par2_path).arg("verify").arg("file.par2").status().unwrap();
//! ```

use std::path::Path;

/// Path to the bundled par2cmdline-turbo binary.
///
/// This is set at compile time by build.rs and points to the pre-built
/// binary downloaded from the par2cmdline-turbo GitHub releases.
pub fn par2_bin_path() -> &'static Path {
    Path::new(env!("PAR2_BIN_PATH"))
}

/// Version of the bundled par2cmdline-turbo binary.
pub const VERSION: &str = "1.4.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_par2_bin_exists() {
        let path = par2_bin_path();
        assert!(path.exists(), "par2 binary should exist at {}", path.display());
    }

    #[test]
    fn test_par2_bin_is_executable() {
        let path = par2_bin_path();
        let output = std::process::Command::new(path)
            .arg("--help")
            .output()
            .expect("Failed to execute par2 binary");
        // par2 --help may exit with 0 or 1 depending on version, but should not crash
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            combined.contains("par2") || combined.contains("PAR"),
            "par2 --help should mention par2: {combined}"
        );
    }

    #[test]
    fn test_par2_supports_threads() {
        let path = par2_bin_path();
        let output = std::process::Command::new(path)
            .arg("--help")
            .output()
            .expect("Failed to execute par2 binary");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            combined.contains("-t") || combined.contains("thread"),
            "Bundled par2 should support -t (threads): {combined}"
        );
    }
}
