use std::fmt::{Debug, Display};
use std::rc::Rc;

use crate::{
    Value,
    functions::{FunctionError, FunctionRegistry, FunctionResult},
    value::StringValue,
};

/// A wrapper around `f64` that allows functions to be called on it.
#[derive(Clone)]
pub struct NumberValue {
    pub value: f64,
    registry: Rc<FunctionRegistry>,
}

impl PartialEq for NumberValue {
    fn eq(&self, other: &Self) -> bool {
        // Handle NaN equality - two NaNs are considered equal for our purposes
        if self.value.is_nan() && other.value.is_nan() {
            true
        } else {
            self.value == other.value
        }
    }
}

impl Debug for NumberValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Display for NumberValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<f64> for NumberValue {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<i64> for NumberValue {
    fn from(value: i64) -> Self {
        Self::new(value as f64)
    }
}

impl NumberValue {
    /// Create a new number value.
    pub fn new(value: f64) -> Self {
        let mut registry = FunctionRegistry::new();

        // We can't capture `value` directly because it's Copy, but we need consistent
        // behavior with other value types. For numbers, we just use the value directly
        // in each function since f64 is Copy.
        let v = value;
        registry.register("toFixed", to_fixed_fn(v));
        registry.register("round", round_fn(v));
        registry.register("abs", abs_fn(v));
        registry.register("ceil", ceil_fn(v));
        registry.register("floor", floor_fn(v));
        registry.register("isEmpty", is_empty_fn(v));

        Self {
            value,
            registry: Rc::new(registry),
        }
    }

    /// Call a function on the number value.
    pub fn call(&self, name: &str, args: &[Value]) -> FunctionResult {
        self.registry.call(name, args)
    }

    /// Numbers don't have fields, so this always returns None.
    pub fn field(&self, _name: &str) -> Option<Value> {
        None
    }
}

/// `number.toFixed(precision)` - Returns a string with the number in fixed-point notation.
fn to_fixed_fn(this: f64) -> crate::functions::Function {
    Box::new(move |args| {
        if args.len() != 1 {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: args.len(),
            });
        }
        let precision = match args.first() {
            Some(Value::Number(n)) => n.value as usize,
            Some(v) => {
                return Err(FunctionError::IncorrectArgumentType {
                    index: 0,
                    found_type: v.type_name().to_string(),
                    expected_type: "number".to_string(),
                });
            }
            None => unreachable!(),
        };
        Ok(Value::String(StringValue::new(format!(
            "{:.prec$}",
            this,
            prec = precision
        ))))
    })
}

/// `number.round(digits?)` - Rounds the number to the nearest integer or decimal places.
fn round_fn(this: f64) -> crate::functions::Function {
    Box::new(move |args| {
        let digits = match args.first() {
            Some(Value::Number(n)) => Some(n.value as i32),
            Some(v) => {
                return Err(FunctionError::IncorrectArgumentType {
                    index: 0,
                    found_type: v.type_name().to_string(),
                    expected_type: "number".to_string(),
                });
            }
            None => None,
        };

        if args.len() > 1 {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: args.len(),
            });
        }

        let result = match digits {
            Some(d) if d > 0 => {
                let multiplier = 10_f64.powi(d);
                (this * multiplier).round() / multiplier
            }
            Some(d) if d < 0 => {
                let multiplier = 10_f64.powi(-d);
                (this / multiplier).round() * multiplier
            }
            _ => this.round(),
        };

        Ok(Value::Number(NumberValue::new(result)))
    })
}

/// `number.abs()` - Returns the absolute value of the number.
fn abs_fn(this: f64) -> crate::functions::Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(Value::Number(NumberValue::new(this.abs())))
    })
}

/// `number.ceil()` - Rounds the number up to the nearest integer.
fn ceil_fn(this: f64) -> crate::functions::Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(Value::Number(NumberValue::new(this.ceil())))
    })
}

/// `number.floor()` - Rounds the number down to the nearest integer.
fn floor_fn(this: f64) -> crate::functions::Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(Value::Number(NumberValue::new(this.floor())))
    })
}

/// `number.isEmpty()` - Returns true if the number is not present (always false for numbers).
fn is_empty_fn(this: f64) -> crate::functions::Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        // A number is considered "empty" if it's exactly zero or NaN
        Ok(Value::Boolean(this.abs() <= f64::EPSILON || this.is_nan()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fixed_formats_correctly() {
        let num = NumberValue::new(3.30958235);
        let result = num.call("toFixed", &[Value::Number(2.0.into())]).unwrap();
        assert_eq!(result, Value::String("3.31".into()));
    }

    #[test]
    fn round_no_args() {
        let num = NumberValue::new(2.7);
        let result = num.call("round", &[]).unwrap();
        assert_eq!(result, Value::Number(3.0.into()));
    }

    #[test]
    fn round_with_digits() {
        let num = NumberValue::new(2.3456);
        let result = num.call("round", &[Value::Number(2.0.into())]).unwrap();
        assert_eq!(result, Value::Number(2.35.into()));
    }

    #[test]
    fn abs_positive() {
        let num = NumberValue::new(-5.0);
        let result = num.call("abs", &[]).unwrap();
        assert_eq!(result, Value::Number(5.0.into()));
    }

    #[test]
    fn ceil_rounds_up() {
        let num = NumberValue::new(2.1);
        let result = num.call("ceil", &[]).unwrap();
        assert_eq!(result, Value::Number(3.0.into()));
    }

    #[test]
    fn floor_rounds_down() {
        let num = NumberValue::new(2.9);
        let result = num.call("floor", &[]).unwrap();
        assert_eq!(result, Value::Number(2.0.into()));
    }
}
