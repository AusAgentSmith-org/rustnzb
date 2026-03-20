use std::path::Path;

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::error::NzbError;
use crate::models::{Article, JobStatus, NzbFile, NzbJob, Priority};

/// Parse an NZB XML file into an NzbJob.
pub fn parse_nzb(name: &str, data: &[u8]) -> Result<NzbJob, NzbError> {
    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);

    let mut files: Vec<NzbFile> = Vec::new();
    let mut current_file: Option<FileBuilder> = None;
    let mut current_groups: Vec<String> = Vec::new();
    let mut current_segments: Vec<SegmentBuilder> = Vec::new();
    let mut in_groups = false;
    let mut in_segments = false;
    let mut buf = Vec::new();
    let mut meta_password: Option<String> = None;
    let mut reading_password_meta = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"file" => {
                    let mut subject = String::new();
                    let mut date = 0i64;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"subject" => {
                                subject = String::from_utf8_lossy(&attr.value).into_owned();
                            }
                            b"date" => {
                                date = String::from_utf8_lossy(&attr.value)
                                    .parse()
                                    .unwrap_or(0);
                            }
                            _ => {}
                        }
                    }
                    current_file = Some(FileBuilder {
                        subject,
                        _date: date,
                    });
                    current_groups.clear();
                    current_segments.clear();
                }
                b"groups" => in_groups = true,
                b"group" => {}
                b"segments" => in_segments = true,
                b"segment" => {
                    let mut number = 0u32;
                    let mut bytes = 0u64;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"number" => {
                                number = String::from_utf8_lossy(&attr.value)
                                    .parse()
                                    .unwrap_or(0);
                            }
                            b"bytes" => {
                                bytes = String::from_utf8_lossy(&attr.value)
                                    .parse()
                                    .unwrap_or(0);
                            }
                            _ => {}
                        }
                    }
                    current_segments.push(SegmentBuilder {
                        number,
                        bytes,
                        message_id: String::new(),
                    });
                }
                b"meta" => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"type" && attr.value.as_ref() == b"password" {
                            reading_password_meta = true;
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"file" => {
                    if let Some(fb) = current_file.take() {
                        let filename = extract_filename(&fb.subject);
                        let total_bytes: u64 = current_segments.iter().map(|s| s.bytes).sum();
                        let articles: Vec<Article> = current_segments
                            .drain(..)
                            .map(|s| Article {
                                message_id: s.message_id,
                                segment_number: s.number,
                                bytes: s.bytes,
                                downloaded: false,
                                data_begin: None,
                                data_size: None,
                                crc32: None,
                                tried_servers: Vec::new(),
                                tries: 0,
                            })
                            .collect();

                        let is_par2 = filename.to_lowercase().ends_with(".par2");
                        let (par2_setname, par2_vol, par2_blocks) = if is_par2 {
                            parse_par2_filename(&filename)
                        } else {
                            (None, None, None)
                        };

                        files.push(NzbFile {
                            id: uuid::Uuid::new_v4().to_string(),
                            filename,
                            bytes: total_bytes,
                            bytes_downloaded: 0,
                            is_par2,
                            par2_setname,
                            par2_vol,
                            par2_blocks,
                            assembled: false,
                            groups: current_groups.clone(),
                            articles,
                        });
                    }
                }
                b"groups" => in_groups = false,
                b"segments" => in_segments = false,
                _ => {}
            },
            Ok(Event::Text(ref t)) => {
                let text = t.unescape().unwrap_or_default().into_owned();
                if reading_password_meta {
                    meta_password = Some(text);
                    reading_password_meta = false;
                } else if in_groups {
                    current_groups.push(text);
                } else if in_segments {
                    if let Some(seg) = current_segments.last_mut() {
                        seg.message_id = text;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(NzbError::ParseError(format!("XML error: {e}"))),
            _ => {}
        }
        buf.clear();
    }

    if files.is_empty() {
        return Err(NzbError::InvalidNzb("No files found in NZB".into()));
    }

    let total_bytes: u64 = files.iter().map(|f| f.bytes).sum();
    let article_count: usize = files.iter().map(|f| f.articles.len()).sum();
    let file_count = files.len();

    Ok(NzbJob {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        category: "Default".into(),
        status: JobStatus::Queued,
        priority: Priority::Normal,
        total_bytes,
        downloaded_bytes: 0,
        file_count,
        files_completed: 0,
        article_count,
        articles_downloaded: 0,
        articles_failed: 0,
        added_at: chrono::Utc::now(),
        completed_at: None,
        work_dir: std::path::PathBuf::new(), // Set by queue manager
        output_dir: std::path::PathBuf::new(),
        password: meta_password,
        error_message: None,
        speed_bps: 0,
        server_stats: Vec::new(),
        files,
    })
}

