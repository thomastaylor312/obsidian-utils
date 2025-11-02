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
                // NOTE(thomastaylor312): We can't just use is_relative here since most links in
                // obsidian are relative to _something_
                if raw_link.starts_with("./")
                    || raw_link.starts_with("../")
                    || raw_link
                        .to_str()
                        // Check if the first character is not a slash or backslash (which means it
                        // is relative to the current file)
                        .is_some_and(|s| s.starts_with(|c| c != '/' && c != '\\'))
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

            // Links may be percent-encoded, so we decode them first
            let decoded_path = match urlencoding::decode(&raw_path).ok() {
                Some(dp) => dp.into_owned(),
                None => {
                    log::warn!("Failed to decode link path: {}", raw_path);
                    return None;
                }
            };

            // Convert to PathBuf
            let mut decoded_path = PathBuf::from(decoded_path);

            // Now remove any fragment components (e.g. #heading) from the path since these are
            // valid in markdown links. These will only be in the filename, so we pull that off,
            // remove the fragment, and reattach it.

            let maybe_cleaned = if let Some((file_stem, _)) = decoded_path
                .file_name()
                .and_then(|fname| fname.to_str())
                .and_then(|s| s.split_once('#'))
            {
                // This would be internal document links (i.e. just a heading), so we skip it
                if file_stem.is_empty() {
                    return None;
                }
                // Clone the cleaned filename so we release the borrow on decoded_path
                Some(file_stem.to_owned())
            } else {
                None
            };
            if let Some(cleaned) = maybe_cleaned {
                decoded_path.set_file_name(cleaned);
            }
            Some(link_style.path_from_link(decoded_path, file_path, vault_root))
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

    fn encoded_file_path() -> PathBuf {
        vault_root().join("links/Encoded.md")
    }

    fn load_source_file<'a>(arena: &'a Arena<AstNode<'a>>) -> Result<ParsedFile<'a>> {
        load_file(arena, source_file_path())
    }

    fn load_encoded_file<'a>(arena: &'a Arena<AstNode<'a>>) -> Result<ParsedFile<'a>> {
        load_file(arena, encoded_file_path())
    }

    fn load_file<'a>(arena: &'a Arena<AstNode<'a>>, path: PathBuf) -> Result<ParsedFile<'a>> {
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
            file_dir.join("nested/Deep.md"),
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

    #[test]
    fn parse_links_decodes_percent_encoding_and_strips_fragments() -> Result<()> {
        let vault = vault_root();
        let arena = Arena::new();
        let parsed = load_encoded_file(&arena)?;
        let file_dir = parsed.path.parent().unwrap().to_path_buf();

        let mut results: Vec<_> = parse_links(vec![parsed], &vault, LinkStyle::Infer).collect();
        assert_eq!(results.len(), 1);
        let (_file, links) = results.pop().unwrap();
        let observed = link_set(links);

        let expected = link_set([
            file_dir.join("./Space Target.md"),
            file_dir.join("./Fragment Target.md"),
        ]);

        assert_eq!(observed, expected);
        Ok(())
    }
}
