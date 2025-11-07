use std::cmp::Ordering;

use chrono::{Duration, NaiveDate, TimeZone, Utc};

use obsidian_bases::{TypeError, Value, ValueDate, ValueDateTime, ValueDuration, ValueError};

fn sample_date() -> ValueDate {
    NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date")
}

fn sample_datetime() -> ValueDateTime {
    Utc.with_ymd_and_hms(2025, 1, 1, 12, 30, 15)
        .single()
        .expect("valid datetime")
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
    let a = Value::Integer(5);
    let b = Value::Integer(3);

    let sum = a.add(&b).expect("add numbers");
    assert_eq!(sum, Value::Integer(8));

    let diff = a.sub(&b).expect("sub numbers");
    assert_eq!(diff, Value::Integer(2));

    let product = a.mul(&b).expect("mul numbers");
    assert_eq!(product, Value::Integer(15));

    let quotient = a.div(&b).expect("div numbers");
    assert_eq!(quotient, Value::Float(5.0 / 3.0));

    let quotient_int = Value::Integer(6)
        .div(&Value::Integer(3))
        .expect("div integers");
    assert_eq!(quotient_int, Value::Integer(2));
}

#[test]
fn concatenates_strings() {
    let result = Value::String("hello ".into())
        .add(&Value::String("world".into()))
        .expect("adds strings");
    assert_eq!(result, Value::String("hello world".into()));
}

#[test]
fn date_duration_arithmetic() {
    let date = sample_date();
    let duration = ValueDuration::days(1);

    let result = Value::Date(date)
        .add(&Value::Duration(duration))
        .expect("date + duration");

    match result {
        Value::Date(next) => assert_eq!(next, date.succ_opt().expect("next day")),
        other => panic!("expected date result, got {other:?}"),
    }

    let date_diff = Value::Date(date.succ_opt().expect("next day"))
        .sub(&Value::Date(date))
        .expect("date difference (dates)");
    match date_diff {
        Value::Duration(actual) => assert_eq!(actual, Duration::days(1)),
        other => panic!("expected duration, got {other:?}"),
    }

    let diff = Value::DateTime(sample_datetime())
        .sub(&Value::DateTime(
            Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0)
                .single()
                .expect("midnight"),
        ))
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
    let cmp = Value::Integer(2)
        .compare(&Value::Float(10.0))
        .expect("compare numbers");
    assert_eq!(cmp, Ordering::Less);

    let cmp = Value::String("abc".into())
        .compare(&Value::String("abc".into()))
        .expect("compare strings");
    assert_eq!(cmp, Ordering::Equal);
}

#[test]
fn truthiness_checks() {
    assert!(!Value::Integer(0).is_truthy());
    assert!(!Value::String(String::new()).is_truthy());
    assert!(Value::Float(5.0).is_truthy());
    assert!(Value::Boolean(true).is_truthy());
}

#[test]
fn list_contains() {
    let list = Value::List(vec![Value::Integer(1), Value::Float(2.0)]);
    assert!(
        list.contains(&Value::Float(2.0))
            .expect("contains succeeds")
    );
    assert!(
        !list
            .contains(&Value::Integer(3))
            .expect("contains succeeds")
    );
}

#[test]
fn display_formatting() {
    let date = sample_date();
    let formatted = Value::Date(date).to_string();
    assert_eq!(formatted, "2025-01-01");

    let integer_display = Value::Integer(42).to_string();
    assert_eq!(integer_display, "42");

    let float_display = Value::Float(2.1200).to_string();
    assert_eq!(float_display, "2.12");
}

#[test]
fn mixed_numeric_addition() {
    let result = Value::Float(2.5)
        .add(&Value::Integer(2))
        .expect("float + int");
    assert_eq!(result, Value::Float(4.5));
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
        .mul(&Value::Integer(2))
        .expect_err("string * integer fails");
    assert_invalid_operation(err, "mul", "string", "integer");

    let err = Value::Date(sample_date())
        .div(&Value::Integer(2))
        .expect_err("date / integer fails");
    assert_invalid_operation(err, "div", "date", "integer");

    let err = Value::Duration(ValueDuration::days(1))
        .rem(&Value::String("foo".into()))
        .expect_err("duration % string fails");
    assert_invalid_operation(err, "mod", "duration", "string");
}

#[test]
fn division_and_remainder_by_zero_error() {
    let err = Value::Integer(4)
        .div(&Value::Integer(0))
        .expect_err("division by zero fails");
    assert_invalid_operation(err, "div", "integer", "integer");

    let err = Value::Float(10.0)
        .rem(&Value::Float(0.0))
        .expect_err("remainder by zero fails");
    assert_invalid_operation(err, "mod", "float", "float");
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
    let err = Value::Integer(2)
        .contains(&Value::Integer(2))
        .expect_err("contains on integer fails");
    assert_invalid_operation(err, "contains", "integer", "integer");
}

#[test]
fn len_errors_on_unsupported_types() {
    let err = Value::Integer(2).len().expect_err("len on integer fails");
    assert_invalid_unary(err, "len", "integer");
}

#[test]
fn negate_errors_on_non_numeric_values() {
    let err = Value::String("foo".into())
        .negate()
        .expect_err("negate string fails");
    assert_invalid_unary(err, "neg", "string");
}
