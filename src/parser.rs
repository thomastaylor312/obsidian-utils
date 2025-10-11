use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use anyhow::Context;
use comrak::{Arena, ExtensionOptions, Options, arena_tree::Node, nodes::Ast};

use crate::reader::FileEntry;

const FRONTMATTER_DELIMITER: &str = "---";
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
    pub ast: &'a Node<'a, RefCell<Ast>>,
}

pub fn parse_files<'a>(
    arena: &'a Arena<Node<'a, RefCell<Ast>>>,
    entries: impl IntoIterator<Item = FileEntry>,
) -> anyhow::Result<Vec<ParsedFile<'a>>> {
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
        .collect()
}

/// Parse a markdown file from disk into an AST node
pub fn parse_file<'a>(
    arena: &'a Arena<Node<'a, RefCell<Ast>>>,
    path: impl AsRef<Path>,
) -> anyhow::Result<&'a Node<'a, RefCell<Ast>>> {
    let content = std::fs::read_to_string(&path).context("Failed to load file from disk")?;

    Ok(parse_content(arena, &content))
}

/// Parse markdown content into an AST node
pub fn parse_content<'a>(
    arena: &'a Arena<Node<'a, RefCell<Ast>>>,
    content: &str,
) -> &'a Node<'a, RefCell<Ast>> {
    comrak::parse_document(arena, content, &PARSE_OPTIONS)
}
