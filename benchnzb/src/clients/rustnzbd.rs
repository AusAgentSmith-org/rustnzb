use super::StageTiming;
use anyhow::Result;

pub struct RustnzbdClient {
    url: String,
    http: reqwest::Client,
}

impl RustnzbdClient {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn healthy(&self) -> bool {
        self.http
            .get(format!("{}/api/status", self.url))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn add_nzb(&self, data: &[u8], filename: &str) -> Result<()> {
        let part = reqwest::multipart::Part::bytes(data.to_vec())
            .file_name(filename.to_string())
            .mime_str("application/x-nzb")?;
        let form = reqwest::multipart::Form::new().part("nzbfile", part);

        let resp = self
            .http
            .post(format!("{}/api/queue/add", self.url))
            .multipart(form)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        resp.error_for_status_ref()?;
        let body: serde_json::Value = resp.json().await?;
        tracing::debug!("rustnzbd add response: {body}");
        Ok(())
    }

    pub async fn all_finished(&self) -> Result<bool> {
        // Queue empty = download phase done, but check history for post-processing
        let queue: serde_json::Value = self
            .http
            .get(format!("{}/api/queue", self.url))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?
            .json()
            .await?;

        let jobs = queue["jobs"].as_array().cloned().unwrap_or_default();
        if !jobs.is_empty() {
            // Check if all jobs are "completed" status
            return Ok(jobs.iter().all(|j| {
                j["status"]
                    .as_str()
                    .map_or(false, |s| s == "completed" || s == "failed")
            }));
        }

        // Check history
        let history: serde_json::Value = self
            .http
            .get(format!("{}/api/history", self.url))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?
            .json()
            .await?;

        let entries = history["entries"].as_array().cloned().unwrap_or_default();
        if entries.is_empty() {
            return Ok(false);
        }
        Ok(entries.iter().all(|e| {
            e["status"]
                .as_str()
                .map_or(false, |s| s == "completed" || s == "failed")
        }))
    }

    pub async fn progress_fraction(&self) -> f64 {
        let queue: serde_json::Value = match self
            .http
            .get(format!("{}/api/queue", self.url))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .and_then(|r| Ok(r))
        {
            Ok(r) => r.json().await.unwrap_or_default(),
            Err(_) => serde_json::Value::default(),
        };

        let jobs = queue["jobs"].as_array().cloned().unwrap_or_default();
        if jobs.is_empty() {
            if self.all_finished().await.unwrap_or(false) {
                return 1.0;
            }
            return 0.0;
        }

        let mut total_progress = 0.0;
        let mut count = 0;
        for job in &jobs {
            let total = job["total_bytes"].as_u64().unwrap_or(1) as f64;
            let downloaded = job["downloaded_bytes"].as_u64().unwrap_or(0) as f64;
            total_progress += downloaded / total;
            count += 1;
        }
        if count > 0 {
            total_progress / count as f64
        } else {
            0.0
        }
    }

    pub async fn download_speed(&self) -> f64 {
        let queue: serde_json::Value = match self
            .http
            .get(format!("{}/api/queue", self.url))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(r) => r.json().await.unwrap_or_default(),
            Err(_) => serde_json::Value::default(),
        };

        queue["speed_bps"]
            .as_u64()
            .map(|v| v as f64)
            .or_else(|| queue["speed_bps"].as_f64())
            .unwrap_or(0.0)
    }

    pub async fn get_stage_timing(&self) -> Result<StageTiming> {
        let history: serde_json::Value = self
            .http
            .get(format!("{}/api/history", self.url))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?
            .json()
            .await?;

        let mut timing = StageTiming::default();

        let entries = history["entries"].as_array().cloned().unwrap_or_default();
        for entry in &entries {
            // rustnzbd provides stages with duration_secs
            if let Some(stages) = entry["stages"].as_array() {
                for stage in stages {
                    let name = stage["name"].as_str().unwrap_or("");
                    let duration = stage["duration_secs"].as_f64().unwrap_or(0.0);
                    match name {
                        "Verify" | "Repair" => timing.par2_sec += duration,
                        "Extract" => timing.unpack_sec += duration,
                        _ => {}
                    }
                }
            }

            // Download time from added_at -> first stage start or completed_at
            if let (Some(added), Some(completed)) = (
                entry["added_at"].as_str(),
                entry["completed_at"].as_str(),
            ) {
                if let (Ok(a), Ok(c)) = (
                    chrono::DateTime::parse_from_rfc3339(added),
                    chrono::DateTime::parse_from_rfc3339(completed),
                ) {
                    let total = (c - a).num_seconds() as f64;
                    timing.download_sec = total - timing.par2_sec - timing.unpack_sec;
                    if timing.download_sec < 0.0 {
                        timing.download_sec = 0.0;
                    }
                }
            }
        }

        Ok(timing)
    }

    pub async fn clear_all(&self) {
        // Delete queue
        let queue: serde_json::Value = match self
            .http
            .get(format!("{}/api/queue", self.url))
            .send()
            .await
        {
            Ok(r) => r.json().await.unwrap_or_default(),
            Err(_) => serde_json::Value::default(),
        };

        if let Some(jobs) = queue["jobs"].as_array() {
            for job in jobs {
                if let Some(id) = job["id"].as_str() {
                    let _ = self
                        .http
                        .delete(format!("{}/api/queue/{id}", self.url))
                        .send()
                        .await;
                }
            }
        }

        // Clear history
        let _ = self
            .http
            .delete(format!("{}/api/history", self.url))
            .send()
            .await;
    }
}
