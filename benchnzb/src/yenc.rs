use crc32fast::Hasher;

const LINE_WIDTH: usize = 128;

/// yEnc encode a raw data block. Returns (encoded_body_with_headers, crc32).
pub fn encode_article(
    raw: &[u8],
    filename: &str,
    part: u32,
    total_parts: u32,
    file_offset: u64,
    total_file_size: u64,
) -> (Vec<u8>, u32) {
    let mut hasher = Hasher::new();
    hasher.update(raw);
    let crc = hasher.finalize();

    let mut out = Vec::with_capacity(raw.len() * 11 / 10 + 256);

    // =ybegin header
    if total_parts > 1 {
        out.extend_from_slice(
            format!(
                "=ybegin part={part} line={LINE_WIDTH} size={total_file_size} name={filename}\r\n"
            )
            .as_bytes(),
        );
        let begin = file_offset + 1;
        let end = file_offset + raw.len() as u64;
        out.extend_from_slice(format!("=ypart begin={begin} end={end}\r\n").as_bytes());
    } else {
        out.extend_from_slice(
            format!("=ybegin line={LINE_WIDTH} size={total_file_size} name={filename}\r\n")
                .as_bytes(),
        );
    }

    // Encode body
    let mut line_pos: usize = 0;
    for &byte in raw {
        let encoded = byte.wrapping_add(42);

        // Escape critical bytes, plus TAB/SPACE/DOT at line start
        let escape = matches!(encoded, 0x00 | 0x0A | 0x0D | 0x3D)
            || (line_pos == 0 && matches!(encoded, 0x09 | 0x20 | 0x2E));

        if escape {
            out.push(b'=');
            out.push(encoded.wrapping_add(64));
            line_pos += 2;
        } else {
            out.push(encoded);
            line_pos += 1;
        }

        if line_pos >= LINE_WIDTH {
            out.extend_from_slice(b"\r\n");
            line_pos = 0;
        }
    }
    if line_pos > 0 {
        out.extend_from_slice(b"\r\n");
    }

    // =yend footer
    if total_parts > 1 {
        out.extend_from_slice(format!("=yend size={} pcrc32={crc:08X}\r\n", raw.len()).as_bytes());
    } else {
        out.extend_from_slice(format!("=yend size={} crc32={crc:08X}\r\n", raw.len()).as_bytes());
    }

    (out, crc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_part_round_trip_preserves_binary_data() {
        let raw = (0..=255).cycle().take(2_048).collect::<Vec<u8>>();
        let (encoded, crc) = encode_article(&raw, "all-bytes.bin", 1, 1, 0, raw.len() as u64);
        let decoded = yenc_simd::decode_yenc(&encoded).unwrap();

        assert_eq!(decoded.data, raw);
        assert_eq!(decoded.filename.as_deref(), Some("all-bytes.bin"));
        assert_eq!(decoded.crc32, crc);
    }

    #[test]
    fn multipart_headers_and_offsets_round_trip() {
        let raw = vec![0, 10, 13, 19, 27, 61, 214, 255];
        let (encoded, crc) = encode_article(&raw, "part.bin", 2, 3, 750_000, 2_000_000);
        let text = String::from_utf8_lossy(&encoded);
        let decoded = yenc_simd::decode_yenc(&encoded).unwrap();

        assert!(text.starts_with("=ybegin part=2 line=128 size=2000000 name=part.bin\r\n"));
        assert!(text.contains("=ypart begin=750001 end=750008\r\n"));
        assert!(text.ends_with(&format!("=yend size=8 pcrc32={crc:08X}\r\n")));
        assert_eq!(decoded.data, raw);
    }

    #[test]
    fn encoded_lines_respect_configured_width() {
        let raw = vec![42; 10_000];
        let (encoded, _) = encode_article(&raw, "lines.bin", 1, 1, 0, raw.len() as u64);
        let text = String::from_utf8(encoded).unwrap();
        let body_lines = text
            .lines()
            .skip(1)
            .take_while(|line| !line.starts_with("=yend"));

        assert!(body_lines.into_iter().all(|line| line.len() <= LINE_WIDTH));
    }
}
