use std::cmp::Ordering;

use chrono::{Duration, TimeZone, Utc};

use obsidian_bases::{
    TypeError, Value, ValueDuration, ValueError,
    value::{DateValue, ListValue, StringValue},
};

fn sample_datetime() -> DateValue {
    DateValue::new(
        Utc.with_ymd_and_hms(2025, 1, 1, 12, 30, 15)
            .single()
            .expect("valid datetime")
            .naive_local(),
    )
}

fn assert_invalid_operation(
    err: ValueError,
    op: &'static str,
    left: &'static str,
    right: &'static str,
) {
    assert_eq!(
        err,
        ValueError::Type(TypeError::InvalidOperation { op, left, right })
    );
}

fn assert_invalid_unary(err: ValueError, op: &'static str, operand: &'static str) {
    assert_eq!(
        err,
        ValueError::Type(TypeError::InvalidUnary { op, operand })
    );
}

#[test]
fn arithmetic_on_numbers() {
    let a = Value::from(5.0);
    let b = Value::from(3.0);

    let sum = a.add(&b).expect("add numbers");
    assert_eq!(sum, Value::from(8.0));

    let diff = a.sub(&b).expect("sub numbers");
    assert_eq!(diff, Value::from(2.0));

    let product = a.mul(&b).expect("mul numbers");
    assert_eq!(product, Value::from(15.0));

    let quotient = a.div(&b).expect("div numbers");
    assert_eq!(quotient, Value::from(5.0 / 3.0));

    let quotient_int = Value::from(6.0)
        .div(&Value::from(3.0))
        .expect("div integers");
    assert_eq!(quotient_int, Value::from(2.0));
}

#[test]
fn concatenates_strings() {
    let result = Value::String("hello ".into())
        .add(&Value::String("world".into()))
        .expect("adds strings");
    assert_eq!(result, Value::String("hello world".into()));
}

#[test]
fn datetime_duration_arithmetic() {
    let diff = Value::DateTime(sample_datetime())
        .sub(&Value::DateTime(DateValue::new(
            Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0)
                .single()
                .expect("midnight")
                .naive_local(),
        )))
        .expect("date difference");
    match diff {
        Value::Duration(actual) => assert_eq!(
            actual,
            Duration::hours(12) + Duration::minutes(30) + Duration::seconds(15)
        ),
        other => panic!("expected duration, got {other:?}"),
    }
}

#[test]
fn comparisons_work() {
    let cmp = Value::from(2.0)
        .compare(&Value::from(10.0))
        .expect("compare numbers");
    assert_eq!(cmp, Ordering::Less);

    let cmp = Value::String("abc".into())
        .compare(&Value::String("abc".into()))
        .expect("compare strings");
    assert_eq!(cmp, Ordering::Equal);
}

#[test]
fn truthiness_checks() {
    assert!(!Value::from(0.0).is_truthy());
    assert!(!Value::String(StringValue::default()).is_truthy());
    assert!(Value::from(5.0).is_truthy());
    assert!(Value::Boolean(true).is_truthy());
}

#[test]
fn list_contains() {
    let list = Value::List(ListValue::new(vec![Value::from(1.0), Value::from(2.0)]));
    assert!(list.contains(&Value::from(2.0)).expect("contains succeeds"));
    assert!(!list.contains(&Value::from(3.0)).expect("contains succeeds"));
}

#[test]
fn display_formatting() {
    let date = sample_datetime();
    let formatted = Value::DateTime(date).to_string();
    assert_eq!(formatted, "2025-01-01 12:30:15");

    let integer_display = Value::from(42.0).to_string();
    assert_eq!(integer_display, "42");

    let float_display = Value::from(2.12).to_string();
    assert_eq!(float_display, "2.12");
}

#[test]
fn mixed_numeric_addition() {
    let result = Value::from(2.5)
        .add(&Value::from(2.0))
        .expect("float + int");
    assert_eq!(result, Value::from(4.5));
}

#[test]
fn arithmetic_errors_on_mismatched_types() {
    let err = Value::String("foo".into())
        .add(&Value::Boolean(true))
        .expect_err("string + boolean fails");
    assert_invalid_operation(err, "add", "string", "boolean");

    let err = Value::Boolean(false)
        .sub(&Value::Duration(ValueDuration::days(1)))
        .expect_err("boolean - duration fails");
    assert_invalid_operation(err, "sub", "boolean", "duration");

    let err = Value::String("foo".into())
        .mul(&Value::from(2.0))
        .expect_err("string * number fails");
    assert_invalid_operation(err, "mul", "string", "number");

    let err = Value::DateTime(sample_datetime())
        .div(&Value::from(2.0))
        .expect_err("date / number fails");
    assert_invalid_operation(err, "div", "datetime", "number");

    let err = Value::Duration(ValueDuration::days(1))
        .rem(&Value::String("foo".into()))
        .expect_err("duration % string fails");
    assert_invalid_operation(err, "mod", "duration", "string");
}

#[test]
fn division_and_remainder_by_zero_error() {
    let err = Value::from(4.0)
        .div(&Value::from(0.0))
        .expect_err("division by zero fails");
    assert_invalid_operation(err, "div", "number", "number");

    let err = Value::from(10.0)
        .rem(&Value::from(0.0))
        .expect_err("remainder by zero fails");
    assert_invalid_operation(err, "mod", "number", "number");
}

#[test]
fn comparison_errors_on_incompatible_types() {
    let err = Value::String("foo".into())
        .compare(&Value::Boolean(true))
        .expect_err("string compare boolean fails");
    assert_invalid_operation(err, "compare", "string", "boolean");
}

#[test]
fn contains_errors_on_non_collection_values() {
    let err = Value::from(2.0)
        .contains(&Value::from(2.0))
        .expect_err("contains on number fails");
    assert_invalid_operation(err, "contains", "number", "number");
}

#[test]
fn len_errors_on_unsupported_types() {
    let err = Value::from(2.0).len().expect_err("len on number fails");
    assert_invalid_unary(err, "len", "number");
}

#[test]
fn negate_errors_on_non_numeric_values() {
    let err = Value::String("foo".into())
        .negate()
        .expect_err("negate string fails");
    assert_invalid_unary(err, "neg", "string");
}
