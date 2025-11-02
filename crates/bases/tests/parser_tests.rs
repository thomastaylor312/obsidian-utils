use nom::Finish;
use obsidian_bases::ast::{BinaryOperator, Expr, PropertyNamespace, PropertyRef, UnaryOperator};
use obsidian_bases::parser::parse_expression;

fn parse_ok(input: &str) -> Expr {
    let (rest, expr) = parse_expression(input)
        .finish()
        .expect("parse should succeed");
    assert!(
        rest.trim().is_empty(),
        "unexpected trailing input: {:?}",
        rest
    );
    expr
}

fn parse_err(input: &str) {
    assert!(
        parse_expression(input).finish().is_err(),
        "expression unexpectedly parsed: {}",
        input
    );
}

#[test]
fn literals_and_basic_types() {
    assert_eq!(parse_ok("42"), Expr::Integer(42));
    assert_eq!(
        parse_ok("-7"),
        Expr::UnaryOp {
            op: UnaryOperator::Neg,
            expr: Box::new(Expr::Integer(7)),
        }
    );
    assert_eq!(parse_ok("3.24"), Expr::Float(3.24));
    assert_eq!(parse_ok("true"), Expr::Boolean(true));
    assert_eq!(parse_ok("false"), Expr::Boolean(false));
    assert_eq!(parse_ok("null"), Expr::Null);

    assert_eq!(parse_ok(r#""hello""#), Expr::String("hello".into()));
    assert_eq!(
        parse_ok("'single quoted'"),
        Expr::String("single quoted".into())
    );
    assert_eq!(
        parse_ok(r#""with \"escape\"""#),
        Expr::String("with \"escape\"".into())
    );
    assert_eq!(
        parse_ok(r#"'with \'escape\''"#),
        Expr::String("with 'escape'".into())
    );
}

#[test]
fn bases_syntax() {
    // file.hasTag("tag")
    let expr = parse_ok("file.hasTag(\"tag\")");
    match expr {
        Expr::MethodCall { method, args, .. } => {
            assert_eq!(method, "hasTag");
            assert_eq!(args, vec![Expr::String("tag".into())]);
        }
        other => panic!("unexpected AST: {other:?}"),
    }

    // file.hasLink("Textbook")
    let expr = parse_ok("file.hasLink(\"Textbook\")");
    match expr {
        Expr::MethodCall { method, args, .. } => {
            assert_eq!(method, "hasLink");
            assert_eq!(args, vec![Expr::String("Textbook".into())]);
        }
        other => panic!("unexpected AST: {other:?}"),
    }

    // file.inFolder("Required Reading")
    let expr = parse_ok("file.inFolder(\"Required Reading\")");
    match expr {
        Expr::MethodCall { method, args, .. } => {
            assert_eq!(method, "inFolder");
            assert_eq!(args, vec![Expr::String("Required Reading".into())]);
        }
        other => panic!("unexpected AST: {other:?}"),
    }

    // if(price, price.toFixed(2) + " dollars")
    let expr = parse_ok("if(price, price.toFixed(2) + \" dollars\")");
    let Expr::FunctionCall { name, args } = expr else {
        panic!("unexpected AST");
    };
    assert_eq!(name, "if");
    assert_eq!(args.len(), 2);
    assert_eq!(
        args[0],
        Expr::Property(PropertyRef {
            namespace: PropertyNamespace::Note,
            path: vec!["price".into()],
        })
    );
    let Expr::BinaryOp { op, .. } = &args[1] else {
        panic!("expected binary op for string concatenation");
    };
    assert_eq!(*op, BinaryOperator::Add);

    // (price / age).toFixed(2)
    let expr = parse_ok("(price / age).toFixed(2)");
    let Expr::MethodCall { method, args, .. } = expr else {
        panic!("expected method call");
    };
    assert_eq!(method, "toFixed");
    assert_eq!(args, vec![Expr::Integer(2)]);

    // status != "done"
    let expr = parse_ok("status != \"done\"");
    let Expr::BinaryOp { op, left, right } = expr else {
        panic!("expected binary op");
    };
    assert_eq!(op, BinaryOperator::Ne);
    assert!(matches!(
        *left,
        Expr::Property(PropertyRef {
            namespace: PropertyNamespace::Note,
            ref path
        }) if path.as_slice() == ["status"]
    ));
    assert_eq!(*right, Expr::String("done".into()));

    // formula.ppu > 5
    let expr = parse_ok("formula.ppu > 5");
    let Expr::BinaryOp { left, right, .. } = expr else {
        panic!("expected binary op");
    };
    assert!(matches!(
        *left.clone(),
        Expr::Property(PropertyRef {
            namespace: PropertyNamespace::Formula,
            ref path
        }) if path.as_slice() == ["ppu"]
    ));
    assert_eq!(*right, Expr::Integer(5));
}

#[test]
fn property_namespace() {
    assert_eq!(
        parse_ok("note.age"),
        Expr::Property(PropertyRef {
            namespace: PropertyNamespace::Note,
            path: vec!["age".into()],
        })
    );

    assert_eq!(
        parse_ok("file.ext"),
        Expr::Property(PropertyRef {
            namespace: PropertyNamespace::File,
            path: vec!["ext".into()],
        })
    );

    assert_eq!(
        parse_ok("formula.formatted_price"),
        Expr::Property(PropertyRef {
            namespace: PropertyNamespace::Formula,
            path: vec!["formatted_price".into()],
        })
    );

    assert_eq!(
        parse_ok("this.file.folder"),
        Expr::Property(PropertyRef {
            namespace: PropertyNamespace::This,
            path: vec!["file".into(), "folder".into()],
        })
    );
}

#[test]
fn global_function_parse() {
    let cases = [
        (
            r#"file(link("[[filename]]"))"#,
            Expr::FunctionCall {
                name: "file".into(),
                args: vec![Expr::FunctionCall {
                    name: "link".into(),
                    args: vec![Expr::String("[[filename]]".into())],
                }],
            },
        ),
        (
            r#"if(isModified, "Modified", "Unmodified")"#,
            Expr::FunctionCall {
                name: "if".into(),
                args: vec![
                    Expr::Property(PropertyRef {
                        namespace: PropertyNamespace::Note,
                        path: vec!["isModified".into()],
                    }),
                    Expr::String("Modified".into()),
                    Expr::String("Unmodified".into()),
                ],
            },
        ),
        (
            r#"image("https://obsidian.md/images/obsidian-logo-gradient.svg")"#,
            Expr::FunctionCall {
                name: "image".into(),
                args: vec![Expr::String(
                    "https://obsidian.md/images/obsidian-logo-gradient.svg".into(),
                )],
            },
        ),
        (
            r#"icon("arrow-right")"#,
            Expr::FunctionCall {
                name: "icon".into(),
                args: vec![Expr::String("arrow-right".into())],
            },
        ),
        (
            r#"list("value")"#,
            Expr::FunctionCall {
                name: "list".into(),
                args: vec![Expr::String("value".into())],
            },
        ),
        (
            r#"number("3.4")"#,
            Expr::FunctionCall {
                name: "number".into(),
                args: vec![Expr::String("3.4".into())],
            },
        ),
        (
            r#"duration('1d')"#,
            Expr::FunctionCall {
                name: "duration".into(),
                args: vec![Expr::String("1d".into())],
            },
        ),
        (
            r#"today()"#,
            Expr::FunctionCall {
                name: "today".into(),
                args: vec![],
            },
        ),
        (
            r#"now()"#,
            Expr::FunctionCall {
                name: "now".into(),
                args: vec![],
            },
        ),
    ];

    for (input, expected) in cases {
        assert_eq!(parse_ok(input), expected, "input: {input}");
    }
}

#[test]
fn method_and_chained_parse() {
    let cases = [
        (
            r#""hello".contains("ell")"#,
            Expr::MethodCall {
                object: Box::new(Expr::String("hello".into())),
                method: "contains".into(),
                args: vec![Expr::String("ell".into())],
            },
        ),
        (
            r#""hello".containsAll("h", "e")"#,
            Expr::MethodCall {
                object: Box::new(Expr::String("hello".into())),
                method: "containsAll".into(),
                args: vec![Expr::String("h".into()), Expr::String("e".into())],
            },
        ),
        (
            r#""hello".containsAny("x", "y", "e")"#,
            Expr::MethodCall {
                object: Box::new(Expr::String("hello".into())),
                method: "containsAny".into(),
                args: vec![
                    Expr::String("x".into()),
                    Expr::String("y".into()),
                    Expr::String("e".into()),
                ],
            },
        ),
        (
            r#""hello".endsWith("lo")"#,
            Expr::MethodCall {
                object: Box::new(Expr::String("hello".into())),
                method: "endsWith".into(),
                args: vec![Expr::String("lo".into())],
            },
        ),
        (
            r#""Hello world".isEmpty()"#,
            Expr::MethodCall {
                object: Box::new(Expr::String("Hello world".into())),
                method: "isEmpty".into(),
                args: vec![],
            },
        ),
        (
            r#""hello".reverse()"#,
            Expr::MethodCall {
                object: Box::new(Expr::String("hello".into())),
                method: "reverse".into(),
                args: vec![],
            },
        ),
        (
            r#""hello".slice(1, 4)"#,
            Expr::MethodCall {
                object: Box::new(Expr::String("hello".into())),
                method: "slice".into(),
                args: vec![Expr::Integer(1), Expr::Integer(4)],
            },
        ),
        (
            r#""a,b,c,d".split(",", 3)"#,
            Expr::MethodCall {
                object: Box::new(Expr::String("a,b,c,d".into())),
                method: "split".into(),
                args: vec![Expr::String(",".into()), Expr::Integer(3)],
            },
        ),
        (
            r#""hello".startsWith("he")"#,
            Expr::MethodCall {
                object: Box::new(Expr::String("hello".into())),
                method: "startsWith".into(),
                args: vec![Expr::String("he".into())],
            },
        ),
        (
            r#""hello world".title()"#,
            Expr::MethodCall {
                object: Box::new(Expr::String("hello world".into())),
                method: "title".into(),
                args: vec![],
            },
        ),
        (
            r#""  hi  ".trim()"#,
            Expr::MethodCall {
                object: Box::new(Expr::String("  hi  ".into())),
                method: "trim".into(),
                args: vec![],
            },
        ),
        (
            r#"123.toString()"#,
            Expr::MethodCall {
                object: Box::new(Expr::Integer(123)),
                method: "toString".into(),
                args: vec![],
            },
        ),
        (
            r#"1.isTruthy()"#,
            Expr::MethodCall {
                object: Box::new(Expr::Integer(1)),
                method: "isTruthy".into(),
                args: vec![],
            },
        ),
        (
            r#"now().date().format("YYYY-MM-DD HH:mm:ss")"#,
            Expr::MethodCall {
                object: Box::new(Expr::MethodCall {
                    object: Box::new(Expr::FunctionCall {
                        name: "now".into(),
                        args: vec![],
                    }),
                    method: "date".into(),
                    args: vec![],
                }),
                method: "format".into(),
                args: vec![Expr::String("YYYY-MM-DD HH:mm:ss".into())],
            },
        ),
        (
            r#"now().time()"#,
            Expr::MethodCall {
                object: Box::new(Expr::FunctionCall {
                    name: "now".into(),
                    args: vec![],
                }),
                method: "time".into(),
                args: vec![],
            },
        ),
        (
            r#"file.mtime.relative()"#,
            Expr::MethodCall {
                object: Box::new(Expr::Property(PropertyRef {
                    namespace: PropertyNamespace::File,
                    path: vec!["mtime".into()],
                })),
                method: "relative".into(),
                args: vec![],
            },
        ),
        (
            r#"(-5).abs()"#,
            Expr::MethodCall {
                object: Box::new(Expr::UnaryOp {
                    op: UnaryOperator::Neg,
                    expr: Box::new(Expr::Integer(5)),
                }),
                method: "abs".into(),
                args: vec![],
            },
        ),
        (
            r#"(2.1).ceil()"#,
            Expr::MethodCall {
                object: Box::new(Expr::Float(2.1)),
                method: "ceil".into(),
                args: vec![],
            },
        ),
        (
            r#"(2.9).floor()"#,
            Expr::MethodCall {
                object: Box::new(Expr::Float(2.9)),
                method: "floor".into(),
                args: vec![],
            },
        ),
        (
            r#"(2.5).round()"#,
            Expr::MethodCall {
                object: Box::new(Expr::Float(2.5)),
                method: "round".into(),
                args: vec![],
            },
        ),
        (
            r#"(2.3333).round(2)"#,
            Expr::MethodCall {
                object: Box::new(Expr::Float(2.3333)),
                method: "round".into(),
                args: vec![Expr::Integer(2)],
            },
        ),
        (
            r#"(3.723).toFixed(2)"#,
            Expr::MethodCall {
                object: Box::new(Expr::Float(3.723)),
                method: "toFixed".into(),
                args: vec![Expr::Integer(2)],
            },
        ),
        (
            r#"link("[[filename]]").asFile()"#,
            Expr::MethodCall {
                object: Box::new(Expr::FunctionCall {
                    name: "link".into(),
                    args: vec![Expr::String("[[filename]]".into())],
                }),
                method: "asFile".into(),
                args: vec![],
            },
        ),
        (
            r#"file.asLink()"#,
            Expr::MethodCall {
                object: Box::new(Expr::Property(PropertyRef {
                    namespace: PropertyNamespace::Note,
                    path: vec!["file".into()],
                })),
                method: "asLink".into(),
                args: vec![],
            },
        ),
        (
            r#"file.hasLink(otherFile)"#,
            Expr::MethodCall {
                object: Box::new(Expr::Property(PropertyRef {
                    namespace: PropertyNamespace::Note,
                    path: vec!["file".into()],
                })),
                method: "hasLink".into(),
                args: vec![Expr::Property(PropertyRef {
                    namespace: PropertyNamespace::Note,
                    path: vec!["otherFile".into()],
                })],
            },
        ),
        (
            r#"file.hasTag("tag1", "tag2")"#,
            Expr::MethodCall {
                object: Box::new(Expr::Property(PropertyRef {
                    namespace: PropertyNamespace::Note,
                    path: vec!["file".into()],
                })),
                method: "hasTag".into(),
                args: vec![Expr::String("tag1".into()), Expr::String("tag2".into())],
            },
        ),
        (
            r#"file.inFolder("notes")"#,
            Expr::MethodCall {
                object: Box::new(Expr::Property(PropertyRef {
                    namespace: PropertyNamespace::Note,
                    path: vec!["file".into()],
                })),
                method: "inFolder".into(),
                args: vec![Expr::String("notes".into())],
            },
        ),
    ];

    for (input, expected) in cases {
        assert_eq!(parse_ok(input), expected, "input: {input}");
    }
}

#[test]
fn operator_precedence() {
    let expr = parse_ok("2 + 3 * 4");
    assert_eq!(
        expr,
        Expr::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(Expr::Integer(2)),
            right: Box::new(Expr::BinaryOp {
                op: BinaryOperator::Mul,
                left: Box::new(Expr::Integer(3)),
                right: Box::new(Expr::Integer(4)),
            }),
        }
    );

    let expr = parse_ok("(2 + 3) * 4");
    assert_eq!(
        expr,
        Expr::BinaryOp {
            op: BinaryOperator::Mul,
            left: Box::new(Expr::BinaryOp {
                op: BinaryOperator::Add,
                left: Box::new(Expr::Integer(2)),
                right: Box::new(Expr::Integer(3)),
            }),
            right: Box::new(Expr::Integer(4)),
        }
    );
}

#[test]
fn boolean_logic_example() {
    let expr = parse_ok("status != \"done\" && price > 10");
    let Expr::BinaryOp { op, left, right } = expr else {
        panic!("expected top-level &&");
    };
    assert_eq!(op, BinaryOperator::And);

    match (&*left, &*right) {
        (
            Expr::BinaryOp {
                op: BinaryOperator::Ne,
                ..
            },
            Expr::BinaryOp {
                op: BinaryOperator::Gt,
                ..
            },
        ) => {}
        other => panic!("unexpected subexpressions: {other:?}"),
    }
}

#[test]
fn whitespace_and_invalid_error() {
    parse_err("functionName (arg)");
    parse_err("file. tags");
}

#[test]
fn unsupported_literal_error() {
    parse_err("note[\"price\"]");
    parse_err("[1,2,3].contains(2)");
    parse_err("{}.isEmpty()");
    parse_err("/abc/.matches(\"abcde\")");
    parse_err("\"a,b,c,d\".replace(/,/, \"-\")");
}

#[test]
fn error_messages_are_user_friendly() {
    // Test that error messages contain helpful context
    let result = parse_expression("123abc");
    assert!(result.is_err());
    if let Err(nom::Err::Error(err)) = result {
        let message = err.to_string();
        // Should have a descriptive message (not just an error code)
        assert!(
            message.contains("expected") || message.contains("identifier"),
            "Error message should be descriptive: {}",
            message
        );
        // Should include location context
        assert!(
            message.contains("123abc") || message.contains("at:"),
            "Error message should include input context: {}",
            message
        );
    }

    // Test incomplete expression
    let result = parse_expression("note.");
    assert!(result.is_err());
    if let Err(nom::Err::Error(err)) = result {
        let message = err.to_string();
        // Should give context about what was expected
        assert!(!message.is_empty(), "Error message should not be empty");
        assert!(
            message.contains("expected") || message.contains("identifier"),
            "Error should explain what's wrong: {}",
            message
        );
    }

    // Test trailing content
    let result = parse_expression("42 extra stuff");
    assert!(result.is_err());
    if let Err(nom::Err::Error(err)) = result {
        let message = err.to_string();
        assert!(
            message.contains("unexpected") && message.contains("content"),
            "Error should mention unexpected content: {}",
            message
        );
    }

    // Test invalid syntax at beginning
    let result = parse_expression("@#$");
    assert!(result.is_err());
    if let Err(nom::Err::Error(err)) = result {
        let message = err.to_string();
        assert!(
            !message.is_empty() && (message.contains("expected") || message.contains("unexpected")),
            "Error should be descriptive for invalid input: {}",
            message
        );
    }
}
