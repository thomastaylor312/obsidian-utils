use std::{
    collections::BTreeSet,
    fmt::Debug,
    path::{Path, PathBuf},
    rc::Rc,
};

use chrono::{DateTime, Local};
use obsidian_core::frontmatter::Frontmatter;
use obsidian_links::FileLinks;

use crate::{
    LinkValue, Value,
    functions::{Function, FunctionError, FunctionRegistry, FunctionResult},
    value::{DateValue, FieldGetter, FieldRegistry, ListValue, NumberValue, StringValue},
};

/// Metadata for a file value.
#[derive(Clone)]
pub struct FileValue {
    pub value: Rc<Inner>,
    registry: Rc<FunctionRegistry>,
    fields: Rc<FieldRegistry>,
}

// TODO: This should probably all be borrowed data since file data is going to be used multiple times by anything filtering it
pub struct Inner {
    pub path: PathBuf,
    pub metadata: std::fs::Metadata,
    pub links: FileLinks,
    pub tags: BTreeSet<String>,
    pub frontmatter: Option<Frontmatter>,
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
        frontmatter: Option<Frontmatter>,
    ) -> Self {
        let data = Rc::new(Inner {
            path: path.as_ref().into(),
            metadata,
            links,
            tags,
            frontmatter,
        });

        let mut registry = FunctionRegistry::default();
        registry.register("hasTag", has_tag_fn(Rc::clone(&data)));
        registry.register("hasLink", has_link_fn(Rc::clone(&data)));
        registry.register("inFolder", in_folder_fn(Rc::clone(&data)));
        registry.register("hasProperty", has_property_fn(Rc::clone(&data)));
        registry.register("asLink", as_link_fn(Rc::clone(&data)));

        let mut fields = FieldRegistry::new();
        fields.register("name", name_getter(Rc::clone(&data)));
        fields.register("path", path_getter(Rc::clone(&data)));
        fields.register("ext", ext_getter(Rc::clone(&data)));
        fields.register("folder", folder_getter(Rc::clone(&data)));
        fields.register("size", size_getter(Rc::clone(&data)));
        fields.register("ctime", ctime_getter(Rc::clone(&data)));
        fields.register("mtime", mtime_getter(Rc::clone(&data)));
        fields.register("tags", tags_getter(Rc::clone(&data)));
        fields.register("links", links_getter(Rc::clone(&data)));

        Self {
            value: data,
            registry: Rc::new(registry),
            fields: Rc::new(fields),
        }
    }

    /// Call a function on the file value.
    pub fn call(&self, name: &str, args: &[Value]) -> FunctionResult {
        self.registry.call(name, args)
    }

    /// Get the value of a field. Returns None if the field doesn't exist
    pub fn field(&self, name: &str) -> Option<Value> {
        self.fields.get(name)
    }
}

// =============================================================================
// Functions
// =============================================================================

/// `file.hasTag(tag, ...)` - Returns true if the file has any of the specified tags.
fn has_tag_fn(this: Rc<Inner>) -> Function {
    Box::new(move |args| {
        if args.is_empty() {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: 0,
            });
        }
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

/// `file.hasLink(target)` - Returns true if the file links to the specified target.
fn has_link_fn(this: Rc<Inner>) -> Function {
    Box::new(move |args| {
        if args.len() != 1 {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: args.len(),
            });
        }

        // TODO: Fix this. It needs to use the btree methods for efficiency
        match args.first() {
            Some(Value::String(s)) => {
                let target = s.value.as_str();
                // Check if any link path contains the target string
                let has_link = this.links.links.iter().any(|link_path| {
                    link_path.to_string_lossy().contains(target)
                        || (link_path.file_stem().and_then(|s| s.to_str()) == Some(target))
                });
                Ok(Value::Boolean(has_link))
            }
            Some(Value::File(f)) => {
                // Check if any link points to this file
                let has_link = this.links.links.contains(&f.value.path);
                Ok(Value::Boolean(has_link))
            }
            Some(v) => Err(FunctionError::IncorrectArgumentType {
                index: 0,
                found_type: v.type_name().to_string(),
                expected_type: "string or file".to_string(),
            }),
            None => unreachable!(),
        }
    })
}

