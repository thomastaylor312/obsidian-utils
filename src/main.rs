use std::path::Path;
use std::str::FromStr;

use notify::{RecursiveMode, Watcher};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};

mod processor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create or open a database file
    let opts = SqliteConnectOptions::from_str("sqlite:obsidian.db")?
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        // Recommended timeout from litestream so backup is possible
        .busy_timeout(std::time::Duration::from_secs(5))
        // More performant at cost of durability in some circumstances
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .create_if_missing(true);
    let conn = SqlitePool::connect_with(opts).await?;

    let processors = vec![processor::tags::Tags::new(conn)];

    let proc = processor::FileHandler::new("./test-vault", processors).await?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |evt| {
        if let Err(e) = tx.send(evt) {
            tracing::error!(err = %e, "channel is closed")
        }
    })?;
    watcher.watch(Path::new("./test-vault"), RecursiveMode::Recursive)?;

    loop {
        tokio::select! {
            maybe_evt = rx.recv() => {
                match maybe_evt {
                    Some(evt) => {
                        proc.handle_event(evt).await;
                    }
                    None => {
                        // channel closed, exit loop
                        break;
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl-C, shutting down");
                break;
            }
        }
    }

    Ok(())
}
