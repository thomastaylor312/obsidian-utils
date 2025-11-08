use std::{fmt::Debug, rc::Rc};

use crate::{
    Value,
    functions::{Function, FunctionError, FunctionRegistry, FunctionResult},
    value::{FieldGetter, FieldRegistry, NumberValue, StringValue},
};

#[derive(Clone)]
pub struct ListValue {
    pub value: Rc<Vec<Value>>,
    registry: Rc<FunctionRegistry>,
    fields: Rc<FieldRegistry>,
}

impl Debug for ListValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl PartialEq for ListValue {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl From<Vec<Value>> for ListValue {
    fn from(value: Vec<Value>) -> Self {
        ListValue::new(value)
    }
}

impl ListValue {
    /// Create a new list value.
    pub fn new(value: Vec<Value>) -> Self {
        let value = Rc::new(value);
        let mut registry = FunctionRegistry::new();
        registry.register("contains", contains_fn(Rc::clone(&value)));
        registry.register("join", join_fn(Rc::clone(&value)));
        registry.register("isEmpty", is_empty_fn(Rc::clone(&value)));
        registry.register("containsAll", contains_all_fn(Rc::clone(&value)));
        registry.register("containsAny", contains_any_fn(Rc::clone(&value)));
        registry.register("reverse", reverse_fn(Rc::clone(&value)));
        registry.register("sort", sort_fn(Rc::clone(&value)));
        registry.register("flat", flat_fn(Rc::clone(&value)));
        registry.register("unique", unique_fn(Rc::clone(&value)));
        registry.register("slice", slice_fn(Rc::clone(&value)));
        registry.register("first", first_fn(Rc::clone(&value)));
        registry.register("last", last_fn(Rc::clone(&value)));
        let mut fields = FieldRegistry::new();
        fields.register("length", length_getter(Rc::clone(&value)));
        Self {
            value,
            registry: Rc::new(registry),
            fields: Rc::new(fields),
        }
    }

    /// Returns an iterator over the wrapped list
    pub fn iter(&self) -> std::slice::Iter<'_, Value> {
        self.value.iter()
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

fn length_getter(this: Rc<Vec<Value>>) -> FieldGetter {
    Box::new(move || Value::Number(NumberValue::new(this.len() as f64)))
}

fn contains_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
        if args.len() != 1 {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: args.len(),
            });
        }
        Ok(Value::Boolean(this.contains(
            args.first().expect("Should have exactly one argument"),
        )))
    })
}

fn join_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
        if args.len() != 1 {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: args.len(),
            });
        }
        let first = args.first().expect("Should have exactly one argument");
        let Value::String(sep) = first else {
            return Err(FunctionError::IncorrectArgumentType {
                index: 0,
                found_type: first.type_name().to_string(),
                expected_type: "string".to_string(),
            });
        };
        // Create a vec of each of the string values. This converts the string representation of each
        // value to a string.
        let mut strings = Vec::with_capacity(this.len());
        for value in this.iter() {
            strings.push(value.to_string());
        }
        Ok(Value::String(StringValue::new(strings.join(sep))))
    })
}

/// `list.isEmpty()` - Returns true if the list is empty.
fn is_empty_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(Value::Boolean(this.is_empty()))
    })
}

/// `list.containsAll(...values)` - Returns true if the list contains all provided values.
fn contains_all_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
        if args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: 0,
            });
        }
        for arg in args.iter() {
            if !this.iter().any(|item| item.equals(arg)) {
                return Ok(Value::Boolean(false));
            }
        }
        Ok(Value::Boolean(true))
    })
}

/// `list.containsAny(...values)` - Returns true if the list contains any of the provided values.
fn contains_any_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
        if args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: 0,
            });
        }
        for arg in args.iter() {
            if this.iter().any(|item| item.equals(arg)) {
                return Ok(Value::Boolean(true));
            }
        }
        Ok(Value::Boolean(false))
    })
}

/// `list.reverse()` - Returns a new list with elements in reverse order.
fn reverse_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        let reversed: Vec<Value> = this.iter().rev().cloned().collect();
        Ok(Value::List(ListValue::new(reversed)))
    })
}

/// `list.sort()` - Returns a new list with elements sorted.
/// For mixed types, uses type_name as secondary sort key.
fn sort_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        let mut sorted: Vec<Value> = this.iter().cloned().collect();
        sorted.sort_by(|a, b| {
            // Try to compare values; if incompatible, sort by type name then string representation
            a.compare(b).unwrap_or_else(|_| {
                // Fallback: compare by type name, then by string representation
                match a.type_name().cmp(b.type_name()) {
                    std::cmp::Ordering::Equal => a.to_string().cmp(&b.to_string()),
                    other => other,
                }
            })
        });
        Ok(Value::List(ListValue::new(sorted)))
    })
}

/// `list.flat()` - Flattens nested lists one level deep.
fn flat_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        let mut flattened = Vec::new();
        for item in this.iter() {
            match item {
                Value::List(inner) => flattened.extend(inner.value.iter().cloned()),
                other => flattened.push(other.clone()),
            }
        }
        Ok(Value::List(ListValue::new(flattened)))
    })
}

/// `list.unique()` - Returns a new list with duplicate values removed.
fn unique_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        // TODO(thomastaylor312): This is fairly inefficient, but to use a HashSet we'd need to
        // implement Hash for Value, so we should revisit this later.
        let mut seen = Vec::new();
        for item in this.iter() {
            if !seen.iter().any(|existing: &Value| existing.equals(item)) {
                seen.push(item.clone());
            }
        }
        Ok(Value::List(ListValue::new(seen)))
    })
}

/// `list.slice(start, end?)` - Returns a sublist from start to end (exclusive).
fn slice_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
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

        let len = this.len() as i64;

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

        Ok(Value::List(ListValue::new(
            this[start_idx..end_idx].to_vec(),
        )))
    })
}

/// `list.first()` - Returns the first element of the list, or null if empty.
fn first_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(this.first().cloned().unwrap_or(Value::Null))
    })
}

/// `list.last()` - Returns the last element of the list, or null if empty.
fn last_fn(this: Rc<Vec<Value>>) -> Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(this.last().cloned().unwrap_or(Value::Null))
    })
}
