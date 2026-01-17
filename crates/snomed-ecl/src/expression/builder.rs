//! Expression builder for constructing postcoordinated SNOMED CT expressions.
//!
//! This module provides a structured way to build expressions from input data,
//! with optional MRCM validation.

use crate::SctId;

use super::ast::{
    Attribute, AttributeValue, ConceptReference, Expression, ExpressionOperator, ExpressionType,
    RoleGroup,
};

/// Request to build an expression.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BuildRequest {
    /// Focus concepts (required, at least one).
    pub focus_concepts: Vec<ConceptInput>,

    /// Attributes to add to the expression.
    #[cfg_attr(feature = "serde", serde(default))]
    pub attributes: Vec<AttributeInput>,

    /// Operator for compound expressions (multiple focus concepts).
    #[cfg_attr(feature = "serde", serde(default))]
    pub operator: Option<ExpressionOperator>,
}

/// Result of building an expression.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BuildResult {
    /// The built expression.
    pub expression: Expression,

    /// Validation warnings (if any).
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub warnings: Vec<BuildWarning>,
}

/// Warning during expression building.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BuildWarning {
    /// Warning code.
    pub code: String,
    /// Warning message.
    pub message: String,
}

/// Input for a concept.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConceptInput {
    /// SNOMED CT concept ID.
    pub id: SctId,
    /// Preferred term.
    #[cfg_attr(feature = "serde", serde(default))]
    pub term: Option<String>,
}

impl ConceptInput {
    /// Create a new concept input with ID and term.
    pub fn new(id: SctId, term: impl Into<String>) -> Self {
        Self {
            id,
            term: Some(term.into()),
        }
    }

    /// Create a new concept input with just an ID.
    pub fn id_only(id: SctId) -> Self {
        Self { id, term: None }
    }
}

/// Input for an attribute.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AttributeInput {
    /// Attribute type concept ID.
    pub attribute_id: SctId,
    /// Attribute type term.
    #[cfg_attr(feature = "serde", serde(default))]
    pub attribute_name: Option<String>,
    /// Value concept ID.
    pub value_id: SctId,
    /// Value term.
    #[cfg_attr(feature = "serde", serde(default))]
    pub value_name: Option<String>,
    /// Role group number (0 = ungrouped).
    #[cfg_attr(feature = "serde", serde(default))]
    pub role_group: u32,
    /// Optional nested expression value (instead of value_id/value_name).
    #[cfg_attr(feature = "serde", serde(default))]
    pub nested_expression: Option<Box<BuildRequest>>,
}

impl AttributeInput {
    /// Create a new attribute input.
    pub fn new(
        attribute_id: SctId,
        attribute_name: impl Into<String>,
        value_id: SctId,
        value_name: impl Into<String>,
    ) -> Self {
        Self {
            attribute_id,
            attribute_name: Some(attribute_name.into()),
            value_id,
            value_name: Some(value_name.into()),
            role_group: 0,
            nested_expression: None,
        }
    }

    /// Set the role group.
    pub fn with_role_group(mut self, group: u32) -> Self {
        self.role_group = group;
        self
    }

    /// Set a nested expression as the value.
    pub fn with_nested(mut self, nested: BuildRequest) -> Self {
        self.nested_expression = Some(Box::new(nested));
        self
    }
}

/// Error during expression building.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BuildError {
    /// No focus concepts provided.
    #[error("at least one focus concept is required")]
    NoFocusConcepts,

    /// Invalid concept ID.
    #[error("invalid concept ID: {0}")]
    InvalidConceptId(String),

    /// Nested expression build failed.
    #[error("nested expression build failed: {0}")]
    NestedBuildFailed(String),
}

/// Expression builder.
#[derive(Debug, Clone, Default)]
pub struct ExpressionBuilder {
    _private: (),
}

impl ExpressionBuilder {
    /// Create a new expression builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build an expression from a request.
    pub fn build(&self, request: &BuildRequest) -> Result<BuildResult, BuildError> {
        let expression = Self::build_expression(request)?;
        Ok(BuildResult {
            expression,
            warnings: vec![],
        })
    }

