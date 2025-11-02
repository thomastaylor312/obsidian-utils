//! Abstract syntax tree for Bases expressions.

/// Expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// String literal.
    String(String),
    /// Numeric float literal.
    Float(f64),
    /// Numeric integer literal.
    Integer(i64),
    /// Boolean literal.
    Boolean(bool),
    /// Null literal.
    Null,

    /// Property reference such as `note.price`.
    Property(PropertyRef),

    /// Global function call, e.g. `hasTag("tag")`.
    FunctionCall { name: String, args: Vec<Expr> },

    /// Binary operation, e.g. `left + right`.
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Unary operation, e.g. `!expr`.
    UnaryOp { op: UnaryOperator, expr: Box<Expr> },

    /// Member access (property or method lookup) such as `object.field`.
    MemberAccess { object: Box<Expr>, member: String },

    /// Method call like `object.method(args...)`.
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
}

/// Reference to a property within a namespace.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyRef {
    pub namespace: PropertyNamespace,
    pub path: Vec<String>,
}

/// Property namespaces recognised by the parser.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropertyNamespace {
    /// `note.*` or bare property names.
    Note,
    /// `file.*`.
    File,
    /// `formula.*`.
    Formula,
    /// `this.*`.
    This,
}

/// Binary operator kinds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Comparison
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
    // Boolean
    And,
    Or,
}

/// Unary operator kinds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Not,
    Neg,
}
