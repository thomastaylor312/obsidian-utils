use std::{collections::HashMap, sync::LazyLock};

use comrak::nodes::{AstNode, NodeValue};
use serde::{Deserialize, Serialize};

use crate::parser::ParsedFile;

// TODO: Figure out if we can dynamically generate a schema from the frontmatter keys
// so we don't have to use serde_norway::Value everywhere

static FRONTMATTER_DELIMITER_CHARS: LazyLock<Vec<char>> =
    LazyLock::new(|| crate::parser::FRONTMATTER_DELIMITER.chars().collect());

/// A struct representing the known frontmatter of a markdown file plus additional values
#[derive(Debug, Serialize, Deserialize)]
pub struct Frontmatter {
    /// The tags associated with this file
    pub tags: Option<Vec<String>>,
    /// The aliases associated with this file
    pub aliases: Option<Vec<String>>,
    /// The CSS classes associated with this file
    pub cssclasses: Option<Vec<String>>,
    /// Any additional frontmatter values not explicitly modeled above
    #[serde(flatten)]
    pub values: HashMap<String, serde_norway::Value>,
}

/// Parse the frontmatter from a list of ParsedFiles, returning an iterator of tuples of the
/// [`ParsedFile`] returned as it and an optional [serde_norway::Value] representing the frontmatter
/// if it exists
pub fn parse_frontmatter<'a>(
    entries: impl IntoIterator<Item = ParsedFile<'a>>,
) -> impl Iterator<Item = (ParsedFile<'a>, Option<Frontmatter>)> {
    entries.into_iter().map(|pf| {
        let fm = parse_frontmatter_from_ast(pf.ast);
        (pf, fm)
    })
}

/// Parse the frontmatter from the AST of a markdown file
fn parse_frontmatter_from_ast<'a>(ast: &'a AstNode<'a>) -> Option<Frontmatter> {
    for node in ast.descendants() {
        if let NodeValue::FrontMatter(ref text) = node.data.borrow().value {
            let trimmed = text
                .trim()
                .trim_matches(FRONTMATTER_DELIMITER_CHARS.as_slice());
            let fm: Frontmatter = serde_norway::from_str(trimmed)
                .map_err(|e| {
                    log::error!("Failed to parse frontmatter: {}", e);
                })
                .ok()?;
            return Some(fm);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_norway::Value;

    #[test]
    fn test_parse_frontmatter() {
        let input = r#"---
tags: [test]
aliases: [test_alias]
cssclasses: [test_class]
extra: "value"
---

"#;
        let arena = comrak::Arena::new();
        let ast = crate::parser::parse_content(&arena, input);
        let frontmatter = parse_frontmatter_from_ast(ast);

        assert!(frontmatter.is_some(), "Failed to parse frontmatter");

        let frontmatter = frontmatter.unwrap();
        assert_eq!(frontmatter.tags, Some(vec!["test".to_string()]));
        assert_eq!(frontmatter.aliases, Some(vec!["test_alias".to_string()]));
        assert_eq!(frontmatter.cssclasses, Some(vec!["test_class".to_string()]));
        assert_eq!(
            frontmatter.values.get("extra"),
            Some(&Value::String("value".into()))
        );
    }
}
