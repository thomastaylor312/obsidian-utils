use std::{collections::HashMap, path::PathBuf};

use chrono::{Duration, Local, NaiveDate, NaiveDateTime};
use nom::{
    Parser,
    branch::alt,
    character::complete::{alpha1, char, digit1, multispace0},
    combinator::{map_res, opt, recognize},
    multi::many1,
    sequence::preceded,
};
use thiserror::Error;

use crate::{
    LinkValue, Value,
    value::{DateValue, NumberValue},
};

#[derive(Debug, Error)]
pub enum FunctionError {
    /// The number of arguments to the function is incorrect
    #[error("incorrect number of arguments, expected {expected}, got {found}")]
    IncorrectArgumentCount { expected: usize, found: usize },
    /// The type of an argument is incorrect
    #[error(
        "incorrect argument type, argument at index {index} is type {found_type}, expected {expected_type}"
    )]
    IncorrectArgumentType {
        index: usize,
        found_type: String,
        expected_type: String,
    },
    /// The function name does not exist
    #[error("function {0} does not exist")]
    DoesNotExist(String),
    /// The error returned from a function. This means the function itself failed and not the
    /// function registry.
    #[error(transparent)]
    CallError(#[from] anyhow::Error),
}

/// A convenience wrapper for a result returned from a function
pub type FunctionResult = Result<Value, FunctionError>;

/// A type alias for a boxed function
pub type Function = Box<dyn for<'a> Fn(&'a [Value]) -> FunctionResult>;

#[derive(Default)]
pub struct FunctionRegistry {
    functions: HashMap<String, Function>,
}

impl FunctionRegistry {
    /// Creates a new, empty function registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a function registry with the global functions registered by default.
    pub fn global() -> Self {
        let mut registry = Self::new();

        // Register global functions
        registry.register("if", if_fn);
        registry.register("now", now_fn);
        registry.register("today", today_fn);
        registry.register("duration", duration_fn);
        registry.register("list", list_fn);
        registry.register("number", number_fn);
        registry.register("link", link_fn);
        registry.register("date", date_fn);
        registry.register("min", min_fn);
        registry.register("max", max_fn);
        registry
    }

    /// Call the given function name with the args
    pub fn call(&self, name: &str, args: &[Value]) -> FunctionResult {
        let f = self
            .functions
            .get(name)
            .ok_or_else(|| FunctionError::DoesNotExist(name.to_string()))?;
        f(args)
    }

    /// Register a function with the given name
    pub fn register<F>(&mut self, name: &'static str, function: F)
    where
        F: for<'a> Fn(&'a [Value]) -> FunctionResult + 'static,
    {
        self.functions.insert(name.to_string(), Box::new(function));
    }
}

// NOTE: For this function, we're going to assume all things are valid values for now, but with this
// function in particular, we might want to lazy evaluate so we aren't evaluating something like
// might not exist. So if the `if` function is checking if something is null, we probably don't want
// to evaluate it.
fn if_fn(args: &[Value]) -> FunctionResult {
    if args.len() > 3 {
        return Err(FunctionError::IncorrectArgumentCount {
            expected: 2,
            found: args.len(),
        });
    }
    let mut iter = args.iter();
    let bool_arg = match iter.next() {
        Some(Value::Boolean(b)) => *b,
        Some(val) => {
            return Err(FunctionError::IncorrectArgumentType {
                index: 1,
                found_type: val.type_name().to_string(),
                expected_type: "boolean".to_string(),
            });
        }
        _ => {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 2,
                found: 0,
            });
        }
    };
    let arg = iter
        .next()
        .ok_or_else(|| FunctionError::IncorrectArgumentCount {
            expected: 2,
            found: args.len(),
        })?;
    if bool_arg {
        Ok(arg.to_owned())
    } else {
        Ok(iter.next().cloned().unwrap_or_else(|| Value::Null))
    }
}

fn now_fn(args: &[Value]) -> FunctionResult {
    if !args.is_empty() {
        return Err(FunctionError::IncorrectArgumentCount {
            expected: 0,
            found: args.len(),
        });
    }
    Ok(Value::DateTime(DateValue::now()))
}

fn today_fn(args: &[Value]) -> FunctionResult {
    if !args.is_empty() {
        return Err(FunctionError::IncorrectArgumentCount {
            expected: 0,
            found: args.len(),
        });
    }
    Ok(Value::DateTime(DateValue::new(
        Local::now().date_naive().into(),
    )))
}

fn duration_fn(args: &[Value]) -> FunctionResult {
    let mut iter = args.iter();
    let duration_str = match iter.next() {
        Some(Value::String(s)) => s,
        Some(val) => {
            return Err(FunctionError::IncorrectArgumentType {
                index: 1,
                found_type: val.type_name().to_string(),
                expected_type: "string".to_string(),
            });
        }
        _ => {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: 0,
            });
        }
    };
    if iter.next().is_some() {
        return Err(FunctionError::IncorrectArgumentCount {
            expected: 1,
            found: args.len(),
        });
    }

    let parsed = parse_duration(&duration_str.value)?;
    Ok(Value::Duration(parsed))
}

