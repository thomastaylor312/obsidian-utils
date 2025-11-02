//! Error types for Bases expression parsing.

use std::fmt;

use nom::error::{ErrorKind, ParseError};

/// A parse error with user-friendly context and error messages.
///
/// This error type provides more helpful information than the default nom errors,
/// including the position of the error and a description of what went wrong.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseErrorInfo<I> {
    /// The input position where the error occurred
    pub input: I,
    /// A description of what went wrong
    pub message: String,
    /// The kind of error (from nom)
    pub kind: ErrorKind,
}

impl<I> ParseErrorInfo<I> {
    /// Create a new parse error with a custom message.
    pub fn new(input: I, message: impl Into<String>, kind: ErrorKind) -> Self {
        Self {
            input,
            message: message.into(),
            kind,
        }
    }

    /// Create a parse error from a nom ErrorKind with a default message.
    pub fn from_kind(input: I, kind: ErrorKind) -> Self {
        let message = match kind {
            ErrorKind::Digit => "expected a number".to_string(),
            ErrorKind::Alpha => "expected an identifier or keyword".to_string(),
            ErrorKind::Tag => "unexpected token".to_string(),
            ErrorKind::Char => "unexpected character".to_string(),
            ErrorKind::NonEmpty => "unexpected trailing input".to_string(),
            ErrorKind::Eof => "unexpected end of input".to_string(),
            _ => format!("parse error: {:?}", kind),
        };

        Self {
            input,
            message,
            kind,
        }
    }
}

impl<I: fmt::Display> fmt::Display for ParseErrorInfo<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at: {}", self.message, self.input)
    }
}

impl<I: fmt::Display + fmt::Debug> std::error::Error for ParseErrorInfo<I> {}

impl<I> ParseError<I> for ParseErrorInfo<I> {
    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        Self::from_kind(input, kind)
    }

    fn append(_input: I, _kind: ErrorKind, other: Self) -> Self {
        // For simplicity this first go around, we just keep the first error. We can come back and
        // make this more sophisticated later if needed.
        other
    }
}

/// Helper function to create a parse error with a custom message.
pub fn parse_error<I>(input: I, message: impl Into<String>) -> ParseErrorInfo<I> {
    ParseErrorInfo::new(input, message, ErrorKind::Fail)
}

/// Helper function to create a parse error from an ErrorKind.
pub fn make_parse_error<I>(input: I, kind: ErrorKind) -> ParseErrorInfo<I> {
    ParseErrorInfo::from_kind(input, kind)
}
