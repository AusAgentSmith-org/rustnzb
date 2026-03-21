//! Native PAR2 file parser and verifier for Usenet/NZB post-processing.
//!
//! This crate provides a pure Rust implementation of PAR2 file parsing and
//! verification — the two operations needed for ~90% of NZB post-processing
//! jobs (where all files download intact and just need to be checked).
//!
//! # Usage
//!
//! ```no_run
//! use std::path::Path;
//!
//! let par2_path = Path::new("/downloads/movie/movie.par2");
//! let job_dir = Path::new("/downloads/movie");
//!
//! // Parse the PAR2 index file
//! let file_set = par2::parse(par2_path).unwrap();
//!
//! // Verify all files
//! let result = par2::verify(&file_set, job_dir);
//!
//! if result.all_correct() {
//!     println!("All files intact — no repair needed");
//! } else {
//!     println!("{}", result);
//!     // Fall back to par2cmdline for repair
//! }
//! ```

mod packets;
pub mod types;
mod verify;

pub use packets::{parse_par2_file as parse, parse_par2_reader, ParseError};
pub use types::{
    DamagedFile, Md5Hash, MissingFile, Par2File, Par2FileSet, SliceChecksum, VerifiedFile,
    VerifyResult,
};
pub use verify::{compute_hash_16k, verify};
