//! Runtime value representation for Bases expressions.
//!
//! Values support conversions, comparisons, arithmetic, and helpers used
//! throughout later evaluation stages. Both integer and floating-point numbers
//! are represented explicitly to preserve precision where possible.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::path::PathBuf;

use chrono::Duration;

mod date;
mod fields;
mod file;
mod list;
mod moment_format;
mod number;
mod string;

pub use date::*;
pub use fields::*;
pub use file::*;
pub use list::*;
pub use number::*;
pub use string::*;

/// Public duration alias used by value consumers.
pub type ValueDuration = Duration;

/// Result type used for value operations.
pub type ValueResult<T> = Result<T, ValueError>;

/// Runtime value produced by evaluating Bases expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    String(StringValue),
    Number(NumberValue),
    Boolean(bool),
    DateTime(DateValue),
    Duration(ValueDuration),
    List(ListValue),
    Object(HashMap<String, Value>),
    File(FileValue),
    Link(LinkValue),
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::String(text) => write!(f, "{text}"),
            // NOTE: We might want to format infinity differently in the future than its current
            // `inf` representation.
            Value::Number(number) => write!(f, "{}", number.value),
            Value::Boolean(value) => write!(f, "{value}"),
            Value::DateTime(datetime) => write!(f, "{}", datetime.value),
            Value::Duration(duration) => write!(f, "{duration}"),
            Value::List(items) => {
                let rendered: Vec<String> = items.iter().map(|item| item.to_string()).collect();
                write!(f, "[{}]", rendered.join(", "))
            }
            Value::Object(entries) => {
                let mut rendered: Vec<String> = entries
                    .iter()
                    .map(|(key, value)| format!("{key}: {}", value))
                    .collect();
                rendered.sort();
                write!(f, "{{{}}}", rendered.join(", "))
            }
            Value::File(file) => file.value.path.display().fmt(f),
            Value::Link(link) => write!(f, "{link}"),
        }
    }
}

