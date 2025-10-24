use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use comrak::nodes::{AstNode, NodeValue};

use obsidian_core::parser::ParsedFile;

/// The style of link to parse from the markdown files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkStyle {
    /// Infer the link style based on the link format. If the link starts with `./` or `../` or it
    /// is a path with a single element (e.g. `file.md`), it is considered relative to the file.
    /// Otherwise, it is considered relative to the vault root.
    #[default]
    Infer,
    /// All links are considered relative to the vault root
    FromVaultRoot,
    /// All links are considered relative to the file they are found in
    RelativeToFile,
}

impl FromStr for LinkStyle {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "infer" => Ok(LinkStyle::Infer),
            "vault" | "from_vault_root" => Ok(LinkStyle::FromVaultRoot),
            "relative" | "relative_to_file" => Ok(LinkStyle::RelativeToFile),
            _ => Err(anyhow::anyhow!("Invalid link style: {}", s)),
        }
    }
}

impl LinkStyle {
    fn path_from_link<'a, T: AsRef<Path>>(
        &self,
        raw_link: PathBuf,
        file_path: &'a Path,
        vault_root: &'a T,
    ) -> PathBuf {
        match self {
            LinkStyle::Infer => {
                if raw_link.starts_with("./")
                    || raw_link.starts_with("../")
                    || raw_link.components().count() == 1
                {
                    file_path.parent().unwrap_or(Path::new("")).join(raw_link)
                } else {
                    vault_root.as_ref().join(raw_link)
                }
            }
            LinkStyle::FromVaultRoot => vault_root.as_ref().join(raw_link),
            LinkStyle::RelativeToFile => file_path.parent().unwrap_or(Path::new("")).join(raw_link),
        }
    }
}

/// Parse the links from a list of ParsedFiles, returning an iterator of tuples of the
/// [`ParsedFile`] returned as is and a vec of PathBufs representing the links found in the file.
/// The returned links are not canonicalized or checked for existence and are created based on the
/// provided `link_style`.
pub fn parse_links<'a, T: AsRef<Path>>(
    entries: impl IntoIterator<Item = ParsedFile<'a>>,
    vault_root: &'a T,
    link_style: LinkStyle,
) -> impl Iterator<Item = (ParsedFile<'a>, Vec<PathBuf>)> {
    entries.into_iter().map(move |pf| {
        let links = parse_links_from_ast(&pf.path, pf.ast, vault_root, link_style);
        (pf, links)
    })
}

/// Parse the links from the AST of a markdown file
fn parse_links_from_ast<'a, T: AsRef<Path>>(
    file_path: &Path,
    ast: &'a AstNode<'a>,
    vault_root: &'a T,
    link_style: LinkStyle,
) -> Vec<PathBuf> {
    ast.descendants()
        .filter_map(|node| {
            let raw_path = match &node.data.borrow().value {
                NodeValue::Link(link) => link.url.clone(),
                NodeValue::WikiLink(link) => link.url.clone(),
                _ => return None,
            };
            // A normal file path does not parse as a URL, so if it does, we skip it
            if url::Url::parse(&raw_path).is_ok() {
                return None;
            }
            // Otherwise, we can convert it to a PathBuf and add it to our list of links
            Some(link_style.path_from_link(PathBuf::from(raw_path), file_path, vault_root))
        })
        .collect()
}
