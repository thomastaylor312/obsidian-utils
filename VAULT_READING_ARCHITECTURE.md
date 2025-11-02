# Obsidian Vault Reading and Markdown Parsing Architecture

## Overview

The obsidian-utils project uses a modular, composable pipeline to read markdown files from an Obsidian vault and parse their frontmatter and content. The architecture is divided into three main stages: **Reading**, **Parsing**, and **Data Extraction**.

---

## 1. File Reading Layer

### Location
- **Primary Module**: `crates/core/src/reader.rs`
- **Lines**: 1-142

### Key Structures

#### `ReaderOpts` (Lines 8-21)
Command-line options for controlling file reading behavior:
```rust
pub struct ReaderOpts {
    /// Whether to recurse into subdirectories (defaults to true)
    pub recurse: bool,
    /// Optional directory containing files to read
    pub dir: Option<PathBuf>,
}
```

#### `FileEntry` (Lines 40-43)
Represents a single file discovered during directory traversal:
```rust
pub struct FileEntry {
    pub path: PathBuf,
    pub metadata: Metadata,  // Contains file size, modified time, etc.
}
```

### Key Functions

#### `read_files()` (Lines 25-37)
Main entry point that determines whether to read from a directory or stdin:
- Returns `Result<Vec<FileEntry>>`
- Prioritizes explicit `--dir` argument over stdin
- Supports both file and directory input from stdin

#### `read_dir(path, recurse)` (Lines 50-63)
Recursively reads all files from a directory:
- Parameters:
  - `path`: Directory path to scan
  - `recurse`: If true, traverses subdirectories
- Returns: `Result<Vec<FileEntry>>`
- Implementation: Uses `std::fs::read_dir()` with recursive descent
- **Does NOT filter by file extension** - returns all files

#### `read_stdin(recurse)` (Lines 65-80)
Reads file/directory paths from stdin, one per line:
- Trims whitespace from each line
- Supports passing directories (if `recurse=true`, expands them)
- Skips non-file entries (directories when `recurse=false`)

### Usage Example
```rust
let cli = Cli::parse();  // Get ReaderOpts from CLI args
let entries = cli.read_opts.read_files()?;  // Read all files from vault
// entries is Vec<FileEntry> ready for parsing
```

---

## 2. Markdown Parsing Layer

### Location
- **Primary Module**: `crates/core/src/parser.rs`
- **Lines**: 1-172

### Key Structures

#### `ParsedFile<'a>` (Lines 27-34)
Represents a parsed markdown file with AST:
```rust
pub struct ParsedFile<'a> {
    pub path: PathBuf,                    // Original file path
    pub metadata: std::fs::Metadata,      // File metadata
    pub ast: &'a AstNode<'a>,           // Parsed AST (lifetime-bound to arena)
}
```

### Key Functions

#### `parse_files<'a>(arena, entries)` (Lines 54-74)
Parses a list of file entries into ParsedFile structures:
- **Filters**: Only processes `.md` files (case-insensitive)
- **Returns**: `impl Iterator<Item = Result<ParsedFile<'a>>>`
- **Memory**: Returns an iterator (lazy evaluation) instead of collecting immediately
- Each entry's content is read from disk and parsed with comrak

#### `parse_file<'a>(arena, path)` (Lines 77-84)
Parses a single markdown file:
- Reads file content with `std::fs::read_to_string()`
- Delegates to `parse_content()` for AST generation
- Returns: `Result<&'a AstNode<'a>>`

#### `parse_content<'a>(arena, content)` (Lines 87-89)
Core parser using comrak library:
```rust
pub fn parse_content<'a>(arena: &'a Arena<AstNode<'a>>, content: &str) -> &'a AstNode<'a> {
    comrak::parse_document(arena, content, &PARSE_OPTIONS)
}
```

### Parse Options (Lines 12-24)
Configured via `PARSE_OPTIONS` static:
```rust
Options {
    extension: Extension {
        strikethrough: true,
        table: true,
        autolink: true,
        footnotes: true,
        front_matter_delimiter: Some("---"),  // YAML frontmatter support
        alerts: true,
        wikilinks_title_after_pipe: true,
        ..Default::default()
    },
    ..Default::default()
}
```

### Frontmatter Delimiter (Line 11)
```rust
pub const FRONTMATTER_DELIMITER: &str = "---";
```

### Utility Functions

#### `ignore_error_iter()` (Lines 38-49)
Filters out parse errors while logging them:
- Takes an iterator of `Result<ParsedFile<'a>>`
- Returns only `Ok` variants, logs errors
- Useful for gracefully handling problematic files

### Memory Management
- Uses **arena allocation** via comrak's `Arena<AstNode>`
- All AST nodes reference the same arena (controlled lifetime)
- See usage in bins/tags/src/main.rs line 122: `let arena = Arena::with_capacity(entries.len());`

---

## 3. Frontmatter Extraction Layer

