use std::{collections::HashMap, path::PathBuf};

use clap::Parser;
use comrak::Arena;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use obsidian_core::{parser, printer, reader};

#[derive(Parser, Debug)]
#[command(name = "obsidian-links", about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub printer: printer::PrinterArgs,

    #[command(flatten)]
    pub read_opts: reader::ReaderOpts,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    env_logger::init();

    let entries = cli.read_opts.read_dir()?;

    let arena = Arena::with_capacity(entries.len());
    let parsed_files = parser::ignore_error_iter(parser::parse_files(&arena, entries));
    let parsed_with_fm = obsidian_links::parser::parse_links(parsed_files);

    // TODO: Build link structure

    // TODO: Print link structure
    // cli.printer.format.print(
    //     obsidian_core::LINKS_DATA_KEY,
    //     &Tags(tags),
    //     &mut std::io::stdout(),
    // )
    Ok(())
}
