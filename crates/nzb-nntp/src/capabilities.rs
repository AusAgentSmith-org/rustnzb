//! NNTP server capability detection (RFC 3977 §5.2 + common extensions).
//!
//! Mirrors SABnzbd's `have_body` / `have_stat` model: query the server once at
//! connect time, store the derived per-server feature flags, and let higher
//! layers pick between BODY/ARTICLE (content fetch) and STAT (existence probe)
//! based on what the server actually advertises — instead of blindly issuing
//! BODY on every connection.
//!
//! Graceful degradation: if the server rejects CAPABILITIES (pre-RFC 3977),
//! we fall back to a conservative default where all four content commands
//! (ARTICLE / BODY / HEAD / STAT) are assumed available. This matches how
//! clients behaved before CAPABILITIES existed.

use std::str;

/// Server-advertised NNTP capabilities + derived per-command feature flags.
///
/// `probed` distinguishes "server told us what it supports" from "we assumed
/// the defaults because CAPABILITIES isn't implemented." Callers can use this
/// to decide whether to trust `have_body` as a hard gate or treat it as a
/// best-effort hint.
#[derive(Debug, Clone, Default)]
pub struct NntpCapabilities {
    /// `VERSION` line (e.g. "2").
    pub version: Option<String>,
    /// `IMPLEMENTATION` line (e.g. "INN 2.7.2").
    pub implementation: Option<String>,
    /// `READER` advertised → ARTICLE/BODY/HEAD/STAT supported (RFC 3977 §5.3).
    pub reader: bool,
    /// `POST` advertised.
    pub post: bool,
    /// `IHAVE` advertised.
    pub ihave: bool,
    /// `NEWNEWS` advertised.
    pub newnews: bool,
    /// `HDR` advertised (successor to XHDR).
    pub hdr: bool,
    /// `OVER` advertised (successor to XOVER).
    pub over: bool,
    /// `OVER MSGID` — server accepts `OVER <message-id>` variant.
    pub over_msgid: bool,
    /// `LIST` keywords advertised (ACTIVE, NEWSGROUPS, OVERVIEW.FMT, ...).
    pub list_keywords: Vec<String>,
    /// Server requires `MODE READER` to transition from transit to reader mode.
    pub mode_reader_required: bool,
    /// `COMPRESS` / `XFEATURE COMPRESS` advertised.
    pub compress: bool,
    /// Raw capability tokens we didn't categorize.
    pub other: Vec<String>,

    // ------------------------------------------------------------------
    // Derived per-command flags — the thing callers actually care about.
    // ------------------------------------------------------------------
    /// BODY command expected to work. True when READER mode is active.
    pub have_body: bool,
    /// STAT command expected to work. True when READER mode is active.
    pub have_stat: bool,
    /// ARTICLE command expected to work. True when READER mode is active.
    pub have_article: bool,
    /// HEAD command expected to work. True when READER mode is active.
    pub have_head: bool,

    /// True if the server responded with 101 (CAPABILITIES supported).
    /// False → pre-RFC 3977 server; flags are conservative defaults.
    pub probed: bool,
}

impl NntpCapabilities {
    /// Conservative defaults for a server that does not implement CAPABILITIES.
    /// Assumes a reader-mode server supporting the standard content commands —
    /// which is how every NNTP client behaved pre-RFC 3977.
    pub fn default_assumed() -> Self {
        Self {
            have_article: true,
            have_body: true,
            have_head: true,
            have_stat: true,
            reader: true,
            probed: false,
            ..Default::default()
        }
    }

