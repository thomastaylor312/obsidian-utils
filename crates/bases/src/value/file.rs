use std::{
    collections::BTreeSet,
    fmt::Debug,
    path::{Path, PathBuf},
    rc::Rc,
};

use obsidian_links::FileLinks;

use crate::{
    Value,
    functions::{Function, FunctionError, FunctionRegistry, FunctionResult},
};

/// Metadata for a file value.
#[derive(Clone)]
pub struct FileValue {
    pub value: Rc<Inner>,
    registry: Rc<FunctionRegistry>,
}

pub struct Inner {
    pub path: PathBuf,
    pub metadata: std::fs::Metadata,
    pub links: FileLinks,
    pub tags: BTreeSet<String>,
}

impl PartialEq for FileValue {
    fn eq(&self, other: &Self) -> bool {
        // For now we're only comparing the path, since the metadata is not guaranteed to be the same.
        // For purposes of what we're doing here, each file path should be unique within the value,
        // so this is the only comparison we care about
        self.value.path == other.value.path
    }
}

impl Debug for FileValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileValue")
            .field("path", &self.value.path)
            .finish()
    }
}

impl FileValue {
    /// Creates a file value from a path-like value.
    pub fn new(
        path: impl AsRef<Path>,
        metadata: std::fs::Metadata,
        links: FileLinks,
        tags: BTreeSet<String>,
    ) -> Self {
        let mut registry = FunctionRegistry::default();
        let data = Rc::new(Inner {
            path: path.as_ref().into(),
            metadata,
            links,
            tags,
        });
        registry.register("hasTag", has_tag_fn(Rc::clone(&data)));
        // TODO: add file specific functions to the registry
        Self {
            value: data,
            registry: Rc::new(registry),
        }
    }

    /// Call a function on the file value.
    pub fn call(&self, name: &str, args: &[Value]) -> FunctionResult {
        self.registry.call(name, args)
    }
}

fn has_tag_fn(this: Rc<Inner>) -> Function {
    Box::new(move |args| {
        for (idx, val) in args.iter().enumerate() {
            let Value::String(tag) = val else {
                return Err(FunctionError::IncorrectArgumentType {
                    index: idx,
                    found_type: val.type_name().to_string(),
                    expected_type: "string".to_string(),
                });
            };
            if this.tags.contains(tag.value.as_ref()) {
                return Ok(Value::Boolean(true));
            }
        }
        Ok(Value::Boolean(false))
    })
}
