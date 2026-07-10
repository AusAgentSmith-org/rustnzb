//! Integration tests that validate NNTP authentication against real servers.
//!
//! Credentials are loaded from `.env.test` (gitignored). If the file is missing
//! or a required variable is unset, the tests are skipped — not failed.
//!
//! These tests are ignored by default because they use paid external services
//! and depend on account/network state. Run them explicitly with:
//! `cargo test -p nzb-nntp --test auth_integration -- --ignored --nocapture`.

use std::path::PathBuf;
use std::sync::Once;

use nzb_nntp::{NntpConnection, ServerConfig};

static INIT: Once = Once::new();

fn init_crypto() {
    INIT.call_once(|| {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("failed to install rustls crypto provider");
    });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load `.env.test` from the crate root and return an env-var getter that
/// checks the dotenvy map first, then falls back to `std::env::var`.
fn load_env() {
    init_crypto();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env.test");
    if path.exists() {
        dotenvy::from_path(&path).ok();
    }
}

/// Read an env var, returning `None` if unset or empty.
fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Build a `ServerConfig` for a given host/port/user/pass.
fn make_config(name: &str, host: &str, port: u16, user: &str, pass: &str) -> ServerConfig {
    let mut config = ServerConfig::new(name, host);
    config.name = name.to_string();
    config.port = port;
    config.username = Some(user.to_string());
    config.password = Some(pass.to_string());
    config.connections = 1;
    config.ramp_up_delay_ms = 0;
    config.recv_buffer_size = 0;
    config
}

/// Connect + authenticate, then QUIT. Returns the welcome message on success.
async fn auth_and_quit(config: &ServerConfig) -> Result<String, String> {
    let mut conn = NntpConnection::new(config.id.clone());
    conn.connect(config).await.map_err(|e| format!("{e}"))?;

    // If we got here, auth succeeded (connect() runs authenticate internally).
    // Send QUIT to be a good citizen.
    let _ = conn.quit().await;
    Ok(format!("Auth OK for {}", config.host))
}

// ---------------------------------------------------------------------------
// Frugal Usenet servers
// ---------------------------------------------------------------------------

const FRUGAL_HOSTS: &[(&str, &str)] = &[
    ("frugal-us", "news.frugalusenet.com"),
    ("frugal-us-west", "newswest.frugalusenet.com"),
    ("frugal-eu", "eunews.frugalusenet.com"),
    ("frugal-au", "aunews.frugalusenet.com"),
    ("frugal-asia", "asnews.frugalusenet.com"),
    ("frugal-sa", "sanews.frugalusenet.com"),
];

#[tokio::test]
#[ignore = "requires live Frugal Usenet credentials and network access"]
async fn frugal_auth_all_endpoints() {
    load_env();

    let user = match env("FRUGAL_USER") {
        Some(u) => u,
        None => {
            eprintln!("SKIP: FRUGAL_USER not set (missing .env.test?)");
            return;
        }
    };
    let pass = match env("FRUGAL_PASS") {
        Some(p) => p,
        None => {
            eprintln!("SKIP: FRUGAL_PASS not set");
            return;
        }
    };

    let mut failures = Vec::new();

    for &(name, host) in FRUGAL_HOSTS {
        let config = make_config(name, host, 563, &user, &pass);
        match auth_and_quit(&config).await {
            Ok(msg) => eprintln!("  [+] {msg}"),
            Err(e) => {
                eprintln!("  [-] FAIL {host}: {e}");
                failures.push(format!("{host}: {e}"));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Frugal auth failed on {} server(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// NewsgroupDirect
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires live NewsgroupDirect credentials and network access"]
async fn ngd_auth() {
    load_env();

    let user = match env("NGD_USER") {
        Some(u) => u,
        None => {
            eprintln!("SKIP: NGD_USER not set (missing .env.test?)");
            return;
        }
    };
    let pass = match env("NGD_PASS") {
        Some(p) => p,
        None => {
            eprintln!("SKIP: NGD_PASS not set");
            return;
        }
    };

    let servers = [
        ("ngd-us", "news.newsgroupdirect.com", 563),
        ("ngd-eu", "eu-tst.newsgroupdirect.com", 563),
    ];

    let mut failures = Vec::new();

    for (name, host, port) in servers {
        let config = make_config(name, host, port, &user, &pass);
        match auth_and_quit(&config).await {
            Ok(msg) => eprintln!("  [+] {msg}"),
            Err(e) => {
                eprintln!("  [-] FAIL {host}: {e}");
                failures.push(format!("{host}: {e}"));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "NGD auth failed on {} server(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Supernews
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires live Supernews credentials and network access"]
async fn supernews_auth() {
    load_env();

    let user = match env("SUPERNEWS_USER") {
        Some(u) => u,
        None => {
            eprintln!("SKIP: SUPERNEWS_USER not set (missing .env.test?)");
            return;
        }
    };
    let pass = match env("SUPERNEWS_PASS") {
        Some(p) => p,
        None => {
            eprintln!("SKIP: SUPERNEWS_PASS not set");
            return;
        }
    };

    let config = make_config("supernews", "super.newsgroupdirect.com", 563, &user, &pass);
    match auth_and_quit(&config).await {
        Ok(msg) => eprintln!("  [+] {msg}"),
        Err(e) => panic!("Supernews auth failed: {e}"),
    }
}

// ---------------------------------------------------------------------------
// ViperNews
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires live ViperNews credentials and network access"]
async fn vipernews_auth() {
    load_env();

    let user = match env("VIPERNEWS_USER") {
        Some(u) => u,
        None => {
            eprintln!("SKIP: VIPERNEWS_USER not set (missing .env.test?)");
            return;
        }
    };
    let pass = match env("VIPERNEWS_PASS") {
        Some(p) => p,
        None => {
            eprintln!("SKIP: VIPERNEWS_PASS not set");
            return;
        }
    };

    let config = make_config("vipernews", "viper.newsgroupdirect.com", 563, &user, &pass);
    match auth_and_quit(&config).await {
        Ok(msg) => eprintln!("  [+] {msg}"),
        Err(e) => panic!("ViperNews auth failed: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Negative: bad credentials must fail
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires a live external NNTP provider"]
async fn bad_credentials_rejected() {
    init_crypto();
    // This test doesn't need real creds — it verifies that wrong creds fail.
    let config = make_config(
        "bad-creds",
        "news.frugalusenet.com",
        563,
        "definitely_not_a_real_user",
        "wrong_password_12345",
    );

    let result = auth_and_quit(&config).await;
    assert!(
        result.is_err(),
        "Expected auth to fail with bad credentials, but it succeeded"
    );
    let err = result.unwrap_err();
    eprintln!("  [+] Bad creds correctly rejected: {err}");
}

// ---------------------------------------------------------------------------
// Negative: masked password sentinel must fail
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires a live external NNTP provider"]
async fn masked_password_sentinel_rejected() {
    load_env();

    // Use a real username but the mask sentinel as password.
    // This is exactly the bug scenario: the UI sends "********" as the password.
    let user = match env("FRUGAL_USER") {
        Some(u) => u,
        None => {
            // Fall back to a dummy user — the point is the password is wrong
            "testuser".to_string()
        }
    };

    let config = make_config(
        "sentinel-test",
        "news.frugalusenet.com",
        563,
        &user,
        "********",
    );

    let result = auth_and_quit(&config).await;
    assert!(
        result.is_err(),
        "Auth must NOT succeed with the mask sentinel '********' as password"
    );
    let err = result.unwrap_err();
    eprintln!("  [+] Sentinel password correctly rejected: {err}");
}
