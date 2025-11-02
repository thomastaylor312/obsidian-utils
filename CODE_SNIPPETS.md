# Code Snippets: Vault Reading Implementation Details

## Reader Module (`crates/core/src/reader.rs`)

### Main Entry Point: `read_files()`
**Location**: Lines 25-37

```rust
pub fn read_files(&self) -> Result<Vec<FileEntry>> {
    // If a directory is explicitly provided, use it regardless of stdin state
    if let Some(dir) = &self.dir {
        read_dir(dir, self.recurse)
    } else if !std::io::stdin().is_terminal() {
        // Only read from stdin if no directory was provided
        read_stdin(self.recurse)
    } else {
        Err(anyhow::anyhow!(
            "No vault directory specified and no input from stdin. Cannot proceed."
        ))
    }
}
```

**Notes**:
- Prioritizes explicit `--dir` over stdin
- Returns `Result<Vec<FileEntry>>` - collects all files
- Delegates to `read_dir()` or `read_stdin()`

---

### Directory Reading: `read_dir()`
**Location**: Lines 50-63

```rust
pub fn read_dir(path: impl AsRef<Path>, recurse: bool) -> Result<Vec<FileEntry>> {
    let mut entries = vec![];
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() && recurse {
            entries.extend(read_dir(&p, true)?);  // Recursive call
        } else if metadata.is_file() {
            entries.push(FileEntry { path: p, metadata });
        }
    }
    Ok(entries)
}
```

**Algorithm**:
1. Call `std::fs::read_dir()` on path
2. For each entry:
   - If directory AND recurse=true: recurse into it
   - If file: add to results
   - Else: skip it

**Memory**: Collects all entries into Vec before returning

---

### stdin Reading: `read_stdin()`
**Location**: Lines 65-80

```rust
pub fn read_stdin(recurse: bool) -> Result<Vec<FileEntry>> {
    let mut entries = vec![];
    for line in std::io::stdin().lines() {
        let line = line?;
        let path = PathBuf::from(line.trim());
        let metadata = std::fs::metadata(&path)?;
        if metadata.is_dir() && recurse {
            entries.extend(read_dir(&path, true)?);
            continue;
        } else if !metadata.is_file() {
            continue;
        }
        entries.push(FileEntry { path, metadata });
    }
    Ok(entries)
}
```

**Algorithm**:
1. Read from stdin, one path per line
2. For each line:
   - Get metadata
   - If directory AND recurse: expand it with `read_dir()`
   - If file: add to results
   - Else: skip

**Input format**: `/path/to/file.md` or `/path/to/directory`, one per line

---

## Parser Module (`crates/core/src/parser.rs`)

### Parse Options Configuration
**Location**: Lines 11-24

```rust
pub const FRONTMATTER_DELIMITER: &str = "---";

static PARSE_OPTIONS: LazyLock<Options<'static>> = LazyLock::new(|| Options {
    extension: Extension {
        strikethrough: true,
        table: true,
        autolink: true,
        footnotes: true,
        front_matter_delimiter: Some(FRONTMATTER_DELIMITER.into()),
        alerts: true,
        wikilinks_title_after_pipe: true,
        ..Default::default()
    },
    ..Default::default()
});
```

**Enabled Extensions**:
- strikethrough: `~~text~~`
- table: Markdown tables
- autolink: Auto-linked URLs
- footnotes: Footnotes syntax
- front_matter_delimiter: `---` for YAML
- alerts: GitHub-style alerts
- wikilinks_title_after_pipe: `[[file|title]]` format

---

### File Batch Parser: `parse_files()`
**Location**: Lines 54-74

```rust
pub fn parse_files<'a>(
    arena: &'a Arena<AstNode<'a>>,
    entries: impl IntoIterator<Item = FileEntry>,
) -> impl Iterator<Item = Result<ParsedFile<'a>>> {
    entries
        .into_iter()
        .filter(|e| {
            e.path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .map(|entry| {
            let root = parse_file(arena, &entry.path)?;
            Ok(ParsedFile {
                path: entry.path,
                metadata: entry.metadata,
                ast: root,
            })
        })
}
```

**Algorithm**:
1. Filter entries: keep only `.md` files (case-insensitive)
2. Map each to a ParsedFile:
   - Call `parse_file()` to get AST
   - Wrap result in Ok variant
3. Return iterator of Results

**Key**: Returns iterator, not Vec - lazy evaluation

---

