use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    str::FromStr,
};

use clap::Parser;
use comrak::Arena;
use serde::{Deserialize, Serialize};

use obsidian_core::{
    frontmatter, parser,
    printer::{self, Format},
    reader,
};

/// A command line tool for extracting and filtering Obsidian tags from markdown files.
///
/// By default, this tool will read all markdown files in the specified directory and parse the frontmatter content for tags.
/// If using a structured printer format (like JSON), it will output a mapping of tags to the files that contain them.
/// For plain text output, it will return a list of unique tags found across all files.
#[derive(Parser, Debug)]
#[command(name = "obsidian-tags", about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub printer: printer::PrinterArgs,

    #[command(flatten)]
    pub read_opts: reader::ReaderOpts,

    /// A filter expression for selecting files based on their tags. When this is passed, the output
    /// will be in the form of a list of files, encoded according to the selected format.
    ///
    /// Filter types:
    /// - tag:<tag1,tag2,...> : Selects files that have all of the specified tags
    /// - tag-any:<tag1,tag2,...> : Selects files that have any of the specified tags
    #[arg(short = 'f', long)]
    pub filter: Option<Filter>,
}

#[derive(Debug, Clone)]
/// A filter for selecting files based on their tags
pub enum Filter {
    TagAll(BTreeSet<String>),
    TagAny(BTreeSet<String>),
}

impl FromStr for Filter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (operator, rest) = s.split_once(':').ok_or_else(|| {
            anyhow::anyhow!("Invalid filter format. Expected 'tag:<tags>' or 'tag-any:<tags>'")
        })?;
        match operator {
            "tag" => Ok(Filter::TagAll(
                rest.split(',').map(|s| s.trim().to_string()).collect(),
            )),
            "tag-any" => Ok(Filter::TagAny(
                rest.split(',').map(|s| s.trim().to_string()).collect(),
            )),
            _ => Err(anyhow::anyhow!(
                "Unknown filter operator: {}. Expected 'tag' or 'tag-any'",
                operator
            )),
        }
    }
}

impl Filter {
    fn get_matches<'a>(&self, tags: &'a BTreeMap<String, TagInfo>) -> BTreeSet<&'a PathBuf> {
        match self {
            Filter::TagAll(required_tags) => {
                let mut sets: Vec<BTreeSet<&PathBuf>> = Vec::with_capacity(required_tags.len());
                for tag in required_tags {
                    if let Some(tag_info) = tags.get(tag) {
                        sets.push(tag_info.files.iter().collect());
                    } else {
                        // If any required tag is missing, no files can match
                        return BTreeSet::new();
                    }
                }
                // Intersect all sets to find files that have all required tags
                sets.into_iter()
                    .reduce(|a, b| a.intersection(&b).copied().collect())
                    .unwrap_or_default()
            }
            Filter::TagAny(possible_tags) => {
                let mut result = BTreeSet::new();
                for tag in possible_tags {
                    if let Some(tag_info) = tags.get(tag) {
                        result.extend(tag_info.files.iter());
                    }
                }
                result
            }
        }
    }
}

/// A struct tying data to a tag. Right now this is really simple, but may be expanded in the future
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TagInfo {
    /// The files associated with this tag
    pub files: BTreeSet<PathBuf>,
}