    /// Parse a CAPABILITIES multi-line response body (everything between the
    /// `101` status line and the terminating `.`).
    ///
    /// Each line is a capability label followed by zero or more arguments:
    ///
    /// ```text
    /// VERSION 2
    /// READER
    /// IHAVE
    /// POST
    /// HDR
    /// OVER MSGID
    /// LIST ACTIVE NEWSGROUPS OVERVIEW.FMT
    /// IMPLEMENTATION INN 2.7.2
    /// ```
    pub fn parse(body: &[u8]) -> Self {
        let text = str::from_utf8(body).unwrap_or("");
        let mut caps = Self {
            probed: true,
            ..Default::default()
        };

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let Some(label) = parts.next() else { continue };
            let args: Vec<&str> = parts.collect();
            match label.to_ascii_uppercase().as_str() {
                "VERSION" => {
                    caps.version = args.first().map(|s| (*s).to_string());
                }
                "IMPLEMENTATION" => {
                    caps.implementation = Some(args.join(" "));
                }
                "READER" => caps.reader = true,
                "POST" => caps.post = true,
                "IHAVE" => caps.ihave = true,
                "NEWNEWS" => caps.newnews = true,
                "HDR" => caps.hdr = true,
                "OVER" => {
                    caps.over = true;
                    caps.over_msgid = args.iter().any(|a| a.eq_ignore_ascii_case("MSGID"));
                }
                "LIST" => {
                    caps.list_keywords = args.iter().map(|s| (*s).to_ascii_uppercase()).collect();
                }
                "MODE-READER" => caps.mode_reader_required = true,
                "COMPRESS" | "XFEATURE-COMPRESS" => caps.compress = true,
                _ => caps.other.push(line.to_string()),
            }
        }

        // RFC 3977 §5.3: READER implies ARTICLE/BODY/HEAD/STAT are all
        // available. If the server advertises MODE-READER instead, it is
        // currently in transit mode and needs a MODE READER before reader
        // commands will work — caller handles that, but the derived flags
        // reflect "will work after MODE READER".
        let reader_active = caps.reader || caps.mode_reader_required;
        caps.have_article = reader_active;
        caps.have_body = reader_active;
        caps.have_head = reader_active;
        caps.have_stat = reader_active;

        // Be lenient with non-compliant servers: some providers (notably
        // older Giganews-derived stacks) advertise neither READER nor
        // MODE-READER but still accept BODY/STAT just fine. If the server
        // advertises any of OVER/HDR/POST/IHAVE it is clearly a reader-ish
        // implementation, so assume the standard commands work too.
        if !reader_active && (caps.over || caps.hdr || caps.post || caps.ihave) {
            caps.have_article = true;
            caps.have_body = true;
            caps.have_head = true;
            caps.have_stat = true;
        }

        caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typical_inn_response() {
        let body = b"VERSION 2\r\n\
                     READER\r\n\
                     IHAVE\r\n\
                     POST\r\n\
                     NEWNEWS\r\n\
                     HDR\r\n\
                     OVER MSGID\r\n\
                     LIST ACTIVE NEWSGROUPS OVERVIEW.FMT\r\n\
                     IMPLEMENTATION INN 2.7.2\r\n";
        let caps = NntpCapabilities::parse(body);
        assert!(caps.probed);
        assert!(caps.reader);
        assert!(caps.hdr);
        assert!(caps.over);
        assert!(caps.over_msgid);
        assert!(caps.have_body);
        assert!(caps.have_stat);
        assert!(caps.have_article);
        assert!(caps.have_head);
        assert_eq!(caps.version.as_deref(), Some("2"));
        assert_eq!(caps.implementation.as_deref(), Some("INN 2.7.2"));
        assert!(caps.list_keywords.contains(&"ACTIVE".to_string()));
        assert!(caps.list_keywords.contains(&"OVERVIEW.FMT".to_string()));
    }

    #[test]
    fn parses_mode_reader_required() {
        let body = b"VERSION 2\r\n\
                     MODE-READER\r\n\
                     IHAVE\r\n";
        let caps = NntpCapabilities::parse(body);
        assert!(caps.mode_reader_required);
        assert!(!caps.reader);
        assert!(caps.have_body);
        assert!(caps.have_stat);
    }

    #[test]
    fn lenient_on_nonstandard_server() {
        // No READER / MODE-READER, but OVER/HDR suggest reader-ish impl.
        let body = b"VERSION 2\r\n\
                     OVER\r\n\
                     HDR\r\n";
        let caps = NntpCapabilities::parse(body);
        assert!(!caps.reader);
        assert!(caps.have_body);
        assert!(caps.have_stat);
    }

    #[test]
    fn strict_when_only_transit() {
        let body = b"VERSION 2\r\n";
        let caps = NntpCapabilities::parse(body);
        assert!(!caps.reader);
        assert!(!caps.have_body);
        assert!(!caps.have_stat);
    }

    #[test]
    fn default_assumed_is_permissive() {
        let caps = NntpCapabilities::default_assumed();
        assert!(!caps.probed);
        assert!(caps.have_body);
        assert!(caps.have_stat);
        assert!(caps.have_article);
        assert!(caps.have_head);
    }
}
