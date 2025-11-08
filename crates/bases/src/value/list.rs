use std::{fmt::Debug, rc::Rc};

use crate::{
    Value,
    functions::{Function, FunctionError, FunctionRegistry, FunctionResult},
    value::{FieldGetter, FieldRegistry, StringValue},
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

impl ListValue {
    /// Create a new list value.
    pub fn new(value: Vec<Value>) -> Self {
        let value = Rc::new(value);
        let mut registry = FunctionRegistry::new();
        registry.register("contains", contains_fn(Rc::clone(&value)));
        registry.register("join", join_fn(Rc::clone(&value)));
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
    Box::new(move || Value::Number(this.len() as f64))
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
