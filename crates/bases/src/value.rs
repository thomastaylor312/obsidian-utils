//! Runtime value representation for Bases expressions.
//!
//! Values support conversions, comparisons, arithmetic, and helpers used
//! throughout later evaluation stages. Both integer and floating-point numbers
//! are represented explicitly to preserve precision where possible.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::path::{Path, PathBuf};

use chrono::{DateTime as ChronoDateTime, Duration as ChronoDuration, NaiveDate, Utc};

/// Public date alias used by value consumers.
pub type ValueDate = NaiveDate;
/// Public datetime alias used by value consumers.
pub type ValueDateTime = ChronoDateTime<Utc>;
/// Public duration alias used by value consumers.
pub type ValueDuration = ChronoDuration;

/// Result type used for value operations.
pub type ValueResult<T> = Result<T, ValueError>;

/// Runtime value produced by evaluating Bases expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Date(ValueDate),
    DateTime(ValueDateTime),
    Duration(ValueDuration),
    List(Vec<Value>),
    Object(HashMap<String, Value>),
    File(FileValue),
    Link(LinkValue),
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::String(text) => write!(f, "{text}"),
            Value::Integer(value) => write!(f, "{value}"),
            // NOTE: We might want to format infinity differently in the future than its current
            // `inf` representation.
            Value::Float(number) => write!(f, "{number}"),
            Value::Boolean(value) => write!(f, "{value}"),
            Value::Date(date) => write!(f, "{}", date.format("%Y-%m-%d")),
            Value::DateTime(datetime) => write!(f, "{}", datetime.to_rfc3339()),
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
            Value::File(file) => file.path.display().fmt(f),
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
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::Boolean(_) => "boolean",
            Value::Date(_) => "date",
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
            Value::Integer(value) => *value != 0,
            Value::Float(number) => !number.is_nan() && *number != 0.0,
            Value::String(text) => !text.is_empty(),
            Value::Date(_) | Value::DateTime(_) => true,
            Value::Duration(duration) => !duration.is_zero(),
            Value::List(items) => !items.is_empty(),
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
            Value::List(items) => items.is_empty(),
            Value::Object(entries) => entries.is_empty(),
            Value::Duration(duration) => duration.is_zero(),
            Value::Integer(value) => *value == 0,
            Value::Float(number) => number.abs() <= f64::EPSILON,
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
            (Value::Integer(a), Value::Integer(b)) => Ok(a.cmp(b)),
            (Value::Float(a), Value::Float(b)) => {
                a.partial_cmp(b).ok_or(ValueError::InvalidComparison {
                    left: "float",
                    right: "float",
                })
            }
            (Value::Integer(_), Value::Float(_)) | (Value::Float(_), Value::Integer(_)) => {
                let lhs = NumericValue::from_value(self).unwrap();
                let rhs = NumericValue::from_value(other).unwrap();
                lhs.partial_cmp(rhs)
            }
            (Value::String(a), Value::String(b)) => Ok(a.cmp(b)),
            (Value::Boolean(a), Value::Boolean(b)) => Ok(a.cmp(b)),
            (Value::Date(a), Value::Date(b)) => Ok(a.cmp(b)),
            (Value::DateTime(a), Value::DateTime(b)) => Ok(a.cmp(b)),
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
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => {
                if a.is_nan() && b.is_nan() {
                    true
                } else {
                    a == b
                }
            }
            (Value::Integer(_), Value::Float(_)) | (Value::Float(_), Value::Integer(_)) => {
                let lhs = NumericValue::from_value(self).unwrap();
                let rhs = NumericValue::from_value(other).unwrap();
                lhs.equals(rhs)
            }
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::DateTime(a), Value::DateTime(b)) => a == b,
            (Value::Duration(a), Value::Duration(b)) => a == b,
            (Value::List(a), Value::List(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(lhs, rhs)| lhs.equals(rhs))
            }
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
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),
            (Value::Date(date), Value::Duration(duration)) => {
                match date.checked_add_signed(*duration) {
                    Some(result) => Ok(Value::Date(result)),
                    None => Err(ValueError::Message(
                        "resulting date is out of range".to_string(),
                    )),
                }
            }
            (Value::DateTime(datetime), Value::Duration(duration)) => {
                Ok(Value::DateTime(*datetime + *duration))
            }
            (Value::Duration(duration), Value::Date(date)) => {
                Value::Date(*date).add(&Value::Duration(*duration))
            }
            (Value::Duration(duration), Value::DateTime(datetime)) => {
                Value::DateTime(*datetime).add(&Value::Duration(*duration))
            }
            (Value::Duration(a), Value::Duration(b)) => Ok(Value::Duration(*a + *b)),
            _ => {
                if let Some((lhs, rhs)) = numeric_pair(self, other) {
                    Ok(lhs.add(rhs))
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
            (Value::Date(date), Value::Duration(duration)) => {
                match date.checked_sub_signed(*duration) {
                    Some(result) => Ok(Value::Date(result)),
                    None => Err(ValueError::Message(
                        "resulting date is out of range".to_string(),
                    )),
                }
            }
            (Value::Date(a), Value::Date(b)) => Ok(Value::Duration(a.signed_duration_since(*b))),
            (Value::DateTime(datetime), Value::Duration(duration)) => {
                Ok(Value::DateTime(*datetime - *duration))
            }
            (Value::DateTime(a), Value::DateTime(b)) => {
                Ok(Value::Duration(a.signed_duration_since(*b)))
            }
            (Value::Duration(a), Value::Duration(b)) => Ok(Value::Duration(*a - *b)),
            _ => {
                if let Some((lhs, rhs)) = numeric_pair(self, other) {
                    Ok(lhs.sub(rhs))
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
            Ok(lhs.mul(rhs))
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
            lhs.div(rhs)
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
            lhs.rem(rhs)
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
            Value::Integer(value) => match value.checked_neg() {
                Some(negated) => Ok(Value::Integer(negated)),
                None => Ok(Value::Float(-(*value as f64))),
            },
            Value::Float(value) => Ok(Value::Float(-value)),
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
            Value::List(items) => Ok(items.len()),
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
                Value::String(sub) => text.contains(sub),
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

/// Metadata for a file value.
#[derive(Debug, Clone)]
pub struct FileValue {
    pub path: PathBuf,
    pub metadata: std::fs::Metadata,
    // TODO: Possibly add tags and links here
}

impl PartialEq for FileValue {
    fn eq(&self, other: &Self) -> bool {
        // For now we're only comparing the path, since the metadata is not guaranteed to be the same.
        // For purposes of what we're doing here, each file path should be unique within the value,
        // so this is the only comparison we care about
        self.path == other.path
    }
}

impl FileValue {
    /// Creates a file value from a path-like value.
    pub fn new(path: impl AsRef<Path>, metadata: std::fs::Metadata) -> Self {
        Self {
            path: path.as_ref().into(),
            metadata,
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

fn numeric_pair(left: &Value, right: &Value) -> Option<(NumericValue, NumericValue)> {
    Some((
        NumericValue::from_value(left)?,
        NumericValue::from_value(right)?,
    ))
}

#[derive(Debug, Clone, Copy)]
enum NumericValue {
    Integer(i64),
    Float(f64),
}

impl NumericValue {
    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Integer(v) => Some(NumericValue::Integer(*v)),
            Value::Float(v) => Some(NumericValue::Float(*v)),
            _ => None,
        }
    }

    fn as_f64(self) -> f64 {
        match self {
            NumericValue::Integer(v) => v as f64,
            NumericValue::Float(v) => v,
        }
    }

    fn add(self, other: NumericValue) -> Value {
        match (self, other) {
            (NumericValue::Integer(a), NumericValue::Integer(b)) => match a.checked_add(b) {
                Some(sum) => Value::Integer(sum),
                None => Value::Float(a as f64 + b as f64),
            },
            _ => Value::Float(self.as_f64() + other.as_f64()),
        }
    }

    fn sub(self, other: NumericValue) -> Value {
        match (self, other) {
            (NumericValue::Integer(a), NumericValue::Integer(b)) => match a.checked_sub(b) {
                Some(diff) => Value::Integer(diff),
                None => Value::Float(a as f64 - b as f64),
            },
            _ => Value::Float(self.as_f64() - other.as_f64()),
        }
    }

    fn mul(self, other: NumericValue) -> Value {
        match (self, other) {
            (NumericValue::Integer(a), NumericValue::Integer(b)) => match a.checked_mul(b) {
                Some(product) => Value::Integer(product),
                None => Value::Float((a as f64) * (b as f64)),
            },
            _ => Value::Float(self.as_f64() * other.as_f64()),
        }
    }

    fn div(self, other: NumericValue) -> ValueResult<Value> {
        if other.is_zero() {
            return Err(ValueError::Type(TypeError::InvalidOperation {
                op: "div",
                left: self.type_name(),
                right: other.type_name(),
            }));
        }

        match (self, other) {
            (NumericValue::Integer(a), NumericValue::Integer(b)) => {
                if a % b == 0 {
                    Ok(Value::Integer(a / b))
                } else {
                    Ok(Value::Float(a as f64 / b as f64))
                }
            }
            _ => Ok(Value::Float(self.as_f64() / other.as_f64())),
        }
    }

    fn rem(self, other: NumericValue) -> ValueResult<Value> {
        if other.is_zero() {
            return Err(ValueError::Type(TypeError::InvalidOperation {
                op: "mod",
                left: self.type_name(),
                right: other.type_name(),
            }));
        }

        match (self, other) {
            (NumericValue::Integer(a), NumericValue::Integer(b)) => Ok(Value::Integer(a % b)),
            _ => Ok(Value::Float(self.as_f64() % other.as_f64())),
        }
    }

    fn partial_cmp(self, other: NumericValue) -> ValueResult<Ordering> {
        match (self, other) {
            (NumericValue::Integer(a), NumericValue::Integer(b)) => Ok(a.cmp(&b)),
            _ => self
                .as_f64()
                .partial_cmp(&other.as_f64())
                .ok_or(ValueError::InvalidComparison {
                    left: self.type_name(),
                    right: other.type_name(),
                }),
        }
    }

    fn equals(self, other: NumericValue) -> bool {
        match (self, other) {
            (NumericValue::Integer(a), NumericValue::Integer(b)) => a == b,
            _ => {
                let lhs = self.as_f64();
                let rhs = other.as_f64();
                if lhs.is_nan() && rhs.is_nan() {
                    true
                } else {
                    lhs == rhs
                }
            }
        }
    }

    fn is_zero(self) -> bool {
        match self {
            NumericValue::Integer(v) => v == 0,
            NumericValue::Float(v) => v == 0.0,
        }
    }

    fn type_name(self) -> &'static str {
        match self {
            NumericValue::Integer(_) => "integer",
            NumericValue::Float(_) => "float",
        }
    }
}
