//! yEnc decoder.
//!
//! yEnc encoding: each byte is (original_byte + 42) % 256
//! Escape sequences: =char means (char - 64 - 42) mod 256
//! Critical characters that must be escaped: NUL, LF, CR, '='
//!
//! Header format:
//!   =ybegin part=1 line=128 size=768000 name=file.bin
//!   =ypart begin=1 end=768000
//! Footer format:
//!   =yend size=768000 part=1 pcrc32=AABBCCDD
//!
//! NNTP articles include headers before the yEnc body:
//!   From: poster@example.com
//!   Subject: ...
//!   Message-Id: <...>
//!   (blank line)
//!   =ybegin ...
//!   (encoded data)
//!   =yend ...

use thiserror::Error;

#[derive(Error, Debug)]
pub enum YencError {
    #[error("Missing =ybegin header")]
    MissingHeader,
    #[error("Missing =yend footer")]
    MissingFooter,
    #[error("CRC32 mismatch: expected {expected:08X}, got {actual:08X}")]
    CrcMismatch { expected: u32, actual: u32 },
    #[error("Invalid yEnc data: {0}")]
    InvalidData(String),
}

/// Result of decoding a yEnc article.
#[derive(Debug)]
pub struct YencDecodeResult {
    /// Decoded binary data.
    pub data: Vec<u8>,
    /// Filename from =ybegin header.
    pub filename: Option<String>,
    /// Byte offset in the final file (from =ypart begin=, 0-indexed).
    pub part_begin: Option<u64>,
    /// Byte offset end in the final file (from =ypart end=).
    pub part_end: Option<u64>,
    /// Total file size (from =ybegin size=).
    pub file_size: Option<u64>,
    /// Part number.
    pub part_number: Option<u32>,
    /// CRC32 of the decoded data.
    pub crc32: u32,
}

/// Decode a yEnc-encoded article body.
///
/// Handles raw NNTP article data including headers before the yEnc body.
/// The data is processed as raw bytes since yEnc-encoded lines are binary.
pub fn decode_yenc(raw: &[u8]) -> Result<YencDecodeResult, YencError> {
    // Split into lines on \n boundaries (handles both \r\n and \n)
    let lines = split_lines(raw);

    // 1. Find =ybegin line (skip NNTP headers)
    let mut ybegin_idx = None;
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with(b"=ybegin ") || line == b"=ybegin" {
            ybegin_idx = Some(i);
            break;
        }
    }
    let ybegin_idx = ybegin_idx.ok_or(YencError::MissingHeader)?;
    let ybegin_str = lossy_str(&lines[ybegin_idx]);

    let filename = extract_param(&ybegin_str, "name");
    let file_size = extract_param(&ybegin_str, "size").and_then(|s| s.parse().ok());
    let part_number = extract_param(&ybegin_str, "part").and_then(|s| s.parse().ok());

    // 2. Scan remaining lines for =ypart, data, and =yend
    let mut part_begin: Option<u64> = None;
    let mut part_end: Option<u64> = None;
    let mut data_start = ybegin_idx + 1;
    let mut data_end = lines.len();
    let mut yend_str = String::new();
    let mut found_yend = false;

    for i in (ybegin_idx + 1)..lines.len() {
        let line = &lines[i];
        if line.starts_with(b"=ypart ") {
            let s = lossy_str(line);
            part_begin = extract_param(&s, "begin").and_then(|v| v.parse().ok());
            part_end = extract_param(&s, "end").and_then(|v| v.parse().ok());
            data_start = i + 1;
        } else if line.starts_with(b"=yend") {
            yend_str = lossy_str(line);
            data_end = i;
            found_yend = true;
            break;
        }
    }

    if !found_yend {
        return Err(YencError::MissingFooter);
    }

    // 3. Decode data lines (binary — NOT utf8)
    let mut decoded = Vec::with_capacity((data_end - data_start) * 128);
    for i in data_start..data_end {
        decode_line(&lines[i], &mut decoded);
    }

    // 4. CRC32
    let crc = crc32fast::hash(&decoded);

    if let Some(expected_crc_str) = extract_param(&yend_str, "pcrc32")
        .or_else(|| extract_param(&yend_str, "crc32"))
    {
        if let Ok(expected_crc) = u32::from_str_radix(&expected_crc_str, 16) {
            if crc != expected_crc {
                return Err(YencError::CrcMismatch {
                    expected: expected_crc,
                    actual: crc,
                });
            }
        }
    }

    // Adjust part_begin to 0-indexed (yEnc uses 1-based)
    let part_begin = part_begin.map(|b| b.saturating_sub(1));

    Ok(YencDecodeResult {
        data: decoded,
        filename,
        part_begin,
        part_end,
        file_size,
        part_number,
        crc32: crc,
    })
}

