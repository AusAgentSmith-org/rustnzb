//! Standalone par2 verify/repair benchmark.
//!
//! Generates test data locally and measures:
//! - Native Rust par2 verify (from the `par2` crate)
//! - par2cmdline-turbo verify (from `par2-sys`)
//! - par2cmdline-turbo repair (from `par2-sys`)
//!
//! Run: cargo run --release -p par2 --example par2_bench -- [size_mb] [damage_pct]
//! E.g.: cargo run --release -p par2 --example par2_bench -- 100 3

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let size_mb: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
    let damage_pct: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3.0);

    let par2_bin = par2_sys::par2_bin_path();
    println!("par2 binary: {}", par2_bin.display());

    // Print par2 version
    let ver = Command::new(par2_bin).arg("--version").output();
    if let Ok(out) = ver {
        let s = String::from_utf8_lossy(&out.stdout);
        println!("par2 version: {}", s.lines().next().unwrap_or("unknown"));
    }

    let work_dir = tempfile::tempdir().unwrap();
    let dir = work_dir.path();

    println!("\n=== Setup: {size_mb} MB data, {damage_pct}% damage ===");

    // 1. Generate random data file
    let data_file = dir.join("testdata.bin");
    let data_size = size_mb * 1024 * 1024;
    print!("Generating {size_mb} MB test data... ");
    std::io::stdout().flush().unwrap();
    generate_random_file(&data_file, data_size);
    println!("done");

    // 2. Create par2 recovery files using par2cmdline-turbo
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let t_arg = format!("-t{threads}");

    print!("Creating par2 recovery files (8% redundancy)... ");
    std::io::stdout().flush().unwrap();
    let start = Instant::now();
    let status = Command::new(par2_bin)
        .args(["create", &t_arg, "-r8", "-s768000"])
        .arg(dir.join("testdata.bin.par2"))
        .arg(&data_file)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Failed to run par2 create");
    assert!(status.success(), "par2 create failed");
    println!("done ({:.1}s)", start.elapsed().as_secs_f64());

    // List par2 files
    let par2_files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |e| e == "par2"))
        .collect();
    let par2_index = dir.join("testdata.bin.par2");
    println!("Par2 files: {} ({} total)", par2_files.len(),
        par2_files.iter().map(|p| std::fs::metadata(p).unwrap().len()).sum::<u64>() / 1024 / 1024);

    // ================================================================
    // Benchmark 1: Native Rust verify (all files intact)
    // ================================================================
    println!("\n=== Benchmark: Native Rust verify (intact files) ===");
    let start = Instant::now();
    let file_set = par2::parse(&par2_index).expect("Failed to parse par2 file");
    let parse_time = start.elapsed();

    let start = Instant::now();
    let result = par2::verify(&file_set, dir);
    let verify_time = start.elapsed();

    println!("  Parse:  {:.3}s", parse_time.as_secs_f64());
    println!("  Verify: {:.3}s", verify_time.as_secs_f64());
    println!("  Total:  {:.3}s", (parse_time + verify_time).as_secs_f64());
    println!("  Result: {result}");

    // ================================================================
    // Benchmark 2: par2cmdline-turbo verify (all files intact)
    // ================================================================
    println!("\n=== Benchmark: par2cmdline-turbo verify (intact files) ===");
    let wildcard = format!("{}/*", dir.display());
    let start = Instant::now();
    let output = Command::new(par2_bin)
        .args(["verify", &t_arg, "-B"])
        .arg(dir)
        .arg(&par2_index)
        .arg(&wildcard)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to run par2 verify");
    let cmdline_verify_time = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("  Time:   {:.3}s", cmdline_verify_time.as_secs_f64());
    println!("  Exit:   {}", output.status);
    if stdout.contains("All files are correct") {
        println!("  Result: All files correct");
    }

    // ================================================================
    // Benchmark 3: Damage files, then measure repair
    // ================================================================
    println!("\n=== Damaging {damage_pct}% of data file ===");
    damage_file(&data_file, data_size, damage_pct);

    // Benchmark 3a: Native verify on damaged files
    println!("\n=== Benchmark: Native Rust verify (damaged files) ===");
    let start = Instant::now();
    let result = par2::verify(&file_set, dir);
    let verify_damaged_time = start.elapsed();
    println!("  Verify: {:.3}s", verify_damaged_time.as_secs_f64());
    println!("  Result: {result}");

    // Benchmark 3b: par2cmdline-turbo repair
    println!("\n=== Benchmark: par2cmdline-turbo repair (damaged files) ===");
    let start = Instant::now();
    let output = Command::new(par2_bin)
        .args(["repair", &t_arg, "-B"])
        .arg(dir)
        .arg(&par2_index)
        .arg(&wildcard)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to run par2 repair");
    let repair_time = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("  Time:   {:.3}s", repair_time.as_secs_f64());
    println!("  Exit:   {}", output.status);
    if stdout.contains("Repair complete") {
        println!("  Result: Repair complete");
    } else if stdout.contains("Repair is not possible") {
        println!("  Result: Repair NOT possible");
    } else {
        // Print last few lines of output
        for line in stdout.lines().rev().take(5) {
            println!("  stdout: {line}");
        }
    }

    // ================================================================
    // Summary
    // ================================================================
    println!("\n=== Summary ({size_mb} MB, {damage_pct}% damage) ===");
    println!("  Native verify (intact):  {:.3}s", (parse_time + verify_time).as_secs_f64());
    println!("  cmdline verify (intact):  {:.3}s", cmdline_verify_time.as_secs_f64());
    println!("  Native verify (damaged):  {:.3}s", verify_damaged_time.as_secs_f64());
    println!("  cmdline repair (damaged): {:.3}s", repair_time.as_secs_f64());

    let speedup = cmdline_verify_time.as_secs_f64() / (parse_time + verify_time).as_secs_f64();
    println!("  Native vs cmdline verify: {:.1}x", speedup);
}

fn generate_random_file(path: &Path, size: u64) {
    use rand::RngCore;
    let mut f = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut rng = rand::rng();
    let chunk = 1024 * 1024; // 1 MB
    let mut buf = vec![0u8; chunk];
    let mut written = 0u64;
    while written < size {
        let n = std::cmp::min(chunk as u64, size - written) as usize;
        rng.fill_bytes(&mut buf[..n]);
        f.write_all(&buf[..n]).unwrap();
        written += n as u64;
    }
    f.flush().unwrap();
}

fn damage_file(path: &Path, size: u64, damage_pct: f64) {
    use rand::RngCore;
    use std::io::{Seek, SeekFrom};

    let damage_bytes = (size as f64 * damage_pct / 100.0) as u64;
    let chunk = 768000u64; // par2 block size
    let num_chunks = damage_bytes / chunk;

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap();
    let mut rng = rand::rng();
    let mut buf = vec![0u8; chunk as usize];

    // Damage evenly spaced chunks
    let total_chunks = size / chunk;
    let step = if num_chunks > 0 { total_chunks / num_chunks } else { 1 };

    let mut damaged = 0;
    for i in 0..num_chunks {
        let chunk_idx = i * step + step / 2;
        let offset = chunk_idx * chunk;
        if offset + chunk > size {
            break;
        }
        f.seek(SeekFrom::Start(offset)).unwrap();
        rng.fill_bytes(&mut buf);
        f.write_all(&buf).unwrap();
        damaged += 1;
    }
    f.flush().unwrap();
    println!("  Damaged {damaged} blocks ({} KB)", damaged * chunk / 1024);
}
