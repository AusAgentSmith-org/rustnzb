mod support;

use std::time::{Duration, Instant};

use nzb_nntp::testutil::{MockConfig, MockNntpServer, test_config};
use support::{NzbFixture, start_test_server};

#[tokio::test]
async fn upload_nzb_downloads_via_mock_server_and_reaches_history() {
    let fixture = NzbFixture::new()
        .add_file(
            "mock-success.bin",
            &[("mock-article-001@test", b"hello from mock nntp")],
        )
        .build();

    let server = MockNntpServer::start(MockConfig {
        articles: fixture.encoded_articles(),
        ..MockConfig::default()
    })
    .await;

    let mut config = test_config(server.port());
    config.id = "mock-primary".into();
    config.name = "Mock Primary".into();
    config.connections = 4;

    let app = start_test_server(vec![config]).await;
    let client = reqwest::Client::new();

    let part = reqwest::multipart::Part::bytes(fixture.xml.clone())
        .file_name("mock-download.nzb")
        .mime_str("application/x-nzb")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    let resp = client
        .post(format!("{}/api/queue/add", app.base_url))
        .multipart(form)
        .send()
        .await
        .expect("Upload failed");
    assert_eq!(resp.status(), 200);
    let add_result: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(add_result["status"], true);

    let deadline = Instant::now() + Duration::from_secs(10);
    let history_entry = loop {
        let history: serde_json::Value = client
            .get(format!("{}/api/history?limit=10", app.base_url))
            .send()
            .await
            .expect("history request failed")
            .json()
            .await
            .expect("history response should be JSON");

        if let Some(entry) = history["entries"]
            .as_array()
            .and_then(|entries| entries.first())
        {
            break entry.clone();
        }

        assert!(
            Instant::now() < deadline,
            "job did not reach history within timeout"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    assert_eq!(history_entry["status"], "completed");
    assert_eq!(history_entry["name"], "mock-download");

    loop {
        let queue: serde_json::Value = client
            .get(format!("{}/api/queue", app.base_url))
            .send()
            .await
            .expect("queue request failed")
            .json()
            .await
            .expect("queue response should be JSON");
        if queue["jobs"].as_array().unwrap().is_empty() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "job reached history but never drained from queue"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let downloaded = app
        .complete_dir
        .join("Default")
        .join("mock-download")
        .join("mock-success.bin");
    assert!(
        downloaded.exists(),
        "downloaded file missing: {}",
        downloaded.display()
    );
    assert_eq!(
        std::fs::read(&downloaded).expect("downloaded file should be readable"),
        b"hello from mock nntp"
    );
}
