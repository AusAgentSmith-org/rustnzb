//! End-to-end integration test for `NewsDispatchEngine`.
//!
//! Spawns a mock NNTP server that serves yEnc-encoded article bodies,
//! constructs a realistic `NzbJob`, drives it through the engine, and
//! asserts:
//! - `ArticleComplete` events arrive with decoded byte counts
//! - `JobFinished { success: true }` is emitted at terminal
//! - The assembled file on disk contains the original payload
//!
//! This exercises the full adapter surface: submit → pump → fetch →
//! decode → assemble → progress translation → terminal emit.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tempfile::TempDir;
use tokio::sync::mpsc;

use nzb_core::models::{JobStatus, NzbFile, NzbJob, Priority};
use nzb_dispatch::dispatch_engine::DispatchEngine;
use nzb_dispatch::download_engine::ProgressUpdate;
use nzb_dispatch::news_engine::{NewsDispatchEngine, NewsEngineConfig};
use nzb_nntp::Article as NntpArticle;
use nzb_nntp::testutil::{MockConfig, MockNntpServer, test_config};

fn yenc_encode(filename: &str, payload: &[u8]) -> Vec<u8> {
    // Single-part article; file_offset=0, total_file_size=payload.len().
    let (body, _crc) = yenc_simd::encode_article(payload, filename, 1, 1, 0, payload.len() as u64);
    body
}

fn make_job(work_dir: PathBuf, filename: &str, message_id: &str, bytes: u64) -> NzbJob {
    let article = NntpArticle {
        message_id: message_id.into(),
        segment_number: 1,
        bytes,
        downloaded: false,
        data_begin: None,
        data_size: None,
        crc32: None,
        tried_servers: Vec::new(),
        tries: 0,
    };
    let file = NzbFile {
        id: "f1".into(),
        filename: filename.into(),
        bytes,
        bytes_downloaded: 0,
        is_par2: false,
        par2_setname: None,
        par2_vol: None,
        par2_blocks: None,
        assembled: false,
        groups: vec!["alt.binaries.test".into()],
        articles: vec![article],
    };
    NzbJob {
        id: "j1".into(),
        name: "integration-test".into(),
        category: "test".into(),
        status: JobStatus::Queued,
        priority: Priority::Normal,
        total_bytes: bytes,
        downloaded_bytes: 0,
        file_count: 1,
        files_completed: 0,
        article_count: 1,
        articles_downloaded: 0,
        articles_failed: 0,
        added_at: Utc::now(),
        completed_at: None,
        work_dir: work_dir.clone(),
        output_dir: work_dir,
        password: None,
        error_message: None,
        speed_bps: 0,
        server_stats: Vec::new(),
        files: vec![file],
    }
}

#[tokio::test]
async fn submit_single_article_job_end_to_end() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("nzb_dispatch=debug,nzb_news=debug")
        .with_test_writer()
        .try_init();

    // 1. Build a yEnc-encoded article body and serve it via mock NNTP.
    let payload = b"hello world via nzb-news engine\n";
    let filename = "hello.txt";
    let msg_id = "msg-integration-1";
    let encoded = yenc_encode(filename, payload);

    let mut articles = HashMap::new();
    articles.insert(msg_id.to_string(), encoded);
    let server = MockNntpServer::start(MockConfig {
        articles,
        ..Default::default()
    })
    .await;

    let mut server_cfg = test_config(server.port());
    server_cfg.id = "s1".into();
    server_cfg.priority = 1;
    server_cfg.connections = 2;
    server_cfg.ramp_up_delay_ms = 0;

    // 2. Build the engine.
    let news_cfg = NewsEngineConfig::new(vec![server_cfg], Duration::from_secs(10));
    let engine: Arc<dyn DispatchEngine> = Arc::new(NewsDispatchEngine::new(news_cfg));
    engine.start();

    // 3. Build the job with a tmpdir as work_dir.
    let tmp = TempDir::new().unwrap();
    let work_dir = tmp.path().to_path_buf();
    let job = make_job(work_dir.clone(), filename, msg_id, payload.len() as u64);

    // 4. Submit and collect progress.
    let (tx, mut rx) = mpsc::channel::<ProgressUpdate>(64);
    engine.submit_job(&job, tx);

    let mut article_complete_seen = false;
    let mut job_finished_success: Option<bool> = None;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
            Ok(Some(ProgressUpdate::ArticleComplete {
                decoded_bytes,
                file_complete,
                server_id,
                ..
            })) => {
                assert_eq!(decoded_bytes as usize, payload.len());
                assert!(file_complete, "single-article file should complete");
                assert_eq!(server_id.as_deref(), Some("s1"));
                article_complete_seen = true;
            }
            Ok(Some(ProgressUpdate::JobFinished { success, .. })) => {
                job_finished_success = Some(success);
                break;
            }
            Ok(Some(ProgressUpdate::ArticleFailed { failure, .. })) => {
                panic!("unexpected ArticleFailed: {failure}");
            }
            Ok(Some(other)) => {
                eprintln!("saw other event: {other:?}");
            }
            Ok(None) => break,
            Err(_) => panic!("timeout waiting for progress events"),
        }
    }

    assert!(article_complete_seen, "never saw ArticleComplete");
    assert_eq!(
        job_finished_success,
        Some(true),
        "job should finish successfully"
    );

    // 5. Verify the file on disk matches the payload.
    let out = std::fs::read(work_dir.join(filename)).expect("output file exists");
    assert_eq!(out, payload, "decoded bytes should equal input payload");

    engine.shutdown().await;
}

#[tokio::test]
async fn cancel_job_removes_from_engine() {
    let server = MockNntpServer::start(MockConfig::default()).await;
    let mut server_cfg = test_config(server.port());
    server_cfg.id = "s1".into();
    server_cfg.connections = 1;
    server_cfg.ramp_up_delay_ms = 0;

    let news_cfg = NewsEngineConfig::new(vec![server_cfg], Duration::from_secs(5));
    let engine: Arc<dyn DispatchEngine> = Arc::new(NewsDispatchEngine::new(news_cfg));
    engine.start();

    let tmp = TempDir::new().unwrap();
    let job = make_job(
        tmp.path().to_path_buf(),
        "unused.txt",
        "no-such-message",
        10,
    );
    let (tx, _rx) = mpsc::channel::<ProgressUpdate>(64);
    engine.submit_job(&job, tx);

    assert!(engine.has_job("j1"), "job should be registered");
    engine.cancel_job("j1");
    assert!(!engine.has_job("j1"), "job should be gone after cancel");

    // Skip engine.shutdown() — nzb-news's graceful drain waits on in-flight
    // wrapper workers, and those keep retrying the (unfetchable) message
    // until max_art_tries is exhausted. Runtime drop at test-fn exit
    // terminates everything.
}
