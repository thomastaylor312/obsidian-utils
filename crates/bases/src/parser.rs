//! Parser for Bases expressions.
//!
//! This parser implements a recursive descent parser for the Obsidian Bases format,
//! which supports property references, function calls, method chaining, and arithmetic
//! and logical expressions.
//!
//! # Grammar
//!
//! The parser implements the following grammar with operator precedence from lowest to highest:
//!
//! ```text
//! expression     → logical_or
//! logical_or     → logical_and ( "||" logical_and )*
//! logical_and    → equality ( "&&" equality )*
//! equality       → comparison ( ("==" | "!=") comparison )*
//! comparison     → additive ( (">=" | "<=" | ">" | "<") additive )*
//! additive       → multiplicative ( ("+" | "-") multiplicative )*
//! multiplicative → unary ( ("*" | "/" | "%") unary )*
//! unary          → ("!" | "-") unary | primary
//! primary        → atom postfix*
//! postfix        → "." identifier [ "(" arguments ")" ]
//! atom           → literal | function_call | property_ref | "(" expression ")"
//! literal        → string | number | boolean | null
//! ```
//!
//! # Property Namespaces
//!
//! Properties can be qualified with namespaces (`note.`, `file.`, `formula.`, `this.`).
//! Unqualified property names default to the `note` namespace.
//!
//! # Examples
//!
//! ```rust
//! use obsidian_bases::parser::parse_expression;
//! use obsidian_bases::ast::{Expr, BinaryOperator};
//!
//! // Simple property access
//! let (_, expr) = parse_expression("note.title").unwrap();
//!
//! // Method call with chaining
//! let (_, expr) = parse_expression("\"hello\".contains(\"ell\")").unwrap();
//!
//! // Complex expression
//! let (_, expr) = parse_expression("price > 10 && status != \"done\"").unwrap();
//! ```

use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, tag};
use nom::character::complete::{char, multispace0, none_of};
use nom::combinator::{cut, value};
use nom::error::{ErrorKind, make_error};
use nom::multi::separated_list0;
use nom::sequence::delimited;
use nom::{Finish, IResult, Parser};

use crate::ast::{BinaryOperator, Expr, PropertyNamespace, PropertyRef, UnaryOperator};
use crate::error::{ParseErrorInfo, parse_error};

/// Parse a full expression from the provided input string.
///
/// This is the main entry point for parsing Bases expressions. It ensures that
/// the entire input is consumed (modulo trailing whitespace) and returns an error
/// if there is unparsed content remaining.
///
/// # Examples
///
/// ```
/// use obsidian_bases::parser::parse_expression;
/// use obsidian_bases::ast::Expr;
///
/// let (rest, expr) = parse_expression("42").unwrap();
/// assert!(rest.trim().is_empty());
/// assert_eq!(expr, Expr::Integer(42));
/// ```
///
/// # Errors
///
/// Returns a [`ParseErrorInfo`] with a user-friendly error message if:
/// - The input contains invalid syntax
/// - There is unexpected content after the expression
/// - The expression is incomplete or malformed
pub fn parse_expression(input: &str) -> IResult<&str, Expr, ParseErrorInfo<&str>> {
    match expression(input).finish() {
        Ok((remaining, expr)) => {
            if remaining.trim_start().is_empty() {
                Ok((remaining, expr))
            } else {
                Err(nom::Err::Error(parse_error(
                    remaining,
                    "unexpected content after expression",
                )))
            }
        }
        Err(nom_err) => {
            // Convert nom's default error to our custom error type
            Err(nom::Err::Error(convert_nom_error(input, nom_err)))
        }
    }
}

/// Convert a nom error to our custom ParseErrorInfo with better messages.
fn convert_nom_error<'a>(
    original_input: &'a str,
    err: nom::error::Error<&'a str>,
) -> ParseErrorInfo<&'a str> {
    // Try to provide context about where in the input the error occurred
    let position = original_input.len() - err.input.len();
    let context = if position > 0 {
        let start = position.saturating_sub(10);
        let end = (position + 10).min(original_input.len());
        format!(
            "near position {}: '...{}...'",
            position,
            &original_input[start..end]
        )
    } else {
        "at start of input".to_string()
    };

    let message = match err.code {
        nom::error::ErrorKind::Digit => {
            format!("expected a number {}", context)
        }
        nom::error::ErrorKind::Alpha => {
            format!("expected an identifier or keyword {}", context)
        }
        nom::error::ErrorKind::Tag => {
            format!("unexpected token {}", context)
        }
        nom::error::ErrorKind::Char => {
            let expected_char = if let Some(c) = err.input.chars().next() {
                format!("found '{}' ", c)
            } else {
                String::from("")
            };
            format!("unexpected character {}{}", expected_char, context)
        }
        nom::error::ErrorKind::Eof => {
            format!("unexpected end of input {}", context)
        }
        _ => {
            format!("parse error {:?} {}", err.code, context)
        }
    };

    ParseErrorInfo::new(err.input, message, err.code)
}

