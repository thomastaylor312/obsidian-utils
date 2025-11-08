use std::collections::HashMap;

use thiserror::Error;

use crate::{Value, value::DateValue};

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
        registry.register("today", today_fn);
        // registry.register("now", now_fn);
        // ... more registrations

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

fn if_fn(args: &[Value]) -> FunctionResult {
    todo!()
}

fn today_fn(args: &[Value]) -> FunctionResult {
    if !args.is_empty() {
        return Err(FunctionError::IncorrectArgumentCount {
            expected: 0,
            found: args.len(),
        });
    }
    Ok(Value::DateTime(DateValue::now()))
}
