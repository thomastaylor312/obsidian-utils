use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use std::str::FromStr;

mod processor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create or open a database file
    let opts = SqliteConnectOptions::from_str("sqlite:obsidian.db")?
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .create_if_missing(true);
    let mut conn = opts.connect().await?;

    let processor = processor::FileHandler::new(vec![]);
    let watcher = notify::recommended_watcher(processor)?;

    Ok(())
}
