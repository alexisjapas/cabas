//! The relay process: bind, serve, and log just enough to be debugged.
//!
//! Configuration is two environment variables, because the Home Assistant
//! add-on frame (M6) passes configuration that way and a flag parser would
//! be a dependency spent on nothing:
//!
//! - `CABAS_RELAY_DATA` — where the sealed logs live. Defaults to `/data`,
//!   the add-on's durable volume, covered by HA's own backups.
//! - `CABAS_RELAY_ADDR` — listen address, default `0.0.0.0:8787`; the
//!   Cloudflare Tunnel (DECISIONS 0012) terminates in front of this.

use std::path::PathBuf;

use cabas_relay::{Relay, router};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).init();

    let data =
        PathBuf::from(std::env::var("CABAS_RELAY_DATA").unwrap_or_else(|_| "/data".to_string()));
    let addr = std::env::var("CABAS_RELAY_ADDR").unwrap_or_else(|_| "0.0.0.0:8787".to_string());

    let relay = match Relay::open(data.clone()) {
        Ok(relay) => relay,
        Err(e) => {
            tracing::error!(error = %e, path = %data.display(), "data directory unusable");
            std::process::exit(1);
        }
    };
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(error = %e, addr, "cannot bind");
            std::process::exit(1);
        }
    };

    tracing::info!(addr, data = %data.display(), "cabas-relay up");
    if let Err(e) = axum::serve(listener, router(relay)).await {
        tracing::error!(error = %e, "server stopped");
        std::process::exit(1);
    }
}
