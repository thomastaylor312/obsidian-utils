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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use comrak::Arena;
    use obsidian_core::parser::{self, ParsedFile};
    use std::collections::HashSet;
    use std::iter::FromIterator;

    fn vault_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-vault")
    }

    fn source_file_path() -> PathBuf {
        vault_root().join("links/Source.md")
    }

    fn load_source_file<'a>(arena: &'a Arena<AstNode<'a>>) -> Result<ParsedFile<'a>> {
        let path = source_file_path();
        let metadata = std::fs::metadata(&path)?;
        let ast = parser::parse_file(arena, &path)?;
        Ok(ParsedFile {
            path,
            metadata,
            ast,
        })
    }

    fn link_set<I>(links: I) -> HashSet<PathBuf>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        HashSet::from_iter(links)
    }

    #[test]
    fn parse_links_infer_style_resolves_relative_and_root_paths() -> Result<()> {
        let vault = vault_root();
        let arena = Arena::new();
        let parsed = load_source_file(&arena)?;
        let file_dir = parsed.path.parent().unwrap().to_path_buf();

        let mut results: Vec<_> = parse_links(vec![parsed], &vault, LinkStyle::Infer).collect();
        assert_eq!(results.len(), 1);
        let (_file, links) = results.pop().unwrap();
        let observed = link_set(links);

        let expected = link_set([
            file_dir.join("../Test.md"),
            vault.join("nested/Deep.md"),
            file_dir.join("./Sibling.md"),
            file_dir.join("WikiTarget"),
            file_dir.join("./WikiSibling"),
        ]);

        assert_eq!(observed, expected);

        Ok(())
    }

    #[test]
    fn parse_links_from_vault_root_style_prefixes_vault_directory() -> Result<()> {
        let vault = vault_root();
        let arena = Arena::new();
        let parsed = load_source_file(&arena)?;

        let mut results: Vec<_> =
            parse_links(vec![parsed], &vault, LinkStyle::FromVaultRoot).collect();
        assert_eq!(results.len(), 1);
        let (_file, links) = results.pop().unwrap();
        let observed = link_set(links);

        let expected = link_set([
            vault.join("../Test.md"),
            vault.join("nested/Deep.md"),
            vault.join("./Sibling.md"),
            vault.join("WikiTarget"),
            vault.join("./WikiSibling"),
        ]);

        assert_eq!(observed, expected);
        Ok(())
    }

    #[test]
    fn parse_links_relative_to_file_style_keeps_links_local() -> Result<()> {
        let vault = vault_root();
        let arena = Arena::new();
        let parsed = load_source_file(&arena)?;
        let file_dir = parsed.path.parent().unwrap().to_path_buf();

        let mut results: Vec<_> =
            parse_links(vec![parsed], &vault, LinkStyle::RelativeToFile).collect();
        assert_eq!(results.len(), 1);
        let (_file, links) = results.pop().unwrap();
        let observed = link_set(links);

        let expected = link_set([
            file_dir.join("../Test.md"),
            file_dir.join("nested/Deep.md"),
            file_dir.join("./Sibling.md"),
            file_dir.join("WikiTarget"),
            file_dir.join("./WikiSibling"),
        ]);

        assert_eq!(observed, expected);

        Ok(())
    }
}
