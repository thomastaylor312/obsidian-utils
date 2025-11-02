use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level `.base` file structure.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BaseFile {
    #[serde(default)]
    pub filters: Option<FilterNode>,

    #[serde(default)]
    pub formulas: HashMap<String, String>,

    #[serde(default)]
    pub properties: HashMap<String, PropertyConfig>,

    #[serde(default)]
    pub views: Vec<View>,
}

/// Recursive filter structure supporting logical operators and expressions.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum FilterNode {
    And { and: Vec<FilterNode> },
    Or { or: Vec<FilterNode> },
    Not { not: Vec<FilterNode> },
    Expression(String),
}

/// Configuration for how a property should be displayed in views.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PropertyConfig {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}

/// Configuration for an individual view defined in the base file.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct View {
    #[serde(rename = "type")]
    pub ty: ViewType,
    pub name: Option<String>,

    #[serde(default)]
    pub filters: Option<FilterNode>,

    #[serde(default)]
    pub order: Vec<String>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub sort: Vec<SortField>,

    #[serde(default)]
    pub image: Option<String>,

    #[serde(rename = "columnSize", default)]
    pub column_size: HashMap<String, usize>,
}

/// Supported view types in `.base` files.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ViewType {
    Table,
    Cards,
    List,
    Map,
}

/// Sort descriptor for view ordering.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SortField {
    pub property: String,
    pub direction: SortDirection,
}

/// Sort direction options.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const EXAMPLE_YAML: &str = r#"filters:
  or:
    - file.hasTag("tag")
    - and:
        - file.hasTag("book")
        - file.hasLink("Textbook")
    - not:
        - file.hasTag("book")
        - file.inFolder("Required Reading")
formulas:
  formatted_price: 'if(price, price.toFixed(2) + " dollars")'
  ppu: "(price / age).toFixed(2)"
properties:
  status:
    displayName: Status
  formula.formatted_price:
    displayName: "Price"
  file.ext:
    displayName: Extension
views:
  - type: table
    name: "My table"
    limit: 10
    filters:
      and:
        - 'status != "done"'
        - or:
            - "formula.ppu > 5"
            - "price > 2.1"
    order:
      - file.name
      - file.ext
      - note.age
      - formula.ppu
      - formula.formatted_price
"#;

    #[test]
    fn deserialize_full_example() {
        let parsed = serde_norway::from_str::<BaseFile>(EXAMPLE_YAML)
            .expect("example YAML in documentation should parse");

        let expected_filters = FilterNode::Or {
            or: vec![
                FilterNode::Expression(r#"file.hasTag("tag")"#.to_string()),
                FilterNode::And {
                    and: vec![
                        FilterNode::Expression(r#"file.hasTag("book")"#.to_string()),
                        FilterNode::Expression(r#"file.hasLink("Textbook")"#.to_string()),
                    ],
                },
                FilterNode::Not {
                    not: vec![
                        FilterNode::Expression(r#"file.hasTag("book")"#.to_string()),
                        FilterNode::Expression(r#"file.inFolder("Required Reading")"#.to_string()),
                    ],
                },
            ],
        };

        assert_eq!(parsed.filters, Some(expected_filters));
        assert_eq!(parsed.formulas.len(), 2);
        assert_eq!(parsed.properties.len(), 3);
        assert_eq!(parsed.views.len(), 1);

        let view = &parsed.views[0];
        assert_eq!(view.ty, ViewType::Table);
        assert_eq!(view.limit, Some(10));
        assert!(view.filters.is_some());
        assert_eq!(
            view.order,
            vec![
                "file.name".to_string(),
                "file.ext".to_string(),
                "note.age".to_string(),
                "formula.ppu".to_string(),
                "formula.formatted_price".to_string()
            ]
        );
    }

    #[test]
    fn deserialize_minimal_base() {
        let yaml = "views: []\n";
        let parsed = serde_norway::from_str::<BaseFile>(yaml).expect("minimal base should parse");
        assert!(parsed.filters.is_none());
        assert!(parsed.formulas.is_empty());
        assert!(parsed.properties.is_empty());
        assert!(parsed.views.is_empty());
    }

    #[test]
    fn invalid_yaml_returns_error() {
        let yaml = "filters: 123";
        let err = serde_norway::from_str::<BaseFile>(yaml).expect_err("should reject invalid yaml");
        assert!(
            err.to_string().contains("FilterNode"),
            "error message should indicate invalid structure: {err}"
        );
    }

    #[test]
    fn deserialize_complex_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-vault/complex.base");
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read complex base fixture: {err}"));
        let parsed = serde_norway::from_str::<BaseFile>(&contents)
            .expect("complex base fixture should parse");

        assert_eq!(parsed.views.len(), 6);
        assert_eq!(
            parsed.formulas.get("MB"),
            Some(&"[(file.size / 1048576).toFixed(2).toString(), \"MB\"].join(\" \")".to_string())
        );

        let image_grid = &parsed.views[0];
        assert_eq!(image_grid.ty, ViewType::Cards);
        assert_eq!(image_grid.name.as_deref(), Some("Image grid"));
        assert_eq!(image_grid.sort.len(), 3);
        assert_eq!(image_grid.sort[0].property, "file.ctime");
        assert_eq!(image_grid.sort[0].direction, SortDirection::Desc);
        assert_eq!(image_grid.image.as_deref(), Some("file.file"));
        assert!(image_grid.column_size.is_empty());

        let image_table = &parsed.views[1];
        assert_eq!(image_table.ty, ViewType::Table);
        assert_eq!(image_table.name.as_deref(), Some("Image table"));
        assert_eq!(image_table.sort.len(), 2);
        assert_eq!(
            image_table.column_size.get("formula.image"),
            Some(&222usize)
        );
        assert_eq!(image_table.column_size.get("file.name"), Some(&317usize));
    }
}
