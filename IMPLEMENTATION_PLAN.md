# Implementation Plan: Obsidian Bases CLI

## Overview

This plan outlines the implementation of a new `obsidian-bases` crate and CLI tool for parsing and rendering Obsidian `.base` files. The implementation follows the established pattern in this repository of separating library logic (`crates/bases/`) from CLI interface (`bins/bases/`).

## Project Context

**Key Requirements:**
- Parse `.base` YAML files into Rust structs
- Implement expression parser for filters and formulas
- Query real Obsidian vault markdown files
- Integrate with existing `obsidian-core` vault reading and frontmatter parsing
- Support output formats: table (using `tabled` crate), json, csv, cbor
- Start with core subset of operators and functions (expandable later)

**Key Integrations:**
- Use `obsidian-core::reader::read_files()` for vault file reading
- Use `obsidian-core::parser::parse_files()` for markdown parsing
- Use `obsidian-core::frontmatter::parse_frontmatter()` for YAML frontmatter
- Follow CLI patterns from `bins/tags/` and `bins/links/`

---

## Stage 1: Project Setup and Base YAML Parsing

**Goal**: Create crate structure and implement YAML deserialization for `.base` file format.

**Success Criteria**:
- New crates created: `crates/bases/` (library) and `bins/bases/` (binary)
- Workspace configuration updated
- Basic Rust structs deserialize from example YAML
- Can read a `.base` file and print its structure

**Implementation Details**:

### Directory Structure
```
crates/bases/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── schema.rs       # YAML data structures
    └── error.rs        # Error types

bins/bases/
├── Cargo.toml
└── src/
    └── main.rs         # CLI entry point
```

### Data Structures (schema.rs)

Based on the YAML example in `Bases syntax.md`:

```rust
// Top-level .base file structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BaseFile {
    #[serde(default)]
    pub filters: Option<FilterNode>,

    #[serde(default)]
    pub formulas: HashMap<String, String>,  // formula_name -> expression string

    #[serde(default)]
    pub properties: HashMap<String, PropertyConfig>,

    #[serde(default)]
    pub views: Vec<View>,
}

// Recursive filter structure (and/or/not)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FilterNode {
    And { and: Vec<FilterNode> },
    Or { or: Vec<FilterNode> },
    Not { not: Vec<FilterNode> },
    Expression(String),  // e.g., "status != 'done'"
}

// Property display configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PropertyConfig {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    // Future: additional config fields
}

// View definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct View {
    pub r#type: ViewType,
    pub name: Option<String>,

    #[serde(default)]
    pub filters: Option<FilterNode>,

    #[serde(default)]
    pub order: Vec<String>,  // Property names to order by

    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewType {
    Table,
    // Future: other view types
}
```

**Tests**:
- Deserialize the full example YAML from `Bases syntax.md`
- Deserialize minimal valid base file (empty views array)
- Handle missing optional fields with defaults
- Reject invalid YAML with clear error messages

**Files to Create**:
- `crates/bases/Cargo.toml` - Dependencies: serde, serde_yaml, anyhow
- `crates/bases/src/lib.rs` - Public exports
- `crates/bases/src/schema.rs` - YAML structs
- `crates/bases/src/error.rs` - Error types
- `bins/bases/Cargo.toml` - Minimal CLI dependencies
- `bins/bases/src/main.rs` - Read file, deserialize, print debug

**Status**: Not Started

---

## Stage 2: Expression Parser and AST

**Goal**: Implement a parser for filter and formula expression strings into an Abstract Syntax Tree (AST).

**Success Criteria**:
- Parse property references: `note.price`, `file.name`, `formula.ppu`
- Parse function calls: `hasTag("tag")`, `if(condition, true, false)`
- Parse operators: arithmetic (`+`, `-`, `*`, `/`, `%`), comparison (`==`, `!=`, `>`, `<`, `>=`, `<=`), boolean (`&&`, `||`, `!`)
- Parse literals: strings (`"value"`), numbers (`42`, `3.14`), booleans (`true`, `false`)
- Proper operator precedence and parentheses support
- Helpful error messages with position information

**Implementation Details**:

