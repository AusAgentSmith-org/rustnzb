//! Downloads the pre-built par2cmdline-turbo binary for the target platform
//! and embeds its path for use at runtime.

use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

const VERSION: &str = "1.4.0";
const BASE_URL: &str = "https://github.com/animetosho/par2cmdline-turbo/releases/download";

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let par2_bin = out_dir.join("par2");

    // Skip download if already cached (cargo caches OUT_DIR across rebuilds)
    if par2_bin.exists() {
        emit_metadata(&par2_bin);
        return;
    }

    let asset_name = asset_for_target();
    let url = format!("{BASE_URL}/v{VERSION}/{asset_name}");

    eprintln!("par2-sys: downloading {url}");

    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("Failed to create HTTP client")
        .get(&url)
        .send()
        .unwrap_or_else(|e| panic!("Failed to download par2cmdline-turbo from {url}: {e}"));

    if !response.status().is_success() {
        panic!(
            "Failed to download par2cmdline-turbo: HTTP {}",
            response.status()
        );
    }

    let zip_bytes = response.bytes().expect("Failed to read response body");

    // Extract the par2 binary from the zip
    let cursor = std::io::Cursor::new(&zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("Failed to open zip archive");

    let mut found = false;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();

        // The binary is named "par2" (or "par2.exe" on Windows)
        if name == "par2" || name == "par2.exe" {
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)
                .expect("Failed to read par2 from zip");
            fs::write(&par2_bin, &contents).expect("Failed to write par2 binary");

            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&par2_bin, fs::Permissions::from_mode(0o755))
                    .expect("Failed to set executable permissions");
            }

            found = true;
            break;
        }
    }

    if !found {
        panic!("par2 binary not found in downloaded zip archive");
    }

    eprintln!("par2-sys: installed par2cmdline-turbo v{VERSION} to {}", par2_bin.display());
    emit_metadata(&par2_bin);
}

fn emit_metadata(par2_bin: &PathBuf) {
    // Expose the binary path to dependent crates via env var
    println!("cargo:rustc-env=PAR2_BIN_PATH={}", par2_bin.display());
    println!("cargo:rerun-if-changed=build.rs");
}

fn asset_for_target() -> String {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    let platform = match (os.as_str(), arch.as_str()) {
        ("linux", "x86_64") => "linux-amd64",
        ("linux", "aarch64") => "linux-arm64",
        ("linux", "arm") => "linux-armhf",
        ("macos", "x86_64") => "macos-amd64",
        ("macos", "aarch64") => "macos-arm64",
        ("windows", "x86_64") => "win-x64",
        ("windows", "aarch64") => "win-arm64",
        ("freebsd", "x86_64") => "freebsd-amd64",
        ("freebsd", "aarch64") => "freebsd-aarch64",
        _ => panic!("Unsupported platform: {os}-{arch}. par2cmdline-turbo does not provide pre-built binaries for this target."),
    };

    format!("par2cmdline-turbo-{VERSION}-{platform}.zip")
}