    /// Build an expression from a request (static method).
    pub fn build_expression(request: &BuildRequest) -> Result<Expression, BuildError> {
        if request.focus_concepts.is_empty() {
            return Err(BuildError::NoFocusConcepts);
        }

        // Create focus concepts
        let focus: Vec<ConceptReference> = request
            .focus_concepts
            .iter()
            .map(|c| {
                if let Some(ref term) = c.term {
                    ConceptReference::with_term(c.id, term)
                } else {
                    ConceptReference::new(c.id)
                }
            })
            .collect();

        // Group attributes by role group
        let mut role_groups: std::collections::HashMap<u32, Vec<Attribute>> =
            std::collections::HashMap::new();

        for attr in &request.attributes {
            let attribute = Self::build_attribute(attr)?;
            role_groups
                .entry(attr.role_group)
                .or_default()
                .push(attribute);
        }

        // Convert to RoleGroup vec, sorted by group number
        let mut refinements: Vec<RoleGroup> = role_groups
            .into_iter()
            .map(|(group, attributes)| RoleGroup { group, attributes })
            .collect();
        refinements.sort_by_key(|rg| rg.group);

        // Determine expression type and operator
        let (expression_type, operator) = if focus.len() > 1 {
            // Default to AND for compound expressions if not specified
            let op = request.operator.unwrap_or(ExpressionOperator::And);
            (ExpressionType::Compound, Some(op))
        } else if !refinements.is_empty() {
            (ExpressionType::Postcoordinated, None)
        } else {
            (ExpressionType::Precoordinated, None)
        };

        Ok(Expression {
            focus,
            refinements,
            expression_type,
            operator,
        })
    }

    /// Build an attribute from input.
    fn build_attribute(attr: &AttributeInput) -> Result<Attribute, BuildError> {
        let attribute_type = if let Some(ref name) = attr.attribute_name {
            ConceptReference::with_term(attr.attribute_id, name)
        } else {
            ConceptReference::new(attr.attribute_id)
        };

        let value = if let Some(ref nested) = attr.nested_expression {
            // Build nested expression
            let nested_expr = Self::build_expression(nested)
                .map_err(|e| BuildError::NestedBuildFailed(e.to_string()))?;
            AttributeValue::Expression(Box::new(nested_expr))
        } else {
            // Simple concept value
            if let Some(ref name) = attr.value_name {
                AttributeValue::concept(attr.value_id, name)
            } else {
                AttributeValue::concept_id(attr.value_id)
            }
        };

        Ok(Attribute {
            attribute_type,
            value,
        })
    }
}

// =============================================================================
// Fluent Builder API
// =============================================================================

/// Fluent builder for creating expressions step by step.
#[derive(Debug, Clone, Default)]
pub struct FluentExpressionBuilder {
    focus: Vec<ConceptReference>,
    refinements: Vec<RoleGroup>,
    operator: Option<ExpressionOperator>,
}

impl FluentExpressionBuilder {
    /// Create a new fluent builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a focus concept.
    pub fn focus_concept(mut self, id: SctId, term: impl Into<String>) -> Self {
        self.focus.push(ConceptReference::with_term(id, term));
        self
    }

    /// Add a focus concept with just an ID.
    pub fn focus_concept_id(mut self, id: SctId) -> Self {
        self.focus.push(ConceptReference::new(id));
        self
    }

    /// Set the operator for compound expressions.
    pub fn operator(mut self, op: ExpressionOperator) -> Self {
        self.operator = Some(op);
        self
    }

    /// Add an ungrouped attribute.
    pub fn attribute(
        mut self,
        attribute_id: SctId,
        attribute_term: impl Into<String>,
        value_id: SctId,
        value_term: impl Into<String>,
    ) -> Self {
        let attr = Attribute::new(attribute_id, attribute_term, value_id, value_term);
        if let Some(rg) = self.refinements.iter_mut().find(|rg| rg.group == 0) {
            rg.attributes.push(attr);
        } else {
            self.refinements.push(RoleGroup {
                group: 0,
                attributes: vec![attr],
            });
        }
        self
    }

    /// Add a role group with attributes.
    pub fn role_group<F>(mut self, group: u32, f: F) -> Self
    where
        F: FnOnce(RoleGroupBuilder) -> RoleGroupBuilder,
    {
        let builder = f(RoleGroupBuilder::new(group));
        self.refinements.push(builder.build());
        self
    }

    /// Build the expression.
    pub fn build(self) -> Result<Expression, BuildError> {
        if self.focus.is_empty() {
            return Err(BuildError::NoFocusConcepts);
        }

        let (expression_type, operator) = if self.focus.len() > 1 {
            let op = self.operator.unwrap_or(ExpressionOperator::And);
            (ExpressionType::Compound, Some(op))
        } else if !self.refinements.is_empty() {
            (ExpressionType::Postcoordinated, None)
        } else {
            (ExpressionType::Precoordinated, None)
        };

        Ok(Expression {
            focus: self.focus,
            refinements: self.refinements,
            expression_type,
            operator,
        })
    }
}

