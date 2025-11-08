use std::{
    borrow::Borrow,
    fmt::{Debug, Display},
    ops::Deref,
    rc::Rc,
};

use crate::{
    Value,
    functions::{Function, FunctionError, FunctionRegistry, FunctionResult},
    value::{FieldGetter, FieldRegistry, ListValue, NumberValue},
};

/// A wrapper around a plain [`String`] that allows functions to be called on it. This type
/// implements [`Deref`], [`Borrow`], and [`AsRef`] to allow it to be used as a plain string.
#[derive(Clone)]
pub struct StringValue {
    pub value: Rc<String>,
    registry: Rc<FunctionRegistry>,
    fields: Rc<FieldRegistry>,
}

impl AsRef<String> for StringValue {
    fn as_ref(&self) -> &String {
        &self.value
    }
}

impl Borrow<String> for StringValue {
    fn borrow(&self) -> &String {
        self.value.borrow()
    }
}

impl Deref for StringValue {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl PartialEq for StringValue {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Debug for StringValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Display for StringValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<String> for StringValue {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for StringValue {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

impl Default for StringValue {
    fn default() -> Self {
        Self::new(String::default())
    }
}

impl StringValue {
    /// Creates a new [`StringValue`] from the given string
    pub fn new(value: String) -> Self {
        let mut registry = FunctionRegistry::default();
        let value = Rc::new(value);
        registry.register("contains", contains_fn(Rc::clone(&value)));
        registry.register("startsWith", starts_with_fn(Rc::clone(&value)));
        registry.register("endsWith", ends_with_fn(Rc::clone(&value)));
        registry.register("lower", lower_fn(Rc::clone(&value)));
        registry.register("upper", upper_fn(Rc::clone(&value)));
        registry.register("trim", trim_fn(Rc::clone(&value)));
        registry.register("split", split_fn(Rc::clone(&value)));
        registry.register("slice", slice_fn(Rc::clone(&value)));
        registry.register("replace", replace_fn(Rc::clone(&value)));
        registry.register("isEmpty", is_empty_fn(Rc::clone(&value)));
        registry.register("containsAll", contains_all_fn(Rc::clone(&value)));
        registry.register("containsAny", contains_any_fn(Rc::clone(&value)));
        let mut fields = FieldRegistry::new();
        fields.register("length", length_getter(Rc::clone(&value)));
        Self {
            value,
            registry: Rc::new(registry),
            fields: Rc::new(fields),
        }
    }

    /// Call a function on the string value.
    pub fn call(&self, name: &str, args: &[Value]) -> FunctionResult {
        self.registry.call(name, args)
    }

    /// Get the value of a field. Returns None if the field doesn't exist
    pub fn field(&self, name: &str) -> Option<Value> {
        self.fields.get(name)
    }
}

fn get_single_string_arg(args: &[Value]) -> Result<&StringValue, FunctionError> {
    match args.first() {
        Some(Value::String(v)) => Ok(v),
        Some(v) => Err(FunctionError::IncorrectArgumentType {
            index: 0,
            found_type: v.type_name().to_string(),
            // TODO: Find a way to not hardcode this. To use `type_name` we'd have to instantiate a
            // new `StringValue` which is not ideal.
            expected_type: "string".to_string(),
        }),
        None => Err(FunctionError::IncorrectArgumentCount {
            expected: 1,
            found: args.len(),
        }),
    }
}

fn contains_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        let val = get_single_string_arg(args)?;
        Ok(Value::Boolean(this.contains(val.value.as_str())))
    })
}

fn length_getter(this: Rc<String>) -> FieldGetter {
    Box::new(move || Value::Number(NumberValue::new(this.len() as f64)))
}

/// `string.startsWith(prefix)` - Returns true if string starts with prefix.
fn starts_with_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        let prefix = get_single_string_arg(args)?;
        Ok(Value::Boolean(this.starts_with(prefix.value.as_str())))
    })
}

/// `string.endsWith(suffix)` - Returns true if string ends with suffix.
fn ends_with_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        let suffix = get_single_string_arg(args)?;
        Ok(Value::Boolean(this.ends_with(suffix.value.as_str())))
    })
}

/// `string.lower()` - Returns string converted to lowercase.
fn lower_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(Value::String(StringValue::new(this.to_lowercase())))
    })
}