fn expression(input: &str) -> IResult<&str, Expr> {
    let (input, _) = multispace0(input)?;
    logical_or(input)
}

/// Parse logical OR expressions (lowest precedence binary operator).
///
/// Handles left-associative `||` operators by parsing the left operand,
/// then repeatedly consuming `||` operators and right operands.
fn logical_or(input: &str) -> IResult<&str, Expr> {
    let (mut input, mut expr) = logical_and(input)?;

    loop {
        let (after_ws, _) = multispace0(input)?;
        let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>("||")(after_ws) else {
            return Ok((input, expr));
        };

        let (after_rhs, rhs) = logical_and(after_op)?;
        expr = Expr::BinaryOp {
            op: BinaryOperator::Or,
            left: Box::new(expr),
            right: Box::new(rhs),
        };
        input = after_rhs;
    }
}

fn logical_and(input: &str) -> IResult<&str, Expr> {
    let (mut input, mut expr) = equality(input)?;

    loop {
        let (after_ws, _) = multispace0(input)?;
        let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>("&&")(after_ws) else {
            return Ok((input, expr));
        };

        let (after_rhs, rhs) = equality(after_op)?;
        expr = Expr::BinaryOp {
            op: BinaryOperator::And,
            left: Box::new(expr),
            right: Box::new(rhs),
        };
        input = after_rhs;
    }
}