impl Value {
    /// Returns a static type name used in diagnostics.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::String(_) => "string",
            Value::Number(_) => "number",
            Value::Boolean(_) => "boolean",
            Value::DateTime(_) => "datetime",
            Value::Duration(_) => "duration",
            Value::List(_) => "list",
            Value::Object(_) => "object",
            Value::File(_) => "file",
            Value::Link(_) => "link",
        }
    }

    /// Returns whether the value is treated as truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Boolean(value) => *value,
            Value::Number(number) => !number.value.is_nan() && number.value != 0.0,
            Value::String(text) => !text.is_empty(),
            Value::DateTime(_) => true,
            Value::Duration(duration) => !duration.is_zero(),
            Value::List(items) => !items.value.is_empty(),
            Value::Object(entries) => !entries.is_empty(),
            Value::File(_) => true,
            Value::Link(_) => true,
        }
    }

    /// Returns whether the value should be considered empty.
    pub fn is_empty(&self) -> bool {
        match self {
            Value::Null => true,
            Value::String(text) => text.is_empty(),
            Value::List(items) => items.value.is_empty(),
            Value::Object(entries) => entries.is_empty(),
            Value::Duration(duration) => duration.is_zero(),
            Value::Number(number) => number.value.abs() <= f64::EPSILON,
            _ => false,
        }
    }

    /// Returns this value coerced into a boolean.
    pub fn to_boolean(&self) -> bool {
        self.is_truthy()
    }

    /// Performs a value comparison, returning the ordering if the comparison is valid.
    pub fn compare(&self, other: &Value) -> ValueResult<Ordering> {
        match (self, other) {
            (Value::Null, Value::Null) => Ok(Ordering::Equal),
            (Value::Number(a), Value::Number(b)) => {
                a.value
                    .partial_cmp(&b.value)
                    .ok_or(ValueError::InvalidComparison {
                        left: self.type_name(),
                        right: self.type_name(),
                    })
            }
            (Value::String(a), Value::String(b)) => Ok(a.cmp(b)),
            (Value::Boolean(a), Value::Boolean(b)) => Ok(a.cmp(b)),
            (Value::DateTime(a), Value::DateTime(b)) => Ok(a.value.cmp(&b.value)),
            (Value::Duration(a), Value::Duration(b)) => Ok(a.cmp(b)),
            _ => Err(ValueError::Type(TypeError::InvalidOperation {
                op: "compare",
                left: self.type_name(),
                right: other.type_name(),
            })),
        }
    }

    /// Returns whether two values are equal, recursively comparing nested values.
    pub fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                if a.value.is_nan() && b.value.is_nan() {
                    true
                } else {
                    a.value == b.value
                }
            }
            (Value::DateTime(a), Value::DateTime(b)) => a == b,
            (Value::Duration(a), Value::Duration(b)) => a == b,
            (Value::List(a), Value::List(b)) => a.value == b.value,
            (Value::Object(a), Value::Object(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| value.equals(other)))
            }
            (Value::File(a), Value::File(b)) => a == b,
            (Value::Link(a), Value::Link(b)) => a == b,
            _ => self == other,
        }
    }

    /// Adds two values together, performing type-specific logic.
    pub fn add(&self, other: &Value) -> ValueResult<Value> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => {
                Ok(Value::String(StringValue::new(format!("{a}{b}"))))
            }
            (Value::DateTime(datetime), Value::Duration(duration))
            | (Value::Duration(duration), Value::DateTime(datetime)) => {
                Ok(Value::DateTime(DateValue::new(*datetime.value + *duration)))
            }
            (Value::Duration(a), Value::Duration(b)) => Ok(Value::Duration(*a + *b)),
            _ => {
                if let Some((lhs, rhs)) = numeric_pair(self, other) {
                    Ok(Value::Number(NumberValue::new(lhs + rhs)))
                } else {
                    Err(ValueError::Type(TypeError::InvalidOperation {
                        op: "add",
                        left: self.type_name(),
                        right: other.type_name(),
                    }))
                }
            }
        }
    }

    /// Subtracts one value from another.
    pub fn sub(&self, other: &Value) -> ValueResult<Value> {
        match (self, other) {
            (Value::DateTime(date), Value::Duration(duration)) => {
                match date.value.checked_sub_signed(*duration) {
                    Some(result) => Ok(Value::DateTime(DateValue::new(result))),
                    None => Err(ValueError::Message(
                        "resulting date is out of range".to_string(),
                    )),
                }
            }
            (Value::DateTime(a), Value::DateTime(b)) => {
                Ok(Value::Duration(a.value.signed_duration_since(*b.value)))
            }
            (Value::Duration(a), Value::Duration(b)) => Ok(Value::Duration(*a - *b)),
            _ => {
                if let Some((lhs, rhs)) = numeric_pair(self, other) {
                    Ok(Value::Number(NumberValue::new(lhs - rhs)))
                } else {
                    Err(ValueError::Type(TypeError::InvalidOperation {
                        op: "sub",
                        left: self.type_name(),
                        right: other.type_name(),
                    }))
                }
            }
        }
    }

    /// Multiplies values together.
    pub fn mul(&self, other: &Value) -> ValueResult<Value> {
        if let Some((lhs, rhs)) = numeric_pair(self, other) {
            Ok(Value::Number(NumberValue::new(lhs * rhs)))
        } else {
            Err(ValueError::Type(TypeError::InvalidOperation {
                op: "mul",
                left: self.type_name(),
                right: other.type_name(),
            }))
        }
    }

    /// Divides one value by another.
    pub fn div(&self, other: &Value) -> ValueResult<Value> {
        if let Some((lhs, rhs)) = numeric_pair(self, other) {
            if rhs == 0.0 {
                Err(ValueError::Type(TypeError::InvalidOperation {
                    op: "div",
                    left: self.type_name(),
                    right: other.type_name(),
                }))
            } else {
                Ok(Value::Number(NumberValue::new(lhs / rhs)))
            }
        } else {
            Err(ValueError::Type(TypeError::InvalidOperation {
                op: "div",
                left: self.type_name(),
                right: other.type_name(),
            }))
        }
    }

    /// Computes the remainder of dividing two values.
    pub fn rem(&self, other: &Value) -> ValueResult<Value> {
        if let Some((lhs, rhs)) = numeric_pair(self, other) {
            if rhs == 0.0 {
                Err(ValueError::Type(TypeError::InvalidOperation {
                    op: "mod",
                    left: self.type_name(),
                    right: other.type_name(),
                }))
            } else {
                Ok(Value::Number(NumberValue::new(lhs % rhs)))
            }
        } else {
            Err(ValueError::Type(TypeError::InvalidOperation {
                op: "mod",
                left: self.type_name(),
                right: other.type_name(),
            }))
        }
    }

    /// Negates numeric or duration values.
    pub fn negate(&self) -> ValueResult<Value> {
        match self {
            Value::Number(value) => Ok(Value::Number(NumberValue::new(-value.value))),
            Value::Duration(duration) => Ok(Value::Duration(-*duration)),
            _ => Err(ValueError::Type(TypeError::InvalidUnary {
                op: "neg",
                operand: self.type_name(),
            })),
        }
    }

    /// Logical negation of the truthiness of a value.
    pub fn not(&self) -> ValueResult<Value> {
        match self {
            Value::Boolean(value) => Ok(Value::Boolean(!value)),
            _ => Ok(Value::Boolean(!self.is_truthy())),
        }
    }

    /// Returns the length of the value if applicable (string, list, object).
    pub fn len(&self) -> ValueResult<usize> {
        match self {
            Value::String(text) => Ok(text.chars().count()),
            Value::List(items) => Ok(items.value.len()),
            Value::Object(entries) => Ok(entries.len()),
            _ => Err(ValueError::Type(TypeError::InvalidUnary {
                op: "len",
                operand: self.type_name(),
            })),
        }
    }

    /// Returns whether the list or string contains a value.
    pub fn contains(&self, needle: &Value) -> ValueResult<bool> {
        match self {
            Value::List(items) => Ok(items.iter().any(|item| item.equals(needle))),
            Value::String(text) => Ok(match needle {
                Value::String(sub) => text.value.contains(sub.value.as_ref()),
                _ => false,
            }),
            _ => Err(ValueError::Type(TypeError::InvalidOperation {
                op: "contains",
                left: self.type_name(),
                right: needle.type_name(),
            })),
        }
    }
}

