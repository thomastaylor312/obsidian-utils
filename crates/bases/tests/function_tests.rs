//! Tests for global functions and type methods.

use chrono::{Duration, NaiveDate, Timelike};
use obsidian_bases::{
    Value,
    functions::FunctionRegistry,
    value::{DateValue, ListValue, NumberValue, StringValue},
};

// =============================================================================
// Global Functions
// =============================================================================

#[test]
fn global_if_true_branch() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call(
            "if",
            &[Value::Boolean(true), Value::from(1.0), Value::from(2.0)],
        )
        .expect("if call succeeds");
    assert_eq!(result, Value::from(1.0));
}

#[test]
fn global_if_false_branch() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call(
            "if",
            &[Value::Boolean(false), Value::from(1.0), Value::from(2.0)],
        )
        .expect("if call succeeds");
    assert_eq!(result, Value::from(2.0));
}

#[test]
fn global_if_false_returns_null_when_no_else() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call("if", &[Value::Boolean(false), Value::from(1.0)])
        .expect("if call succeeds");
    assert_eq!(result, Value::Null);
}

#[test]
fn global_now_returns_datetime() {
    let registry = FunctionRegistry::global();
    let result = registry.call("now", &[]).expect("now call succeeds");
    assert!(matches!(result, Value::DateTime(_)));
}

#[test]
fn global_today_returns_date() {
    let registry = FunctionRegistry::global();
    let result = registry.call("today", &[]).expect("today call succeeds");
    match result {
        Value::DateTime(d) => {
            // Today should have time set to midnight (0:0:0)
            assert_eq!(d.value.time().hour(), 0);
            assert_eq!(d.value.time().minute(), 0);
            assert_eq!(d.value.time().second(), 0);
        }
        _ => panic!("Expected DateTime"),
    }
}

#[test]
fn global_date_parses_date_string() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call("date", &[Value::from("2025-01-15")])
        .expect("date call succeeds");
    match result {
        Value::DateTime(d) => {
            assert_eq!(
                d.value.date(),
                NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()
            );
        }
        _ => panic!("Expected DateTime"),
    }
}

#[test]
fn global_date_parses_datetime_string() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call("date", &[Value::from("2025-01-15 14:30:00")])
        .expect("date call succeeds");
    match result {
        Value::DateTime(d) => {
            assert_eq!(
                d.value.date(),
                NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()
            );
            assert_eq!(d.value.time().hour(), 14);
            assert_eq!(d.value.time().minute(), 30);
        }
        _ => panic!("Expected DateTime"),
    }
}

#[test]
fn global_duration_parses_days() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call("duration", &[Value::from("5d")])
        .expect("duration call succeeds");
    assert_eq!(result, Value::Duration(Duration::days(5)));
}

#[test]
fn global_duration_parses_complex_string() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call("duration", &[Value::from("1d 2h 30m")])
        .expect("duration call succeeds");
    let expected = Duration::days(1) + Duration::hours(2) + Duration::minutes(30);
    assert_eq!(result, Value::Duration(expected));
}

#[test]
fn global_list_wraps_single_value() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call("list", &[Value::from(42.0)])
        .expect("list call succeeds");
    match result {
        Value::List(l) => {
            assert_eq!(l.value.len(), 1);
            assert_eq!(l.value[0], Value::from(42.0));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn global_list_returns_list_unchanged() {
    let registry = FunctionRegistry::global();
    let input = Value::List(ListValue::new(vec![Value::from(1.0), Value::from(2.0)]));
    let result = registry
        .call("list", std::slice::from_ref(&input))
        .expect("list call succeeds");
    assert_eq!(result, input);
}

#[test]
fn global_number_converts_string() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call("number", &[Value::from("3.26")])
        .expect("number call succeeds");
    assert_eq!(result, Value::from(3.26));
}

#[test]
fn global_number_converts_boolean() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call("number", &[Value::Boolean(true)])
        .expect("number call succeeds");
    assert_eq!(result, Value::from(1.0));
}

#[test]
fn global_min_returns_smallest() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call(
            "min",
            &[Value::from(5.0), Value::from(2.0), Value::from(8.0)],
        )
        .expect("min call succeeds");
    assert_eq!(result, Value::from(2.0));
}

