use unicode_normalization::UnicodeNormalization;

/// Normalize a string to Unicode NFC form.
///
/// Used for filenames from sources not covered by nzb-core's parser
/// (e.g. yEnc headers from nzb-decode). NZB-derived names are already
/// normalized by nzb-core v0.1.1+.
pub fn normalize_nfc(s: &str) -> String {
    s.nfc().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfc_normalization_composes_decomposed() {
        // "café" in NFD: 'e' + combining acute accent (U+0301)
        let nfd = "caf\u{0065}\u{0301}";
        // "café" in NFC: precomposed 'é' (U+00E9)
        let nfc = "caf\u{00E9}";
        assert_eq!(normalize_nfc(nfd), nfc);
    }

    #[test]
    fn nfc_normalization_preserves_ascii() {
        let ascii = "My.Show.S01E01.720p.mkv";
        assert_eq!(normalize_nfc(ascii), ascii);
    }

    #[test]
    fn nfc_normalization_preserves_already_nfc() {
        let already_nfc = "Stra\u{00DF}e.nzb";
        assert_eq!(normalize_nfc(already_nfc), already_nfc);
    }

    #[test]
    fn nfc_normalization_handles_hangul() {
        // Hangul decomposed: ᄀ (U+1100) + ᅡ (U+1161) → 가 (U+AC00)
        let nfd = "\u{1100}\u{1161}";
        let nfc = "\u{AC00}";
        assert_eq!(normalize_nfc(nfd), nfc);
    }
}
