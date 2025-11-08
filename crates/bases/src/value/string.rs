use std::{
    borrow::Borrow,
    fmt::{Debug, Display},
    ops::Deref,
    rc::Rc,
};

use crate::{
    Value,
    functions::{Function, FunctionError, FunctionRegistry, FunctionResult},
    value::{FieldGetter, FieldRegistry},
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
    Box::new(move || Value::Number(this.len() as f64))
}
