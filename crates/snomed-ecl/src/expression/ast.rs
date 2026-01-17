//! Postcoordinated SNOMED CT Expression AST types.
//!
//! These types represent postcoordinated expressions using SNOMED CT compositional grammar.
//! This is different from ECL (Expression Constraint Language) which is for querying.
//!
//! ## Compositional Grammar vs ECL
//!
//! - **Compositional Grammar**: Builds specific clinical concepts
//!   ```text
//!   29857009 |Chest pain| : 246112005 |Severity| = 24484000 |Severe|
//!   ```
//!
//! - **ECL**: Defines sets of concepts (constraints)
//!   ```text
//!   << 404684003 |Clinical finding|
//!   ```

use crate::SctId;

/// A complete postcoordinated SNOMED CT expression.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Expression {
    /// Focus concepts (one or more).
    pub focus: Vec<ConceptReference>,

    /// Refinements (attributes grouped by role group).
    pub refinements: Vec<RoleGroup>,

    /// Expression type.
    pub expression_type: ExpressionType,

    /// Operator for compound expressions (when multiple focus concepts).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub operator: Option<ExpressionOperator>,
}

/// Reference to a SNOMED CT concept.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConceptReference {
    /// SNOMED CT Identifier.
    pub id: SctId,

    /// Preferred term (optional).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub term: Option<String>,
}

/// A role group containing attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoleGroup {
    /// Group number (0 = ungrouped).
    pub group: u32,

    /// Attributes in this group.
    pub attributes: Vec<Attribute>,
}

/// An attribute (relationship type = value).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Attribute {
    /// Attribute type concept.
    pub attribute_type: ConceptReference,

    /// Attribute value - can be a concept or nested expression.
    pub value: AttributeValue,
}

/// Attribute value - either a simple concept reference or a nested expression.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum AttributeValue {
    /// Simple concept reference.
    Concept(ConceptReference),
    /// Nested expression.
    Expression(Box<Expression>),
}

/// Type of expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ExpressionType {
    /// Single concept, no refinement.
    Precoordinated,

    /// Single concept with refinements.
    Postcoordinated,

    /// Multiple focus concepts.
    Compound,
}

/// Operator for combining expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
pub enum ExpressionOperator {
    /// Conjunction: both expressions must match.
    And,
    /// Disjunction: either expression can match.
    Or,
    /// Exclusion: first minus second.
    Minus,
}

// =============================================================================
// Implementations
// =============================================================================

impl ConceptReference {
    /// Create a concept reference with just an ID.
    pub fn new(id: SctId) -> Self {
        Self { id, term: None }
    }

    /// Create a concept reference with ID and term.
    pub fn with_term(id: SctId, term: impl Into<String>) -> Self {
        Self {
            id,
            term: Some(term.into()),
        }
    }
}

impl Expression {
    /// Create a new precoordinated expression (single concept, no refinements).
    pub fn precoordinated(id: SctId, term: impl Into<String>) -> Self {
        Self {
            focus: vec![ConceptReference::with_term(id, term)],
            refinements: vec![],
            expression_type: ExpressionType::Precoordinated,
            operator: None,
        }
    }

    /// Create a precoordinated expression with just an ID.
    pub fn precoordinated_id(id: SctId) -> Self {
        Self {
            focus: vec![ConceptReference::new(id)],
            refinements: vec![],
            expression_type: ExpressionType::Precoordinated,
            operator: None,
        }
    }

    /// Create a compound expression with an operator.
    pub fn compound(focus: Vec<ConceptReference>, operator: ExpressionOperator) -> Self {
        Self {
            focus,
            refinements: vec![],
            expression_type: ExpressionType::Compound,
            operator: Some(operator),
        }
    }

    /// Check if expression has refinements.
    pub fn has_refinements(&self) -> bool {
        !self.refinements.is_empty()
    }

    /// Check if expression has nested values.
    pub fn has_nested_expressions(&self) -> bool {
        self.refinements
            .iter()
            .flat_map(|rg| &rg.attributes)
            .any(|attr| attr.value.is_expression())
    }

