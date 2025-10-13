use std::io::Write;

use anyhow::Context;
use clap::Args;
use serde::Serialize;
use tabled::{Tabled, settings::Style};

#[derive(Debug, Args)]
pub struct PrinterArgs {
    #[arg(long, short = 'o', default_value_t = Format::Table)]
    pub format: Format,
}

/// A trait for converting a type into a type that implements Tabled. This is useful for cases where
/// the type we want to print does not implement Tabled directly, but we can convert it into a type
/// that does.
pub trait AsTabled {
    type Table: Tabled;

    fn as_tabled(&self) -> impl IntoIterator<Item = &Self::Table>;
}

impl<T: Tabled> AsTabled for Vec<T> {
    type Table = T;

    fn as_tabled(&self) -> impl IntoIterator<Item = &Self::Table> {
        self
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
    /// both `Serialize` and `IntoTabled` traits. This may change in the future if we need more
    /// flexibility (i.e. different types for different formats implementing only one of the
    /// traits).
    pub fn print<S: Serialize + AsTabled, W: Write>(
        &self,
        data: &S,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        match self {
            Format::Table => {
                let mut table = tabled::Table::new(data.as_tabled());
                table.with(Style::modern());
                writeln!(writer, "{table}").context("Error writing table data")
            }
            Format::Json => {
                serde_json::to_writer(writer, data).context("JSON serialization failed")
            }
            Format::Binary => {
                ciborium::into_writer(data, writer).context("CBOR serialization failed")
            }
        }
    }
}
