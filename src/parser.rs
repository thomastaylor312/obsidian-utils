use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use anyhow::{Context, Result};
use comrak::{Arena, ExtensionOptions, Options, nodes::AstNode};

use crate::reader::FileEntry;

pub const FRONTMATTER_DELIMITER: &str = "---";
static PARSE_OPTIONS: LazyLock<Options<'static>> = LazyLock::new(|| Options {
    extension: ExtensionOptions {
        strikethrough: true,
        table: true,
        autolink: true,
        footnotes: true,
        front_matter_delimiter: Some(FRONTMATTER_DELIMITER.into()),
        alerts: true,
        wikilinks_title_after_pipe: true,
        ..Default::default()
    },
    ..Default::default()
});

/// A struct that can be used to store file data after parsing
pub struct ParsedFile<'a> {
    /// The path to the file on disk
    pub path: PathBuf,
    /// The Metadata of the file on disk
    pub metadata: std::fs::Metadata,
    /// The parsed AST of the file
    pub ast: &'a AstNode<'a>,
}

/// A helper to ignore errors from an iterator of Results, yielding only the Ok values and logging
/// the error instead
pub fn ignore_error_iter<'a, I>(iter: I) -> impl Iterator<Item = ParsedFile<'a>>
where
    I: IntoIterator<Item = Result<ParsedFile<'a>>>,
{
    iter.into_iter().filter_map(|res| match res {
        Ok(v) => Some(v),
        Err(e) => {
            log::error!("Ignoring error when parsing file: {e}");
            None
        }
    })
}

/// Parse a list of file entries into markdown ASTs. This consumes the iterator, but returns back
/// all the same data from entries as well as the parsed AST. This returns an iterator so the caller
/// can decide whether they want to allocated by collecting into a Vec or process one at a time.
pub fn parse_files<'a>(
    arena: &'a Arena<AstNode<'a>>,
    entries: impl IntoIterator<Item = FileEntry>,
) -> impl Iterator<Item = Result<ParsedFile<'a>>> {
    entries
        .into_iter()
        .filter(|e| {
            e.path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .map(|entry| {
            let root = parse_file(arena, &entry.path)?;
            Ok(ParsedFile {
                path: entry.path,
                metadata: entry.metadata,
                ast: root,
            })
        })
}

/// Parse a markdown file from disk into an AST node
pub fn parse_file<'a>(
    arena: &'a Arena<AstNode<'a>>,
    path: impl AsRef<Path>,
) -> Result<&'a AstNode<'a>> {
    let content = std::fs::read_to_string(&path).context("Failed to load file from disk")?;

    Ok(parse_content(arena, &content))
}

/// Parse markdown content into an AST node
pub fn parse_content<'a>(arena: &'a Arena<AstNode<'a>>, content: &str) -> &'a AstNode<'a> {
    comrak::parse_document(arena, content, &PARSE_OPTIONS)
}
