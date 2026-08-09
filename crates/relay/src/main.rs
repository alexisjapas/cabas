//! The relay process: bind, serve, and log just enough to be debugged —
//! plus the two things a person with a shell occasionally needs to do to
//! the data directory (DECISIONS 0050).
//!
//! Configuration is two environment variables, because the Home Assistant
//! add-on frame passes configuration that way and a flag parser would be a
//! dependency spent on nothing:
//!
//! - `CABAS_RELAY_DATA` — where the sealed logs live. Defaults to `/data`,
//!   the add-on's durable volume, covered by HA's own backups.
//! - `CABAS_RELAY_ADDR` — listen address, default `0.0.0.0:8787`; the
//!   Cloudflare Tunnel (DECISIONS 0012) terminates in front of this.
//!
//! The same reasoning covers the subcommands: `families` and `forget` are
//! matched by name, positionally, and everything else is a usage message.
//! Two verbs do not need a parser either.

use std::path::Path;
use std::process::ExitCode;
use std::time::SystemTime;

use cabas_relay::{Relay, admin, embedded, router};

const USAGE: &str = "\
cabas-relay — sync relay and host for the cabas app

  cabas-relay                 serve (the add-on's default)
  cabas-relay families        what is on disk, per family
  cabas-relay forget <id>     delete one family's log, irreversibly

Environment:
  CABAS_RELAY_DATA  where the sealed logs live (default /data)
  CABAS_RELAY_ADDR  listen address (default 0.0.0.0:8787)

`forget` is how an abandoned family goes away — rotating the phrase leaves
its log here and nothing collects it, because nothing here can tell an
abandoned family from a quiet one. See cabas-relay/DOCS.md.
";

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => serve().await,
        Some("families") => families(),
        Some("forget") => match args.next() {
            Some(id) => forget(&id),
            None => {
                eprintln!("forget: which family? `cabas-relay families` lists them.");
                ExitCode::from(2)
            }
        },
        Some("help" | "-h" | "--help") => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("cabas-relay: no such command: {other}\n");
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

async fn serve() -> ExitCode {
    // Only on this path: the subcommands print a report, and an INFO line
    // about a data directory in the middle of it helps nobody.
    tracing_subscriber::fmt().with_target(false).init();

    let data = admin::data_dir();
    let addr = std::env::var("CABAS_RELAY_ADDR").unwrap_or_else(|_| "0.0.0.0:8787".to_string());

    let relay = match Relay::open(data.clone()) {
        Ok(relay) => relay,
        Err(e) => {
            tracing::error!(error = %e, path = %data.display(), "data directory unusable");
            return ExitCode::FAILURE;
        }
    };
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(error = %e, addr, "cannot bind");
            return ExitCode::FAILURE;
        }
    };

    // The asset count is here because "the app does not load" and "this
    // build has no app in it" look identical from a phone. A relay built
    // without `ui/dist` is legitimate — it is what development runs behind
    // `ui-serve` — so it is a line to read, not a refusal.
    let files = embedded();
    if files == 0 {
        tracing::warn!("no PWA embedded — brokering sync only");
    }
    tracing::info!(addr, data = %data.display(), files, "cabas-relay up");
    if let Err(e) = axum::serve(listener, router(relay)).await {
        tracing::error!(error = %e, "server stopped");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn families() -> ExitCode {
    let data = admin::data_dir();
    match admin::survey(&data) {
        Ok(families) => {
            print!("{}", admin::render(&families, SystemTime::now()));
            ExitCode::SUCCESS
        }
        Err(e) => fail(&data, e),
    }
}

fn forget(id: &str) -> ExitCode {
    let data = admin::data_dir();
    match admin::forget(&data, id) {
        Ok(family) => {
            println!(
                "forgot {} — {} frames, gone for good",
                family.id, family.frames
            );
            ExitCode::SUCCESS
        }
        Err(e) => fail(&data, e),
    }
}

fn fail(data: &Path, e: std::io::Error) -> ExitCode {
    eprintln!("cabas-relay: {e}");
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        eprintln!(
            "({} is the add-on's volume; run this inside the add-on)",
            data.display()
        );
    }
    ExitCode::FAILURE
}