    /// Get all attributes across all role groups.
    pub fn all_attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.refinements.iter().flat_map(|rg| &rg.attributes)
    }

    /// Add an ungrouped attribute (group 0).
    pub fn add_ungrouped_attribute(&mut self, attr: Attribute) {
        if let Some(rg) = self.refinements.iter_mut().find(|rg| rg.group == 0) {
            rg.attributes.push(attr);
        } else {
            self.refinements.push(RoleGroup {
                group: 0,
                attributes: vec![attr],
            });
        }
        self.update_expression_type();
    }

    /// Add a grouped attribute.
    pub fn add_grouped_attribute(&mut self, group: u32, attr: Attribute) {
        if let Some(rg) = self.refinements.iter_mut().find(|rg| rg.group == group) {
            rg.attributes.push(attr);
        } else {
            self.refinements.push(RoleGroup {
                group,
                attributes: vec![attr],
            });
        }
        self.update_expression_type();
    }

    /// Set the operator for compound expressions.
    pub fn with_operator(mut self, operator: ExpressionOperator) -> Self {
        self.operator = Some(operator);
        self
    }

    fn update_expression_type(&mut self) {
        self.expression_type = if self.focus.len() > 1 {
            ExpressionType::Compound
        } else if self.has_refinements() {
            ExpressionType::Postcoordinated
        } else {
            ExpressionType::Precoordinated
        };
    }
}

impl Attribute {
    /// Create a new attribute with a concept value.
    pub fn new(
        attribute_id: SctId,
        attribute_term: impl Into<String>,
        value_id: SctId,
        value_term: impl Into<String>,
    ) -> Self {
        Self {
            attribute_type: ConceptReference::with_term(attribute_id, attribute_term),
            value: AttributeValue::Concept(ConceptReference::with_term(value_id, value_term)),
        }
    }

    /// Create a new attribute with concept references.
    pub fn from_concepts(attribute_type: ConceptReference, value: ConceptReference) -> Self {
        Self {
            attribute_type,
            value: AttributeValue::Concept(value),
        }
    }

    /// Create a new attribute with a nested expression value.
    pub fn with_expression(
        attribute_id: SctId,
        attribute_term: impl Into<String>,
        expr: Expression,
    ) -> Self {
        Self {
            attribute_type: ConceptReference::with_term(attribute_id, attribute_term),
            value: AttributeValue::Expression(Box::new(expr)),
        }
    }
}

impl AttributeValue {
    /// Create a concept value.
    pub fn concept(id: SctId, term: impl Into<String>) -> Self {
        Self::Concept(ConceptReference::with_term(id, term))
    }

    /// Create a concept value with just an ID.
    pub fn concept_id(id: SctId) -> Self {
        Self::Concept(ConceptReference::new(id))
    }

    /// Create a nested expression value.
    pub fn expression(expr: Expression) -> Self {
        Self::Expression(Box::new(expr))
    }

    /// Check if this is a concept value.
    pub fn is_concept(&self) -> bool {
        matches!(self, Self::Concept(_))
    }

    /// Check if this is an expression value.
    pub fn is_expression(&self) -> bool {
        matches!(self, Self::Expression(_))
    }

    /// Get the concept reference if this is a concept value.
    pub fn as_concept(&self) -> Option<&ConceptReference> {
        match self {
            Self::Concept(c) => Some(c),
            Self::Expression(_) => None,
        }
    }

    /// Get the expression if this is an expression value.
    pub fn as_expression(&self) -> Option<&Expression> {
        match self {
            Self::Concept(_) => None,
            Self::Expression(e) => Some(e),
        }
    }
}

// =============================================================================
// Display implementations
// =============================================================================

impl std::fmt::Display for ConceptReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref term) = self.term {
            write!(f, "{} |{}|", self.id, term)
        } else {
            write!(f, "{}", self.id)
        }
    }
}

impl std::fmt::Display for ExpressionOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionOperator::And => write!(f, "+"),
            ExpressionOperator::Or => write!(f, "OR"),
            ExpressionOperator::Minus => write!(f, "MINUS"),
        }
    }
}

impl std::fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeValue::Concept(c) => write!(f, "{}", c),
            AttributeValue::Expression(e) => write!(f, "({})", e),
        }
    }
}

impl std::fmt::Display for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.attribute_type, self.value)
    }
}

