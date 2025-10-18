pub mod frontmatter;
pub mod parser;
pub mod printer;
pub mod reader;

/// The key used in serialized representations of tag data. This key is used when combining data
/// together from different commands before (i.e. having both tags and links data in one output).
pub const TAGS_DATA_KEY: &str = "tags";
/// The key used in serialized representations of link data. This key is used when combining data
/// together from different commands before (i.e. having both tags and links data in one output).
pub const LINKS_DATA_KEY: &str = "links";