### Location
- **Primary Module**: `crates/core/src/frontmatter.rs`
- **Lines**: 1-90

### Key Structures

#### `Frontmatter` (Lines 15-26)
Represents extracted frontmatter metadata:
```rust
pub struct Frontmatter {
    pub tags: Option<Vec<String>>,              // YAML tags field
    pub aliases: Option<Vec<String>>,           // YAML aliases field
    pub cssclasses: Option<Vec<String>>,        // YAML cssclasses field
    #[serde(flatten)]
    pub values: HashMap<String, serde_norway::Value>,  // All other fields
}
```

### Key Functions

#### `parse_frontmatter()` (Lines 31-38)
Converts an iterator of ParsedFiles to tuples with optional Frontmatter:
```rust
pub fn parse_frontmatter<'a>(
    entries: impl IntoIterator<Item = ParsedFile<'a>>,
) -> impl Iterator<Item = (ParsedFile<'a>, Option<Frontmatter>)>
```
- Returns: Iterator of tuples: `(ParsedFile, Option<Frontmatter>)`
- Non-destructive - returns original ParsedFile with extracted data
- Lazy evaluation - returns iterator

#### `parse_frontmatter_from_ast()` (Lines 41-56)
Extracts frontmatter from a single AST node:
- Traverses AST nodes looking for `NodeValue::FrontMatter` variant
- Trims delimiters (`---`) from YAML text
- Uses `serde_norway::from_str()` to deserialize YAML
- Returns: `Option<Frontmatter>` (None if no frontmatter found)
- Error handling: Logs errors but returns None gracefully

### YAML Parsing
- **Crate**: `serde_norway` (0.9.42) - YAML deserializer
- **Serialization**: `serde` (1.0.228) with derive features
- **AST Generation**: `comrak` (0.47) - CommonMark parser with frontmatter support

### Supported Frontmatter Format
Standard YAML frontmatter at top of markdown files:
```yaml
---
tags: [tag1, tag2]
aliases: [alias1]
cssclasses: [class1]
customField: customValue
---

# Markdown content starts here...
```

---

## 4. Data Structures & Dependencies

### Crate Structure
- **obsidian-core** (`crates/core/Cargo.toml`):
  - Main library with reader, parser, and frontmatter modules
  - Exports: `pub mod frontmatter`, `pub mod parser`, `pub mod printer`, `pub mod reader`

### Key Dependencies
| Crate | Version | Purpose |
|-------|---------|---------|
| `comrak` | 0.47 | CommonMark markdown parser with AST generation |
| `serde_norway` | 0.9.42 | YAML deserialization |
| `serde` | 1.0.228 | Serialization framework |
| `serde_json` | 1.0 | JSON output formatting |
| `ciborium` | 0.2.2 | CBOR binary serialization |
| `anyhow` | 1.0.100 | Error handling |
| `clap` | 4.5 | CLI argument parsing |
| `log` | 0.4 | Logging framework |
| `env_logger` | 0.11 | Logger initialization |

### Data Flow Chain

```
┌──────────────────┐
│  FileEntry(s)    │  ← From read_dir() or read_stdin()
└────────┬─────────┘
         │
         ▼
┌──────────────────────────────┐
│  parse_files()               │  ← Filters .md files, creates AST
│  Returns: Vec<ParsedFile>    │  ← Holds path, metadata, &AstNode
└────────┬─────────────────────┘
         │
         ▼
┌──────────────────────────────┐
│  parse_frontmatter()         │  ← Extracts YAML frontmatter
│  Returns: (ParsedFile,       │  ← Deserializes using serde_norway
│            Option<Frontmatter>)
└────────┬─────────────────────┘
         │
         ▼
┌──────────────────────────────┐
│  Application Logic           │  ← Tags extraction, link parsing, etc.
│  (e.g., bins/tags/main.rs)   │
└──────────────────────────────┘
```

---

## 5. Real-World Usage Examples

### Example 1: Tags Extraction (bins/tags/src/main.rs)
```rust
// 1. Read files from vault directory
let entries = cli.read_opts.read_files()?;

// 2. Create arena for AST allocation
let arena = Arena::with_capacity(entries.len());

// 3. Parse markdown files to AST
let parsed_files = parser::ignore_error_iter(parser::parse_files(&arena, entries));

// 4. Extract frontmatter from parsed files
let parsed_with_fm = frontmatter::parse_frontmatter(parsed_files);

// 5. Process frontmatter data (fold over iterator)
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
```

### Example 2: Links Extraction (bins/links/src/main.rs)
```rust
// Same reading and parsing as above
let entries = cli.read_opts.read_files()?;
let arena = Arena::with_capacity(entries.len());
let parsed_files = parser::ignore_error_iter(parser::parse_files(&arena, entries));

// Extract links using obsidian_links crate
let parsed_with_links = obsidian_links::parser::parse_links(
    parsed_files,
    &vault_root,
    link_style,
);

// Fold results into Links graph structure
let mut links = parsed_with_links.try_fold(
    obsidian_links::Links::new(),
    |mut acc, (from, to)| {
        let from_path = from.path.canonicalize()?;
        let to = to.into_iter()
            .map(|p| p.canonicalize().or_else(|_| std::path::absolute(&p)))
            .collect::<Result<Vec<_>>>()?;
        acc.insert_links(from_path, to);
        Ok(acc)
    }
)?;
```