/// Parse a duration string like "1d", "2h30m", "1 week", etc.
/// Supported units:
/// - y, year, years
/// - M, month, months (30 days)
/// - w, week, weeks
/// - d, day, days
/// - h, hour, hours
/// - m, minute, minutes
/// - s, second, seconds
pub fn parse_duration(s: &str) -> Result<Duration, FunctionError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(FunctionError::CallError(anyhow::anyhow!(
            "Empty duration string"
        )));
    }

    let components = parse_duration_components(s).map_err(|e| {
        FunctionError::CallError(anyhow::anyhow!("Failed to parse duration '{}': {}", s, e))
    })?;

    let mut total = Duration::zero();
    for (num, unit) in components {
        let duration = unit_to_duration(num, &unit)?;
        total += duration;
    }

    Ok(total)
}

/// Parse a floating point number (e.g., "1", "2.5", ".5")
fn parse_float(input: &str) -> nom::IResult<&str, f64> {
    map_res(
        recognize((
            opt(char('-')),
            alt((
                // Integer with optional decimal part: "123" or "123.456"
                recognize((digit1, opt((char('.'), opt(digit1))))),
                // Decimal only: ".456"
                recognize((char('.'), digit1)),
            )),
        )),
        |s: &str| s.parse::<f64>(),
    )
    .parse(input)
}

/// Parse a single duration component: number followed by optional whitespace and unit
fn parse_duration_component(input: &str) -> nom::IResult<&str, (f64, String)> {
    (
        preceded(multispace0, parse_float),
        preceded(multispace0, alpha1),
    )
        .map(|(num, unit): (f64, &str)| (num, unit.to_string()))
        .parse(input)
}

/// Parse all duration components from the input string
fn parse_duration_components(input: &str) -> Result<Vec<(f64, String)>, String> {
    let result = many1(parse_duration_component).parse(input);

    match result {
        Ok((remaining, components)) => {
            // Check that we consumed all input (except trailing whitespace)
            let remaining = remaining.trim();
            if !remaining.is_empty() {
                return Err(format!("unexpected text: '{}'", remaining));
            }
            Ok(components)
        }
        Err(e) => Err(format!("parse error: {}", e)),
    }
}

/// Convert a numeric value and unit string to a Duration
fn unit_to_duration(num: f64, unit: &str) -> Result<Duration, FunctionError> {
    match unit {
        "y" | "year" | "years" => Ok(Duration::days((num * 365.0) as i64)),
        "M" | "month" | "months" => Ok(Duration::days((num * 30.0) as i64)),
        "w" | "week" | "weeks" => Ok(Duration::weeks(num as i64)),
        "d" | "day" | "days" => Ok(Duration::days(num as i64)),
        "h" | "hour" | "hours" => Ok(Duration::hours(num as i64)),
        "m" | "minute" | "minutes" => Ok(Duration::minutes(num as i64)),
        "s" | "second" | "seconds" => Ok(Duration::seconds(num as i64)),
        other => Err(FunctionError::CallError(anyhow::anyhow!(
            "Unknown duration unit: {}",
            other
        ))),
    }
}

// parses the provided string and returns a date object. By definition of the function, the date
// string should be in the format YYYY-MM-DD HH:mm:ss. For flexibility we support parsing from
// ISO8601 format as well.
fn date_fn(args: &[Value]) -> FunctionResult {
    let mut iter = args.iter();
    let date_str = match iter.next() {
        Some(Value::String(s)) => s,
        Some(val) => {
            return Err(FunctionError::IncorrectArgumentType {
                index: 1,
                found_type: val.type_name().to_string(),
                expected_type: "string".to_string(),
            });
        }
        _ => {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: 0,
            });
        }
    };
    if iter.next().is_some() {
        return Err(FunctionError::IncorrectArgumentCount {
            expected: 1,
            found: args.len(),
        });
    }

    // Try parsing various formats in order of specificity
    let parsed = parse_datetime(&date_str.value)?;
    Ok(Value::DateTime(DateValue::new(parsed)))
}

/// Parse a datetime string in various formats.
/// Supported formats:
/// - YYYY-MM-DD HH:mm:ss (spec format)
/// - YYYY-MM-DD HH:mm
/// - YYYY-MM-DD
/// - ISO8601 formats (with T separator)
fn parse_datetime(s: &str) -> Result<NaiveDateTime, FunctionError> {
    // Try YYYY-MM-DD HH:mm:ss
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(dt);
    }

    // Try YYYY-MM-DD HH:mm
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
        return Ok(dt);
    }

    // Try ISO8601 with T separator
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt);
    }

    // Try ISO8601 with T separator and optional milliseconds
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok(dt);
    }

    // Try date only
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).expect("valid time"));
    }

    Err(FunctionError::CallError(anyhow::anyhow!(
        "Could not parse '{}' as a date. Expected format: YYYY-MM-DD HH:mm:ss",
        s
    )))
}

