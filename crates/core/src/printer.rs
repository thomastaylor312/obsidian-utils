use std::{borrow::Cow, collections::HashMap, hash::Hash, io::Write};

use anyhow::Context;
use clap::Args;
use serde::{Deserialize, Serialize};
use tabled::{Tabled, settings::Style};

#[derive(Debug, Args)]
pub struct PrinterArgs {
    #[arg(long, short = 'o', default_value_t = Format::Table)]
    pub format: Format,
}

/// A trait for converting a type into a tabled::Table. This is mostly a utility trait to allow for
/// different input types to be converted into a table for printing. Most likely you'll be using
/// this via blanket implementations of the trait or via the [`HashMapTabler`] struct.
pub trait IntoTable {
    fn into_table(self) -> tabled::Table;
}

impl IntoTable for tabled::Table {
    fn into_table(self) -> tabled::Table {
        self
    }
}

impl<T: Tabled> IntoTable for Vec<T> {
    fn into_table(self) -> tabled::Table {
        tabled::Table::new(self)
    }
}

/// A wrapper around a HashMap to allow printing it as a tabled::Table. The key of the map will be
/// used as the first column, with the rest of the columns coming from the values in the map. This
/// will delegate serialization and deserialization to underlying map.
pub struct HashMapTabler<K, V> {
    map: HashMap<K, V>,
    key_name: String,
}

impl<K, V> HashMapTabler<K, V> {
    /// Create a new HashMapTabler with the given key name and map. The key name will be used as the
    /// header for the first column in the table.
    pub fn new(key_name: impl Into<String>, map: HashMap<K, V>) -> Self {
        Self {
            map,
            key_name: key_name.into(),
        }
    }
}

impl<K: Serialize, V: Serialize> Serialize for HashMapTabler<K, V> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.map.serialize(serializer)
    }
}

impl<'de, K, V> Deserialize<'de> for HashMapTabler<K, V>
where
    K: Deserialize<'de> + Hash + Eq,
    V: Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = HashMap::<K, V>::deserialize(deserializer)?;
        Ok(Self {
            map,
            key_name: "Key".to_string(),
        })
    }
}

impl<K: Into<String>, V: Tabled> IntoTable for HashMapTabler<K, V> {
    fn into_table(self) -> tabled::Table {
        let mut columns = Vec::with_capacity(V::LENGTH + 1); // +1 for the key column
        columns.push(Cow::Owned(self.key_name));
        columns.extend(V::headers());
        let mut builder = tabled::builder::Builder::with_capacity(self.map.len(), columns.len());
        builder.push_record(columns);
        for (key, value) in self.map {
            let row = std::iter::once(Cow::Owned(key.into())).chain(value.fields());
            builder.push_record(row);
        }
        builder.build()
    }
}

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Table,
    Json,
    Binary,
}

impl std::str::FromStr for Format {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Format::Table),
            "json" => Ok(Format::Json),
            "binary" => Ok(Format::Binary),
            _ => Err(anyhow::anyhow!("Unknown format: {}", s)),
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Format::Table => "table",
            Format::Json => "json",
            Format::Binary => "binary",
        };
        write!(f, "{}", s)
    }
}

impl Format {
    /// Print the given data in the specified format to the given writer. The data must implement
    /// both `Serialize` and `IntoTable` traits. This may change in the future if we need more
    /// flexibility (i.e. different types for different formats implementing only one of the
    /// traits). The `data_key` parameter is used as the top level key of the outputted object. It
    /// essentially serves as a "namespace" for the data being output so it can be combined down the
    /// line (e.g. `{"tags": {...}, "links": {...}}`).
    pub fn print<S: Serialize + IntoTable, W: Write>(
        &self,
        data_key: &str,
        data: S,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        match self {
            Format::Table => {
                let mut table = data.into_table();
                table.with(Style::modern());
                writeln!(writer, "{table}").context("Error writing table data")
            }
            Format::Json => serde_json::to_writer(writer, &HashMap::from([(data_key, data)]))
                .context("JSON serialization failed"),
            Format::Binary => ciborium::into_writer(&HashMap::from([(data_key, data)]), writer)
                .context("CBOR serialization failed"),
        }
    }
}
