use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use comrak::Arena;

use obsidian_core::{
    parser,
    printer::{self, Format},
    reader,
};

/// Generate and print an adjacency list of links between markdown files in an Obsidian vault.
///
/// By default, this tool will read all markdown files in the specified directory and parse all
/// links found within them. If passing via stdin, it is highly recommended you set the
/// `--vault-dir` option to ensure links are resolved correctly.
///
/// The output will be an adjacency list where each line contains a source file followed by an
/// object that indicate whether the file exists and any links and backlinks it has. For this tool,
/// the plain text output format is mostly useful for human consumption as there isn't a great way
/// to represent a graph in plain text.
///
/// Example output (json):
/// {
///   "/path/to/vault/References/Aider.md": {
///     "exists": true,
///     "links": [
///       "/path/to/vault/References/Claude Code.md",
///       "/path/to/vault/References/Roo Code.md"
///     ],
///     "backlinks": []
///   },
///   "/path/to/vault/References/Claude Code.md": {
///     "exists": true,
///     "links": [],
///     "backlinks": [
///       "/path/to/vault/References/Aider.md"
///     ]
///   },
///   "/path/to/vault/References/Ghostty.md": {
///     "exists": true,
///     "links": [
///       "/path/to/vault/References/Warp"
///     ],
///     "backlinks": []
///   },
///   "/path/to/vault/References/OpenRouter.md": {
///     "exists": false,
///     "links": [],
///     "backlinks": [
///       "/path/to/vault/References/aichat.md"
///     ]
///   },
///   "/path/to/vault/References/Roo Code.md": {
///     "exists": false,
///     "links": [],
///     "backlinks": [
///       "/path/to/vault/References/Aider.md"
///     ]
///   },
///   "/path/to/vault/References/Warp": {
///     "exists": false,
///     "links": [],
///     "backlinks": [
///       "/path/to/vault/References/Ghostty.md"
///     ]
///   },
///   "/path/to/vault/References/aichat.md": {
///     "exists": true,
///     "links": [
///       "/path/to/vault/References/OpenRouter.md"
///     ],
///     "backlinks": []
///   }
/// }
#[derive(Parser, Debug)]
#[command(name = "obsidian-links", version)]
pub struct Cli {
    #[command(flatten)]
    pub printer: printer::PrinterArgs,

    #[command(flatten)]
    pub read_opts: reader::ReaderOpts,

    /// The vault directory to use as the root for resolving links. If not specified, the directory
    /// specified in the reader options will be used. If neither is specified, links will be
    /// resolved relative to the current working directory.
    #[arg(long = "vault-dir")]
    pub vault_dir: Option<PathBuf>,

    /// The style of link to parse from the markdown files. Valid options are "infer", "from_vault_root",
    /// and "relative_to_file". Default is "infer".
    ///
    /// "infer": If the link starts with `./` or `../` or it is a path with a single element (e.g.
    /// `file.md`), it is considered relative to the file. Otherwise, it is considered relative to
    /// the vault root.
    ///
    /// "from_vault_root": All links are considered relative to the vault root.
    ///
    /// "relative_to_file": All links are considered relative to the file they are found in.
    #[arg(long = "link-style")]
    pub link_style: Option<obsidian_links::parser::LinkStyle>,

    /// Whether to include orphaned files (i.e. files with no links and no backlinks) in the output.
    /// Defaults to false
    #[arg(long = "include-orphans", default_value_t = false)]
    pub include_orphans: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    env_logger::init();

    let entries = cli.read_opts.read_files()?;

    let arena = Arena::with_capacity(entries.len());
    let parsed_files = parser::ignore_error_iter(parser::parse_files(&arena, entries));
    let vault_root = cli
        .vault_dir
        .clone()
        .or_else(|| cli.read_opts.dir.clone())
        .unwrap_or_else(|| PathBuf::from("."));
    let mut parsed_with_fm = obsidian_links::parser::parse_links(
        parsed_files,
        &vault_root,
        cli.link_style.unwrap_or_default(),
    );

    let mut links =
        parsed_with_fm.try_fold(obsidian_links::Links::new(), |mut acc, (from, to)| {
            // Unlike below, this file should exist, so we can canonicalize it
            let from_path = from.path.canonicalize()?;
            let to = to
                .into_iter()
                .map(|p| {
                    match p.canonicalize() {
                        Ok(canon) => Ok(canon),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            // Obsidian allows linking to files that don't exist yet, so we can't
                            // canonicalize here. Instead, we just make the path absolute as much as
                            // possible
                            std::path::absolute(&p).map_err(|e| {
                                anyhow::anyhow!("Failed to get absolute path for {:?}: {}", p, e)
                            })
                        }
                        Err(e) => Err(e).context("Error canonicalizing path"),
                    }
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            acc.insert_links(from_path, to);
            anyhow::Ok(acc)
        })?;

    if !cli.include_orphans {
        links.prune_orphans();
    }

    let format = cli.printer.output;
    let mut writer = std::io::stdout();
    match format {
        Format::Plain => format.print_plain(
            links.into_iter().map(|(p, info)| {
                format!(
                    "{}: [{}]",
                    p.display(),
                    // This does a bunch of allocations. If for some reason this slows things down
                    // or takes up a lot of memory with big vaults, we can optimize by converting to
                    // a string and then building the final string manually
                    info.links
                        .into_iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }),
            &mut writer,
        ),
        Format::Json | Format::Binary => format.print_structured(links, &mut writer),
    }
}