impl std::fmt::Display for RoleGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.group == 0 {
            // Ungrouped attributes
            let attrs: Vec<String> = self.attributes.iter().map(|a| a.to_string()).collect();
            write!(f, "{}", attrs.join(", "))
        } else {
            // Grouped attributes
            write!(f, "{{ ")?;
            let attrs: Vec<String> = self.attributes.iter().map(|a| a.to_string()).collect();
            write!(f, "{}", attrs.join(", "))?;
            write!(f, " }}")
        }
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format focus concepts
        let separator = match self.operator {
            Some(ExpressionOperator::And) | None => " + ",
            Some(ExpressionOperator::Or) => " OR ",
            Some(ExpressionOperator::Minus) => " MINUS ",
        };
        let focus: Vec<String> = self.focus.iter().map(|c| c.to_string()).collect();
        write!(f, "{}", focus.join(separator))?;

        // Format refinements
        if !self.refinements.is_empty() {
            write!(f, " : ")?;
            let refs: Vec<String> = self.refinements.iter().map(|rg| rg.to_string()).collect();
            write!(f, "{}", refs.join(", "))?;
        }

        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precoordinated_expression() {
        let expr = Expression::precoordinated(29857009, "Chest pain");
        assert_eq!(expr.expression_type, ExpressionType::Precoordinated);
        assert_eq!(expr.focus.len(), 1);
        assert!(expr.refinements.is_empty());
        assert!(expr.operator.is_none());
    }

    #[test]
    fn test_add_ungrouped_attribute() {
        let mut expr = Expression::precoordinated(29857009, "Chest pain");
        expr.add_ungrouped_attribute(Attribute::new(246112005, "Severity", 24484000, "Severe"));

        assert_eq!(expr.expression_type, ExpressionType::Postcoordinated);
        assert_eq!(expr.refinements.len(), 1);
        assert_eq!(expr.refinements[0].group, 0);
    }

    #[test]
    fn test_add_grouped_attribute() {
        let mut expr = Expression::precoordinated(29857009, "Chest pain");
        expr.add_grouped_attribute(
            1,
            Attribute::new(363698007, "Finding site", 368208006, "Left upper arm structure"),
        );

        assert_eq!(expr.expression_type, ExpressionType::Postcoordinated);
        assert_eq!(expr.refinements.len(), 1);
        assert_eq!(expr.refinements[0].group, 1);
    }

    #[test]
    fn test_compound_expression() {
        let expr = Expression::compound(
            vec![
                ConceptReference::with_term(29857009, "Chest pain"),
                ConceptReference::with_term(267036007, "Dyspnea"),
            ],
            ExpressionOperator::And,
        );

        assert_eq!(expr.expression_type, ExpressionType::Compound);
        assert_eq!(expr.focus.len(), 2);
        assert_eq!(expr.operator, Some(ExpressionOperator::And));
    }

    #[test]
    fn test_expression_with_nested_value() {
        let nested_expr = Expression::precoordinated(368208006, "Left upper arm structure");
        let mut expr = Expression::precoordinated(29857009, "Chest pain");
        expr.add_ungrouped_attribute(Attribute::with_expression(
            363698007,
            "Finding site",
            nested_expr,
        ));

        assert!(expr.has_nested_expressions());
        assert!(expr.refinements[0].attributes[0].value.is_expression());
    }

    #[test]
    fn test_display_precoordinated() {
        let expr = Expression::precoordinated(29857009, "Chest pain");
        assert_eq!(expr.to_string(), "29857009 |Chest pain|");
    }

    #[test]
    fn test_display_postcoordinated() {
        let mut expr = Expression::precoordinated(29857009, "Chest pain");
        expr.add_ungrouped_attribute(Attribute::new(246112005, "Severity", 24484000, "Severe"));
        assert_eq!(
            expr.to_string(),
            "29857009 |Chest pain| : 246112005 |Severity| = 24484000 |Severe|"
        );
    }

    #[test]
    fn test_display_compound() {
        let expr = Expression::compound(
            vec![
                ConceptReference::with_term(29857009, "Chest pain"),
                ConceptReference::with_term(267036007, "Dyspnea"),
            ],
            ExpressionOperator::And,
        );
        assert_eq!(
            expr.to_string(),
            "29857009 |Chest pain| + 267036007 |Dyspnea|"
        );
    }

    #[test]
    fn test_display_grouped_attributes() {
        let mut expr = Expression::precoordinated(29857009, "Chest pain");
        expr.add_grouped_attribute(
            1,
            Attribute::new(363698007, "Finding site", 368208006, "Left upper arm structure"),
        );
        expr.add_grouped_attribute(1, Attribute::new(246112005, "Severity", 24484000, "Severe"));

        let output = expr.to_string();
        assert!(output.contains("{ "));
        assert!(output.contains(" }"));
    }
}
