use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use comrak::Arena;
use obsidian_bases::load_base_file;
use obsidian_core::parser;

/// A command line tool for working with Obsidian `.base` files.
///
/// This tool requires 2 positional arguments: the path to the vault root directory,
/// and the path to the `.base` YAML file. The vault directory is used to provide
/// context for the base file, such as available files and their metadata.
#[derive(Debug, Parser)]
#[command(name = "obsidian-bases", version)]
struct Args {
    /// The view name to use for displaying the base file. If not provided, the first view in the
    /// base file will be used.
    #[arg(short = 'v', long = "view")]
    view: Option<String>,

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
    /// Path to the vault root directory. This is used for providing data to the base file.
    #[arg(value_name = "VAULT_DIR")]
    vault_dir: PathBuf,
    /// Path to the .base YAML file.
    #[arg(value_name = "BASE_FILE")]
    path: PathBuf,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    let entries = obsidian_core::reader::read_dir(&args.vault_dir, true)?;
    let arena = Arena::with_capacity(entries.len());
    let parsed_files = parser::ignore_error_iter(parser::parse_files(&arena, entries));

    let (parsed, links): (Vec<_>, Vec<_>) = obsidian_links::parser::parse_links(
        parsed_files,
        &args.vault_dir,
        args.link_style.unwrap_or_default(),
    )
    .unzip();

    let links = links.into_iter().enumerate().try_fold(
        obsidian_links::Links::new(),
        |mut acc, (i, to)| {
            let from_path = parsed[i].path.canonicalize()?;
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
        },
    )?;

    // TODO: Convert tags logic into a separate crate so we can get the same struct here from files
    let base =
        load_base_file(&args.path).with_context(|| format!("reading {}", args.path.display()))?;

    println!("{:#?}", base);

    Ok(())
}
