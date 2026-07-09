//! End-to-end download pipeline tests against the reusable mock NNTP server.

use std::fmt::Write;

use mock_nntp_server::{
    FixtureFile, SAMPLE_GROUP, SAMPLE_MULTI_FILENAME, SAMPLE_MULTI_MESSAGE_IDS,
    SAMPLE_SINGLE_FILENAME, SAMPLE_SINGLE_MESSAGE_ID, fixture_files, server_config,
    start_sample_server,
};
use nzb_web::nzb_core::nzb_nntp::{NntpConnection, NntpError};
use nzb_web::nzb_core::nzb_parser;
use nzb_web::nzb_decode::yenc;

fn build_nzb(files: &[FixtureFile]) -> Vec<u8> {
    let mut xml = String::new();
    writeln!(xml, r#"<?xml version="1.0" encoding="UTF-8"?>"#).unwrap();
    writeln!(xml, r#"<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">"#).unwrap();
    for file in files {
        let total_segments = file.segments.len();
        writeln!(
            xml,
            r#"  <file poster="mock@rustnzb.test" date="1743465600" subject='"{name}" yEnc (1/{parts})'>"#,
            name = file.filename,
            parts = total_segments
        )
        .unwrap();
        writeln!(xml, "    <groups>").unwrap();
        writeln!(xml, "      <group>{SAMPLE_GROUP}</group>").unwrap();
        writeln!(xml, "    </groups>").unwrap();
        writeln!(xml, "    <segments>").unwrap();
        for segment in &file.segments {
            writeln!(
                xml,
                r#"      <segment bytes="{bytes}" number="{number}">{message_id}</segment>"#,
                bytes = segment.body.len(),
                number = segment.segment_number,
                message_id = segment.message_id
            )
            .unwrap();
        }
        writeln!(xml, "    </segments>").unwrap();
        writeln!(xml, "  </file>").unwrap();
    }
    writeln!(xml, "</nzb>").unwrap();
    xml.into_bytes()
}

fn fixture_file(name: &str) -> FixtureFile {
    fixture_files()
        .into_iter()
        .find(|file| file.filename == name)
        .expect("fixture file should exist")
}

#[tokio::test]
async fn test_fetch_single_article_and_decode() {
    let server = start_sample_server().await;
    let mut conn = NntpConnection::new("mock-single".to_string());
    conn.connect(&server_config(server.port()))
        .await
        .expect("connect to mock server");

    let sample_file = fixture_file(SAMPLE_SINGLE_FILENAME);
    let job = nzb_parser::parse_nzb(
        "sample-single",
        &build_nzb(std::slice::from_ref(&sample_file)),
    )
    .expect("parse generated nzb");
    let article = &job.files[0].articles[0];

    assert_eq!(article.message_id, SAMPLE_SINGLE_MESSAGE_ID);

    let response = conn
        .fetch_article(&article.message_id)
        .await
        .expect("fetch sample article");
    assert_eq!(response.code, 220, "mock article should fetch successfully");

    let raw_data = response.data.expect("mock article should include a body");
    let decoded = yenc::decode_yenc(&raw_data).expect("sample article should decode cleanly");

    assert_eq!(decoded.filename.as_deref(), Some(SAMPLE_SINGLE_FILENAME));
    assert_eq!(decoded.data, sample_file.segments[0].body);
    assert_eq!(decoded.data.len(), 102_400);

    conn.quit().await.expect("quit mock connection");
}

#[tokio::test]
async fn test_fetch_multiple_articles_from_rar() {
    let server = start_sample_server().await;
    let mut conn = NntpConnection::new("mock-multi".to_string());
    conn.connect(&server_config(server.port()))
        .await
        .expect("connect to mock server");

    let multi_file = fixture_file(SAMPLE_MULTI_FILENAME);
    let job = nzb_parser::parse_nzb(
        "sample-multi",
        &build_nzb(std::slice::from_ref(&multi_file)),
    )
    .expect("parse generated nzb");
    let target = &job.files[0];

    assert_eq!(target.articles.len(), SAMPLE_MULTI_MESSAGE_IDS.len());

    let mut assembled = Vec::new();
    for article in &target.articles {
        let response = conn
            .fetch_article(&article.message_id)
            .await
            .expect("fetch multi-segment article");
        assert_eq!(response.code, 220);
        let raw_data = response
            .data
            .expect("multi-segment mock should return article data");
        let decoded = yenc::decode_yenc(&raw_data).expect("multi-segment sample should decode");
        assembled.extend_from_slice(&decoded.data);
    }

    let expected = multi_file
        .segments
        .iter()
        .flat_map(|segment| segment.body.iter().copied())
        .collect::<Vec<_>>();
    assert_eq!(assembled, expected);

    conn.quit().await.expect("quit mock connection");
}

#[tokio::test]
async fn test_article_not_found_handling() {
    let server = start_sample_server().await;
    let mut conn = NntpConnection::new("mock-not-found".to_string());
    conn.connect(&server_config(server.port()))
        .await
        .expect("connect to mock server");

    let result = conn
        .fetch_article("nonexistent-fake-id-12345@nowhere.invalid")
        .await;

    match result {
        Err(NntpError::ArticleNotFound(message_id)) => {
            assert_eq!(message_id, "<nonexistent-fake-id-12345@nowhere.invalid>");
        }
        Err(other) => panic!("expected ArticleNotFound, got {other}"),
        Ok(_) => panic!("missing article should not fetch successfully"),
    }

    assert!(conn.is_connected(), "430 should not kill the NNTP session");
    conn.quit().await.expect("quit mock connection");
}
