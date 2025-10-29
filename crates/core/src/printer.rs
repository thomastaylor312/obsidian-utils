use std::{fmt::Display, io::Write};

use anyhow::Context;
use clap::Args;
use serde::Serialize;

#[derive(Debug, Args)]
pub struct PrinterArgs {
    /// The output format to use. Valid options are "plain", "json", and "binary". Default is "plain".
    #[arg(long, short = 'o', default_value_t = Format::default())]
    pub output: Format,
}

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Plain,
    Json,
    Binary,
}

impl std::str::FromStr for Format {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plain" => Ok(Format::Plain),
            "json" => Ok(Format::Json),
            "binary" => Ok(Format::Binary),
            _ => Err(anyhow::anyhow!("Unknown format: {}", s)),
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Format::Plain => "plain",
            Format::Json => "json",
            Format::Binary => "binary",
        };
        write!(f, "{}", s)
    }
}

impl Format {
    /// Print the given data in the specified format to the given writer. If the format is not a
    /// structured type, this method will return an error.
    pub fn print_structured<S: Serialize, W: Write>(
        &self,
        data: S,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        match self {
            Format::Json => {
                serde_json::to_writer(writer, &data).context("JSON serialization failed")
            }
            Format::Binary => {
                ciborium::into_writer(&data, writer).context("CBOR serialization failed")
            }
            Format::Plain => {
                anyhow::bail!("Plain format not supported")
            }
        }
    }

    /// Print the data as plain text to the given writer. If the format is not plain text, this
    /// method will return an error. This is fairly generic to allow the caller to control which
    /// data is printed.
    pub fn print_plain<T, D, W>(&self, data: T, writer: &mut W) -> anyhow::Result<()>
    where
        T: Iterator<Item = D>,
        D: Display,
        W: Write,
    {
        match self {
            Format::Plain => {
                for item in data {
                    writeln!(writer, "{}", item)?;
                }
                Ok(())
            }
            _ => {
                anyhow::bail!("Non-plain format not supported for plain text output")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    use serde_json::Value;

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

        Format::Json.print_structured(rows, &mut buffer)?;

        let value: Value = serde_json::from_slice(&buffer)?;
        assert_eq!(value[0]["name"], "gamma");

        Ok(())
    }

    #[test]
    fn binary_format_round_trip() -> anyhow::Result<()> {
        let rows = vec![Row {
            name: "delta".into(),
        }];
        let mut buffer = Vec::new();

        Format::Binary.print_structured(rows, &mut buffer)?;

        let mut cursor = Cursor::new(buffer);
        let decoded: Vec<Row> = ciborium::from_reader(&mut cursor)?;

        assert_eq!(decoded[0].name, "delta");

        Ok(())
    }
}
