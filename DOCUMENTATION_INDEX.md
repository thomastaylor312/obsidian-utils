# Documentation Index: Vault Reading & Parsing

This directory contains comprehensive documentation for the obsidian-utils vault reading and markdown parsing architecture.

## Quick Navigation

### For Beginners
Start here if you're new to the codebase:
1. **QUICK_REFERENCE.md** - High-level overview with simple examples
2. **IMPLEMENTATION_SUMMARY.txt** - Structured text summary of all components

### For Developers Implementing Features
1. **CODE_SNIPPETS.md** - Detailed code examples with line numbers
2. **VAULT_READING_ARCHITECTURE.md** - Complete architectural reference

### For Specific Questions

**Q: Where is code that reads markdown files from a vault?**
A: See `crates/core/src/reader.rs` (lines 50-63) for `read_dir()` function
   Or start with QUICK_REFERENCE.md → Stage 1: Read Files

**Q: How does the frontmatter parser work?**
A: See `crates/core/src/frontmatter.rs` (lines 41-56) for `parse_frontmatter_from_ast()`
   Or see CODE_SNIPPETS.md → Frontmatter Module section

**Q: What data structures are used?**
A: See VAULT_READING_ARCHITECTURE.md → Section 4: Data Structures & Dependencies
   Or see CODE_SNIPPETS.md → Memory & Lifetime Patterns

**Q: What are the main dependencies?**
A: See IMPLEMENTATION_SUMMARY.txt → KEY DEPENDENCIES section
   Or see VAULT_READING_ARCHITECTURE.md → Section 4

**Q: Show me a working example**
A: See CODE_SNIPPETS.md → Integration Example: Tags Command
   Or see QUICK_REFERENCE.md → Common Patterns

---

## File Organization

### Documentation Files (in this repo root)
- `DOCUMENTATION_INDEX.md` (this file)
- `QUICK_REFERENCE.md` - Quick start guide
- `CODE_SNIPPETS.md` - Detailed code examples
- `VAULT_READING_ARCHITECTURE.md` - Complete architecture
- `IMPLEMENTATION_SUMMARY.txt` - Structured summary

### Source Code Files (referenced)
- `crates/core/src/reader.rs` - File discovery (142 lines)
- `crates/core/src/parser.rs` - AST generation (172 lines)
- `crates/core/src/frontmatter.rs` - YAML parsing (90 lines)
- `crates/core/src/printer.rs` - Output formatting (144 lines)
- `bins/tags/src/main.rs` - Tags example (267 lines)
- `bins/links/src/main.rs` - Links example (182 lines)
- `crates/bases/src/lib.rs` - `.base` file schema + YAML loader (Stage 1 complete)
- `bins/bases/src/main.rs` - CLI for inspecting `.base` files

---

## The Three-Stage Pipeline

All processing follows this pattern:

```
Stage 1: Read Files
  Input:  directory path or stdin
  Output: Vec<FileEntry>
  Module: reader.rs
  Filter: none (all files)

        ↓

Stage 2: Parse Markdown
  Input:  Vec<FileEntry>
  Output: Iterator<Result<ParsedFile>>
  Module: parser.rs
  Filter: .md files only

        ↓

Stage 3: Extract Frontmatter
  Input:  Iterator<ParsedFile>
  Output: Iterator<(ParsedFile, Option<Frontmatter>)>
  Module: frontmatter.rs
  Filter: none (all input files)

        ↓

Application Logic
  (tags extraction, link parsing, etc.)
```

---

## Key Modules at a Glance

### reader.rs
```
Functions:
  read_files()  - main entry point (lines 25-37)
  read_dir()    - recursive directory traversal (lines 50-63)
  read_stdin()  - read paths from stdin (lines 65-80)

Structures:
  FileEntry     - path + metadata (lines 40-43)
  ReaderOpts    - CLI options (lines 8-21)
```

### parser.rs
```
Functions:
  parse_files()          - batch parser (lines 54-74)
  parse_file()           - single file (lines 77-84)
  parse_content()        - core parsing (lines 87-89)
  ignore_error_iter()    - error filtering (lines 38-49)

Structures:
  ParsedFile<'a>         - path + metadata + AST (lines 27-34)

Config:
  PARSE_OPTIONS          - comrak settings (lines 12-24)
  FRONTMATTER_DELIMITER  - "---" (line 11)
```