/// Decode a single yEnc-encoded line into the output buffer.
fn decode_line(line: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    let len = line.len();
    while i < len {
        let b = line[i];
        if b == b'=' && i + 1 < len {
            // Escape sequence: next byte - 64, then the standard -42
            out.push(line[i + 1].wrapping_sub(106)); // (byte - 64 - 42) mod 256
            i += 2;
        } else {
            out.push(b.wrapping_sub(42));
            i += 1;
        }
    }
}

/// Split raw bytes into lines on \n boundaries, stripping trailing \r.
fn split_lines(data: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0;
    for i in 0..data.len() {
        if data[i] == b'\n' {
            let mut end = i;
            // Strip trailing \r
            if end > start && data[end - 1] == b'\r' {
                end -= 1;
            }
            lines.push(&data[start..end]);
            start = i + 1;
        }
    }
    // Last line without trailing newline
    if start < data.len() {
        let mut end = data.len();
        if end > start && data[end - 1] == b'\r' {
            end -= 1;
        }
        lines.push(&data[start..end]);
    }
    lines
}

/// Convert a byte slice to a string lossily (for parsing header/footer params).
fn lossy_str(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Extract a named parameter from a yEnc header/footer line.
/// e.g., extract_param("=ybegin part=1 size=1234 name=file.bin", "size") → Some("1234")
fn extract_param(line: &str, param: &str) -> Option<String> {
    let search = format!("{param}=");

    // Special handling for "name=" which takes the rest of the line
    if param == "name" {
        if let Some(pos) = line.find(&search) {
            return Some(line[pos + search.len()..].to_string());
        }
        return None;
    }

    if let Some(pos) = line.find(&search) {
        let start = pos + search.len();
        let rest = &line[start..];
        let end = rest.find(' ').unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_line_basic() {
        let hello = b"Hello";
        let encoded: Vec<u8> = hello.iter().map(|b| b.wrapping_add(42)).collect();
        let mut decoded = Vec::new();
        decode_line(&encoded, &mut decoded);
        assert_eq!(decoded, hello);
    }

    #[test]
    fn test_extract_param() {
        let line = "=ybegin part=1 line=128 size=768000 name=test file.bin";
        assert_eq!(extract_param(line, "part"), Some("1".into()));
        assert_eq!(extract_param(line, "size"), Some("768000".into()));
        assert_eq!(extract_param(line, "name"), Some("test file.bin".into()));
        assert_eq!(extract_param(line, "missing"), None);
    }

    #[test]
    fn test_full_decode() {
        let original: Vec<u8> = (65..80).collect(); // A-O (safe range)
        let encoded_line: String = original
            .iter()
            .map(|b| (b.wrapping_add(42)) as char)
            .collect();
        let crc = crc32fast::hash(&original);

        let article = format!(
            "=ybegin part=1 line=128 size={} name=test.bin\n\
             =ypart begin=1 end={}\n\
             {encoded_line}\n\
             =yend size={} part=1 pcrc32={crc:08X}\n",
            original.len(), original.len(), original.len(),
        );

        let result = decode_yenc(article.as_bytes()).unwrap();
        assert_eq!(result.data, original);
        assert_eq!(result.filename, Some("test.bin".into()));
        assert_eq!(result.part_begin, Some(0));
        assert_eq!(result.file_size, Some(original.len() as u64));
        assert_eq!(result.crc32, crc);
    }

    #[test]
    fn test_decode_with_nntp_headers() {
        // Simulate real NNTP article with headers before yEnc body
        let original: Vec<u8> = (65..80).collect();
        let encoded_line: String = original
            .iter()
            .map(|b| (b.wrapping_add(42)) as char)
            .collect();
        let crc = crc32fast::hash(&original);

        let article = format!(
            "From: poster@example.com\r\n\
             Newsgroups: alt.binaries.test\r\n\
             Subject: test post\r\n\
             Message-Id: <test@example.com>\r\n\
             \r\n\
             =ybegin part=1 line=128 size={} name=test.bin\r\n\
             =ypart begin=1 end={}\r\n\
             {encoded_line}\r\n\
             =yend size={} part=1 pcrc32={crc:08X}\r\n",
            original.len(), original.len(), original.len(),
        );

        let result = decode_yenc(article.as_bytes()).unwrap();
        assert_eq!(result.data, original);
        assert_eq!(result.filename, Some("test.bin".into()));
    }

    #[test]
    fn test_split_lines() {
        let data = b"line1\r\nline2\nline3\r\n";
        let lines = split_lines(data);
        assert_eq!(lines, vec![b"line1".as_slice(), b"line2", b"line3"]);
    }
}
