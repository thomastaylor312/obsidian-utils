use std::{fmt::Debug, rc::Rc};

use chrono::{Datelike, Local, NaiveDateTime, Timelike};

use crate::{
    Value,
    functions::{Function, FunctionError, FunctionRegistry, FunctionResult},
    value::{FieldGetter, FieldRegistry},
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
        let mut fields = FieldRegistry::new();
        fields.register("year", year_getter(Rc::clone(&value)));
        fields.register("month", month_getter(Rc::clone(&value)));
        fields.register("day", day_getter(Rc::clone(&value)));
        fields.register("hour", hour_getter(Rc::clone(&value)));
        fields.register("minute", minute_getter(Rc::clone(&value)));
        fields.register("second", second_getter(Rc::clone(&value)));
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

// TODO: Create a mapping (in a separate submodule) of the strftime format used by chrono to the momentJS format. https://momentjs.com/docs/#/displaying/format/. It might be a good idea to use some of the nom parser helpers to parse out the various format specifiers and then convert that to the matching `Item` enum from the chrono crate.

fn year_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(this.date().year() as f64))
}

fn month_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(this.date().month() as f64))
}

fn day_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(this.date().day() as f64))
}

fn hour_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(this.time().hour() as f64))
}

fn minute_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(this.time().minute() as f64))
}

fn second_getter(this: Rc<NaiveDateTime>) -> FieldGetter {
    Box::new(move || Value::Number(this.time().second() as f64))
}