### Single File Parser: `parse_file()`
**Location**: Lines 77-84

```rust
pub fn parse_file<'a>(
    arena: &'a Arena<AstNode<'a>>,
    path: impl AsRef<Path>,
) -> Result<&'a AstNode<'a>> {
    let content = std::fs::read_to_string(&path)
        .context("Failed to load file from disk")?;
    Ok(parse_content(arena, &content))
}
```

**Process**:
1. Read file to string
2. Pass to `parse_content()` for AST generation
3. Return AST reference (lifetime tied to arena)

---

### Content Parser: `parse_content()`
**Location**: Lines 87-89

```rust
pub fn parse_content<'a>(
    arena: &'a Arena<AstNode<'a>>,
    content: &str,
) -> &'a AstNode<'a> {
    comrak::parse_document(arena, content, &PARSE_OPTIONS)
}
```

**Note**: Core parsing delegated to `comrak::parse_document()`

---

### Error Filtering: `ignore_error_iter()`
**Location**: Lines 38-49

```rust
pub fn ignore_error_iter<'a, I>(iter: I) -> impl Iterator<Item = ParsedFile<'a>>
where
    I: IntoIterator<Item = Result<ParsedFile<'a>>>,
{
    iter.into_iter().filter_map(|res| match res {
        Ok(v) => Some(v),
        Err(e) => {
            log::error!("Ignoring error when parsing file: {e}");
            None
        }
    })
}
```

**Effect**: Converts `Iterator<Result<T>>` to `Iterator<T>`, logging errors

---

## Frontmatter Module (`crates/core/src/frontmatter.rs`)

### Frontmatter Struct Definition
**Location**: Lines 15-26

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Frontmatter {
    /// The tags associated with this file
    pub tags: Option<Vec<String>>,
    /// The aliases associated with this file
    pub aliases: Option<Vec<String>>,
    /// The CSS classes associated with this file
    pub cssclasses: Option<Vec<String>>,
    /// Any additional frontmatter values not explicitly modeled above
    #[serde(flatten)]
    pub values: HashMap<String, serde_norway::Value>,
}
```

**Features**:
- Typed fields: tags, aliases, cssclasses (all optional)
- Flattened HashMap: captures any extra YAML fields
- Uses serde for serialization/deserialization

---

### Main Parser: `parse_frontmatter()`
**Location**: Lines 31-38

```rust
pub fn parse_frontmatter<'a>(
    entries: impl IntoIterator<Item = ParsedFile<'a>>,
) -> impl Iterator<Item = (ParsedFile<'a>, Option<Frontmatter>)> {
    entries.into_iter().map(|pf| {
        let fm = parse_frontmatter_from_ast(pf.ast);
        (pf, fm)
    })
}
```

**Result**: Tuple of (ParsedFile, Option<Frontmatter>)
- ParsedFile is unchanged
- Frontmatter is extracted (or None if absent)
- Returns iterator (lazy evaluation)

---

### AST-based Extractor: `parse_frontmatter_from_ast()`
**Location**: Lines 41-56

```rust
fn parse_frontmatter_from_ast<'a>(ast: &'a AstNode<'a>) -> Option<Frontmatter> {
    for node in ast.descendants() {
        if let NodeValue::FrontMatter(ref text) = node.data.borrow().value {
            let trimmed = text
                .trim()
                .trim_matches(FRONTMATTER_DELIMITER_CHARS.as_slice());
            let fm: Frontmatter = serde_norway::from_str(trimmed)
                .map_err(|e| {
                    log::error!("Failed to parse frontmatter: {}", e);
                })
                .ok()?;
            return Some(fm);
        }
    }
    None
}
```

**Algorithm**:
1. Traverse AST nodes
2. Find first `NodeValue::FrontMatter` variant
3. Extract text
4. Trim delimiters (`---`)
5. Deserialize with `serde_norway::from_str()`
6. Log errors but return None

**Returns**: `Option<Frontmatter>` - first FM found, or None

---

## Integration Example: Tags Command
**Location**: `bins/tags/src/main.rs:116-137`

```rust
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    env_logger::init();

    // STAGE 1: Read files
    let entries = cli.read_opts.read_files()?;

    // STAGE 2: Create arena and parse
    let arena = Arena::with_capacity(entries.len());
    let parsed_files = parser::ignore_error_iter(parser::parse_files(&arena, entries));

    // STAGE 3: Extract frontmatter
    let parsed_with_fm = frontmatter::parse_frontmatter(parsed_files);

    // APPLICATION LOGIC: Process tags
    let tags = parsed_with_fm.fold(BTreeMap::new(), |mut acc, (pf, fm)| {
        if let Some(fm) = fm {
            for tag in fm.tags.unwrap_or_default() {
                acc.entry(tag)
                    .or_insert_with(TagInfo::new)
                    .files
                    .insert(pf.path.clone());
            }
        }
        acc
    });

    // OUTPUT: Format and print
    let format = cli.printer.output;
    let mut writer = std::io::stdout();
    // ... print based on format (json/plain/binary)
}
```

**Complete Pipeline**:
1. Parse CLI arguments (includes ReaderOpts)
2. Read all files from vault
3. Create arena for AST allocation
4. Parse markdown files to AST (filters .md only)
5. Extract frontmatter from AST
6. Process tags (fold operation)
7. Output in requested format

---

## Memory & Lifetime Patterns

### Arena Allocation Pattern
**Scope**: Lines 122 in `bins/tags/src/main.rs`

```rust
let arena = Arena::with_capacity(entries.len());
let parsed_files = parser::ignore_error_iter(parser::parse_files(&arena, entries));
```

**Lifetime Flow**:
```
┌─ Arena created with capacity
│
├─ parse_files() receives &'a arena reference
│  └─ Creates AstNode<'a> values in arena
│
├─ ParsedFile<'a> holds &'a AstNode<'a>
│  └─ Cannot outlive arena
│
└─ All data dropped when arena dropped
   └─ Single deallocation operation
