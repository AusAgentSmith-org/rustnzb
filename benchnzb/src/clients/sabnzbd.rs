use super::StageTiming;
use crate::config;
use anyhow::Result;

pub struct SabnzbdClient {
    url: String,
    http: reqwest::Client,
    api_key: String,
}

impl SabnzbdClient {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
            api_key: config::SABNZBD_API_KEY.to_string(),
        }
    }

    async fn api_get(&self, params: &[(&str, &str)]) -> Result<serde_json::Value> {
        let mut all_params = vec![("apikey", self.api_key.as_str()), ("output", "json")];
        all_params.extend_from_slice(params);
        let resp = self
            .http
            .get(format!("{}/api", self.url))
            .query(&all_params)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn healthy(&self) -> bool {
        self.api_get(&[("mode", "version")])
            .await
            .is_ok()
    }

    pub async fn add_nzb(&self, data: &[u8], filename: &str) -> Result<()> {
        let part = reqwest::multipart::Part::bytes(data.to_vec())
            .file_name(filename.to_string())
            .mime_str("application/x-nzb")?;
        let form = reqwest::multipart::Form::new()
            .text("mode", "addfile")
            .text("apikey", self.api_key.clone())
            .text("output", "json")
            .part("nzbfile", part);

        let resp = self
            .http
            .post(format!("{}/api", self.url))
            .multipart(form)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        let body: serde_json::Value = resp.json().await?;
        tracing::debug!("SABnzbd addfile response: {body}");
        Ok(())
    }

    pub async fn all_finished(&self) -> Result<bool> {
        // Queue must be empty
        let queue = self.api_get(&[("mode", "queue")]).await?;
        if let Some(slots) = queue["queue"]["slots"].as_array() {
            if !slots.is_empty() {
                return Ok(false);
            }
        }

        // History must have completed entries
        let history = self.api_get(&[("mode", "history")]).await?;
        let slots = history["history"]["slots"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if slots.is_empty() {
            return Ok(false);
        }
        Ok(slots.iter().all(|s| {
            s["status"]
                .as_str()
                .map_or(false, |st| st == "Completed" || st == "Failed")
        }))
    }

    pub async fn progress_fraction(&self) -> f64 {
        let queue = self.api_get(&[("mode", "queue")]).await.unwrap_or_default();
        if let Some(slots) = queue["queue"]["slots"].as_array() {
            if slots.is_empty() {
                if self.all_finished().await.unwrap_or(false) {
                    return 1.0;
                }
                return 0.0;
            }
            let total: f64 = slots
                .iter()
                .filter_map(|s| {
                    s["percentage"]
                        .as_str()
                        .and_then(|p| p.parse::<f64>().ok())
                })
                .sum();
            return total / 100.0 / slots.len() as f64;
        }
        0.0
    }

    pub async fn download_speed(&self) -> f64 {
        let queue = self.api_get(&[("mode", "queue")]).await.unwrap_or_default();
        queue["queue"]["kbpersec"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .map(|kbps| kbps * 1024.0)
            .unwrap_or(0.0)
    }

    pub async fn get_stage_timing(&self) -> Result<StageTiming> {
        let history = self
            .api_get(&[("mode", "history"), ("limit", "1")])
            .await?;
        let mut timing = StageTiming::default();

        if let Some(slots) = history["history"]["slots"].as_array() {
            // Only read the most recent entry to avoid stale history contamination
            if let Some(slot) = slots.first() {
                if let Some(dl) = slot["download_time"].as_u64() {
                    timing.download_sec = dl as f64;
                }
                let pp = slot["postproc_time"].as_u64().unwrap_or(0) as f64;
                if let Some(stages) = slot["stage_log"].as_array() {
                    for stage in stages {
                        let name = stage["name"].as_str().unwrap_or("");
                        if let Some(actions) = stage["actions"].as_array() {
                            for action in actions {
                                let text = action.as_str().unwrap_or("");
                                if name.contains("Repair") || name.contains("Verif") {
                                    if let Some(secs) = parse_duration(text) {
                                        timing.par2_sec += secs;
                                    }
                                }
                                if name.contains("Unpack") {
                                    if let Some(secs) = parse_duration(text) {
                                        timing.unpack_sec += secs;
                                    }
                                }
                            }
                        }
                    }
                }
                // Fallback: if we couldn't parse stages, assign all pp time to par2
                if timing.par2_sec == 0.0 && timing.unpack_sec == 0.0 && pp > 0.0 {
                    timing.par2_sec = pp;
                }
            }
        }
        Ok(timing)
    }

    pub async fn clear_all(&self) {
        let _ = self
            .api_get(&[("mode", "queue"), ("name", "delete"), ("value", "all")])
            .await;
        let _ = self
            .api_get(&[
                ("mode", "history"),
                ("name", "delete"),
                ("value", "all"),
            ])
            .await;
    }
}

fn parse_duration(text: &str) -> Option<f64> {
    let text = text.to_lowercase();
    let mut total: f64 = 0.0;
    if let Some(idx) = text.find("min") {
        if let Some(num_str) = text[..idx].trim().split_whitespace().last() {
            if let Ok(m) = num_str.parse::<f64>() {
                total += m * 60.0;
            }
        }
    }
    if let Some(idx) = text.find("sec") {
        if let Some(num_str) = text[..idx].trim().split_whitespace().last() {
            if let Ok(s) = num_str.parse::<f64>() {
                total += s;
            }
        }
    }
    if total > 0.0 {
        Some(total)
    } else {
        None
    }
}
