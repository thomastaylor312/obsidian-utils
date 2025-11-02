# Quick Reference: Vault Reading & Parsing

## The Three-Stage Pipeline

### Stage 1: Read Files
**Function**: `reader::read_files()` → `Vec<FileEntry>`

```rust
let entries = cli.read_opts.read_files()?;
// Returns ALL files (any extension) from directory/stdin
```

**Key Points**:
- Returns raw files, no filtering
- Supports recursive directory traversal
- Can read from stdin or explicit `--dir`

---

### Stage 2: Parse Markdown to AST
**Function**: `parser::parse_files(arena, entries)` → `Iterator<ParsedFile>`

```rust
let arena = Arena::with_capacity(entries.len());
let parsed = parser::parse_files(&arena, entries);
// Filters to only .md files
// Converts to Abstract Syntax Tree
// Returns lazy iterator
```

**Key Points**:
- Automatically filters `.md` files only
- Uses `comrak` crate (CommonMark parser)
- Returns iterator (not a Vec)
- Parse errors wrapped in `Result`
- Supports: strikethrough, tables, footnotes, wikilinks, frontmatter

---

### Stage 3: Extract Frontmatter
**Function**: `frontmatter::parse_frontmatter(parsed_files)` → `Iterator<(ParsedFile, Option<Frontmatter>)>`

```rust
let with_fm = frontmatter::parse_frontmatter(parsed);
// Extracts YAML frontmatter from each file
// Deserializes using serde_norway
```

**Key Points**:
- Uses `serde_norway` for YAML parsing
- Returns tuple: (ParsedFile, optional Frontmatter)
- Frontmatter struct has: tags, aliases, cssclasses, + any extra values
- Errors logged but don't fail the pipeline

---

## Data Structures at a Glance

### FileEntry (from Stage 1)
```rust
pub struct FileEntry {
    pub path: PathBuf,
    pub metadata: Metadata,  // file size, timestamps, etc.
}
```

### ParsedFile (from Stage 2)
```rust
pub struct ParsedFile<'a> {
    pub path: PathBuf,
    pub metadata: Metadata,
    pub ast: &'a AstNode<'a>,  // Markdown AST
}
```

### Frontmatter (from Stage 3)
```rust
pub struct Frontmatter {
    pub tags: Option<Vec<String>>,
    pub aliases: Option<Vec<String>>,
    pub cssclasses: Option<Vec<String>>,
    pub values: HashMap<String, serde_norway::Value>,
}
```

---

## Common Patterns

### Pattern 1: Process All Files with Frontmatter
```rust
let entries = cli.read_opts.read_files()?;
let arena = Arena::with_capacity(entries.len());
let parsed = parser::ignore_error_iter(parser::parse_files(&arena, entries));
let with_fm = frontmatter::parse_frontmatter(parsed);

for (file, frontmatter_opt) in with_fm {
    println!("File: {}", file.path.display());
    if let Some(fm) = frontmatter_opt {
        println!("Tags: {:?}", fm.tags);
    }
}
```

### Pattern 2: Collect Results
```rust
let tags_and_files: BTreeMap<String, Vec<PathBuf>> = 
    with_fm.fold(BTreeMap::new(), |mut acc, (file, fm_opt)| {
        if let Some(fm) = fm_opt {
            for tag in fm.tags.unwrap_or_default() {
                acc.entry(tag).or_insert_default().push(file.path.clone());
            }
        }
        acc
    });
```

### Pattern 3: Handle Errors Gracefully
```rust
// Individual file errors don't stop pipeline
let parsed = parser::ignore_error_iter(parser::parse_files(&arena, entries));
// Failed files are silently skipped (with logging)
```

---

## File Extensions & Filtering

| Stage | Filters? | Result |
|-------|----------|--------|
| reader::read_files() | NO | All files |
| parser::parse_files() | YES | Only .md files |
| frontmatter::parse_frontmatter() | NO | All input files |

---

## Arena Allocation Pattern

```rust
// Create arena once
let arena = Arena::with_capacity(entries.len());

// Pass to parser
let parsed = parser::parse_files(&arena, entries);

// All AST nodes live in this arena
// Lifetime tied to arena: &'a AstNode<'a>
// When arena is dropped, all ASTs are freed
```

---

## Error Handling

### `anyhow::Result<T>`
Used throughout for error propagation:
```rust
pub fn read_files(&self) -> Result<Vec<FileEntry>>
pub fn parse_file<'a>(...) -> Result<&'a AstNode<'a>>
pub fn read_dir(...) -> Result<Vec<FileEntry>>
```

### Error Logging
```rust
log::error!("Failed to parse frontmatter: {}", e);
```

Initialize logging:
```rust
env_logger::init();  // In main()
```

---

## Dependencies Summary

| Crate | Version | Use |
|-------|---------|-----|
| comrak | 0.47 | Markdown parsing → AST |
| serde_norway | 0.9.42 | YAML deserialization |
| serde | 1.0 | Serialization framework |
| anyhow | 1.0 | Error handling |
| clap | 4.5 | CLI parsing |

---

## Module Structure

```
obsidian-core/
├── reader.rs      ← File discovery
├── parser.rs      ← AST generation
├── frontmatter.rs ← YAML extraction
├── printer.rs     ← Output formatting
└── lib.rs         ← Module exports
```

---

## Real Examples in Codebase

### Tags Example
**Location**: `/Users/oftaylor/Documents/code/obsidian-utils/bins/tags/src/main.rs:116-137`

Shows complete pipeline: read → parse → extract FM → process tags

### Links Example
**Location**: `/Users/oftaylor/Documents/code/obsidian-utils/bins/links/src/main.rs:111-129`

Shows how to parse links from AST (uses obsidian_links crate)

---

## Test Vault for Reference

Located at: `/Users/oftaylor/Documents/code/obsidian-utils/test-vault/`

Files:
- `Test.md` - Has frontmatter with tags
- `other/Other.md` - Nested markdown
- `notes.txt` - Non-markdown file

Used in tests to verify:
- Directory recursion
- MD file filtering
- Frontmatter parsing

---

## Performance Notes

1. **Iterator-based**: Functions return iterators, not Vecs
2. **Arena reuse**: Single arena for all ASTs
3. **Lazy evaluation**: Files parsed only when iterated
4. **Partial failure**: Bad files don't stop pipeline

---

## Common Gotchas

1. **Arena lifetime**: `&'a AstNode` lifetime tied to arena - can't outlive it
2. **Extension filtering**: `reader` doesn't filter; only `parser` does
3. **Frontmatter optional**: Some files won't have frontmatter - check `Option`
4. **Clap CLI parsing**: ReaderOpts is parsed from CLI, not created manually