/// Builder for role groups.
#[derive(Debug, Clone)]
pub struct RoleGroupBuilder {
    group: u32,
    attributes: Vec<Attribute>,
}

impl RoleGroupBuilder {
    /// Create a new role group builder.
    pub fn new(group: u32) -> Self {
        Self {
            group,
            attributes: vec![],
        }
    }

    /// Add an attribute to the role group.
    pub fn attribute(
        mut self,
        attribute_id: SctId,
        attribute_term: impl Into<String>,
        value_id: SctId,
        value_term: impl Into<String>,
    ) -> Self {
        self.attributes
            .push(Attribute::new(attribute_id, attribute_term, value_id, value_term));
        self
    }

    /// Build the role group.
    pub fn build(self) -> RoleGroup {
        RoleGroup {
            group: self.group,
            attributes: self.attributes,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_request() -> BuildRequest {
        BuildRequest {
            focus_concepts: vec![ConceptInput::new(29857009, "Chest pain")],
            attributes: vec![],
            operator: None,
        }
    }

    #[test]
    fn test_build_precoordinated() {
        let builder = ExpressionBuilder::new();
        let request = simple_request();
        let result = builder.build(&request).unwrap();
        assert_eq!(
            result.expression.expression_type,
            ExpressionType::Precoordinated
        );
        assert_eq!(result.expression.focus.len(), 1);
        assert!(result.expression.refinements.is_empty());
    }

    #[test]
    fn test_build_postcoordinated() {
        let request = BuildRequest {
            focus_concepts: vec![ConceptInput::new(29857009, "Chest pain")],
            attributes: vec![AttributeInput::new(246112005, "Severity", 24484000, "Severe")],
            operator: None,
        };

        let result = ExpressionBuilder::build_expression(&request).unwrap();
        assert_eq!(result.expression_type, ExpressionType::Postcoordinated);
        assert_eq!(result.refinements.len(), 1);
    }

    #[test]
    fn test_build_with_role_groups() {
        let request = BuildRequest {
            focus_concepts: vec![ConceptInput::new(29857009, "Chest pain")],
            attributes: vec![
                AttributeInput::new(246112005, "Severity", 24484000, "Severe"),
                AttributeInput::new(363698007, "Finding site", 368208006, "Left upper arm")
                    .with_role_group(1),
            ],
            operator: None,
        };

        let result = ExpressionBuilder::build_expression(&request).unwrap();
        assert_eq!(result.refinements.len(), 2);
        assert_eq!(result.refinements[0].group, 0);
        assert_eq!(result.refinements[1].group, 1);
    }

    #[test]
    fn test_build_compound() {
        let request = BuildRequest {
            focus_concepts: vec![
                ConceptInput::new(29857009, "Chest pain"),
                ConceptInput::new(267036007, "Dyspnea"),
            ],
            attributes: vec![],
            operator: Some(ExpressionOperator::Or),
        };

        let result = ExpressionBuilder::build_expression(&request).unwrap();
        assert_eq!(result.expression_type, ExpressionType::Compound);
        assert_eq!(result.operator, Some(ExpressionOperator::Or));
    }

    #[test]
    fn test_build_fails_no_focus() {
        let request = BuildRequest {
            focus_concepts: vec![],
            attributes: vec![],
            operator: None,
        };

        let result = ExpressionBuilder::build_expression(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_fluent_builder() {
        let expr = FluentExpressionBuilder::new()
            .focus_concept(29857009, "Chest pain")
            .attribute(246112005, "Severity", 24484000, "Severe")
            .role_group(1, |rg| {
                rg.attribute(363698007, "Finding site", 368208006, "Left upper arm")
            })
            .build()
            .unwrap();

        assert_eq!(expr.expression_type, ExpressionType::Postcoordinated);
        assert_eq!(expr.refinements.len(), 2);
    }

    #[test]
    fn test_fluent_builder_compound() {
        let expr = FluentExpressionBuilder::new()
            .focus_concept(29857009, "Chest pain")
            .focus_concept(267036007, "Dyspnea")
            .operator(ExpressionOperator::And)
            .build()
            .unwrap();

        assert_eq!(expr.expression_type, ExpressionType::Compound);
        assert_eq!(expr.focus.len(), 2);
    }
}
