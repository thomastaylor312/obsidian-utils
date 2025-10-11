use std::{collections::HashMap, path::PathBuf};

use clap::Parser;
use comrak::Arena;
use serde::{Deserialize, Serialize};

pub mod frontmatter;
pub mod parser;
pub mod reader;

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

// A struct tying data to a tag. Right now this is really simple, but may be expanded in the future
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TagInfo {
    pub files: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    env_logger::init();

    let entries = reader::read_dir(&cli.vault_dir, cli.recurse)?;

    let arena = Arena::with_capacity(entries.len());
    let parsed_files = parser::ignore_error_iter(parser::parse_files(&arena, entries));
    let parsed_with_fm = frontmatter::parse_frontmatter(parsed_files);

    let tags = parsed_with_fm.fold(HashMap::new(), |mut acc, (pf, fm)| {
        if let Some(fm) = fm {
            for tag in fm.tags.unwrap_or_default() {
                acc.entry(tag)
                    .or_insert_with(TagInfo::default)
                    .files
                    .push(pf.path.clone());
            }
        }
        acc
    });

    // TODO: printers (breaking out to a common args as well)
    println!("{}", serde_json::to_string_pretty(&tags)?);

    Ok(())
}
