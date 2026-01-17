//! Expression formatter for different output styles.
//!
//! Formats postcoordinated expressions in brief, long, or nested styles.

use super::ast::{AttributeValue, ConceptReference, Expression, ExpressionOperator, RoleGroup};

/// Output format for expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Format {
    /// Brief format: IDs only.
    /// Example: `29857009:{246112005=24484000}`
    Brief,

    /// Long format: IDs with terms (default).
    /// Example: `29857009 |Chest pain| : 246112005 |Severity| = 24484000 |Severe|`
    #[default]
    Long,

    /// Nested format: Multi-line with indentation.
    Nested,
}

/// Formatted expression output in all styles.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FormattedExpression {
    /// Brief format output.
    pub brief: String,
    /// Long format output.
    pub long: String,
    /// Nested format output.
    pub nested: String,
}

/// Expression formatter.
#[derive(Debug, Clone, Default)]
pub struct Formatter {
    _private: (),
}

impl Formatter {
    /// Create a new formatter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Format an expression in the specified style.
    pub fn format(&self, expr: &Expression, format: Format) -> String {
        Self::format_expression(expr, format)
    }

    /// Format an expression (static method).
    pub fn format_expression(expr: &Expression, format: Format) -> String {
        match format {
            Format::Brief => Self::format_brief(expr),
            Format::Long => Self::format_long(expr),
            Format::Nested => Self::format_nested(expr),
        }
    }

    /// Format in all styles.
    pub fn format_all(expr: &Expression) -> FormattedExpression {
        FormattedExpression {
            brief: Self::format_brief(expr),
            long: Self::format_long(expr),
            nested: Self::format_nested(expr),
        }
    }

    // =========================================================================
    // Brief format
    // =========================================================================

    fn format_brief(expr: &Expression) -> String {
        let separator = Self::operator_separator(expr.operator, false);
        let focus = expr
            .focus
            .iter()
            .map(|c| c.id.to_string())
            .collect::<Vec<_>>()
            .join(&separator);

        if expr.refinements.is_empty() {
            return focus;
        }

        let refinements = Self::format_refinements_brief(&expr.refinements);
        format!("{}:{}", focus, refinements)
    }

    fn format_refinements_brief(role_groups: &[RoleGroup]) -> String {
        let parts: Vec<String> = role_groups
            .iter()
            .map(|rg| {
                let attrs: Vec<String> = rg
                    .attributes
                    .iter()
                    .map(|a| {
                        let value_str = Self::format_value_brief(&a.value);
                        format!("{}={}", a.attribute_type.id, value_str)
                    })
                    .collect();

                if rg.group == 0 {
                    attrs.join(",")
                } else {
                    format!("{{{}}}", attrs.join(","))
                }
            })
            .collect();

        parts.join(",")
    }

    fn format_value_brief(value: &AttributeValue) -> String {
        match value {
            AttributeValue::Concept(c) => c.id.to_string(),
            AttributeValue::Expression(e) => format!("({})", Self::format_brief(e)),
        }
    }

    // =========================================================================
    // Long format
    // =========================================================================

    fn format_long(expr: &Expression) -> String {
        let separator = Self::operator_separator(expr.operator, true);
        let focus = expr
            .focus
            .iter()
            .map(|c| Self::format_concept(c))
            .collect::<Vec<_>>()
            .join(&separator);

        if expr.refinements.is_empty() {
            return focus;
        }

        let refinements = Self::format_refinements_long(&expr.refinements);
        format!("{} : {}", focus, refinements)
    }

    fn format_refinements_long(role_groups: &[RoleGroup]) -> String {
        let parts: Vec<String> = role_groups
            .iter()
            .map(|rg| {
                let attrs: Vec<String> = rg
                    .attributes
                    .iter()
                    .map(|a| {
                        let value_str = Self::format_value_long(&a.value);
                        format!(
                            "{} = {}",
                            Self::format_concept(&a.attribute_type),
                            value_str
                        )
                    })
                    .collect();

                if rg.group == 0 {
                    attrs.join(", ")
                } else {
                    format!("{{ {} }}", attrs.join(", "))
                }
            })
            .collect();

        parts.join(", ")
    }

    fn format_value_long(value: &AttributeValue) -> String {
        match value {
            AttributeValue::Concept(c) => Self::format_concept(c),
            AttributeValue::Expression(e) => format!("({})", Self::format_long(e)),
        }
    }

    // =========================================================================
    // Nested format
    // =========================================================================

    fn format_nested(expr: &Expression) -> String {
        Self::format_nested_with_indent(expr, 0)
    }

    fn format_nested_with_indent(expr: &Expression, indent: usize) -> String {
        let mut lines = Vec::new();
        let base_indent = "  ".repeat(indent);
        let attr_indent = "  ".repeat(indent + 1);
        let group_attr_indent = "  ".repeat(indent + 2);

        // Focus concepts with operator
        let separator = Self::operator_separator_nested(expr.operator);
        let focus = expr
            .focus
            .iter()
            .map(|c| Self::format_concept(c))
            .collect::<Vec<_>>()
            .join(&format!("{}\n{}", separator, base_indent));
        lines.push(format!("{}{}", base_indent, focus));

        if !expr.refinements.is_empty() {
            lines.push(format!("{}:", base_indent));

            for rg in &expr.refinements {
                if rg.group == 0 {
                    // Ungrouped attributes
                    for attr in &rg.attributes {
                        let value_str = Self::format_value_nested(&attr.value, indent + 1);
                        lines.push(format!(
                            "{}{} = {}",
                            attr_indent,
                            Self::format_concept(&attr.attribute_type),
                            value_str
                        ));
                    }
                } else {
                    // Grouped attributes
                    lines.push(format!("{}{{", attr_indent));
                    for attr in &rg.attributes {
                        let value_str = Self::format_value_nested(&attr.value, indent + 2);
                        lines.push(format!(
                            "{}{} = {}",
                            group_attr_indent,
                            Self::format_concept(&attr.attribute_type),
                            value_str
                        ));
                    }
                    lines.push(format!("{}}}", attr_indent));
                }
            }
        }

        // Remove base indent from first line if at root level
        if indent == 0 {
            lines.join("\n").trim_start().to_string()
        } else {
            lines.join("\n")
        }
    }

