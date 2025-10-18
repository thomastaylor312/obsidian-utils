use std::path::PathBuf;

use comrak::nodes::{AstNode, NodeValue};

use obsidian_core::parser::ParsedFile;

/// Parse the links from a list of ParsedFiles, returning an iterator of tuples of the
/// [`ParsedFile`] returned as is and a vec of PathBufs representing the links found in the file
pub fn parse_links<'a>(
    entries: impl IntoIterator<Item = ParsedFile<'a>>,
) -> impl Iterator<Item = (ParsedFile<'a>, Vec<PathBuf>)> {
    entries.into_iter().map(|pf| {
        let links = parse_links_from_ast(pf.ast);
        (pf, links)
    })
}

/// Parse the links from the AST of a markdown file
fn parse_links_from_ast<'a>(ast: &'a AstNode<'a>) -> Vec<PathBuf> {
    // TODO: We probably want to pass the vault root here and then resolve relative links to be
    // absolute. This is going to be a more complex change as we'll need to handle whether the link
    // is relative to the file or absolute within the vault. We'll probably be able to do this by
    // checking for `./` or `../` at the start of the link and then assume the rest are absolute
    // within the vault.
    ast.descendants()
        .filter_map(|node| {
            let raw_path = match &node.data.borrow().value {
                NodeValue::Link(link) => link.url.clone(),
                NodeValue::WikiLink(link) => link.url.clone(),
                _ => return None,
            };
            // If this is a URL (which we can naively check for "://" in the string), skip it
            if raw_path.contains("://") {
                return None;
            }
            // Otherwise, we can convert it to a PathBuf and add it to our list of links
            Some(PathBuf::from(raw_path))
        })
        .collect()
}
