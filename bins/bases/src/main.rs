use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use obsidian_bases::load_base_file;

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

    let base =
        load_base_file(&args.path).with_context(|| format!("reading {}", args.path.display()))?;

    println!("{:#?}", base);

    Ok(())
}
