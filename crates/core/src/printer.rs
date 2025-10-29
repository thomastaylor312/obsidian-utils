use std::{collections::HashMap, io::Write};

use anyhow::Context;
use clap::Args;
use serde::Serialize;

#[derive(Debug, Args)]
pub struct PrinterArgs {
    #[arg(long, short = 'o', default_value_t = Format::Json)]
    pub format: Format,
}

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Json,
    Binary,
}

impl std::str::FromStr for Format {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Format::Json),
            "binary" => Ok(Format::Binary),
            _ => Err(anyhow::anyhow!("Unknown format: {}", s)),
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
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
    pub fn print<S: Serialize, W: Write>(
        &self,
        data_key: &str,
        data: S,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        match self {
            Format::Json => serde_json::to_writer(writer, &HashMap::from([(data_key, data)]))
                .context("JSON serialization failed"),
            Format::Binary => ciborium::into_writer(&HashMap::from([(data_key, data)]), writer)
                .context("CBOR serialization failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::io::Cursor;

    #[derive(Debug, Clone, Serialize, serde::Deserialize)]
    struct Row {
        name: String,
    }

    #[test]
    fn format_from_str_accepts_supported_values() {
        assert_eq!("json".parse::<Format>().unwrap(), Format::Json);
        assert_eq!("binary".parse::<Format>().unwrap(), Format::Binary);
        assert!("unknown".parse::<Format>().is_err());
    }

    #[test]
    fn json_format_round_trip() -> anyhow::Result<()> {
        let rows = vec![Row {
            name: "gamma".into(),
        }];
        let mut buffer = Vec::new();

        Format::Json.print("rows", rows, &mut buffer)?;

        let value: Value = serde_json::from_slice(&buffer)?;
        assert_eq!(value["rows"][0]["name"], "gamma");

        Ok(())
    }

    #[test]
    fn binary_format_round_trip() -> anyhow::Result<()> {
        let rows = vec![Row {
            name: "delta".into(),
        }];
        let mut buffer = Vec::new();

        Format::Binary.print("rows", rows, &mut buffer)?;

        let mut cursor = Cursor::new(buffer);
        let decoded: HashMap<String, Vec<Row>> = ciborium::from_reader(&mut cursor)?;

        assert_eq!(decoded.get("rows").unwrap()[0].name, "delta");

        Ok(())
    }
}