### Expression AST (ast.rs)

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // Literals
    String(String),
    Number(f64),
    Boolean(bool),
    Null,

    // Property access
    Property(PropertyRef),

    // Function call
    FunctionCall {
        name: String,
        args: Vec<Expr>,
    },

    // Binary operations
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    // Unary operations
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expr>,
    },

    // Member access (for method calls like string.contains())
    MemberAccess {
        object: Box<Expr>,
        member: String,
    },

    // Method call (object.method(args))
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyRef {
    pub namespace: PropertyNamespace,
    pub path: Vec<String>,  // For nested access like note.author.name
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropertyNamespace {
    Note,      // note.* or bare property names
    File,      // file.*
    Formula,   // formula.*
    This,      // this.*
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
    // Arithmetic
    Add, Sub, Mul, Div, Mod,
    // Comparison
    Eq, Ne, Gt, Lt, Gte, Lte,
    // Boolean
    And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Not,
    Neg,
}
```

### Parser Implementation (parser.rs)

Write a parser that leverages the `nom` crate (v8) for robust parsing

**Operator Precedence** (highest to lowest):
1. Primary (literals, properties, parentheses)
2. Member access (`.`)
3. Unary (`!`, `-`)
4. Multiplicative (`*`, `/`, `%`)
5. Additive (`+`, `-`)
6. Comparison (`>`, `<`, `>=`, `<=`)
7. Equality (`==`, `!=`)
8. Logical AND (`&&`)
9. Logical OR (`||`)

**Tests**:
- Parse simple literals: `"hello"`, `42`, `true`
- Parse property access: `note.title`, `file.name`
- Parse function calls: `hasTag("book")`, `if(price, "yes", "no")`
- Parse complex expressions: `(price / age).toFixed(2)`, `status != "done" && price > 10`
- Test operator precedence: `2 + 3 * 4` = `2 + (3 * 4)`
- Test error reporting with position information

**Files to Create**:
- `crates/bases/src/ast.rs` - AST data structures
- `crates/bases/src/parser.rs` - Expression parser
- `crates/bases/tests/parser_tests.rs` - Comprehensive parser tests

**Status**: Not Started

---

## Stage 3: Type System and Value Representation

**Goal**: Implement runtime value types that expressions evaluate to.

**Success Criteria**:
- Value enum supports all Bases types: String, Number, Boolean, Date, List, Object, File, Link
- Type conversion and coercion work correctly
- Values are comparable and support arithmetic operations
- Proper null/empty handling

**Implementation Details**:

### Value Type (value.rs)

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    String(String),
    Number(f64),
    Boolean(bool),
    Date(DateTime),
    Duration(Duration),
    List(Vec<Value>),
    Object(HashMap<String, Value>),
    File(FileValue),
    Link(LinkValue),
}

impl Value {
    // Type checks
    pub fn is_truthy(&self) -> bool { ... }
    pub fn is_empty(&self) -> bool { ... }

    // Conversions
    pub fn to_string(&self) -> String { ... }
    pub fn to_number(&self) -> Result<f64> { ... }
    pub fn to_boolean(&self) -> bool { ... }

    // Comparisons
    pub fn compare(&self, other: &Value) -> Result<Ordering> { ... }
    pub fn equals(&self, other: &Value) -> bool { ... }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DateTime {
    pub timestamp: i64,  // milliseconds since epoch
    pub has_time: bool,  // true for datetime, false for date-only
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileValue {
    pub path: PathBuf,
    // Cached properties loaded lazily
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkValue {
    pub target: String,  // File path or URL
    pub display: Option<String>,
}
```

### Arithmetic Operations

```rust
impl Value {
    pub fn add(&self, other: &Value) -> Result<Value> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::Date(d), Value::Duration(dur)) => Ok(Value::Date(d.add_duration(dur))),
            _ => Err(TypeError::InvalidOperation("add", self.type_name(), other.type_name())),
        }
    }

    // Similar for sub, mul, div, mod
}
```

**Tests**:
- Type conversions: `Value::String("123").to_number()` = `123.0`
- Arithmetic: `Number(5) + Number(3)` = `Number(8)`
- String concatenation: `String("a") + String("b")` = `String("ab")`
- Date arithmetic: `Date + Duration("1d")`
- Comparisons: `Number(5) > Number(3)` = `true`
- List operations: `List([1, 2]).contains(Number(2))` = `true`
- Truthy evaluation: `Number(0)` is falsy, `String("")` is falsy

**Files to Create**:
- `crates/bases/src/value.rs` - Value type and operations
- `crates/bases/src/datetime.rs` - Date/time handling
- `crates/bases/tests/value_tests.rs` - Value operation tests

**Status**: Not Started

---

## Stage 4: Core Operators and Functions (Subset)

**Goal**: Implement the essential subset of operators and built-in functions.

**Success Criteria**:
- All operators work correctly
- Core global functions implemented
- Core methods on values implemented
- Function registry extensible for future additions

**Core Subset to Implement**:

### Global Functions
- `if(condition, trueResult, falseResult)` - Conditional
- `today()` - Current date (no time)
- `now()` - Current datetime
- `date(string)` - Parse date string
- `list(element)` - Ensure value is a list
- `number(value)` - Convert to number
- `link(path, display?)` - Create link

### File Methods (on file.*)
- `file.hasTag(...tags)` - Check if file has any tag
- `file.hasLink(otherFile)` - Check if file links to another
- `file.inFolder(folder)` - Check if in folder
- `file.hasProperty(name)` - Check if property exists

### String Methods
- `string.contains(substring)` - Contains check
- `string.startsWith(prefix)` - Starts with check
- `string.endsWith(suffix)` - Ends with check
- `string.lower()` - Lowercase conversion
- `string.split(separator)` - Split into list
- `string.length` - Length field

### Number Methods
- `number.toFixed(precision)` - Format with decimals
- `number.round(digits?)` - Round to integer or decimal places
- `number.abs()` - Absolute value

### List Methods
- `list.contains(value)` - Contains check
- `list.length` - Length field
- `list.join(separator)` - Join to string

### Date Methods
- `date.format(formatString)` - Format with moment.js style
- `date.date()` - Remove time portion
- `date.year`, `date.month`, `date.day` - Date fields
- `date.hour`, `date.minute`, `date.second` - Time fields

**Implementation Details**:

### Function Registry (functions.rs)

```rust
pub type FunctionImpl = fn(args: Vec<Value>) -> Result<Value>;

pub struct FunctionRegistry {
    functions: HashMap<String, FunctionImpl>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        let mut registry = Self { functions: HashMap::new() };

        // Register global functions
        registry.register("if", functions::if_fn);
        registry.register("today", functions::today_fn);
        registry.register("now", functions::now_fn);
        // ... more registrations

        registry
    }

    pub fn call(&self, name: &str, args: Vec<Value>) -> Result<Value> { ... }
}

// Implement individual functions
mod functions {
    pub fn if_fn(args: Vec<Value>) -> Result<Value> {
        match args.as_slice() {
            [condition, true_result, false_result] => {
                if condition.is_truthy() {
                    Ok(true_result.clone())
                } else {
                    Ok(false_result.clone())
                }
            }
            [condition, true_result] => {
                if condition.is_truthy() {
                    Ok(true_result.clone())
                } else {
                    Ok(Value::Null)
                }
            }
            _ => Err(Error::InvalidArgumentCount("if", 2..=3, args.len())),
        }
    }

    // ... more function implementations
}
```

### Method Dispatch (methods.rs)

```rust
impl Value {
    pub fn call_method(&self, method: &str, args: Vec<Value>) -> Result<Value> {
        match (self, method) {
            // String methods
            (Value::String(s), "contains") => {
                let substring = args.get(0).ok_or(...)?.as_string()?;
                Ok(Value::Boolean(s.contains(&substring)))
            }
            (Value::String(s), "lower") => {
                Ok(Value::String(s.to_lowercase()))
            }

            // Number methods
            (Value::Number(n), "toFixed") => {
                let precision = args.get(0).ok_or(...)?.as_number()? as usize;
                Ok(Value::String(format!("{:.prec$}", n, prec = precision)))
            }

            // List methods
            (Value::List(items), "contains") => {
                let value = args.get(0).ok_or(...)?;
                Ok(Value::Boolean(items.iter().any(|item| item.equals(value))))
            }

            _ => Err(Error::UnknownMethod(self.type_name(), method)),
        }
    }
}
```

**Tests**:
- Global functions: `if(true, "yes", "no")` = `"yes"`
- File methods: `file.hasTag("book")` on file with `#book` tag = `true`
- String methods: `"hello".contains("ell")` = `true`
- Number methods: `(3.14159).toFixed(2)` = `"3.14"`
- List methods: `[1, 2, 3].contains(2)` = `true`
- Date functions: `now().format("YYYY-MM-DD")` produces valid date string

**Files to Create**:
- `crates/bases/src/functions.rs` - Function registry and implementations
- `crates/bases/src/methods.rs` - Method dispatch (or integrate into value.rs)
- `crates/bases/tests/function_tests.rs` - Function tests

**Status**: Not Started

---

## Stage 5: Filter Evaluation Engine

**Goal**: Evaluate filter expressions against vault files to determine which files match.

**Success Criteria**:
- Can evaluate parsed expressions against a file context
- Recursive filter nodes (and/or/not) work correctly
- Property resolution works (note.*, file.*, formula.*)
- Filter evaluation is lazy where possible
- Clear errors for invalid expressions

**Implementation Details**:

### Evaluation Context (context.rs)

```rust
pub struct EvalContext<'a> {
    // Current file being evaluated
    pub file: &'a ParsedFile,
    pub frontmatter: &'a Option<Frontmatter>,

    // Vault context for cross-file operations
    pub vault: &'a VaultContext,

    // Formula values for this file (computed)
    pub formulas: HashMap<String, Value>,

    // Base file configuration
    pub base: &'a BaseFile,
}

impl<'a> EvalContext<'a> {
    pub fn resolve_property(&self, prop: &PropertyRef) -> Result<Value> {
        match prop.namespace {
            PropertyNamespace::Note => self.resolve_note_property(&prop.path),
            PropertyNamespace::File => self.resolve_file_property(&prop.path),
            PropertyNamespace::Formula => self.resolve_formula(&prop.path[0]),
            PropertyNamespace::This => todo!("Handle 'this' context"),
        }
    }

    fn resolve_note_property(&self, path: &[String]) -> Result<Value> {
        // Look up in frontmatter
        if let Some(fm) = self.frontmatter {
            // Access nested path in frontmatter YAML
            // ... implementation
        }
        Ok(Value::Null)
    }

    fn resolve_file_property(&self, path: &[String]) -> Result<Value> {
        match path.get(0).map(|s| s.as_str()) {
            Some("name") => Ok(Value::String(self.file.name().to_string())),
            Some("path") => Ok(Value::String(self.file.path().to_string_lossy().to_string())),
            Some("ext") => Ok(Value::String(self.file.extension())),
            Some("size") => Ok(Value::Number(self.file.size() as f64)),
            Some("mtime") => Ok(Value::Date(DateTime::from_timestamp(self.file.mtime()))),
            Some("ctime") => Ok(Value::Date(DateTime::from_timestamp(self.file.ctime()))),
            Some("tags") => Ok(self.get_file_tags()),
            // ... more file properties
            _ => Err(Error::UnknownProperty("file", path)),
        }
    }
}
```

### Evaluator (evaluator.rs)

```rust
pub struct Evaluator {
    functions: FunctionRegistry,
}

impl Evaluator {
    pub fn new() -> Self { ... }

    pub fn eval_expr(&self, expr: &Expr, ctx: &EvalContext) -> Result<Value> {
        match expr {
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::Boolean(b) => Ok(Value::Boolean(*b)),
            Expr::Null => Ok(Value::Null),

            Expr::Property(prop) => ctx.resolve_property(prop),

            Expr::FunctionCall { name, args } => {
                let arg_values = args.iter()
                    .map(|arg| self.eval_expr(arg, ctx))
                    .collect::<Result<Vec<_>>>()?;
                self.functions.call(name, arg_values)
            }

            Expr::BinaryOp { op, left, right } => {
                let left_val = self.eval_expr(left, ctx)?;
                let right_val = self.eval_expr(right, ctx)?;
                self.eval_binary_op(*op, left_val, right_val)
            }

            Expr::UnaryOp { op, expr } => {
                let val = self.eval_expr(expr, ctx)?;
                self.eval_unary_op(*op, val)
            }

            Expr::MethodCall { object, method, args } => {
                let obj_val = self.eval_expr(object, ctx)?;
                let arg_values = args.iter()
                    .map(|arg| self.eval_expr(arg, ctx))
                    .collect::<Result<Vec<_>>>()?;
                obj_val.call_method(method, arg_values)
            }

            Expr::MemberAccess { object, member } => {
                let obj_val = self.eval_expr(object, ctx)?;
                obj_val.get_field(member)
            }
        }
    }

    pub fn eval_filter(&self, filter: &FilterNode, ctx: &EvalContext) -> Result<bool> {
        match filter {
            FilterNode::And { and } => {
                for child in and {
                    if !self.eval_filter(child, ctx)? {
                        return Ok(false);  // Short-circuit
                    }
                }
                Ok(true)
            }

            FilterNode::Or { or } => {
                for child in or {
                    if self.eval_filter(child, ctx)? {
                        return Ok(true);  // Short-circuit
                    }
                }
                Ok(false)
            }

            FilterNode::Not { not } => {
                // All must be false
                for child in not {
                    if self.eval_filter(child, ctx)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }

            FilterNode::Expression(expr_str) => {
                let expr = Parser::new(expr_str).parse_expr()?;
                let result = self.eval_expr(&expr, ctx)?;
                Ok(result.is_truthy())
            }
        }
    }
}
```

**Tests**:
- Simple filters: `file.ext == "md"` on markdown file = `true`
- Complex filters: `(status != "done" && price > 10) || priority == "high"`
- Function calls: `file.hasTag("book")` on file with tag
- Nested filters: `and: [or: [...], not: [...]]`
- Property access: `note.title`, `file.mtime`, `formula.computed`
- Error handling: undefined property, type mismatch, invalid function

**Files to Create**:
- `crates/bases/src/context.rs` - Evaluation context
- `crates/bases/src/evaluator.rs` - Expression evaluator
- `crates/bases/tests/evaluator_tests.rs` - Evaluation tests

**Status**: Not Started

---

## Stage 6: Formula Computation

**Goal**: Compute formula values for each file that passes filters.

**Success Criteria**:
- Formulas are evaluated in dependency order (no circular refs)
- Formula results are cached per file
- Formulas can reference other formulas
- Clear error on circular dependencies

**Implementation Details**:

### Formula Dependency Analysis (formulas.rs)

```rust
pub struct FormulaSet {
    formulas: HashMap<String, String>,  // name -> expression
    dependencies: HashMap<String, Vec<String>>,  // name -> depends on
    evaluation_order: Vec<String>,  // Topologically sorted
}

impl FormulaSet {
    pub fn new(formulas: HashMap<String, String>) -> Result<Self> {
        let mut set = Self {
            formulas,
            dependencies: HashMap::new(),
            evaluation_order: Vec::new(),
        };

        set.analyze_dependencies()?;
        set.compute_evaluation_order()?;

        Ok(set)
    }

    fn analyze_dependencies(&mut self) -> Result<()> {
        for (name, expr_str) in &self.formulas {
            let expr = Parser::new(expr_str).parse_expr()?;
            let deps = Self::extract_formula_refs(&expr);
            self.dependencies.insert(name.clone(), deps);
        }
        Ok(())
    }

    fn extract_formula_refs(expr: &Expr) -> Vec<String> {
        // Walk AST and find all PropertyRef with namespace=Formula
        // ... implementation
    }

    fn compute_evaluation_order(&mut self) -> Result<()> {
        // Topological sort using Kahn's algorithm or DFS
        // Return error on circular dependency
        // ... implementation
    }

    pub fn evaluate_all(
        &self,
        evaluator: &Evaluator,
        ctx: &mut EvalContext,
    ) -> Result<HashMap<String, Value>> {
        let mut results = HashMap::new();

        for formula_name in &self.evaluation_order {
            let expr_str = &self.formulas[formula_name];
            let expr = Parser::new(expr_str).parse_expr()?;

            let value = evaluator.eval_expr(&expr, ctx)?;
            results.insert(formula_name.clone(), value.clone());

            // Update context so later formulas can reference this one
            ctx.formulas.insert(formula_name.clone(), value);
        }

        Ok(results)
    }
}
```

**Tests**:
- Simple formula: `formatted_price: 'price.toFixed(2)'`
- Dependent formulas: `ppu: "price / quantity"`, `total: "ppu * 10"`
- Detect circular dependency: `a: "b + 1"`, `b: "a + 1"` → Error
- Formula referencing note property: `full_name: "firstName + ' ' + lastName"`
- Formula with conditionals: `discount: 'if(price > 100, price * 0.1, 0)'`

**Files to Create**:
- `crates/bases/src/formulas.rs` - Formula dependency analysis and evaluation
- `crates/bases/tests/formula_tests.rs` - Formula tests

**Status**: Not Started

---

## Stage 7: View Processing and Output Formatting

**Goal**: Process views (filtering, ordering, limiting) and render to multiple output formats.

**Success Criteria**:
- View-specific filters applied correctly
- Ordering by multiple properties works
- Limit restricts result count
- Four output formats work: table, json, csv, cbor
- Table format uses `tabled` crate for dynamic columns

**Implementation Details**:

### View Processor (view.rs)

```rust
pub struct ViewProcessor {
    evaluator: Evaluator,
}

impl ViewProcessor {
    pub fn process_view(
        &self,
        view: &View,
        vault_files: Vec<ParsedFile>,
        base: &BaseFile,
    ) -> Result<ViewResult> {
        // 1. Apply global filters
        let mut filtered: Vec<_> = vault_files
            .into_iter()
            .filter(|file| {
                if let Some(global_filter) = &base.filters {
                    let ctx = EvalContext::new(file, base, ...);
                    self.evaluator.eval_filter(global_filter, &ctx)
                        .unwrap_or(false)
                } else {
                    true
                }
            })
            .collect();

        // 2. Compute formulas for each file
        let mut results = Vec::new();
        for file in filtered {
            let mut ctx = EvalContext::new(&file, base, ...);
            let formulas = FormulaSet::new(base.formulas.clone())?
                .evaluate_all(&self.evaluator, &mut ctx)?;

            results.push(FileResult { file, formulas });
        }

        // 3. Apply view-specific filters
        if let Some(view_filter) = &view.filters {
            results.retain(|result| {
                let ctx = EvalContext::from_result(result, base, ...);
                self.evaluator.eval_filter(view_filter, &ctx)
                    .unwrap_or(false)
            });
        }

        // 4. Order results
        if !view.order.is_empty() {
            results.sort_by(|a, b| self.compare_by_order(a, b, &view.order));
        }

        // 5. Apply limit
        if let Some(limit) = view.limit {
            results.truncate(limit);
        }

        Ok(ViewResult {
            columns: view.order.clone(),
            rows: results,
        })
    }

    fn compare_by_order(&self, a: &FileResult, b: &FileResult, order: &[String]) -> Ordering {
        for prop_name in order {
            let a_val = self.get_property_value(a, prop_name);
            let b_val = self.get_property_value(b, prop_name);

            match a_val.compare(&b_val) {
                Ok(Ordering::Equal) => continue,
                Ok(ordering) => return ordering,
                Err(_) => continue,
            }
        }
        Ordering::Equal
    }
}

pub struct ViewResult {
    pub columns: Vec<String>,
    pub rows: Vec<FileResult>,
}

pub struct FileResult {
    pub file: ParsedFile,
    pub formulas: HashMap<String, Value>,
}
```

### Output Formatters (output.rs)

```rust
pub enum OutputFormat {
    Table,
    Json,
    Csv,
    Cbor,
}

pub trait Formatter {
    fn format(&self, result: &ViewResult, base: &BaseFile) -> Result<Vec<u8>>;
}

// JSON Formatter
pub struct JsonFormatter;
impl Formatter for JsonFormatter {
    fn format(&self, result: &ViewResult, base: &BaseFile) -> Result<Vec<u8>> {
        let rows: Vec<_> = result.rows.iter()
            .map(|row| self.row_to_json(row, &result.columns, base))
            .collect();

        Ok(serde_json::to_vec_pretty(&rows)?)
    }
}

// CSV Formatter
pub struct CsvFormatter;
impl Formatter for CsvFormatter {
    fn format(&self, result: &ViewResult, base: &BaseFile) -> Result<Vec<u8>> {
        let mut wtr = csv::Writer::from_writer(vec![]);

        // Write header
        let headers: Vec<_> = result.columns.iter()
            .map(|col| self.get_display_name(col, base))
            .collect();
        wtr.write_record(&headers)?;

        // Write rows
        for row in &result.rows {
            let values: Vec<_> = result.columns.iter()
                .map(|col| self.get_cell_value(row, col).to_string())
                .collect();
            wtr.write_record(&values)?;
        }

        Ok(wtr.into_inner()?)
    }
}

// Table Formatter (using tabled)
pub struct TableFormatter;
impl Formatter for TableFormatter {
    fn format(&self, result: &ViewResult, base: &BaseFile) -> Result<Vec<u8>> {
        use tabled::{Table, Tabled};

        // Build rows as dynamic structures
        #[derive(Tabled)]
        struct DynamicRow {
            #[tabled(rename = "...")] // Dynamic column names
            values: Vec<String>,
        }

        // Better approach: use tabled::builder::Builder for truly dynamic tables
        let mut builder = tabled::builder::Builder::default();

        // Header
        let headers: Vec<_> = result.columns.iter()
            .map(|col| self.get_display_name(col, base))
            .collect();
        builder.set_header(headers);

        // Rows
        for row in &result.rows {
            let values: Vec<_> = result.columns.iter()
                .map(|col| self.get_cell_value(row, col).to_string())
                .collect();
            builder.push_record(values);
        }

        let table = builder.build();
        Ok(table.to_string().into_bytes())
    }
}

// CBOR Formatter
pub struct CborFormatter;
impl Formatter for CborFormatter {
    fn format(&self, result: &ViewResult, base: &BaseFile) -> Result<Vec<u8>> {
        let rows: Vec<_> = result.rows.iter()
            .map(|row| self.row_to_cbor(row, &result.columns, base))
            .collect();

        let mut buf = Vec::new();
        ciborium::into_writer(&rows, &mut buf)?;
        Ok(buf)
    }
}
```

**Tests**:
- View filtering: Apply view-specific filter on top of global filter
- Ordering: Sort by multiple properties (file.name, then formula.price)
- Limit: Limit to 10 results
- JSON output: Valid JSON array of objects
- CSV output: Valid CSV with headers and data rows
- Table output: Well-formatted ASCII table
- CBOR output: Valid binary that deserializes correctly
- Display names: Use propertyConfig.displayName when available

**Files to Create**:
- `crates/bases/src/view.rs` - View processing
- `crates/bases/src/output.rs` - Output formatters
- `crates/bases/tests/view_tests.rs` - View processing tests
- `crates/bases/tests/output_tests.rs` - Formatter tests

**Status**: Not Started

---

## Stage 8: CLI Interface

**Goal**: Create the CLI binary that ties everything together.

**Success Criteria**:
- CLI accepts base file path and vault directory as arguments
- Output format can be specified
- CLI follows existing patterns from tags and links CLIs
- Helpful error messages for user
- Works end-to-end with real vault data

**Implementation Details**:

### CLI Structure (bins/bases/src/main.rs)

```rust
use clap::Parser;
use obsidian_core::{reader, parser as md_parser, frontmatter};
use obsidian_bases::*;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "obsidian-bases", version, about = "Process Obsidian .base files")]
pub struct Cli {
    /// Path to the .base file to process
    pub base_file: PathBuf,

    /// Path to the Obsidian vault directory
    pub vault_dir: PathBuf,

    /// Output format
    #[arg(long, short = 'o', default_value = "table")]
    pub output: OutputFormat,

    /// View name to render (defaults to first view)
    #[arg(long)]
    pub view: Option<String>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    // 1. Load and parse base file
    let base_yaml = std::fs::read_to_string(&cli.base_file)
        .context("Failed to read base file")?;
    let base: BaseFile = serde_yaml::from_str(&base_yaml)
        .context("Failed to parse base file")?;

    // 2. Read vault files using obsidian-core
    let reader_opts = reader::ReaderOpts {
        dir: Some(cli.vault_dir),
        recurse: true,
    };
    let file_entries = reader_opts.read_files()
        .context("Failed to read vault files")?;

    // 3. Parse markdown and extract frontmatter
    let arena = comrak::Arena::new();
    let parsed_files: Vec<_> = md_parser::parse_files(&file_entries, &arena).collect();

    let files_with_frontmatter: Vec<_> = frontmatter::parse_frontmatter(parsed_files)
        .filter_map(|(file, fm)| Some((file, fm?)))
        .collect();

    // 4. Select view
    let view = if let Some(view_name) = &cli.view {
        base.views.iter()
            .find(|v| v.name.as_ref() == Some(view_name))
            .ok_or_else(|| anyhow::anyhow!("View '{}' not found", view_name))?
    } else {
        base.views.first()
            .ok_or_else(|| anyhow::anyhow!("No views defined in base file"))?
    };

    // 5. Process view
    let processor = ViewProcessor::new();
    let result = processor.process_view(view, files_with_frontmatter, &base)
        .context("Failed to process view")?;

    // 6. Format and output
    let formatter: Box<dyn Formatter> = match cli.output {
        OutputFormat::Table => Box::new(TableFormatter),
        OutputFormat::Json => Box::new(JsonFormatter),
        OutputFormat::Csv => Box::new(CsvFormatter),
        OutputFormat::Cbor => Box::new(CborFormatter),
    };

    let output = formatter.format(&result, &base)?;
    std::io::stdout().write_all(&output)?;

    Ok(())
}
```

### Cargo Configuration

**bins/bases/Cargo.toml**:
```toml
[package]
name = "bases"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "obsidian-bases"
path = "src/main.rs"

[dependencies]
anyhow = { workspace = true }
clap = { workspace = true }
env_logger = { workspace = true }
log = { workspace = true }
obsidian-core = { workspace = true }
obsidian-bases = { workspace = true }
serde = { workspace = true }
serde_yaml = { workspace = true }
comrak = { workspace = true }
```

**crates/bases/Cargo.toml**:
```toml
[package]
name = "obsidian-bases"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = { workspace = true }
obsidian-core = { workspace = true }
ciborium = { workspace = true }
csv = "1.3"
tabled = { workspace = true }
chrono = "0.4"  # For date/time handling

[dev-dependencies]
pretty_assertions = "1.4"
```

**Tests**:
- End-to-end: Process example base file with test vault
- CLI args: Test argument parsing
- Error messages: Invalid base file, missing vault, etc.
- Each output format: Verify output is valid
- View selection: Select specific view by name

**Files to Create**:
- `bins/bases/Cargo.toml` - Binary dependencies
- `bins/bases/src/main.rs` - CLI entry point
- Update `crates/bases/Cargo.toml` - Add missing deps (csv, chrono)
- Update root `Cargo.toml` - Add workspace members and csv dependency
- `bins/bases/tests/integration_tests.rs` - End-to-end tests

**Status**: Not Started

---

## Dependencies Summary

### Workspace Dependencies to Add (root Cargo.toml)
```toml
[workspace.dependencies]
# ... existing ...
obsidian-bases = { path = "crates/bases" }
csv = "1.3"
chrono = "0.4"
```

### Crate-Specific Dependencies

**crates/bases**:
- serde, serde_json, serde_yaml (YAML parsing, JSON output)
- anyhow (error handling)
- obsidian-core (vault reading, frontmatter parsing)
- ciborium (CBOR output)
- csv (CSV output)
- tabled (table output)
- chrono (date/time handling)

**bins/bases**:
- clap (CLI parsing)
- env_logger, log (logging)
- All from crates/bases

---

## Testing Strategy

Each stage should have comprehensive tests:

1. **Unit tests**: Test individual components in isolation
   - Parser: Parse various expressions
   - Value operations: Arithmetic, comparisons, conversions
   - Functions: Each function with various inputs
   - Evaluator: Expression evaluation

2. **Integration tests**: Test component interactions
   - Filter evaluation with real file data
   - Formula computation with dependencies
   - View processing end-to-end

3. **End-to-end tests**: Full CLI workflow
   - Create test vault with markdown files
   - Create test base file
   - Run CLI and verify output

4. **Test data**: Create reusable test fixtures
   - `tests/fixtures/vault/` - Small test vault
   - `tests/fixtures/bases/` - Various base file examples

---

## Implementation Order

Follow these stages in order. Each stage builds on previous stages:

1. **Stage 1**: Foundation - can deserialize base files
2. **Stage 2**: Parser - can parse expressions into AST
3. **Stage 3**: Types - can represent runtime values
4. **Stage 4**: Functions - can execute operations
5. **Stage 5**: Filters - can evaluate filters against files
6. **Stage 6**: Formulas - can compute formulas
7. **Stage 7**: Views - can process and format views
8. **Stage 8**: CLI - user-facing interface

**Key Checkpoints**:
- After Stage 2: Can parse any valid expression
- After Stage 4: Can execute simple expressions
- After Stage 5: Can filter vault files
- After Stage 8: Full working CLI

---

## Future Enhancements (Post-MVP)

Items explicitly out of scope for this plan but noted for future:

1. **Additional functions**: Implement remaining functions from Functions.md
2. **Regular expressions**: Add regex support
3. **Link resolution**: Implement link following and backlinks
4. **Date arithmetic**: Full duration parsing and date operations
5. **List operations**: map, filter, sort, etc.
6. **Object operations**: keys, values, nested access
7. **Performance**: Caching, parallel processing, incremental updates
8. **Multiple views**: Support rendering multiple views in one run
9. **Watch mode**: Re-render on file changes
10. **Error recovery**: Continue processing despite individual file errors

---

## Notes

- Follow existing patterns from `obsidian-tags` and `obsidian-links` CLIs
- Reuse `obsidian-core` utilities wherever possible
- Write tests alongside implementation (TDD approach)
- Keep code simple and readable - avoid premature optimization
- Document public APIs with rustdoc comments
- Each stage should compile and pass all tests before moving to next
- Update this plan as implementation reveals necessary changes
