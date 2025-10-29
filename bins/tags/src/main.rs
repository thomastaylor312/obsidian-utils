use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use clap::Parser;
use comrak::Arena;
use serde::{Deserialize, Serialize};

use obsidian_core::{frontmatter, parser, printer, reader};

#[derive(Parser, Debug)]
#[command(name = "obsidian-tags", about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub printer: printer::PrinterArgs,

    #[command(flatten)]
    pub read_opts: reader::ReaderOpts,
}

// A struct tying data to a tag. Right now this is really simple, but may be expanded in the future
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TagInfo {
    /// The files associated with this tag
    pub files: HashSet<PathBuf>,
}

impl TagInfo {
    pub fn new() -> Self {
        Self::default()
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    env_logger::init();

    let entries = cli.read_opts.read_dir()?;

    let arena = Arena::with_capacity(entries.len());
    let parsed_files = parser::ignore_error_iter(parser::parse_files(&arena, entries));
    let parsed_with_fm = frontmatter::parse_frontmatter(parsed_files);

    let tags = parsed_with_fm.fold(HashMap::new(), |mut acc, (pf, fm)| {
        if let Some(fm) = fm {
            for tag in fm.tags.unwrap_or_default() {
                acc.entry(tag)
                    .or_insert_with(TagInfo::new)
                    .files
                    // Have to clone because pf has a lifetime
                    .insert(pf.path.clone());
            }
        }
        acc
    });

    cli.printer
        .format
        .print(obsidian_core::TAGS_DATA_KEY, tags, &mut std::io::stdout())
}
