use std::collections::HashMap;

use anyhow::Result;
use obsidian_bases::{
    BaseFile, FilterNode, PreparedBase, PreparedFilter, PropertyConfig, SortDirection, SortField,
    View, ViewType, ast::PropertyNamespace, parser::parse_expression,
};

#[test]
fn prepare_base_parses_structures() -> Result<()> {
    let mut properties = HashMap::new();
    properties.insert(
        "status".to_string(),
        PropertyConfig {
            display_name: Some("Status".to_string()),
        },
    );

    let view_filter = FilterNode::And {
        and: vec![
            FilterNode::Expression(r#"status != "done""#.to_string()),
            FilterNode::Or {
                or: vec![
                    FilterNode::Expression("price > 10".to_string()),
                    FilterNode::Expression("formula.ppu > 5".to_string()),
                ],
            },
        ],
    };

    let base = BaseFile {
        filters: Some(FilterNode::Expression(
            r#"file.hasTag("important")"#.to_string(),
        )),
        formulas: HashMap::from([(
            "ppu".to_string(),
            "(price / quantity).toFixed(2)".to_string(),
        )]),
        properties: properties.clone(),
        views: vec![View {
            ty: ViewType::Table,
            name: Some("main".to_string()),
            filters: Some(view_filter),
            order: vec!["file.name".to_string(), "formula.ppu".to_string()],
            limit: Some(50),
            sort: vec![SortField {
                property: "file.name".to_string(),
                direction: SortDirection::Asc,
            }],
            image: Some("cover".to_string()),
            column_size: HashMap::from([("file.name".to_string(), 200)]),
        }],
    };

    let base_clone = base.clone();
    let prepared = PreparedBase::from_base(base)?;

    let (_, expected_global) = parse_expression(r#"file.hasTag("important")"#)?;
    assert_eq!(
        prepared.filters,
        Some(PreparedFilter::Expr(expected_global))
    );

    assert_eq!(prepared.properties, properties);
    assert_eq!(prepared.original(), &base_clone);

    let (_, ppu_expr) = parse_expression("(price / quantity).toFixed(2)")?;
    assert_eq!(prepared.formulas.get("ppu"), Some(&ppu_expr));

    assert_eq!(prepared.views.len(), 1);
    let view = &prepared.views[0];
    assert_eq!(view.ty, ViewType::Table);
    assert_eq!(view.name.as_deref(), Some("main"));
    assert_eq!(view.limit, Some(50));
    assert_eq!(view.image.as_deref(), Some("cover"));
    assert_eq!(view.column_size.get("file.name"), Some(&200));

    // Validate parsed view filter structure.
    let Some(PreparedFilter::And(children)) = &view.filters else {
        panic!("expected AND filter");
    };
    assert_eq!(children.len(), 2);

    let (_, first_child) = parse_expression(r#"status != "done""#)?;
    assert_eq!(children[0], PreparedFilter::Expr(first_child));

    let PreparedFilter::Or(grand_children) = &children[1] else {
        panic!("expected OR in second child");
    };
    assert_eq!(grand_children.len(), 2);
    let (_, expected_left) = parse_expression("price > 10")?;
    let (_, expected_right) = parse_expression("formula.ppu > 5")?;
    assert_eq!(grand_children[0], PreparedFilter::Expr(expected_left));
    assert_eq!(grand_children[1], PreparedFilter::Expr(expected_right));

    // Validate order property references.
    assert_eq!(view.order.len(), 2);
    let file_name = &view.order[0];
    assert_eq!(file_name.namespace, PropertyNamespace::File);
    assert_eq!(file_name.path, vec!["name".to_string()]);

    let formula_ppu = &view.order[1];
    assert_eq!(formula_ppu.namespace, PropertyNamespace::Formula);
    assert_eq!(formula_ppu.path, vec!["ppu".to_string()]);

    Ok(())
}

#[test]
fn prepare_base_reports_formula_errors() {
    let mut base = minimal_base();
    base.formulas.insert("bad".to_string(), "(".to_string());

    let err = PreparedBase::from_base(base).expect_err("formula parse should fail");
    assert!(err.to_string().contains("formula 'bad'"));
}

#[test]
fn prepare_base_reports_view_filter_errors() {
    let mut base = minimal_base();
    base.views[0].name = Some("invalid-filter".to_string());
    base.views[0].filters = Some(FilterNode::Expression("status ==".to_string()));

    let err = PreparedBase::from_base(base).expect_err("view filter parse should fail");
    assert!(
        err.to_string()
            .contains("view 'invalid-filter' (index 0).filters")
    );
}

#[test]
fn prepare_base_rejects_invalid_order_entry() {
    let mut base = minimal_base();
    base.views[0].name = Some("order-view".to_string());
    base.views[0].order = vec!["42".to_string()];

    let err = PreparedBase::from_base(base).expect_err("order parsing should fail");
    assert!(
        err.to_string()
            .contains("Order entry '42' at view 'order-view' (index 0).order[0]")
    );
}

#[test]
fn prepare_base_rejects_duplicate_names() {
    let mut base = minimal_base();
    base.views[0].name = Some("duplicate".to_string());
    base.views.push(View {
        ty: ViewType::Table,
        name: Some("duplicate".to_string()),
        filters: None,
        order: Vec::new(),
        limit: None,
        sort: Vec::new(),
        image: None,
        column_size: HashMap::new(),
    });

    let err = PreparedBase::from_base(base).expect_err("duplicate view names should fail");
    assert!(err.to_string().contains("Duplicate view name 'duplicate'"));
}

fn minimal_base() -> BaseFile {
    BaseFile {
        filters: None,
        formulas: HashMap::new(),
        properties: HashMap::new(),
        views: vec![View {
            ty: ViewType::Table,
            name: None,
            filters: None,
            order: Vec::new(),
            limit: None,
            sort: Vec::new(),
            image: None,
            column_size: HashMap::new(),
        }],
    }
}
