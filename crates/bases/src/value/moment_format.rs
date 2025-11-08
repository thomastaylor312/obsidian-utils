//! Moment.js to chrono format string converter.
//!
//! This module provides functionality to convert moment.js format strings
//! (commonly used in Obsidian/Dataview) to chrono format strings.
//!
//! Reference: https://momentjs.com/docs/#/displaying/format/

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_while1},
    combinator::value,
    multi::many0,
};

/// A token in a moment.js format string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MomentToken {
    /// A literal string that should be passed through unchanged.
    Literal(String),
    /// Year tokens
    YearFour, // YYYY -> %Y
    YearTwo, // YY -> %y
    /// Month tokens
    MonthPadded, // MM -> %m
    MonthUnpadded, // M -> %-m
    MonthFull, // MMMM -> %B
    MonthAbbrev, // MMM -> %b
    /// Day of month tokens
    DayPadded, // DD -> %d
    DayUnpadded, // D -> %-d
    /// Day of week tokens
    WeekdayFull, // dddd -> %A
    WeekdayAbbrev, // ddd -> %a
    WeekdayMin, // dd -> two-letter abbreviation (no direct chrono equivalent)
    WeekdayNum, // d -> weekday number (0-6)
    /// Hour tokens (24-hour)
    Hour24Padded, // HH -> %H
    Hour24Unpadded, // H -> %-H
    /// Hour tokens (12-hour)
    Hour12Padded, // hh -> %I
    Hour12Unpadded, // h -> %-I
    /// Minute tokens
    MinutePadded, // mm -> %M
    MinuteUnpadded, // m -> %-M
    /// Second tokens
    SecondPadded, // ss -> %S
    SecondUnpadded, // s -> %-S
    /// Milliseconds
    Milliseconds, // SSS -> %3f
    /// AM/PM
    AmPmUpper, // A -> %p (uppercase AM/PM)
    AmPmLower, // a -> %P (lowercase am/pm)
    /// Timezone tokens
    TimezoneColon, // Z -> %:z (+01:00)
    TimezoneNoColon, // ZZ -> %z (+0100)
    /// Unix timestamp
    UnixSeconds, // X -> %s
    /// Day of year
    DayOfYear, // DDD -> %j (001-366)
    /// Week of year
    WeekOfYear, // ww or WW -> %W or %U (depending on ISO vs US week)
    /// Quarter (no chrono equivalent, needs manual handling)
    Quarter, // Q -> 1-4
}

impl MomentToken {
    /// Convert this token to its chrono format string equivalent.
    pub fn to_chrono(&self) -> &'static str {
        match self {
            MomentToken::Literal(_) => "", // Handled specially
            MomentToken::YearFour => "%Y",
            MomentToken::YearTwo => "%y",
            MomentToken::MonthPadded => "%m",
            MomentToken::MonthUnpadded => "%-m",
            MomentToken::MonthFull => "%B",
            MomentToken::MonthAbbrev => "%b",
            MomentToken::DayPadded => "%d",
            MomentToken::DayUnpadded => "%-d",
            MomentToken::WeekdayFull => "%A",
            MomentToken::WeekdayAbbrev => "%a",
            MomentToken::WeekdayMin => "%a", // Fallback to abbreviated (chrono doesn't have 2-letter)
            MomentToken::WeekdayNum => "%w",
            MomentToken::Hour24Padded => "%H",
            MomentToken::Hour24Unpadded => "%-H",
            MomentToken::Hour12Padded => "%I",
            MomentToken::Hour12Unpadded => "%-I",
            MomentToken::MinutePadded => "%M",
            MomentToken::MinuteUnpadded => "%-M",
            MomentToken::SecondPadded => "%S",
            MomentToken::SecondUnpadded => "%-S",
            MomentToken::Milliseconds => "%3f",
            MomentToken::AmPmUpper => "%p",
            MomentToken::AmPmLower => "%P",
            MomentToken::TimezoneColon => "%:z",
            MomentToken::TimezoneNoColon => "%z",
            MomentToken::UnixSeconds => "%s",
            MomentToken::DayOfYear => "%j",
            MomentToken::WeekOfYear => "%W",
            MomentToken::Quarter => "Q", // Not a chrono specifier; handled as literal
        }
    }
}

/// Parse a moment.js format string into tokens.
pub fn parse_moment_format(input: &str) -> Result<Vec<MomentToken>, String> {
    match many0(parse_token).parse(input) {
        Ok(("", tokens)) => Ok(tokens),
        Ok((remaining, _)) => Err(format!(
            "Unexpected characters in format string: {remaining}"
        )),
        Err(e) => Err(format!("Failed to parse format string: {e}")),
    }
}

/// Convert a moment.js format string to a chrono format string.
pub fn to_chrono_format(input: &str) -> Result<String, String> {
    let tokens = parse_moment_format(input)?;
    let mut result = String::new();

    for token in tokens {
        match token {
            MomentToken::Literal(s) => result.push_str(&s),
            MomentToken::Quarter => {
                // Quarter is not directly supported by chrono, pass through as-is
                result.push('Q');
            }
            other => result.push_str(other.to_chrono()),
        }
    }

    Ok(result)
}

// Parser combinators for each token type.
// Order matters: longer tokens must be tried before shorter ones.

