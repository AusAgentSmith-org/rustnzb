mod harness;

use std::time::Duration;

use harness::nzb_fixture::NzbFixture;
use harness::{HarnessBuilder, ServerProfile, yenc_articles};
use nzb_nntp::testutil::MockConfig;
use nzb_web::nzb_core::models::JobStatus;

#[tokio::test]
async fn global_pause_stops_dispatch_and_requires_global_resume() {
    let bodies: Vec<Vec<u8>> = (0..12)
        .map(|index| format!("pause-segment-{index}").into_bytes())
        .collect();
    let message_ids: Vec<String> = (0..12).map(|index| format!("pause-{index}@test")).collect();
    let segments: Vec<(&str, &[u8])> = message_ids
        .iter()
        .zip(&bodies)
        .map(|(message_id, body)| (message_id.as_str(), body.as_slice()))
        .collect();
    let fixture = NzbFixture::new("global-pause")
        .add_file("payload.bin", &segments)
        .build();
    let articles: Vec<(&str, &[u8], &str)> = fixture
        .articles
        .iter()
        .map(|(message_id, body, filename)| (*message_id, *body, filename.as_str()))
        .collect();
    let server = ServerProfile::start(
        "pause-server",
        MockConfig {
            articles: yenc_articles(&articles),
            response_delay: Some(Duration::from_millis(20)),
            ..Default::default()
        },
        1,
    )
    .await;
    let engine = HarnessBuilder::new().with_server(server).build();
    let job_id = engine
        .submit_nzb_xml("global-pause", fixture.xml)
        .expect("submit job");

    assert!(
        engine
            .wait_for(Duration::from_secs(5), |snapshot| {
                snapshot
                    .job(&job_id)
                    .is_some_and(|job| job.articles_downloaded >= 1)
            })
            .await,
        "download never started"
    );

    engine.queue_manager.pause_all();
    assert!(engine.queue_manager.is_paused());
    assert_eq!(engine.job(&job_id).unwrap().status, JobStatus::Paused);
    assert!(engine.queue_manager.resume_job(&job_id).is_err());

    // One already in-flight article may finish. Once that settles, no new
    // article may be dispatched until the global control resumes the queue.
    tokio::time::sleep(Duration::from_millis(350)).await;
    let settled = engine.job(&job_id).unwrap().articles_downloaded;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(engine.job(&job_id).unwrap().articles_downloaded, settled);

    engine.queue_manager.resume_all();
    assert!(
        engine
            .wait_for(Duration::from_secs(8), |snapshot| {
                snapshot
                    .job(&job_id)
                    .is_none_or(|job| job.articles_downloaded + job.articles_failed == 12)
            })
            .await,
        "download did not continue after global resume"
    );
}
