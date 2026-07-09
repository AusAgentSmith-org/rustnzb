mod support;

use nzb_web::nzb_core::nzb_parser;
use std::path::Path;

#[test]
fn parse_sample_fixture_nzb() {
    let data = support::sample_nzb_bytes();
    let job = nzb_parser::parse_nzb("sample.nzb", &data).expect("fixture should parse");
    assert_eq!(job.file_count, 1);
    assert_eq!(job.article_count, 1);
    assert!(job.total_bytes > 0);
    assert_eq!(job.files[0].filename, "sample.bin");
    assert_eq!(
        job.files[0].articles[0].message_id,
        "sample-article-001@rustnzb.test"
    );
}

#[test]
fn parse_all_test_nzbs() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping on CI");
        return;
    }
    let dir = Path::new("TestData");
    if !dir.exists() {
        eprintln!("TestData directory not found, skipping");
        return;
    }

    let mut count = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e == "nzb").unwrap_or(false) {
            let result = nzb_parser::parse_nzb_file(&path);
            match &result {
                Ok(job) => {
                    let size_mb = job.total_bytes as f64 / 1_048_576.0;
                    eprintln!(
                        "{}: {} files, {} articles, {:.1} MB",
                        path.file_name().unwrap().to_string_lossy(),
                        job.file_count,
                        job.article_count,
                        size_mb
                    );
                    assert!(job.file_count > 0);
                    assert!(job.article_count > 0);
                    assert!(job.total_bytes > 0);
                    // Check all articles have message IDs
                    for f in &job.files {
                        for a in &f.articles {
                            assert!(
                                !a.message_id.is_empty(),
                                "Empty message ID in {}",
                                f.filename
                            );
                        }
                    }
                }
                Err(e) => {
                    panic!("Failed to parse {}: {}", path.display(), e);
                }
            }
            count += 1;
        }
    }
    eprintln!("Successfully parsed {count} NZB files");
    assert!(count > 0, "No NZB files found in TestData/");
}
