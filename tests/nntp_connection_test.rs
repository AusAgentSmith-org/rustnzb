//! Integration test: connect to real NNTP servers with all port/SSL variants.
//!
//! Tests both Usenet Farm and Frugal Usenet with:
//!   - SSL on port 563
//!   - SSL on port 443
//!   - Plaintext on port 119
//!   - Plaintext on port 80

use nzb_core::config::ServerConfig;
use nzb_nntp::NntpConnection;

/// Helper to build a ServerConfig for testing.
fn make_config(
    id: &str,
    host: &str,
    port: u16,
    ssl: bool,
    username: &str,
    password: &str,
    connections: u16,
) -> ServerConfig {
    ServerConfig {
        id: id.to_string(),
        name: format!("{host}:{port} ({})", if ssl { "SSL" } else { "plain" }),
        host: host.to_string(),
        port,
        ssl,
        ssl_verify: true,
        username: Some(username.to_string()),
        password: Some(password.to_string()),
        connections,
        priority: 0,
        enabled: true,
        retention: 0,
        pipelining: 1,
        optional: false,
    }
}

/// Test a single connection: connect, authenticate, then quit.
async fn test_connection(config: &ServerConfig) -> Result<String, String> {
    let mut conn = NntpConnection::new(config.id.clone());

    match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        conn.connect(config),
    )
    .await
    {
        Ok(Ok(())) => {
            let msg = format!(
                "OK  {}:{} {} — connected and authenticated",
                config.host,
                config.port,
                if config.ssl { "SSL" } else { "PLAIN" }
            );
            // Graceful disconnect
            let _ = conn.quit().await;
            Ok(msg)
        }
        Ok(Err(e)) => Err(format!(
            "FAIL {}:{} {} — {}",
            config.host,
            config.port,
            if config.ssl { "SSL" } else { "PLAIN" },
            e
        )),
        Err(_) => Err(format!(
            "TIMEOUT {}:{} {} — no response in 15s",
            config.host,
            config.port,
            if config.ssl { "SSL" } else { "PLAIN" },
        )),
    }
}

#[tokio::test]
async fn test_usenet_farm_ssl_563() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping on CI");
        return;
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = make_config(
        "uf-ssl-563",
        "news.usenet.farm",
        563,
        true,
        "uf8ea2a82f370952aa92",
        "ff24a05910fd23cb0040ff",
        1,
    );
    let result = test_connection(&config).await;
    eprintln!("{}", result.as_ref().unwrap_or_else(|e| e));
    assert!(result.is_ok(), "{}", result.unwrap_err());
}

#[tokio::test]
async fn test_usenet_farm_ssl_443() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping on CI");
        return;
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = make_config(
        "uf-ssl-443",
        "news.usenet.farm",
        443,
        true,
        "uf8ea2a82f370952aa92",
        "ff24a05910fd23cb0040ff",
        1,
    );
    let result = test_connection(&config).await;
    eprintln!("{}", result.as_ref().unwrap_or_else(|e| e));
    assert!(result.is_ok(), "{}", result.unwrap_err());
}

#[tokio::test]
async fn test_usenet_farm_plain_119() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping on CI");
        return;
    }
    let config = make_config(
        "uf-plain-119",
        "news.usenet.farm",
        119,
        false,
        "uf8ea2a82f370952aa92",
        "ff24a05910fd23cb0040ff",
        1,
    );
    let result = test_connection(&config).await;
    eprintln!("{}", result.as_ref().unwrap_or_else(|e| e));
    assert!(result.is_ok(), "{}", result.unwrap_err());
}

#[tokio::test]
async fn test_usenet_farm_plain_80() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping on CI");
        return;
    }
    let config = make_config(
        "uf-plain-80",
        "news.usenet.farm",
        80,
        false,
        "uf8ea2a82f370952aa92",
        "ff24a05910fd23cb0040ff",
        1,
    );
    let result = test_connection(&config).await;
    eprintln!("{}", result.as_ref().unwrap_or_else(|e| e));
    assert!(result.is_ok(), "{}", result.unwrap_err());
}

#[tokio::test]
async fn test_frugal_ssl_563() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping on CI");
        return;
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = make_config(
        "frugal-ssl-563",
        "news.frugalusenet.com",
        563,
        true,
        "sprooty",
        "ff24a05910fd23cb0040ff", // Using same password — update if different
        1,
    );
    let result = test_connection(&config).await;
    eprintln!("{}", result.as_ref().unwrap_or_else(|e| e));
    // Frugal may need different credentials — log but don't hard-fail
    if let Err(ref e) = result {
        if e.contains("Auth") {
            eprintln!("NOTE: Frugal credentials may differ — check creds.env for password");
        }
    }
    assert!(result.is_ok(), "{}", result.unwrap_err());
}

#[tokio::test]
async fn test_frugal_eu_ssl_563() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping on CI");
        return;
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = make_config(
        "frugal-eu-ssl-563",
        "aunews.frugalusenet.com",
        563,
        true,
        "sprooty",
        "ff24a05910fd23cb0040ff", // Using same password — update if different
        1,
    );
    let result = test_connection(&config).await;
    eprintln!("{}", result.as_ref().unwrap_or_else(|e| e));
    if let Err(ref e) = result {
        if e.contains("Auth") {
            eprintln!("NOTE: Frugal AU credentials may differ — check creds.env for password");
        }
    }
    assert!(result.is_ok(), "{}", result.unwrap_err());
}

/// Run all variants and print a summary table.
#[tokio::test]
async fn test_all_connections_summary() {
    if std::env::var("CI").is_ok() {
        eprintln!("Skipping on CI");
        return;
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let configs = vec![
        make_config("uf-ssl-563", "news.usenet.farm", 563, true, "uf8ea2a82f370952aa92", "ff24a05910fd23cb0040ff", 1),
        make_config("uf-ssl-443", "news.usenet.farm", 443, true, "uf8ea2a82f370952aa92", "ff24a05910fd23cb0040ff", 1),
        make_config("uf-plain-119", "news.usenet.farm", 119, false, "uf8ea2a82f370952aa92", "ff24a05910fd23cb0040ff", 1),
        make_config("uf-plain-80", "news.usenet.farm", 80, false, "uf8ea2a82f370952aa92", "ff24a05910fd23cb0040ff", 1),
        make_config("frugal-ssl-563", "news.frugalusenet.com", 563, true, "sprooty", "ff24a05910fd23cb0040ff", 1),
        make_config("frugal-eu-ssl-563", "aunews.frugalusenet.com", 563, true, "sprooty", "ff24a05910fd23cb0040ff", 1),
    ];

    eprintln!("\n============================================================");
    eprintln!("  NNTP Connection Test Summary");
    eprintln!("============================================================");

    let mut pass = 0;
    let mut fail = 0;

    for config in &configs {
        let result = test_connection(config).await;
        match &result {
            Ok(msg) => {
                eprintln!("  {msg}");
                pass += 1;
            }
            Err(msg) => {
                eprintln!("  {msg}");
                fail += 1;
            }
        }
    }

    eprintln!("============================================================");
    eprintln!("  Results: {pass} passed, {fail} failed");
    eprintln!("============================================================\n");
}