fn parse_token(input: &str) -> IResult<&str, MomentToken> {
    alt((
        parse_year_tokens,
        parse_month_tokens,
        parse_day_tokens,
        parse_weekday_tokens,
        parse_hour_tokens,
        parse_minute_second_tokens,
        parse_other_tokens,
        parse_escaped,
        parse_literal,
    ))
    .parse(input)
}

fn parse_year_tokens(input: &str) -> IResult<&str, MomentToken> {
    alt((
        value(MomentToken::YearFour, tag("YYYY")),
        value(MomentToken::YearTwo, tag("YY")),
    ))
    .parse(input)
}

fn parse_month_tokens(input: &str) -> IResult<&str, MomentToken> {
    alt((
        value(MomentToken::MonthFull, tag("MMMM")),
        value(MomentToken::MonthAbbrev, tag("MMM")),
        value(MomentToken::MonthPadded, tag("MM")),
        // Note: single M is ambiguous with minute m; we handle M before m
        value(MomentToken::MonthUnpadded, tag("M")),
    ))
    .parse(input)
}

fn parse_day_tokens(input: &str) -> IResult<&str, MomentToken> {
    alt((
        value(MomentToken::DayOfYear, tag("DDDD")),
        value(MomentToken::DayOfYear, tag("DDD")),
        value(MomentToken::DayPadded, tag("DD")),
        value(MomentToken::DayUnpadded, tag("D")),
    ))
    .parse(input)
}

fn parse_weekday_tokens(input: &str) -> IResult<&str, MomentToken> {
    alt((
        value(MomentToken::WeekdayFull, tag("dddd")),
        value(MomentToken::WeekdayAbbrev, tag("ddd")),
        value(MomentToken::WeekdayMin, tag("dd")),
        value(MomentToken::WeekdayNum, tag("d")),
    ))
    .parse(input)
}

fn parse_hour_tokens(input: &str) -> IResult<&str, MomentToken> {
    alt((
        value(MomentToken::Hour24Padded, tag("HH")),
        value(MomentToken::Hour24Unpadded, tag("H")),
        value(MomentToken::Hour12Padded, tag("hh")),
        value(MomentToken::Hour12Unpadded, tag("h")),
    ))
    .parse(input)
}

fn parse_minute_second_tokens(input: &str) -> IResult<&str, MomentToken> {
    alt((
        // Milliseconds must come before seconds (SSS before ss)
        value(MomentToken::Milliseconds, tag("SSS")),
        value(MomentToken::SecondPadded, tag("ss")),
        value(MomentToken::SecondUnpadded, tag("s")),
        value(MomentToken::MinutePadded, tag("mm")),
        value(MomentToken::MinuteUnpadded, tag("m")),
    ))
    .parse(input)
}

fn parse_other_tokens(input: &str) -> IResult<&str, MomentToken> {
    alt((
        value(MomentToken::AmPmUpper, tag("A")),
        value(MomentToken::AmPmLower, tag("a")),
        value(MomentToken::TimezoneNoColon, tag("ZZ")),
        value(MomentToken::TimezoneColon, tag("Z")),
        value(MomentToken::UnixSeconds, tag("X")),
        value(MomentToken::WeekOfYear, tag("ww")),
        value(MomentToken::WeekOfYear, tag("WW")),
        value(MomentToken::Quarter, tag("Q")),
    ))
    .parse(input)
}

/// Parse escaped text within square brackets [literal text].
fn parse_escaped(input: &str) -> IResult<&str, MomentToken> {
    let (input, _) = tag("[").parse(input)?;
    let (input, content) = take_while1(|c| c != ']').parse(input)?;
    let (input, _) = tag("]").parse(input)?;
    Ok((input, MomentToken::Literal(content.to_string())))
}

/// Parse a literal character that isn't a format specifier.
fn parse_literal(input: &str) -> IResult<&str, MomentToken> {
    // Match any single character that isn't a format specifier start
    let (rest, c) = nom::character::complete::anychar.parse(input)?;
    Ok((rest, MomentToken::Literal(c.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_date_format() {
        let result = to_chrono_format("YYYY-MM-DD").unwrap();
        assert_eq!(result, "%Y-%m-%d");
    }

    #[test]
    fn parse_datetime_format() {
        let result = to_chrono_format("YYYY-MM-DD HH:mm:ss").unwrap();
        assert_eq!(result, "%Y-%m-%d %H:%M:%S");
    }

    #[test]
    fn parse_12hour_format() {
        let result = to_chrono_format("hh:mm A").unwrap();
        assert_eq!(result, "%I:%M %p");
    }

    #[test]
    fn parse_full_month_day() {
        let result = to_chrono_format("MMMM D, YYYY").unwrap();
        assert_eq!(result, "%B %-d, %Y");
    }

    #[test]
    fn parse_weekday_format() {
        let result = to_chrono_format("dddd, MMMM D").unwrap();
        assert_eq!(result, "%A, %B %-d");
    }

    #[test]
    fn parse_with_escaped_text() {
        let result = to_chrono_format("[Date: ]YYYY-MM-DD").unwrap();
        assert_eq!(result, "Date: %Y-%m-%d");
    }

    #[test]
    fn parse_iso_format() {
        let result = to_chrono_format("YYYY-MM-DDTHH:mm:ss").unwrap();
        assert_eq!(result, "%Y-%m-%dT%H:%M:%S");
    }

    #[test]
    fn parse_with_milliseconds() {
        let result = to_chrono_format("HH:mm:ss.SSS").unwrap();
        assert_eq!(result, "%H:%M:%S.%3f");
    }
}
