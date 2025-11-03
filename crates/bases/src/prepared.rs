//! Prepared versions of schema structures with parsed expressions.
//!
//! This module converts the deserialized [`BaseFile`](crate::schema::BaseFile)
//! into a representation where every string expression has already been parsed
//! into an [`Expr`](crate::ast::Expr`). Downstream stages can rely on these
//! prepared structures without re-parsing strings at evaluation time.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use nom::Finish;

use crate::ast::{Expr, PropertyRef};
use crate::parser::parse_expression;
use crate::schema::{BaseFile, FilterNode, PropertyConfig, SortField, View, ViewType};

/// Prepared representation of a base file with parsed expressions.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedBase {
    original: BaseFile,
    pub filters: Option<PreparedFilter>,
    pub formulas: HashMap<String, Expr>,
    pub properties: HashMap<String, PropertyConfig>,
    pub views: Vec<PreparedView>,
}

impl PreparedBase {
    /// Returns the original `BaseFile` used to construct this prepared version.
    pub fn original(&self) -> &BaseFile {
        &self.original
    }
}

/// Prepared representation of an individual view with parsed filters and order.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedView {
    pub ty: ViewType,
    pub name: Option<String>,
    pub filters: Option<PreparedFilter>,
    pub order: Vec<PropertyRef>,
    pub limit: Option<usize>,
    pub sort: Vec<SortField>,
    pub image: Option<String>,
    pub column_size: HashMap<String, usize>,
}

/// Prepared version of a filter tree with string expressions parsed into `Expr`.
#[derive(Debug, Clone, PartialEq)]
pub enum PreparedFilter {
    And(Vec<PreparedFilter>),
    Or(Vec<PreparedFilter>),
    Not(Vec<PreparedFilter>),
    Expr(Expr),
}

impl PreparedBase {
    /// Convert a deserialized `BaseFile` into a prepared representation.
    pub fn from_base(base: BaseFile) -> Result<PreparedBase> {
        ensure_unique_view_names(&base)?;

        let filters = base
            .filters
            .as_ref()
            .map(|node| convert_filter_node(node, "base.filters"))
            .transpose()?;

        let formulas = parse_formula_map(&base.formulas)?;

        let properties = base.properties.clone();

        let views = base
            .views
            .iter()
            .enumerate()
            .map(|(idx, view)| convert_view(view, idx))
            .collect::<Result<Vec<_>>>()?;

        Ok(PreparedBase {
            original: base,
            filters,
            formulas,
            properties,
            views,
        })
    }
}

impl TryFrom<BaseFile> for PreparedBase {
    type Error = anyhow::Error;

    fn try_from(base: BaseFile) -> Result<Self> {
        Self::from_base(base)
    }
}

fn ensure_unique_view_names(base: &BaseFile) -> Result<()> {
    let mut seen = HashMap::new();
    for (idx, view) in base.views.iter().enumerate() {
        if let Some(name) = &view.name
            && let Some(previous) = seen.insert(name.clone(), idx)
        {
            bail!("Duplicate view name '{name}' detected at indices {previous} and {idx}");
        }
    }
    Ok(())
}

fn convert_view(view: &View, index: usize) -> Result<PreparedView> {
    let context = view_context(view, index);

    let filters = view
        .filters
        .as_ref()
        .map(|node| convert_filter_node(node, &format!("{context}.filters")))
        .transpose()?;

    let order = parse_order(&view.order, &format!("{context}.order"))?;

    Ok(PreparedView {
        ty: view.ty,
        name: view.name.clone(),
        filters,
        order,
        limit: view.limit,
        sort: view.sort.clone(),
        image: view.image.clone(),
        column_size: view.column_size.clone(),
    })
}

fn convert_filter_node(node: &FilterNode, context: &str) -> Result<PreparedFilter> {
    match node {
        FilterNode::And { and } => {
            let mut converted = Vec::with_capacity(and.len());
            for (idx, child) in and.iter().enumerate() {
                let child_context = format!("{context}.and[{idx}]");
                converted.push(convert_filter_node(child, &child_context)?);
            }
            Ok(PreparedFilter::And(converted))
        }
        FilterNode::Or { or } => {
            let mut converted = Vec::with_capacity(or.len());
            for (idx, child) in or.iter().enumerate() {
                let child_context = format!("{context}.or[{idx}]");
                converted.push(convert_filter_node(child, &child_context)?);
            }
            Ok(PreparedFilter::Or(converted))
        }
        FilterNode::Not { not } => {
            let mut converted = Vec::with_capacity(not.len());
            for (idx, child) in not.iter().enumerate() {
                let child_context = format!("{context}.not[{idx}]");
                converted.push(convert_filter_node(child, &child_context)?);
            }
            Ok(PreparedFilter::Not(converted))
        }
        FilterNode::Expression(expr) => {
            let (_, parsed) = parse_expression(expr)
                .finish()
                .map_err(|err| anyhow::anyhow!(err.to_string()))
                .with_context(|| format!("Failed to parse filter expression at {context}"))?;
            Ok(PreparedFilter::Expr(parsed))
        }
    }
}

fn parse_formula_map(formulas: &HashMap<String, String>) -> Result<HashMap<String, Expr>> {
    formulas
        .iter()
        .map(|(name, expr)| {
            parse_expression(expr)
                .finish()
                .map(|(_, parsed)| (name.clone(), parsed))
                .map_err(|err| anyhow::anyhow!(err.to_string()))
                .with_context(|| format!("Failed to parse formula '{name}'"))
        })
        .collect()
}

fn parse_order(entries: &[String], context: &str) -> Result<Vec<PropertyRef>> {
    entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let (_, parsed) = parse_expression(entry)
                .finish()
                .map_err(|err| anyhow::anyhow!(err.to_string()))
                .with_context(|| {
                    format!(
                        "Failed to parse order entry '{}' at {}[{}]",
                        entry, context, idx
                    )
                })?;

            if let Expr::Property(prop) = parsed {
                Ok(prop)
            } else {
                bail!(
                    "Order entry '{}' at {}[{}] must be a property reference",
                    entry,
                    context,
                    idx
                );
            }
        })
        .collect()
}

fn view_context(view: &View, index: usize) -> String {
    match &view.name {
        Some(name) => format!("view '{name}' (index {index})"),
        None => format!("view at index {index}"),
    }
}