/// Metadata for a link value (either wiki link or URL).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkValue {
    pub target: PathBuf,
    pub display: Option<String>,
}

impl LinkValue {
    /// Creates a new link value.
    pub fn new(target: impl Into<PathBuf>, display: Option<String>) -> Self {
        Self {
            target: target.into(),
            display,
        }
    }
}

impl Display for LinkValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.target.display())?;
        if let Some(display) = &self.display {
            write!(f, "|{display}")?;
        }
        Ok(())
    }
}

/// Errors produced by value operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueError {
    Type(TypeError),
    InvalidConversion {
        from: &'static str,
        to: &'static str,
    },
    InvalidComparison {
        left: &'static str,
        right: &'static str,
    },
    Message(String),
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueError::Type(err) => write!(f, "{err}"),
            ValueError::InvalidConversion { from, to } => {
                write!(f, "cannot convert {from} to {to}")
            }
            ValueError::InvalidComparison { left, right } => {
                write!(f, "cannot compare {left} with {right}")
            }
            ValueError::Message(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ValueError {}

impl From<TypeError> for ValueError {
    fn from(value: TypeError) -> Self {
        ValueError::Type(value)
    }
}

/// Type-level errors describing invalid operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    InvalidOperation {
        op: &'static str,
        left: &'static str,
        right: &'static str,
    },
    InvalidUnary {
        op: &'static str,
        operand: &'static str,
    },
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::InvalidOperation { op, left, right } => {
                write!(
                    f,
                    "operation '{op}' is not supported for {left} and {right}"
                )
            }
            TypeError::InvalidUnary { op, operand } => {
                write!(f, "operation '{op}' is not supported for {operand}")
            }
        }
    }
}

impl std::error::Error for TypeError {}

// Convenience From implementations for Value
impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Number(NumberValue::new(value))
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Number(NumberValue::new(value as f64))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(StringValue::new(value))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(StringValue::new(value.to_string()))
    }
}

fn numeric_pair(left: &Value, right: &Value) -> Option<(f64, f64)> {
    match (left, right) {
        (Value::Number(lhs), Value::Number(rhs)) => Some((lhs.value, rhs.value)),
        _ => None,
    }
}
