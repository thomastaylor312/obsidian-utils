//! Library for working with Obsidian `.base` files.

pub mod ast;
pub mod error;
pub mod parser;
pub mod prepared;
pub mod value;

mod schema;

pub use crate::error::ParseErrorInfo;
pub use crate::prepared::{PreparedBase, PreparedFilter, PreparedView};
pub use crate::schema::{
    BaseFile, FilterNode, PropertyConfig, SortDirection, SortField, View, ViewType,
};
pub use anyhow::Result;
pub use value::{
    FileValue, LinkValue, TypeError, Value, ValueDate, ValueDateTime, ValueDuration, ValueError,
    ValueResult,
};

use std::path::Path;

/// Loads a base file from the provided path.
pub fn load_base_file(path: impl AsRef<Path>) -> Result<BaseFile> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path)?;
    from_yaml_str(&contents)
}

/// Deserializes a base file from a YAML string slice.
pub fn from_yaml_str(yaml: &str) -> Result<BaseFile> {
    Ok(serde_norway::from_str(yaml)?)
}

/// Deserializes a base file from a YAML reader.
pub fn from_yaml_reader<R>(reader: R) -> Result<BaseFile>
where
    R: std::io::Read,
{
    Ok(serde_norway::from_reader(reader)?)
}
