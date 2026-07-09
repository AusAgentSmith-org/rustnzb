use anyhow::Context;
use clap::Parser;
use mock_nntp_server::{DEFAULT_LISTEN_ADDR, start_sample_server_on};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = DEFAULT_LISTEN_ADDR)]
    listen: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let server = start_sample_server_on(&args.listen).await;
    eprintln!("mock-nntp-server listening on {}", server.addr);

    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate =
            signal(SignalKind::terminate()).context("install SIGTERM handler for mock server")?;
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .context("wait for ctrl-c in mock server")?;
    }

    drop(server);
    Ok(())
}
