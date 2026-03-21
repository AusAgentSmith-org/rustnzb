use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use tracing::{info, warn};

use nzb_core::config::RssFeedConfig;

use crate::queue_manager::QueueManager;

/// Background RSS feed monitor that polls configured feeds for NZB links
/// and automatically enqueues them for download.
pub struct RssMonitor {
    feeds: Vec<RssFeedConfig>,
    queue_manager: Arc<QueueManager>,
    seen: Arc<Mutex<HashSet<String>>>,
    seen_file: PathBuf,
}

impl RssMonitor {
    pub fn new(
        feeds: Vec<RssFeedConfig>,
        queue_manager: Arc<QueueManager>,
        data_dir: PathBuf,
    ) -> Self {
        let seen_file = data_dir.join("rss_seen.json");
        let seen = Self::load_seen(&seen_file);
        Self {
            feeds,
            queue_manager,
            seen: Arc::new(Mutex::new(seen)),
            seen_file,
        }
    }

    fn load_seen(path: &PathBuf) -> HashSet<String> {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save_seen(&self) {
        let seen = self.seen.lock();
        if let Ok(json) = serde_json::to_string(&*seen) {
            let _ = std::fs::write(&self.seen_file, json);
        }
    }

    /// Run the monitor loop forever, polling feeds at their configured intervals.
    pub async fn run(self) {
        info!("Starting RSS monitor with {} feed(s)", self.feeds.len());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        loop {
            for feed in &self.feeds {
                if !feed.enabled {
                    continue;
                }

                if let Err(e) = self.check_feed(&client, feed).await {
                    warn!(feed = %feed.name, error = %e, "RSS feed check failed");
                }
            }

            self.save_seen();

            // Use the minimum poll interval across all enabled feeds, defaulting to 15 min
            let interval = self
                .feeds
                .iter()
                .filter(|f| f.enabled)
                .map(|f| f.poll_interval_secs)
                .min()
                .unwrap_or(900);

            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    }

    async fn check_feed(
        &self,
        client: &reqwest::Client,
        feed: &RssFeedConfig,
    ) -> anyhow::Result<()> {
        info!(feed = %feed.name, url = %feed.url, "Checking RSS feed");

        let response = client.get(&feed.url).send().await?;
        let body = response.bytes().await?;
        let parsed = feed_rs::parser::parse(&body[..])?;

        // Compile filter regex if provided
        let filter = feed
            .filter_regex
            .as_ref()
            .and_then(|r| regex::Regex::new(r).ok());

        for entry in &parsed.entries {
            let title = entry
                .title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_default();
            let entry_id = entry.id.clone();

            // Skip if already seen
            if self.seen.lock().contains(&entry_id) {
                continue;
            }

            // Apply filter
            if let Some(ref re) = filter {
                if !re.is_match(&title) {
                    continue;
                }
            }

            // Find NZB URL from links or media content
            let nzb_url = entry
                .links
                .iter()
                .find(|l| {
                    l.href.ends_with(".nzb")
                        || l.media_type
                            .as_deref()
                            .is_some_and(|mt| mt == "application/x-nzb")
                })
                .map(|l| l.href.clone())
                .or_else(|| {
                    // Check media content for NZB URLs
                    entry
                        .media
                        .iter()
                        .flat_map(|m| &m.content)
                        .find(|c| {
                            c.url
                                .as_ref()
                                .is_some_and(|u| u.as_str().ends_with(".nzb"))
                        })
                        .and_then(|c| c.url.as_ref().map(|u| u.to_string()))
                })
                .or_else(|| {
                    // Fall back to first link
                    entry.links.first().map(|l| l.href.clone())
                });

            let Some(url) = nzb_url else { continue };

            info!(feed = %feed.name, title = %title, url = %url, "Found new NZB in RSS feed");

            match self.fetch_and_enqueue(client, &url, &title, feed).await {
                Ok(()) => {
                    self.seen.lock().insert(entry_id);
                    info!(title = %title, "RSS item enqueued successfully");
                }
                Err(e) => {
                    warn!(title = %title, error = %e, "Failed to enqueue RSS item");
                }
            }
        }

        Ok(())
    }

    async fn fetch_and_enqueue(
        &self,
        client: &reqwest::Client,
        url: &str,
        name: &str,
        feed: &RssFeedConfig,
    ) -> anyhow::Result<()> {
        let response = client.get(url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("HTTP {}", response.status());
        }
        let data = response.bytes().await?;

        let mut job = nzb_core::nzb_parser::parse_nzb(name, &data)?;

        if let Some(ref cat) = feed.category {
            job.category = cat.clone();
        }

        job.work_dir = self.queue_manager.incomplete_dir().join(&job.id);
        job.output_dir = if let Some(ref cat) = feed.category {
            self.queue_manager.complete_dir().join(cat)
        } else {
            self.queue_manager.complete_dir().to_path_buf()
        };

        std::fs::create_dir_all(&job.work_dir)?;

        self.queue_manager.add_job(job, Some(data.to_vec()))?;
        Ok(())
    }
}