/// `string.upper()` - Returns string converted to uppercase.
fn upper_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(Value::String(StringValue::new(this.to_uppercase())))
    })
}

/// `string.trim()` - Returns string with leading and trailing whitespace removed.
fn trim_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(Value::String(StringValue::new(this.trim().to_string())))
    })
}

/// `string.split(separator)` - Splits string by separator and returns a list.
fn split_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        let separator = get_single_string_arg(args)?;
        let parts: Vec<Value> = this
            .split(separator.value.as_str())
            .map(|s| Value::String(StringValue::new(s.to_string())))
            .collect();
        Ok(Value::List(ListValue::new(parts)))
    })
}

/// `string.slice(start, end?)` - Returns a substring from start to end (exclusive).
fn slice_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        if args.is_empty() || args.len() > 2 {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: args.len(),
            });
        }

        let start = match args.first() {
            Some(Value::Number(n)) => n.value as i64,
            Some(v) => {
                return Err(FunctionError::IncorrectArgumentType {
                    index: 0,
                    found_type: v.type_name().to_string(),
                    expected_type: "number".to_string(),
                });
            }
            None => unreachable!(),
        };

        let len = this.chars().count() as i64;

        // Handle negative indices (count from end)
        let start_idx = if start < 0 {
            (len + start).max(0) as usize
        } else {
            start.min(len) as usize
        };

        let end_idx = match args.get(1) {
            Some(Value::Number(n)) => {
                let end = n.value as i64;
                if end < 0 {
                    (len + end).max(0) as usize
                } else {
                    end.min(len) as usize
                }
            }
            Some(v) => {
                return Err(FunctionError::IncorrectArgumentType {
                    index: 1,
                    found_type: v.type_name().to_string(),
                    expected_type: "number".to_string(),
                });
            }
            None => len as usize,
        };

        let result: String = this
            .chars()
            .skip(start_idx)
            .take(end_idx.saturating_sub(start_idx))
            .collect();
        Ok(Value::String(StringValue::new(result)))
    })
}

/// `string.replace(pattern, replacement)` - Replaces all occurrences of pattern with replacement.
fn replace_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        if args.len() != 2 {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 2,
                found: args.len(),
            });
        }

        let pattern = match args.first() {
            Some(Value::String(s)) => s.value.as_str(),
            Some(v) => {
                return Err(FunctionError::IncorrectArgumentType {
                    index: 0,
                    found_type: v.type_name().to_string(),
                    expected_type: "string".to_string(),
                });
            }
            None => unreachable!(),
        };

        let replacement = match args.get(1) {
            Some(Value::String(s)) => s.value.as_str(),
            Some(v) => {
                return Err(FunctionError::IncorrectArgumentType {
                    index: 1,
                    found_type: v.type_name().to_string(),
                    expected_type: "string".to_string(),
                });
            }
            None => unreachable!(),
        };

        Ok(Value::String(StringValue::new(
            this.replace(pattern, replacement),
        )))
    })
}

/// `string.isEmpty()` - Returns true if string is empty.
fn is_empty_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(Value::Boolean(this.is_empty()))
    })
}

/// `string.containsAll(...values)` - Returns true if string contains all provided substrings.
fn contains_all_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        if args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: 0,
            });
        }
        for (idx, arg) in args.iter().enumerate() {
            match arg {
                Value::String(s) => {
                    if !this.contains(s.value.as_str()) {
                        return Ok(Value::Boolean(false));
                    }
                }
                v => {
                    return Err(FunctionError::IncorrectArgumentType {
                        index: idx,
                        found_type: v.type_name().to_string(),
                        expected_type: "string".to_string(),
                    });
                }
            }
        }
        Ok(Value::Boolean(true))
    })
}

/// `string.containsAny(...values)` - Returns true if string contains any of the provided substrings.
fn contains_any_fn(this: Rc<String>) -> Function {
    Box::new(move |args: &[Value]| {
        if args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: 0,
            });
        }
        for (idx, arg) in args.iter().enumerate() {
            match arg {
                Value::String(s) => {
                    if this.contains(s.value.as_str()) {
                        return Ok(Value::Boolean(true));
                    }
                }
                v => {
                    return Err(FunctionError::IncorrectArgumentType {
                        index: idx,
                        found_type: v.type_name().to_string(),
                        expected_type: "string".to_string(),
                    });
                }
            }
        }
        Ok(Value::Boolean(false))
    })
}