    fn format_value_nested(value: &AttributeValue, indent: usize) -> String {
        match value {
            AttributeValue::Concept(c) => Self::format_concept(c),
            AttributeValue::Expression(e) => {
                format!(
                    "(\n{}\n{})",
                    Self::format_nested_with_indent(e, indent + 1),
                    "  ".repeat(indent)
                )
            }
        }
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    fn format_concept(c: &ConceptReference) -> String {
        if let Some(ref term) = c.term {
            format!("{} |{}|", c.id, term)
        } else {
            c.id.to_string()
        }
    }

    fn operator_separator(operator: Option<ExpressionOperator>, spaced: bool) -> String {
        let sep = match operator {
            Some(ExpressionOperator::And) | None => "+",
            Some(ExpressionOperator::Or) => "OR",
            Some(ExpressionOperator::Minus) => "MINUS",
        };
        if spaced {
            format!(" {} ", sep)
        } else {
            format!(" {} ", sep)
        }
    }

    fn operator_separator_nested(operator: Option<ExpressionOperator>) -> String {
        match operator {
            Some(ExpressionOperator::And) | None => " +".to_string(),
            Some(ExpressionOperator::Or) => " OR".to_string(),
            Some(ExpressionOperator::Minus) => " MINUS".to_string(),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::ast::{Attribute, Expression, ExpressionOperator};

    fn sample_precoordinated() -> Expression {
        Expression::precoordinated(29857009, "Chest pain")
    }

    fn sample_postcoordinated() -> Expression {
        let mut expr = Expression::precoordinated(29857009, "Chest pain");
        expr.add_ungrouped_attribute(Attribute::new(246112005, "Severity", 24484000, "Severe"));
        expr
    }

    fn sample_with_groups() -> Expression {
        let mut expr = Expression::precoordinated(29857009, "Chest pain");
        expr.add_ungrouped_attribute(Attribute::new(246112005, "Severity", 24484000, "Severe"));
        expr.add_grouped_attribute(
            1,
            Attribute::new(363698007, "Finding site", 368208006, "Left upper arm structure"),
        );
        expr
    }

    #[test]
    fn test_format_brief_precoordinated() {
        let expr = sample_precoordinated();
        let result = Formatter::format_expression(&expr, Format::Brief);
        assert_eq!(result, "29857009");
    }

    #[test]
    fn test_format_brief_postcoordinated() {
        let expr = sample_postcoordinated();
        let result = Formatter::format_expression(&expr, Format::Brief);
        assert_eq!(result, "29857009:246112005=24484000");
    }

    #[test]
    fn test_format_brief_with_groups() {
        let expr = sample_with_groups();
        let result = Formatter::format_expression(&expr, Format::Brief);
        assert_eq!(result, "29857009:246112005=24484000,{363698007=368208006}");
    }

    #[test]
    fn test_format_long_precoordinated() {
        let expr = sample_precoordinated();
        let result = Formatter::format_expression(&expr, Format::Long);
        assert_eq!(result, "29857009 |Chest pain|");
    }

    #[test]
    fn test_format_long_postcoordinated() {
        let expr = sample_postcoordinated();
        let result = Formatter::format_expression(&expr, Format::Long);
        assert_eq!(
            result,
            "29857009 |Chest pain| : 246112005 |Severity| = 24484000 |Severe|"
        );
    }

    #[test]
    fn test_format_long_compound() {
        let expr = Expression::compound(
            vec![
                ConceptReference::with_term(29857009, "Chest pain"),
                ConceptReference::with_term(267036007, "Dyspnea"),
            ],
            ExpressionOperator::And,
        );
        let result = Formatter::format_expression(&expr, Format::Long);
        assert_eq!(
            result,
            "29857009 |Chest pain| + 267036007 |Dyspnea|"
        );
    }

    #[test]
    fn test_format_long_or() {
        let expr = Expression::compound(
            vec![
                ConceptReference::with_term(29857009, "Chest pain"),
                ConceptReference::with_term(267036007, "Dyspnea"),
            ],
            ExpressionOperator::Or,
        );
        let result = Formatter::format_expression(&expr, Format::Long);
        assert!(result.contains(" OR "));
    }

    #[test]
    fn test_format_nested() {
        let expr = sample_postcoordinated();
        let result = Formatter::format_expression(&expr, Format::Nested);
        assert!(result.contains("29857009 |Chest pain|"));
        assert!(result.contains(":"));
        assert!(result.contains("246112005 |Severity|"));
    }

    #[test]
    fn test_format_all() {
        let expr = sample_postcoordinated();
        let result = Formatter::format_all(&expr);
        assert!(!result.brief.is_empty());
        assert!(!result.long.is_empty());
        assert!(!result.nested.is_empty());
    }

    #[test]
    fn test_format_nested_expression_value() {
        let nested = Expression::precoordinated(368208006, "Left upper arm structure");
        let mut expr = Expression::precoordinated(29857009, "Chest pain");
        expr.add_ungrouped_attribute(Attribute::with_expression(363698007, "Finding site", nested));

        let result = Formatter::format_expression(&expr, Format::Long);
        assert!(result.contains("("));
        assert!(result.contains("368208006"));
    }
}
