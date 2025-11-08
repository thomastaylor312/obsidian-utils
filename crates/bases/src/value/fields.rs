use std::collections::HashMap;

use crate::Value;

pub type FieldGetter = Box<dyn Fn() -> Value>;

#[derive(Default)]
pub struct FieldRegistry {
    fields: HashMap<String, FieldGetter>,
}

impl FieldRegistry {
    /// Return a new, empty field registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a field's value, if it exists.
    pub fn get(&self, field_name: &str) -> Option<Value> {
        self.fields.get(field_name).map(|f| f())
    }

    /// Register a field getter
    pub fn register(&mut self, name: impl Into<String>, func: FieldGetter) {
        self.fields.insert(name.into(), func);
    }
}