---

## 6. Key Design Patterns

### Iterator-Based Processing
- All major functions return iterators instead of collecting into Vecs
- Enables lazy evaluation and memory efficiency
- Allows processing of large vaults without allocating everything upfront
- Example: `parse_files()` returns `impl Iterator<Item = Result<ParsedFile>>`

### Lifetime Management
- ParsedFile holds `&'a AstNode<'a>` - lifetime tied to arena
- Arena created once and passed to all parsing operations
- Prevents dangling references while keeping memory management simple

### Error Handling
- Uses `anyhow::Result<T>` for fallible operations
- Explicit error logging via `log` crate (initialized with `env_logger`)
- `ignore_error_iter()` pattern allows graceful degradation when individual files fail

### Composable Pipeline
- Each stage is independent and can be composed
- Reader → Parser → Extractor → Application
- Supports multiple extraction backends (tags, links, custom)

### CLI Integration
- Uses `clap` for structured CLI argument parsing
- ReaderOpts encapsulates all reading behavior
- PrinterArgs encapsulates output formatting

---

## 7. Test Vault Location

Test fixtures located at: `test-vault/`

Verified test scenarios:
- `Test.md` - Root level file with frontmatter containing tags
- `other/Other.md` - Nested file for recursive reading tests
- `notes.txt` - Non-markdown file (filtered out during parsing)

---

## 8. Performance Considerations

1. **No Premature Collection**: Functions return iterators for lazy evaluation
2. **Arena Allocation**: Single arena for all AST nodes reduces allocation overhead
3. **Metadata Caching**: File metadata retrieved once during directory walk
4. **Partial Failure**: Individual file parse failures don't stop the pipeline
5. **Memory Scaling**: Memory usage scales with largest AST, not total vault size

### TODO Items in Code
- `reader.rs:45` - "Figure out if we can turn this into an iter instead so we don't have to allocate a big Vec of all entries before processing them"
- `frontmatter.rs:8` - "Figure out if we can dynamically generate a schema from the types.json schema in the vault so we don't have to use serde_norway::Value everywhere"

---

## Summary Table

| Component | File | Module | Input | Output |
|-----------|------|--------|-------|--------|
| Reader | `crates/core/src/reader.rs` | `obsidian_core::reader` | CLI args/stdin | `Vec<FileEntry>` |
| Parser | `crates/core/src/parser.rs` | `obsidian_core::parser` | `Vec<FileEntry>` + `Arena` | `Iterator<ParsedFile>` |
| Frontmatter | `crates/core/src/frontmatter.rs` | `obsidian_core::frontmatter` | `Iterator<ParsedFile>` | `Iterator<(ParsedFile, Option<Frontmatter>)>` |
| Printer | `crates/core/src/printer.rs` | `obsidian_core::printer` | Serializable data | Formatted output (Plain/JSON/Binary) |
| Links | `crates/links/src/lib.rs` | `obsidian_links` | Parsed links + vault path | `Links` graph structure |

---

## File Locations (Absolute Paths)

### Core Implementation Files
- `/Users/oftaylor/Documents/code/obsidian-utils/crates/core/src/reader.rs` - File reading (lines 1-142)
- `/Users/oftaylor/Documents/code/obsidian-utils/crates/core/src/parser.rs` - Markdown parsing (lines 1-172)
- `/Users/oftaylor/Documents/code/obsidian-utils/crates/core/src/frontmatter.rs` - Frontmatter extraction (lines 1-90)
- `/Users/oftaylor/Documents/code/obsidian-utils/crates/core/src/printer.rs` - Output formatting (lines 1-144)
- `/Users/oftaylor/Documents/code/obsidian-utils/crates/core/src/lib.rs` - Module exports

### Binary Examples
- `/Users/oftaylor/Documents/code/obsidian-utils/bins/tags/src/main.rs` - Tags extraction example (lines 1-267)
- `/Users/oftaylor/Documents/code/obsidian-utils/bins/links/src/main.rs` - Links extraction example (lines 1-182)

### Links Crate
- `/Users/oftaylor/Documents/code/obsidian-utils/crates/links/src/lib.rs` - Graph data structure (lines 1-437)

### Configuration
- `/Users/oftaylor/Documents/code/obsidian-utils/Cargo.toml` - Workspace definition
- `/Users/oftaylor/Documents/code/obsidian-utils/crates/core/Cargo.toml` - Core crate dependencies

### Test Fixtures
- `/Users/oftaylor/Documents/code/obsidian-utils/test-vault/` - Test markdown files