impl TagInfo {
    pub fn new() -> Self {
        Self::default()
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    env_logger::init();

    let entries = cli.read_opts.read_files()?;

    let arena = Arena::with_capacity(entries.len());
    let parsed_files = parser::ignore_error_iter(parser::parse_files(&arena, entries));
    let parsed_with_fm = frontmatter::parse_frontmatter(parsed_files);

    let tags = parsed_with_fm.fold(BTreeMap::new(), |mut acc, (pf, fm)| {
        if let Some(fm) = fm {
            for tag in fm.tags.unwrap_or_default() {
                acc.entry(tag)
                    .or_insert_with(TagInfo::new)
                    .files
                    // Have to clone because pf has a lifetime
                    .insert(pf.path.clone());
            }
        }
        acc
    });

    let format = cli.printer.output;
    let mut writer = std::io::stdout();
    match format {
        Format::Plain => {
            if let Some(filter) = cli.filter {
                let matches = filter.get_matches(&tags);
                format.print_plain(matches.into_iter().map(|p| p.display()), &mut writer)
            } else {
                format.print_plain(tags.keys(), &mut writer)
            }
        }
        Format::Json | Format::Binary => {
            if let Some(filter) = cli.filter {
                let matches = filter.get_matches(&tags);
                format.print_structured(matches, &mut writer)
            } else {
                format.print_structured(tags, &mut writer)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Filter, TagInfo};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;
    use std::str::FromStr;

    fn expected_tag_set(tags: &[&str]) -> BTreeSet<String> {
        tags.iter().map(|tag| tag.to_string()).collect()
    }

    fn tag_info(paths: &[&str]) -> TagInfo {
        TagInfo {
            files: paths.iter().map(PathBuf::from).collect::<BTreeSet<_>>(),
        }
    }

    #[test]
    fn parses_tag_all_filter() {
        let filter = Filter::from_str("tag:one,two").expect("expected filter to parse");
        match filter {
            Filter::TagAll(tags) => assert_eq!(tags, expected_tag_set(&["one", "two"])),
            _ => panic!("expected Filter::TagAll variant"),
        }
    }

    #[test]
    fn parses_tag_any_filter_with_whitespace() {
        let filter =
            Filter::from_str("tag-any: one , two ").expect("expected filter to parse with trim");
        match filter {
            Filter::TagAny(tags) => assert_eq!(tags, expected_tag_set(&["one", "two"])),
            _ => panic!("expected Filter::TagAny variant"),
        }
    }

    #[test]
    fn parsing_rejects_unknown_operator() {
        let err = Filter::from_str("not-real:one").expect_err("expected parsing to fail");
        assert!(
            err.to_string().contains("Unknown filter operator"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn parsing_rejects_missing_separator() {
        let err = Filter::from_str("tag").expect_err("expected parsing to fail");
        assert!(
            err.to_string()
                .contains("Invalid filter format. Expected 'tag:<tags>' or 'tag-any:<tags>'"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn get_matches_requires_all_tags() {
        let mut tags: BTreeMap<String, TagInfo> = BTreeMap::new();
        tags.insert("one".into(), tag_info(&["note1.md", "note2.md"]));
        tags.insert("two".into(), tag_info(&["note2.md", "note3.md"]));

        let filter = Filter::from_str("tag:one,two").unwrap();
        let matches = filter
            .get_matches(&tags)
            .into_iter()
            .cloned()
            .collect::<BTreeSet<PathBuf>>();
        let expected = ["note2.md"]
            .into_iter()
            .map(PathBuf::from)
            .collect::<BTreeSet<_>>();

        assert_eq!(matches, expected);
    }

    #[test]
    fn get_matches_handles_missing_required_tag() {
        let mut tags: BTreeMap<String, TagInfo> = BTreeMap::new();
        tags.insert("one".into(), tag_info(&["note1.md", "note2.md"]));

        let filter = Filter::from_str("tag:one,two").unwrap();
        let matches = filter.get_matches(&tags);

        assert!(matches.is_empty());
    }

    #[test]
    fn get_matches_collects_any_tags() {
        let mut tags: BTreeMap<String, TagInfo> = BTreeMap::new();
        tags.insert("one".into(), tag_info(&["note1.md", "note2.md"]));
        tags.insert("two".into(), tag_info(&["note2.md", "note3.md"]));

        let filter = Filter::from_str("tag-any:two,missing").unwrap();
        let matches = filter
            .get_matches(&tags)
            .into_iter()
            .cloned()
            .collect::<BTreeSet<PathBuf>>();
        let expected = ["note2.md", "note3.md"]
            .into_iter()
            .map(PathBuf::from)
            .collect::<BTreeSet<_>>();

        assert_eq!(matches, expected);
    }
}