#[test]
fn global_max_returns_largest() {
    let registry = FunctionRegistry::global();
    let result = registry
        .call(
            "max",
            &[Value::from(5.0), Value::from(2.0), Value::from(8.0)],
        )
        .expect("max call succeeds");
    assert_eq!(result, Value::from(8.0));
}

// =============================================================================
// String Methods
// =============================================================================

#[test]
fn string_contains() {
    let s = StringValue::new("hello world".to_string());
    let result = s
        .call("contains", &[Value::from("world")])
        .expect("contains succeeds");
    assert_eq!(result, Value::Boolean(true));

    let result = s
        .call("contains", &[Value::from("foo")])
        .expect("contains succeeds");
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn string_starts_with() {
    let s = StringValue::new("hello world".to_string());
    let result = s
        .call("startsWith", &[Value::from("hello")])
        .expect("startsWith succeeds");
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn string_ends_with() {
    let s = StringValue::new("hello world".to_string());
    let result = s
        .call("endsWith", &[Value::from("world")])
        .expect("endsWith succeeds");
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn string_lower() {
    let s = StringValue::new("Hello World".to_string());
    let result = s.call("lower", &[]).expect("lower succeeds");
    assert_eq!(result, Value::from("hello world"));
}

#[test]
fn string_upper() {
    let s = StringValue::new("Hello World".to_string());
    let result = s.call("upper", &[]).expect("upper succeeds");
    assert_eq!(result, Value::from("HELLO WORLD"));
}

#[test]
fn string_trim() {
    let s = StringValue::new("  hello  ".to_string());
    let result = s.call("trim", &[]).expect("trim succeeds");
    assert_eq!(result, Value::from("hello"));
}

#[test]
fn string_split() {
    let s = StringValue::new("a,b,c".to_string());
    let result = s
        .call("split", &[Value::from(",")])
        .expect("split succeeds");
    match result {
        Value::List(l) => {
            assert_eq!(l.value.len(), 3);
            assert_eq!(l.value[0], Value::from("a"));
            assert_eq!(l.value[1], Value::from("b"));
            assert_eq!(l.value[2], Value::from("c"));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn string_slice() {
    let s = StringValue::new("hello world".to_string());
    let result = s
        .call("slice", &[Value::from(0.0), Value::from(5.0)])
        .expect("slice succeeds");
    assert_eq!(result, Value::from("hello"));
}

#[test]
fn string_slice_negative_index() {
    let s = StringValue::new("hello world".to_string());
    let result = s
        .call("slice", &[Value::from(-5.0)])
        .expect("slice succeeds");
    assert_eq!(result, Value::from("world"));
}

#[test]
fn string_replace() {
    let s = StringValue::new("hello world".to_string());
    let result = s
        .call("replace", &[Value::from("world"), Value::from("rust")])
        .expect("replace succeeds");
    assert_eq!(result, Value::from("hello rust"));
}

#[test]
fn string_is_empty() {
    let empty = StringValue::new("".to_string());
    let result = empty.call("isEmpty", &[]).expect("isEmpty succeeds");
    assert_eq!(result, Value::Boolean(true));

    let non_empty = StringValue::new("hello".to_string());
    let result = non_empty.call("isEmpty", &[]).expect("isEmpty succeeds");
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn string_contains_all() {
    let s = StringValue::new("hello world".to_string());
    let result = s
        .call("containsAll", &[Value::from("hello"), Value::from("world")])
        .expect("containsAll succeeds");
    assert_eq!(result, Value::Boolean(true));

    let result = s
        .call("containsAll", &[Value::from("hello"), Value::from("foo")])
        .expect("containsAll succeeds");
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn string_contains_any() {
    let s = StringValue::new("hello world".to_string());
    let result = s
        .call("containsAny", &[Value::from("foo"), Value::from("world")])
        .expect("containsAny succeeds");
    assert_eq!(result, Value::Boolean(true));

    let result = s
        .call("containsAny", &[Value::from("foo"), Value::from("bar")])
        .expect("containsAny succeeds");
    assert_eq!(result, Value::Boolean(false));
}

// =============================================================================
// Number Methods
// =============================================================================

#[test]
fn number_to_fixed() {
    let n = NumberValue::new(3.289);
    let result = n
        .call("toFixed", &[Value::from(2.0)])
        .expect("toFixed succeeds");
    assert_eq!(result, Value::from("3.29"));
}

#[test]
fn number_round() {
    let n = NumberValue::new(2.7);
    let result = n.call("round", &[]).expect("round succeeds");
    assert_eq!(result, Value::from(3.0));
}

#[test]
fn number_round_with_digits() {
    let n = NumberValue::new(2.345);
    let result = n
        .call("round", &[Value::from(2.0)])
        .expect("round succeeds");
    assert_eq!(result, Value::from(2.35));
}

#[test]
fn number_abs() {
    let n = NumberValue::new(-5.0);
    let result = n.call("abs", &[]).expect("abs succeeds");
    assert_eq!(result, Value::from(5.0));
}

#[test]
fn number_ceil() {
    let n = NumberValue::new(2.1);
    let result = n.call("ceil", &[]).expect("ceil succeeds");
    assert_eq!(result, Value::from(3.0));
}

#[test]
fn number_floor() {
    let n = NumberValue::new(2.9);
    let result = n.call("floor", &[]).expect("floor succeeds");
    assert_eq!(result, Value::from(2.0));
}

// =============================================================================
// List Methods
// =============================================================================

#[test]
fn list_contains() {
    let l = ListValue::new(vec![Value::from(1.0), Value::from(2.0), Value::from(3.0)]);
    let result = l
        .call("contains", &[Value::from(2.0)])
        .expect("contains succeeds");
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn list_join() {
    let l = ListValue::new(vec![Value::from("a"), Value::from("b"), Value::from("c")]);
    let result = l.call("join", &[Value::from(", ")]).expect("join succeeds");
    assert_eq!(result, Value::from("a, b, c"));
}

#[test]
fn list_is_empty() {
    let empty = ListValue::new(vec![]);
    let result = empty.call("isEmpty", &[]).expect("isEmpty succeeds");
    assert_eq!(result, Value::Boolean(true));

    let non_empty = ListValue::new(vec![Value::from(1.0)]);
    let result = non_empty.call("isEmpty", &[]).expect("isEmpty succeeds");
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn list_contains_all() {
    let l = ListValue::new(vec![Value::from(1.0), Value::from(2.0), Value::from(3.0)]);
    let result = l
        .call("containsAll", &[Value::from(1.0), Value::from(3.0)])
        .expect("containsAll succeeds");
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn list_contains_any() {
    let l = ListValue::new(vec![Value::from(1.0), Value::from(2.0)]);
    let result = l
        .call("containsAny", &[Value::from(5.0), Value::from(2.0)])
        .expect("containsAny succeeds");
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn list_reverse() {
    let l = ListValue::new(vec![Value::from(1.0), Value::from(2.0), Value::from(3.0)]);
    let result = l.call("reverse", &[]).expect("reverse succeeds");
    match result {
        Value::List(r) => {
            assert_eq!(r.value[0], Value::from(3.0));
            assert_eq!(r.value[1], Value::from(2.0));
            assert_eq!(r.value[2], Value::from(1.0));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn list_sort() {
    let l = ListValue::new(vec![Value::from(3.0), Value::from(1.0), Value::from(2.0)]);
    let result = l.call("sort", &[]).expect("sort succeeds");
    match result {
        Value::List(r) => {
            assert_eq!(r.value[0], Value::from(1.0));
            assert_eq!(r.value[1], Value::from(2.0));
            assert_eq!(r.value[2], Value::from(3.0));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn list_flat() {
    let inner = Value::List(ListValue::new(vec![Value::from(2.0), Value::from(3.0)]));
    let l = ListValue::new(vec![Value::from(1.0), inner, Value::from(4.0)]);
    let result = l.call("flat", &[]).expect("flat succeeds");
    match result {
        Value::List(r) => {
            assert_eq!(r.value.len(), 4);
            assert_eq!(r.value[0], Value::from(1.0));
            assert_eq!(r.value[1], Value::from(2.0));
            assert_eq!(r.value[2], Value::from(3.0));
            assert_eq!(r.value[3], Value::from(4.0));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn list_unique() {
    let l = ListValue::new(vec![
        Value::from(1.0),
        Value::from(2.0),
        Value::from(1.0),
        Value::from(3.0),
    ]);
    let result = l.call("unique", &[]).expect("unique succeeds");
    match result {
        Value::List(r) => {
            assert_eq!(r.value.len(), 3);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn list_slice() {
    let l = ListValue::new(vec![
        Value::from(1.0),
        Value::from(2.0),
        Value::from(3.0),
        Value::from(4.0),
    ]);
    let result = l
        .call("slice", &[Value::from(1.0), Value::from(3.0)])
        .expect("slice succeeds");
    match result {
        Value::List(r) => {
            assert_eq!(r.value.len(), 2);
            assert_eq!(r.value[0], Value::from(2.0));
            assert_eq!(r.value[1], Value::from(3.0));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn list_first() {
    let l = ListValue::new(vec![Value::from(1.0), Value::from(2.0)]);
    let result = l.call("first", &[]).expect("first succeeds");
    assert_eq!(result, Value::from(1.0));
}

#[test]
fn list_first_empty_returns_null() {
    let l = ListValue::new(vec![]);
    let result = l.call("first", &[]).expect("first succeeds");
    assert_eq!(result, Value::Null);
}

#[test]
fn list_last() {
    let l = ListValue::new(vec![Value::from(1.0), Value::from(2.0)]);
    let result = l.call("last", &[]).expect("last succeeds");
    assert_eq!(result, Value::from(2.0));
}

// =============================================================================
// Date Methods
// =============================================================================

#[test]
fn date_format() {
    let d = DateValue::new(
        NaiveDate::from_ymd_opt(2025, 1, 15)
            .unwrap()
            .and_hms_opt(14, 30, 0)
            .unwrap(),
    );
    let result = d
        .call("format", &[Value::from("YYYY-MM-DD")])
        .expect("format succeeds");
    assert_eq!(result, Value::from("2025-01-15"));
}

#[test]
fn date_format_complex() {
    let d = DateValue::new(
        NaiveDate::from_ymd_opt(2025, 1, 15)
            .unwrap()
            .and_hms_opt(14, 30, 0)
            .unwrap(),
    );
    let result = d
        .call("format", &[Value::from("MMMM D, YYYY")])
        .expect("format succeeds");
    assert_eq!(result, Value::from("January 15, 2025"));
}

#[test]
fn date_time_method() {
    let d = DateValue::new(
        NaiveDate::from_ymd_opt(2025, 1, 15)
            .unwrap()
            .and_hms_opt(14, 30, 45)
            .unwrap(),
    );
    let result = d.call("time", &[]).expect("time succeeds");
    assert_eq!(result, Value::from("14:30:45"));
}

#[test]
fn date_is_empty() {
    let d = DateValue::new(
        NaiveDate::from_ymd_opt(2025, 1, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    );
    let result = d.call("isEmpty", &[]).expect("isEmpty succeeds");
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn date_field_year() {
    let d = DateValue::new(
        NaiveDate::from_ymd_opt(2025, 3, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    );
    assert_eq!(d.field("year"), Some(Value::from(2025.0)));
}

#[test]
fn date_field_month() {
    let d = DateValue::new(
        NaiveDate::from_ymd_opt(2025, 3, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    );
    assert_eq!(d.field("month"), Some(Value::from(3.0)));
}

#[test]
fn date_field_day() {
    let d = DateValue::new(
        NaiveDate::from_ymd_opt(2025, 3, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    );
    assert_eq!(d.field("day"), Some(Value::from(15.0)));
}

#[test]
fn date_field_hour() {
    let d = DateValue::new(
        NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(14, 30, 0)
            .unwrap(),
    );
    assert_eq!(d.field("hour"), Some(Value::from(14.0)));
}

#[test]
fn date_field_minute() {
    let d = DateValue::new(
        NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(14, 30, 0)
            .unwrap(),
    );
    assert_eq!(d.field("minute"), Some(Value::from(30.0)));
}

#[test]
fn date_field_second() {
    let d = DateValue::new(
        NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(14, 30, 45)
            .unwrap(),
    );
    assert_eq!(d.field("second"), Some(Value::from(45.0)));
}

// =============================================================================
// String Field
// =============================================================================

#[test]
fn string_field_length() {
    let s = StringValue::new("hello".to_string());
    assert_eq!(s.field("length"), Some(Value::from(5.0)));
}

// =============================================================================
// List Field
// =============================================================================

#[test]
fn list_field_length() {
    let l = ListValue::new(vec![Value::from(1.0), Value::from(2.0), Value::from(3.0)]);
    assert_eq!(l.field("length"), Some(Value::from(3.0)));
}