```

---

## Error Handling Examples

### File Reading Error
```rust
let entries = cli.read_opts.read_files()?;
// Returns: Result<Vec<FileEntry>>
// ? operator propagates anyhow::Error
```

### Parse Error Handling
```rust
let parsed_files = parser::parse_files(&arena, entries);
// Returns: Iterator<Result<ParsedFile>>

let filtered = parser::ignore_error_iter(parsed_files);
// Converts to: Iterator<ParsedFile>
// Errors are logged and skipped
```

### Frontmatter Deserialization
```rust
let fm: Frontmatter = serde_norway::from_str(trimmed)
    .map_err(|e| {
        log::error!("Failed to parse frontmatter: {}", e);
    })
    .ok()?;
// On error: logs message, returns None
// Continues processing other files
```

---

## Supported Markdown Features

From PARSE_OPTIONS (lines 12-24):

| Feature | Example |
|---------|---------|
| Strikethrough | `~~crossed out~~` |
| Tables | Standard markdown tables |
| Autolink | Auto-links URLs |
| Footnotes | `[^1]` footnote syntax |
| **Frontmatter** | `---\nyaml: content\n---` |
| Alerts | `> [!NOTE]` blocks |
| Wikilinks | `[[file\|title]]` format |

---

## Test Examples

### Test: Parse File with Frontmatter
**Location**: `crates/core/src/parser.rs:101-124`

```rust
#[test]
fn parse_file_builds_ast() -> Result<()> {
    let vault = vault_path();
    let path = vault.join("Test.md");
    let arena = Arena::new();
    let ast = parse_file(&arena, &path)?;

    let frontmatter = crate::frontmatter::parse_frontmatter([ParsedFile {
        path: path.clone(),
        metadata: std::fs::metadata(&path)?,
        ast,
    }])
    .next()
    .and_then(|(_, fm)| fm);

    assert!(frontmatter.is_some(), "expected frontmatter to parse");
    let frontmatter = frontmatter.unwrap();
    assert_eq!(
        frontmatter.tags,
        Some(vec!["foo".to_string(), "bar".to_string()])
    );

    Ok(())
}
```

**Validates**: Complete pipeline from file → AST → frontmatter

---

## Dependencies Used

### Direct imports in each module:

**reader.rs**:
- `std::fs::Metadata`
- `std::path::{Path, PathBuf}`
- `anyhow::Result`
- `clap::Args`
- `std::io::IsTerminal`

**parser.rs**:
- `comrak::Arena, Options`
- `comrak::nodes::{AstNode, NodeValue}`
- `std::sync::LazyLock`
- `anyhow::Context`

**frontmatter.rs**:
- `serde::{Serialize, Deserialize}`
- `serde_norway::Value`
- `comrak::nodes::{AstNode, NodeValue}`
- `std::collections::HashMap`