fn list_fn(args: &[Value]) -> FunctionResult {
    if args.len() != 1 {
        return Err(FunctionError::IncorrectArgumentCount {
            expected: 1,
            found: args.len(),
        });
    }
    let item = args.first().expect("Should have one item");
    if matches!(item, Value::List(_)) {
        return Ok(item.clone());
    }
    Ok(Value::List(vec![item.clone()].into()))
}

fn number_fn(args: &[Value]) -> FunctionResult {
    if args.len() != 1 {
        return Err(FunctionError::IncorrectArgumentCount {
            expected: 1,
            found: args.len(),
        });
    }
    let item = args.first().expect("Should have one item");
    let num = match item {
        Value::Number(_) => return Ok(item.clone()),
        Value::String(s) => match s.parse::<f64>() {
            Ok(n) => n,
            Err(e) => {
                return Err(FunctionError::CallError(anyhow::anyhow!(
                    "Could not parse string '{}' as a number: {e}",
                    s.value
                )));
            }
        },
        Value::Null => {
            return Err(FunctionError::IncorrectArgumentType {
                index: 1,
                found_type: item.type_name().to_string(),
                expected_type: "non-null".to_string(),
            });
        }
        Value::Boolean(true) => 1.0,
        Value::Boolean(false) => 0.0,
        Value::DateTime(val) => val.value.and_utc().timestamp_millis() as f64,
        Value::Duration(d) => d.num_milliseconds() as f64,
        Value::List(_) | Value::Object(_) | Value::File(_) | Value::Link(_) => {
            return Err(FunctionError::IncorrectArgumentType {
                index: 1,
                found_type: item.type_name().to_string(),
                expected_type: "number".to_string(),
            });
        }
    };
    Ok(Value::Number(NumberValue::new(num)))
}

fn link_fn(args: &[Value]) -> FunctionResult {
    if args.len() > 2 {
        return Err(FunctionError::IncorrectArgumentCount {
            expected: 2,
            found: args.len(),
        });
    }
    let mut iter = args.iter();
    let path = match iter.next() {
        Some(Value::String(val)) => PathBuf::from(val.value.as_str()),
        Some(Value::File(val)) => val.value.path.clone(),
        Some(val) => {
            return Err(FunctionError::IncorrectArgumentType {
                index: 1,
                found_type: val.type_name().to_string(),
                expected_type: "string or file".to_string(),
            });
        }
        None => {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: args.len(),
            });
        }
    };
    let display = match iter.next() {
        Some(Value::String(val)) => Some(val.value.as_ref().clone()),
        Some(val) => {
            return Err(FunctionError::IncorrectArgumentType {
                index: 2,
                found_type: val.type_name().to_string(),
                expected_type: "string".to_string(),
            });
        }
        None => None,
    };
    Ok(Value::Link(LinkValue {
        target: path,
        display,
    }))
}

/// Returns the smallest of all provided numbers.
fn min_fn(args: &[Value]) -> FunctionResult {
    if args.is_empty() {
        return Err(FunctionError::IncorrectArgumentCount {
            expected: 1,
            found: 0,
        });
    }

    let mut min_val: Option<f64> = None;

    for (idx, arg) in args.iter().enumerate() {
        let num = match arg {
            Value::Number(n) => n.value,
            other => {
                return Err(FunctionError::IncorrectArgumentType {
                    index: idx,
                    found_type: other.type_name().to_string(),
                    expected_type: "number".to_string(),
                });
            }
        };

        min_val = Some(match min_val {
            Some(current) => current.min(num),
            None => num,
        });
    }

    Ok(Value::Number(NumberValue::new(
        min_val.expect("at least one value"),
    )))
}

/// Returns the largest of all provided numbers.
fn max_fn(args: &[Value]) -> FunctionResult {
    if args.is_empty() {
        return Err(FunctionError::IncorrectArgumentCount {
            expected: 1,
            found: 0,
        });
    }

    let mut max_val: Option<f64> = None;

    for (idx, arg) in args.iter().enumerate() {
        let num = match arg {
            Value::Number(n) => n.value,
            other => {
                return Err(FunctionError::IncorrectArgumentType {
                    index: idx,
                    found_type: other.type_name().to_string(),
                    expected_type: "number".to_string(),
                });
            }
        };

        max_val = Some(match max_val {
            Some(current) => current.max(num),
            None => num,
        });
    }

    Ok(Value::Number(NumberValue::new(
        max_val.expect("at least one value"),
    )))
}