### frontmatter.rs
```
Functions:
  parse_frontmatter()            - main extractor (lines 31-38)
  parse_frontmatter_from_ast()   - AST traversal (lines 41-56)

Structures:
  Frontmatter                    - YAML data (lines 15-26)
```

---

## Common Tasks

### Read all markdown files from a vault
See: CODE_SNIPPETS.md → Directory Reading: read_dir()
Or:  QUICK_REFERENCE.md → Pattern 1

### Extract frontmatter tags
See: CODE_SNIPPETS.md → Integration Example: Tags Command
Or:  QUICK_REFERENCE.md → Example Code

### Handle files that fail to parse
See: CODE_SNIPPETS.md → Error Filtering: ignore_error_iter()
Or:  QUICK_REFERENCE.md → Pattern 3

### Understand memory management
See: CODE_SNIPPETS.md → Memory & Lifetime Patterns
Or:  VAULT_READING_ARCHITECTURE.md → Section 6: Key Design Patterns

### Extend with custom extraction
See: CODE_SNIPPETS.md → Integration Example
Or:  bins/links/src/main.rs for another example

---

## Performance Characteristics

All documented in: VAULT_READING_ARCHITECTURE.md → Section 8

Key points:
- Iterator-based processing (lazy evaluation)
- Arena allocation (fast, minimal fragmentation)
- Single parse pass (no intermediate formats)
- Partial failure (bad files don't stop pipeline)
- Memory scales with largest file, not total size

---

## Dependencies Summary

Core crates (from Cargo.toml):
- `comrak` 0.47 - CommonMark parser → AST
- `serde_norway` 0.9.42 - YAML deserialization
- `serde` 1.0.228 - Serialization framework
- `anyhow` 1.0.100 - Error handling
- `clap` 4.5 - CLI argument parsing

Complete list: IMPLEMENTATION_SUMMARY.txt → KEY DEPENDENCIES

---

## Architecture Patterns

Five key patterns used throughout:

1. **Composable Pipeline** - Each stage independent
2. **Iterator-based Processing** - Lazy evaluation
3. **Error Graceful Degradation** - Log and continue
4. **Type Safety** - Compiler-enforced lifetimes
5. **CLI Integration** - Structured argument parsing

See: VAULT_READING_ARCHITECTURE.md → Section 6

---

## Test Resources

Test vault location: `test-vault/`

Files:
- `Test.md` - Has frontmatter with tags
- `other/Other.md` - Nested file
- `notes.txt` - Non-markdown (filtered out)

Tested scenarios:
- Directory recursion
- File type filtering
- Frontmatter parsing

---

## Code Quality Standards

From the codebase:

1. **Error Handling** - Always use Result<T>
2. **Logging** - Use log crate, not println!
3. **Iterators** - Prefer lazy over eager
4. **Lifetimes** - Leverage compiler for safety
5. **Testing** - Each module has tests

All modules follow these patterns consistently.

---

## Next Steps

1. **Understand the pipeline**: Read QUICK_REFERENCE.md
2. **Find specific code**: Use CODE_SNIPPETS.md with line numbers
3. **Deep dive**: Read VAULT_READING_ARCHITECTURE.md
4. **Implement feature**: See Integration Example in CODE_SNIPPETS.md
5. **Debug issues**: Reference IMPLEMENTATION_SUMMARY.txt

---

## File Sizes & Scope

| Component | Lines | Scope |
|-----------|-------|-------|
| reader.rs | 142 | File discovery only |
| parser.rs | 172 | Markdown parsing only |
| frontmatter.rs | 90 | YAML extraction only |
| printer.rs | 144 | Output formatting only |
| tags example | 267 | Complete pipeline + application logic |
| links example | 182 | Complete pipeline + graph building |

Each module is focused and testable in isolation.

---

## Questions?

1. Where is X? → Check file locations in DOCUMENTATION_INDEX.md
2. How does X work? → See CODE_SNIPPETS.md with line numbers
3. Show me example → See QUICK_REFERENCE.md or CODE_SNIPPETS.md
4. What are the patterns? → See VAULT_READING_ARCHITECTURE.md
5. Summary? → See IMPLEMENTATION_SUMMARY.txt
