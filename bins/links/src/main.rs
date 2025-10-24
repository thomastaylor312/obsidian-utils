use clap::Parser;
use comrak::Arena;

use obsidian_core::{parser, printer, reader};

/// Print links found in the vault into a graph of all links
#[derive(Parser, Debug)]
#[command(name = "obsidian-links")]
pub struct Cli {
    #[command(flatten)]
    pub printer: printer::PrinterArgs,

    #[command(flatten)]
    pub read_opts: reader::ReaderOpts,

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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    env_logger::init();

    let entries = cli.read_opts.read_dir()?;
    let num_entries = entries.len();

    let arena = Arena::with_capacity(entries.len());
    let parsed_files = parser::ignore_error_iter(parser::parse_files(&arena, entries));
    let mut parsed_with_fm = obsidian_links::parser::parse_links(
        parsed_files,
        &cli.read_opts.vault_dir,
        cli.link_style.unwrap_or_default(),
    );

    let links = parsed_with_fm.try_fold(
        obsidian_links::Links::with_capacity(num_entries),
        |mut acc, (from, to)| {
            let to = to
                .into_iter()
                .map(|p| {
                    // We don't care about errors here, just try to get the absolute path
                    std::path::absolute(&p).map_err(|e| {
                        anyhow::anyhow!("Failed to get absolute path for {:?}: {}", p, e)
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            acc.insert_links(from.path.clone(), to);
            anyhow::Ok(acc)
        },
    )?;

    cli.printer
        .format
        .print(obsidian_core::LINKS_DATA_KEY, links, &mut std::io::stdout())
}