/// `file.inFolder(folder)` - Returns true if the file is in the specified folder.
fn in_folder_fn(this: Rc<Inner>) -> Function {
    Box::new(move |args| {
        if args.len() != 1 {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: args.len(),
            });
        }

        let folder = match args.first() {
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

        // TODO: Handle normalizing the paths from the vault root
        // Check if the file's parent path contains or equals the folder
        let parent = this.path.parent().and_then(|p| p.to_str()).unwrap_or("");
        let in_folder = parent == folder
            || parent.starts_with(&format!("{}/", folder))
            || parent.ends_with(&format!("/{}", folder))
            || parent.contains(&format!("/{}/", folder));

        Ok(Value::Boolean(in_folder))
    })
}

/// `file.hasProperty(name)` - Returns true if the file has the specified property in frontmatter.
fn has_property_fn(this: Rc<Inner>) -> Function {
    Box::new(move |args| {
        if args.len() != 1 {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: args.len(),
            });
        }

        let prop_name = match args.first() {
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

        let has_prop = this.frontmatter.as_ref().is_some_and(|fm| {
            // Check known fields first
            match prop_name {
                "tags" => fm.tags.is_some(),
                "aliases" => fm.aliases.is_some(),
                "cssclasses" => fm.cssclasses.is_some(),
                // Check in the generic values map
                _ => fm.values.contains_key(prop_name),
            }
        });

        Ok(Value::Boolean(has_prop))
    })
}

/// `file.asLink(display?)` - Returns the file as a link value.
fn as_link_fn(this: Rc<Inner>) -> Function {
    Box::new(move |args| {
        if args.len() > 1 {
            return Err(FunctionError::IncorrectArgumentCount {
                expected: 1,
                found: args.len(),
            });
        }

        let display = match args.first() {
            Some(Value::String(s)) => Some(s.value.as_ref().clone()),
            Some(v) => {
                return Err(FunctionError::IncorrectArgumentType {
                    index: 0,
                    found_type: v.type_name().to_string(),
                    expected_type: "string".to_string(),
                });
            }
            None => None,
        };

        Ok(Value::Link(LinkValue {
            target: this.path.clone(),
            display,
        }))
    })
}

// =============================================================================
// Field getters
// =============================================================================

/// `file.name` - The file name without extension.
fn name_getter(this: Rc<Inner>) -> FieldGetter {
    Box::new(move || {
        let name = this
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        Value::String(StringValue::new(name))
    })
}

/// `file.path` - The full path to the file.
fn path_getter(this: Rc<Inner>) -> FieldGetter {
    Box::new(move || {
        // NOTE(thomastaylor312): This might not encode the paths right on Windows
        let path = this.path.to_string_lossy().to_string();
        Value::String(StringValue::new(path))
    })
}

/// `file.ext` - The file extension (without the dot).
fn ext_getter(this: Rc<Inner>) -> FieldGetter {
    Box::new(move || {
        let ext = this
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        Value::String(StringValue::new(ext))
    })
}

/// `file.folder` - The parent folder of the file.
fn folder_getter(this: Rc<Inner>) -> FieldGetter {
    Box::new(move || {
        let folder = this
            .path
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();
        Value::String(StringValue::new(folder))
    })
}

/// `file.size` - The file size in bytes.
fn size_getter(this: Rc<Inner>) -> FieldGetter {
    Box::new(move || Value::Number(NumberValue::new(this.metadata.len() as f64)))
}

/// `file.ctime` - The file creation time.
fn ctime_getter(this: Rc<Inner>) -> FieldGetter {
    Box::new(move || match this.metadata.created() {
        Ok(time) => {
            let datetime: DateTime<Local> = time.into();
            Value::DateTime(DateValue::new(datetime.naive_local()))
        }
        Err(_) => Value::Null,
    })
}

/// `file.mtime` - The file modification time.
fn mtime_getter(this: Rc<Inner>) -> FieldGetter {
    Box::new(move || match this.metadata.modified() {
        Ok(time) => {
            let datetime: DateTime<Local> = time.into();
            Value::DateTime(DateValue::new(datetime.naive_local()))
        }
        Err(_) => Value::Null,
    })
}

/// `file.tags` - A list of all tags in the file.
fn tags_getter(this: Rc<Inner>) -> FieldGetter {
    Box::new(move || {
        let tags: Vec<Value> = this
            .tags
            .iter()
            .map(|t| Value::String(StringValue::new(t.clone())))
            .collect();
        Value::List(ListValue::new(tags))
    })
}

/// `file.links` - A list of all links in the file (as paths).
fn links_getter(this: Rc<Inner>) -> FieldGetter {
    Box::new(move || {
        let links: Vec<Value> = this
            .links
            .links
            .iter()
            .map(|link_path| Value::String(link_path.to_string_lossy().to_string().into()))
            .collect();
        Value::List(ListValue::new(links))
    })
}
