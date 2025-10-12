use std::{collections::HashMap, path::PathBuf};

use clap::Parser;
use comrak::Arena;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

pub mod frontmatter;
pub mod parser;
pub mod printer;
pub mod reader;

#[derive(Parser, Debug)]
#[command(name = "obsidian-tags", about, long_about = None)]
pub struct Cli {
    /// Whether to recurse into subdirectories when reading the vault. Defaults to true
    #[arg(long, default_value_t = true)]
    pub recurse: bool,

    #[command(flatten)]
    pub printer: printer::PrinterArgs,

    /// The directory containing the vault to operate on
    // TODO: Make this optional once we support a list of files from stdin
    pub vault_dir: PathBuf,
}

// A struct tying data to a tag. Right now this is really simple, but may be expanded in the future
#[derive(Debug, Serialize, Deserialize, Default, Tabled)]
pub struct TagInfo {
    /// The name of the tag
    // Ignored in serde output as it's redundant with the key in the map
    #[serde(skip)]
    #[tabled(rename = "Tag")]
    pub tag: String,

    /// The files associated with this tag
    #[tabled(format("{}", self.files.len()), rename = "File Count")]
    pub files: Vec<PathBuf>,
}

impl TagInfo {
    pub fn new(tag: String) -> Self {
        Self {
            tag,
            files: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Tags(HashMap<String, TagInfo>);

impl printer::AsTabled for Tags {
    type Table = TagInfo;

    fn as_tabled(&self) -> impl IntoIterator<Item = &Self::Table> {
        self.0.values()
    }
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
                // We duplicate tag names here to make it easier for table formatting the data with the proper name
                acc.entry(tag.clone())
                    .or_insert_with(|| TagInfo::new(tag))
                    .files
                    .push(pf.path.clone());
            }
        }
        acc
    });

    cli.printer
        .format
        .print(&Tags(tags), &mut std::io::stdout())
}
