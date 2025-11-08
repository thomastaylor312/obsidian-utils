use std::{fmt::Debug, rc::Rc};

use chrono::{Datelike, Local, NaiveDateTime, Timelike};

use crate::{
    Value,
    functions::{Function, FunctionError, FunctionRegistry, FunctionResult},
    value::{FieldGetter, FieldRegistry, NumberValue, StringValue, moment_format},
};

#[derive(Clone)]
pub struct DateValue {
    // NOTE: As far as I can tell, I don't think there are any TZ offsets by default in things like
    // Obsidian frontmatter, but I definitely could be wrong. If this is the case, we can use an
    // actual datetime with a timezone
    pub value: Rc<NaiveDateTime>,
    registry: Rc<FunctionRegistry>,
    fields: Rc<FieldRegistry>,
}

impl Debug for DateValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl PartialEq for DateValue {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl DateValue {
    pub fn new(value: NaiveDateTime) -> Self {
        let value = Rc::new(value);
        let mut registry = FunctionRegistry::new();
        registry.register("date", date_fn(Rc::clone(&value)));
        registry.register("format", format_fn(Rc::clone(&value)));
        registry.register("time", time_fn(Rc::clone(&value)));
        registry.register("isEmpty", is_empty_fn());
        let mut fields = FieldRegistry::new();
        fields.register("year", year_getter(Rc::clone(&value)));
        fields.register("month", month_getter(Rc::clone(&value)));
        fields.register("day", day_getter(Rc::clone(&value)));
        fields.register("hour", hour_getter(Rc::clone(&value)));
        fields.register("minute", minute_getter(Rc::clone(&value)));
        fields.register("second", second_getter(Rc::clone(&value)));
        fields.register("millisecond", millisecond_getter(Rc::clone(&value)));
        Self {
            value,
            registry: Rc::new(registry),
            fields: Rc::new(fields),
        }
    }

    /// Create a date value from the current time
    pub fn now() -> Self {
        Self::new(Local::now().naive_local())
    }

    /// Call a function on the date value.
    pub fn call(&self, name: &str, args: &[Value]) -> FunctionResult {
        self.registry.call(name, args)
    }

    /// Get the value of a field. Returns None if the field doesn't exist
    pub fn field(&self, name: &str) -> Option<Value> {
        self.fields.get(name)
    }
}

fn date_fn(this: Rc<NaiveDateTime>) -> Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(Value::DateTime(DateValue::new(this.date().into())))
    })
}

/// `date.format(formatString)` - Format the date using a moment.js format string.
fn format_fn(this: Rc<NaiveDateTime>) -> Function {
    Box::new(move |args| {
        if args.len() != 1 {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: args.len(),
            });
        }
        let format_str = match args.first() {
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
        // Convert moment.js format to chrono format
        let chrono_format = moment_format::to_chrono_format(format_str)
            .map_err(|e| FunctionError::CallError(anyhow::anyhow!("{}", e)))?;
        let formatted = this.format(&chrono_format).to_string();
        Ok(Value::String(StringValue::new(formatted)))
    })
}

/// `date.time()` - Returns the time portion as HH:mm:ss string.
fn time_fn(this: Rc<NaiveDateTime>) -> Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        let formatted = this.format("%H:%M:%S").to_string();
        Ok(Value::String(StringValue::new(formatted)))
    })
}

/// `date.isEmpty()` - Returns false (dates are never empty).
fn is_empty_fn() -> Function {
    Box::new(move |args| {
        if !args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 0,
                found: args.len(),
            });
        }
        Ok(Value::Boolean(false))
    })
}

fn year_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(NumberValue::new(this.date().year() as f64)))
}

fn month_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(NumberValue::new(this.date().month() as f64)))
}

fn day_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(NumberValue::new(this.date().day() as f64)))
}

fn hour_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(NumberValue::new(this.time().hour() as f64)))
}

fn minute_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(NumberValue::new(this.time().minute() as f64)))
}

fn second_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(NumberValue::new(this.time().second() as f64)))
}

fn millisecond_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || {
        // NaiveDateTime stores subseconds as nanoseconds, so divide by 1_000_000 to get milliseconds
        let millis = this.time().nanosecond() / 1_000_000;
        Value::Number(NumberValue::new(millis as f64))
    })
}