/// Parse NZB from a file path.
pub fn parse_nzb_file(path: &Path) -> Result<NzbJob, NzbError> {
    let data = std::fs::read(path)?;
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Unknown".into());
    parse_nzb(&name, &data)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct FileBuilder {
    subject: String,
    _date: i64,
}

struct SegmentBuilder {
    number: u32,
    bytes: u64,
    message_id: String,
}

/// Extract a filename from an NZB subject line.
/// Common pattern: `"Some Post" filename.ext (01/10)`
fn extract_filename(subject: &str) -> String {
    // Try to find quoted filename first
    if let Some(start) = subject.find('"') {
        if let Some(end) = subject[start + 1..].find('"') {
            return subject[start + 1..start + 1 + end].to_string();
        }
    }

    // Try to find filename before (xx/yy) pattern
    if let Some(paren_pos) = subject.rfind('(') {
        let before_paren = subject[..paren_pos].trim();
        // Take the last space-separated token as filename
        if let Some(last_space) = before_paren.rfind(' ') {
            let candidate = &before_paren[last_space + 1..];
            if candidate.contains('.') {
                return candidate.to_string();
            }
        }
        if before_paren.contains('.') {
            return before_paren.to_string();
        }
    }

    subject.to_string()
}

/// Parse par2 filename for volume/block info.
/// Pattern: `setname.vol00+01.par2`
fn parse_par2_filename(filename: &str) -> (Option<String>, Option<u32>, Option<u32>) {
    let lower = filename.to_lowercase();
    if !lower.ends_with(".par2") {
        return (None, None, None);
    }

    let without_ext = &filename[..filename.len() - 5];

    // Check for .volNN+NN pattern
    if let Some(vol_pos) = without_ext.to_lowercase().rfind(".vol") {
        let setname = without_ext[..vol_pos].to_string();
        let vol_part = &without_ext[vol_pos + 4..];

        if let Some(plus_pos) = vol_part.find('+') {
            let vol: u32 = vol_part[..plus_pos].parse().unwrap_or(0);
            let blocks: u32 = vol_part[plus_pos + 1..].parse().unwrap_or(0);
            return (Some(setname), Some(vol), Some(blocks));
        }
    }

    // No volume info — this is the index par2
    let setname = without_ext.to_string();
    (Some(setname), None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_filename_quoted() {
        let subject = r#"Some Poster "myfile.rar" (01/10)"#;
        assert_eq!(extract_filename(subject), "myfile.rar");
    }

    #[test]
    fn test_parse_par2_with_volume() {
        let (set, vol, blocks) = parse_par2_filename("MyDownload.vol00+01.par2");
        assert_eq!(set, Some("MyDownload".into()));
        assert_eq!(vol, Some(0));
        assert_eq!(blocks, Some(1));
    }

    #[test]
    fn test_parse_par2_index() {
        let (set, vol, blocks) = parse_par2_filename("MyDownload.par2");
        assert_eq!(set, Some("MyDownload".into()));
        assert_eq!(vol, None);
        assert_eq!(blocks, None);
    }

    #[test]
    fn test_parse_nzb_basic() {
        let nzb_data = br#"<?xml version="1.0" encoding="UTF-8"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="test@example.com" date="1234567890" subject="test.rar (1/2)">
    <groups><group>alt.binaries.test</group></groups>
    <segments>
      <segment number="1" bytes="768000">article1@example.com</segment>
      <segment number="2" bytes="768000">article2@example.com</segment>
    </segments>
  </file>
</nzb>"#;

        let job = parse_nzb("test", nzb_data).unwrap();
        assert_eq!(job.name, "test");
        assert_eq!(job.file_count, 1);
        assert_eq!(job.article_count, 2);
        assert_eq!(job.total_bytes, 1536000);
        assert_eq!(job.files[0].articles[0].message_id, "article1@example.com");
    }
}
