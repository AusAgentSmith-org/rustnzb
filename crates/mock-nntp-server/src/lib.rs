use std::collections::HashMap;

use nzb_nntp::config::ServerConfig;
use nzb_nntp::testutil::{MockConfig, MockNntpServer, test_config};

pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:19119";
pub const SAMPLE_GROUP: &str = "alt.binaries.test";
pub const SAMPLE_SINGLE_FILENAME: &str = "sample.bin";
pub const SAMPLE_SINGLE_MESSAGE_ID: &str = "sample-article-001@rustnzb.test";
pub const SAMPLE_MULTI_FILENAME: &str = "sample-multi.bin";
pub const SAMPLE_MULTI_MESSAGE_IDS: [&str; 3] = [
    "sample-multi-001@rustnzb.test",
    "sample-multi-002@rustnzb.test",
    "sample-multi-003@rustnzb.test",
];

#[derive(Clone, Debug)]
pub struct FixtureSegment {
    pub message_id: String,
    pub filename: String,
    pub body: Vec<u8>,
    pub segment_number: u32,
    pub total_segments: u32,
    pub total_file_size: u64,
    pub offset: u64,
}

#[derive(Clone, Debug)]
pub struct FixtureFile {
    pub filename: String,
    pub segments: Vec<FixtureSegment>,
}

pub fn fixture_files() -> Vec<FixtureFile> {
    let single_body = deterministic_body(102_400, 0x11);
    let single_file = FixtureFile {
        filename: SAMPLE_SINGLE_FILENAME.to_string(),
        segments: vec![FixtureSegment {
            message_id: SAMPLE_SINGLE_MESSAGE_ID.to_string(),
            filename: SAMPLE_SINGLE_FILENAME.to_string(),
            body: single_body.clone(),
            segment_number: 1,
            total_segments: 1,
            total_file_size: single_body.len() as u64,
            offset: 0,
        }],
    };

    let multi_bodies = [
        deterministic_body(4096, 0x21),
        deterministic_body(4096, 0x43),
        deterministic_body(4096, 0x65),
    ];
    let multi_total = multi_bodies
        .iter()
        .map(|body| body.len() as u64)
        .sum::<u64>();
    let mut offset = 0u64;
    let mut multi_segments = Vec::new();
    for (idx, (message_id, body)) in SAMPLE_MULTI_MESSAGE_IDS
        .iter()
        .zip(multi_bodies)
        .enumerate()
    {
        let len = body.len() as u64;
        multi_segments.push(FixtureSegment {
            message_id: (*message_id).to_string(),
            filename: SAMPLE_MULTI_FILENAME.to_string(),
            body,
            segment_number: (idx + 1) as u32,
            total_segments: SAMPLE_MULTI_MESSAGE_IDS.len() as u32,
            total_file_size: multi_total,
            offset,
        });
        offset += len;
    }

    vec![
        single_file,
        FixtureFile {
            filename: SAMPLE_MULTI_FILENAME.to_string(),
            segments: multi_segments,
        },
    ]
}

pub fn fixture_segments() -> Vec<FixtureSegment> {
    fixture_files()
        .into_iter()
        .flat_map(|file| file.segments)
        .collect()
}

pub fn sample_mock_config() -> MockConfig {
    let segments = fixture_segments();
    let mut articles = HashMap::new();
    for segment in &segments {
        let (encoded, _crc) = yenc_simd::encode_article(
            &segment.body,
            &segment.filename,
            segment.segment_number,
            segment.total_segments,
            segment.offset,
            segment.total_file_size,
        );
        articles.insert(segment.message_id.clone(), encoded);
    }

    MockConfig {
        articles,
        groups: HashMap::from([(
            SAMPLE_GROUP.to_string(),
            (segments.len() as u64, 1, segments.len() as u64),
        )]),
        ..Default::default()
    }
}

pub async fn start_sample_server() -> MockNntpServer {
    MockNntpServer::start(sample_mock_config()).await
}

pub async fn start_sample_server_on(bind_addr: &str) -> MockNntpServer {
    MockNntpServer::start_on(bind_addr, sample_mock_config()).await
}

pub fn server_config(port: u16) -> ServerConfig {
    test_config(port)
}

fn deterministic_body(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|idx| seed.wrapping_add((idx % 251) as u8))
        .collect()
}
