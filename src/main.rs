use std::path::{Path, PathBuf};
use std::str::FromStr;

use clap::{Args, Parser, Subcommand};
use comrak::Arena;

mod parser;
mod reader;

#[derive(Parser, Debug)]
#[command(name = "obsidian-tags", about, long_about = None)]
pub struct Cli {
    /// Whether to recurse into subdirectories when reading the vault. Defaults to true
    #[arg(long, default_value_t = true)]
    pub recurse: bool,

    /// The directory containing the vault to operate on
    // TODO: Make this optional once we support a list of files from stdin
    pub vault_dir: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let entries = reader::read_dir(&cli.vault_dir, cli.recurse)?;

    let arena = Arena::new();
    let parsed_files = parser::parse_files(&arena, entries)?;

    Ok(())
}