fn equality(input: &str) -> IResult<&str, Expr> {
    let (mut input, mut expr) = comparison(input)?;

    loop {
        let (after_ws, _) = multispace0(input)?;

        if let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>("==")(after_ws) {
            let (after_rhs, rhs) = comparison(after_op)?;
            expr = Expr::BinaryOp {
                op: BinaryOperator::Eq,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
            input = after_rhs;
            continue;
        }

        if let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>("!=")(after_ws) {
            let (after_rhs, rhs) = comparison(after_op)?;
            expr = Expr::BinaryOp {
                op: BinaryOperator::Ne,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
            input = after_rhs;
            continue;
        }

        return Ok((input, expr));
    }
}

fn comparison(input: &str) -> IResult<&str, Expr> {
    let (mut input, mut expr) = additive(input)?;

    loop {
        let (after_ws, _) = multispace0(input)?;

        if let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>(">=")(after_ws) {
            let (after_rhs, rhs) = additive(after_op)?;
            expr = Expr::BinaryOp {
                op: BinaryOperator::Gte,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
            input = after_rhs;
            continue;
        }

        if let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>("<=")(after_ws) {
            let (after_rhs, rhs) = additive(after_op)?;
            expr = Expr::BinaryOp {
                op: BinaryOperator::Lte,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
            input = after_rhs;
            continue;
        }

        if let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>(">")(after_ws) {
            let (after_rhs, rhs) = additive(after_op)?;
            expr = Expr::BinaryOp {
                op: BinaryOperator::Gt,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
            input = after_rhs;
            continue;
        }

        if let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>("<")(after_ws) {
            let (after_rhs, rhs) = additive(after_op)?;
            expr = Expr::BinaryOp {
                op: BinaryOperator::Lt,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
            input = after_rhs;
            continue;
        }

        return Ok((input, expr));
    }
}

fn additive(input: &str) -> IResult<&str, Expr> {
    let (mut input, mut expr) = multiplicative(input)?;

    loop {
        let (after_ws, _) = multispace0(input)?;

        if let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>("+")(after_ws) {
            let (after_rhs, rhs) = multiplicative(after_op)?;
            expr = Expr::BinaryOp {
                op: BinaryOperator::Add,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
            input = after_rhs;
            continue;
        }

        if let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>("-")(after_ws) {
            let (after_rhs, rhs) = multiplicative(after_op)?;
            expr = Expr::BinaryOp {
                op: BinaryOperator::Sub,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
            input = after_rhs;
            continue;
        }

        return Ok((input, expr));
    }
}

fn multiplicative(input: &str) -> IResult<&str, Expr> {
    let (mut input, mut expr) = unary(input)?;

    loop {
        let (after_ws, _) = multispace0(input)?;

        if let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>("*")(after_ws) {
            let (after_rhs, rhs) = unary(after_op)?;
            expr = Expr::BinaryOp {
                op: BinaryOperator::Mul,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
            input = after_rhs;
            continue;
        }

        if let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>("/")(after_ws) {
            let (after_rhs, rhs) = unary(after_op)?;
            expr = Expr::BinaryOp {
                op: BinaryOperator::Div,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
            input = after_rhs;
            continue;
        }

        if let Ok((after_op, _)) = tag::<_, _, nom::error::Error<_>>("%")(after_ws) {
            let (after_rhs, rhs) = unary(after_op)?;
            expr = Expr::BinaryOp {
                op: BinaryOperator::Mod,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
            input = after_rhs;
            continue;
        }

        return Ok((input, expr));
    }
}

/// Parse unary expressions (`!` and `-` operators).
///
/// Unary operators are right-associative and have higher precedence than
/// binary operators but lower than primary expressions.
fn unary(input: &str) -> IResult<&str, Expr> {
    let (input, _) = multispace0(input)?;

    if let Ok((rest, _)) = char::<_, nom::error::Error<_>>('!')(input) {
        let (rest, expr) = unary(rest)?;
        return Ok((
            rest,
            Expr::UnaryOp {
                op: UnaryOperator::Not,
                expr: Box::new(expr),
            },
        ));
    }

    if let Ok((rest, _)) = char::<_, nom::error::Error<_>>('-')(input) {
        let (rest, expr) = unary(rest)?;
        return Ok((
            rest,
            Expr::UnaryOp {
                op: UnaryOperator::Neg,
                expr: Box::new(expr),
            },
        ));
    }

    primary(input)
}

/// Parse primary expressions (atoms with optional postfix operations).
///
/// A primary expression consists of an atom (literal, function call, property
/// reference, or parenthesized expression) followed by zero or more postfix
/// operations (member access or method calls via `.`).
fn primary(input: &str) -> IResult<&str, Expr> {
    let (input, _) = multispace0(input)?;
    let (input, base) = atom(input)?;
    parse_postfix(input, base)
}

fn atom(input: &str) -> IResult<&str, Expr> {
    alt((
        string_literal,
        number_literal,
        boolean_literal,
        null_literal,
        parenthesized_expression,
        function_or_property,
    ))
    .parse(input)
}

/// Parse a function call or property reference.
///
/// This function distinguishes between:
/// - Function calls: `functionName(args)`
/// - Property references: `name.property.chain`
/// - Namespaced properties: `note.property`, `file.property`, etc.
fn function_or_property(input: &str) -> IResult<&str, Expr> {
    let (rest, first) = identifier(input)?;
    if rest.starts_with('(') {
        let (rest_after_args, args) = argument_list(rest)?;
        return Ok((rest_after_args, Expr::FunctionCall { name: first, args }));
    }

    let (rest, segments) = parse_ident_chain(rest)?;
    let (namespace, path) = build_property_path(first, segments);

    if path.is_empty() {
        return Err(nom::Err::Error(make_error(rest, ErrorKind::Alpha)));
    }

    Ok((rest, Expr::Property(PropertyRef { namespace, path })))
}

/// Parse a chain of dot-separated identifiers for property access.
///
/// This function consumes `.identifier` sequences but stops before method calls.
/// For example, in `note.status.toString()`, this parses `[status]` and stops
/// before `.toString()` because it's followed by `(`.
fn parse_ident_chain(input: &str) -> IResult<&str, Vec<String>> {
    let mut segments = Vec::new();
    let mut rest = input;

    loop {
        if !rest.starts_with('.') {
            break;
        }

        // Save position before consuming the dot and identifier, so we can
        // "back out" if this turns out to be a method call rather than property access
        let original_rest = rest;
        let after_dot = &rest[1..];
        let (after_ident, ident) = identifier(after_dot)?;

        // If the identifier is followed by '(', it's a method call, not property access.
        // Return the position before we consumed the dot, so the caller can handle it.
        if after_ident.starts_with('(') {
            return Ok((original_rest, segments));
        }

        segments.push(ident.to_string());
        rest = after_ident;
    }

    Ok((rest, segments))
}

/// Build a property namespace and path from parsed identifier segments.
///
/// If the first segment is a recognized namespace (`note`, `file`, `formula`, `this`),
/// it becomes the namespace and subsequent segments form the path. Otherwise, all
/// segments (including the first) form the path under the default `note` namespace.
fn build_property_path(first: String, segments: Vec<String>) -> (PropertyNamespace, Vec<String>) {
    if segments.is_empty() {
        return (PropertyNamespace::Note, vec![first]);
    }

    match first.as_str() {
        "note" => (PropertyNamespace::Note, segments),
        "file" => (PropertyNamespace::File, segments),
        "formula" => (PropertyNamespace::Formula, segments),
        "this" => (PropertyNamespace::This, segments),
        _ => {
            let mut path = Vec::with_capacity(1 + segments.len());
            path.push(first);
            path.extend(segments);
            (PropertyNamespace::Note, path)
        }
    }
}

/// Parse postfix operations (member access and method calls) on an expression.
///
/// This function handles chained operations like:
/// - Member access: `expr.member`
/// - Method calls: `expr.method(args)`
/// - Chaining: `expr.method1().member.method2()`
fn parse_postfix(mut input: &str, mut expr: Expr) -> IResult<&str, Expr> {
    loop {
        match input.chars().next() {
            Some('.') => {
                let (after_dot, _) = char::<_, nom::error::Error<_>>('.')(input)?;
                let (after_ident, member) = identifier(after_dot)?;

                if after_ident.starts_with('(') {
                    let (after_args, args) = argument_list(after_ident)?;
                    expr = Expr::MethodCall {
                        object: Box::new(expr),
                        method: member,
                        args,
                    };
                    input = after_args;
                } else {
                    expr = Expr::MemberAccess {
                        object: Box::new(expr),
                        member,
                    };
                    input = after_ident;
                }
            }
            _ => return Ok((input, expr)),
        }
    }
}

fn parenthesized_expression(input: &str) -> IResult<&str, Expr> {
    let (input, _) = char('(')(input)?;
    let (input, expr) = expression(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = cut(char(')')).parse(input)?;
    Ok((input, expr))
}

fn argument_list(input: &str) -> IResult<&str, Vec<Expr>> {
    let (input, _) = char('(')(input)?;
    let (input, _) = multispace0(input)?;

    // If there are no arguments, return early
    if input.starts_with(')') {
        let (input, _) = char(')')(input)?;
        return Ok((input, Vec::new()));
    }

    let (input, args) = separated_list0(comma_separator, expression).parse(input)?;

    let (input, _) = multispace0(input)?;
    let (input, _) = cut(char(')')).parse(input)?;
    Ok((input, args))
}

fn comma_separator(input: &str) -> IResult<&str, ()> {
    let (input, _) = multispace0(input)?;
    let (input, _) = char(',')(input)?;
    let (input, _) = multispace0(input)?;
    Ok((input, ()))
}

fn string_literal(input: &str) -> IResult<&str, Expr> {
    alt((parse_string_with_quote('"'), parse_string_with_quote('\''))).parse(input)
}

fn parse_string_with_quote(quote: char) -> impl FnMut(&str) -> IResult<&str, Expr> {
    move |input| {
        delimited(char(quote), escape_string(quote), char(quote))
            .map(Expr::String)
            .parse(input)
    }
}

/// Parse the contents of a string literal, handling escape sequences.
///
/// Supports the following escape sequences:
/// - `\\` → backslash
/// - `\"` or `\'` → the quote character (depending on which quote is used)
/// - `\n` → newline
/// - `\r` → carriage return
/// - `\t` → tab
fn escape_string(quote: char) -> impl FnMut(&str) -> IResult<&str, String> {
    // Characters that cannot appear unescaped inside the string:
    // backslash (\) and the quote character being used
    let forbidden_chars = match quote {
        '"' => r#"\""#, // backslash and double quote
        '\'' => r"\'",  // backslash and single quote
        _ => r"\",      // just backslash
    };

    // The actual character to produce when we see an escaped quote
    let quote_char_str = quote.to_string();

    move |input: &str| {
        escaped_transform(
            none_of(forbidden_chars),
            '\\',
            alt((
                value(r"\", tag(r"\")),                                       // \\ → \
                value(quote_char_str.as_str(), tag(quote_char_str.as_str())), // \" or \' → " or '
                value("\n", tag("n")),                                        // \n → newline
                value("\r", tag("r")), // \r → carriage return
                value("\t", tag("t")), // \t → tab
            )),
        )
        .parse(input)
    }
}

/// Parse a sequence of ASCII digits and return the byte position after the last digit.
///
/// Returns `None` if no digits are found.
fn parse_digit_sequence(input: &str) -> Option<usize> {
    let mut end = 0usize;
    let mut found_digit = false;

    for (idx, ch) in input.char_indices() {
        if ch.is_ascii_digit() {
            end = idx + ch.len_utf8();
            found_digit = true;
        } else {
            break;
        }
    }

    if found_digit { Some(end) } else { None }
}

/// Try to parse a fractional part (`.` followed by digits).
///
/// Returns the number of bytes consumed (including the dot) if successful,
/// or `None` if the input doesn't start with `.` followed by a digit.
fn try_parse_fractional_part(input: &str) -> Option<usize> {
    let stripped = input.strip_prefix('.')?;

    // Only treat as fractional part if the dot is followed by a digit
    if !stripped.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }

    // Parse digits after the dot
    let frac_len = parse_digit_sequence(stripped)?;

    Some(1 + frac_len) // 1 for the dot + length of fractional digits
}

/// Parse a number literal (integer or float).
///
/// This implementation carefully handles the case where a number is followed by
/// a method call, like `123.toString()`. We only treat a dot as part of the number
/// if it's followed by at least one digit. This allows `123.toString()` to parse
/// `123` as an integer, leaving `.toString()` for the postfix parser to handle.
fn number_literal(input: &str) -> IResult<&str, Expr> {
    // TODO(thomastaylor312): I don't know if we need to handle signs or exponents when parsing
    // numbers here. I don't think so for now, but we'll have to come back to this if we do.

    // Parse the integer part
    let int_end = parse_digit_sequence(input)
        .ok_or_else(|| nom::Err::Error(make_error(input, ErrorKind::Digit)))?;

    let mut end = int_end;
    let rest_after_int = &input[int_end..];

    // Try to parse fractional part
    let has_fraction = if let Some(frac_len) = try_parse_fractional_part(rest_after_int) {
        end += frac_len;
        true
    } else {
        false
    };

    let rest = &input[end..];

    // Ensure no identifier character immediately follows the number
    if rest.chars().next().is_some_and(is_ident_start) {
        return Err(nom::Err::Error(make_error(rest, ErrorKind::Alpha)));
    }

    let literal = &input[..end];

    // Convert to appropriate numeric type
    if has_fraction {
        let value: f64 = literal
            .parse()
            .expect("validated digits should parse as f64");
        Ok((rest, Expr::Float(value)))
    } else {
        let value: i64 = literal
            .parse()
            .expect("validated digits should parse as i64");
        Ok((rest, Expr::Integer(value)))
    }
}

fn boolean_literal(input: &str) -> IResult<&str, Expr> {
    alt((
        value(Expr::Boolean(true), keyword("true")),
        value(Expr::Boolean(false), keyword("false")),
    ))
    .parse(input)
}

fn null_literal(input: &str) -> IResult<&str, Expr> {
    value(Expr::Null, keyword("null")).parse(input)
}

fn keyword<'a>(keyword: &'static str) -> impl FnMut(&'a str) -> IResult<&'a str, ()> {
    move |input: &'a str| {
        let (rest, _) = tag(keyword).parse(input)?;
        if rest.chars().next().is_some_and(is_ident_continue) {
            Err(nom::Err::Error(make_error(rest, ErrorKind::Alpha)))
        } else {
            Ok((rest, ()))
        }
    }
}

fn identifier(input: &str) -> IResult<&str, String> {
    let mut chars = input.char_indices();
    let Some((_, first)) = chars.next() else {
        return Err(nom::Err::Error(make_error(input, ErrorKind::Alpha)));
    };

    if !is_ident_start(first) {
        return Err(nom::Err::Error(make_error(input, ErrorKind::Alpha)));
    }

    let mut end = first.len_utf8();

    for (idx, ch) in chars {
        if is_ident_continue(ch) {
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    Ok((&input[end..], input[..end].to_string()))
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
