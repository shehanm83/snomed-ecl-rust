//! ECL parser implementation using nom.
//!
//! This module implements a parser for SNOMED CT Expression Constraint Language (ECL)
//! following the [official specification](https://confluence.ihtsdotools.org/display/DOCECL).

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_until, take_while, take_while1},
    character::complete::{char, digit1, multispace0, multispace1},
    combinator::{all_consuming, map, opt, recognize, value},
    multi::separated_list1,
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};

use crate::ast::{
    AttributeConstraint, AttributeGroup, Cardinality, ComparisonOperator, ConcreteValue,
    EclExpression, EclFilter, Refinement, RefinementOperator, TermMatchType,
};
use crate::error::{EclError, EclResult};
use crate::SctId;

/// Parse an ECL expression string.
///
/// # Arguments
/// * `input` - The ECL expression string to parse
///
/// # Returns
/// The parsed ECL expression AST or an error
///
/// # Examples
///
/// ```rust
/// use snomed_ecl::parse;
///
/// // Simple concept reference
/// let expr = parse("404684003").unwrap();
///
/// // With term
/// let expr = parse("404684003 |Clinical finding|").unwrap();
///
/// // Descendants
/// let expr = parse("<< 404684003").unwrap();
///
/// // Compound expression
/// let expr = parse("< 19829001 AND < 301867009").unwrap();
/// ```
pub fn parse(input: &str) -> EclResult<EclExpression> {
    let input = input.trim();
    if input.is_empty() {
        return Err(EclError::EmptyExpression);
    }

    match all_consuming(expression_constraint)(input) {
        Ok((_, expr)) => Ok(expr),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            let position = input.len() - e.input.len();
            Err(EclError::ParseError {
                position,
                message: format!("unexpected input at: '{}'", truncate(e.input, 20)),
            })
        }
        Err(nom::Err::Incomplete(_)) => Err(EclError::Incomplete("expression".to_string())),
    }
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

// ============================================================================
// Top-level expression constraint
// ============================================================================

fn expression_constraint(input: &str) -> IResult<&str, EclExpression> {
    delimited(ws, compound_or_simple_expression, ws)(input)
}

fn compound_or_simple_expression(input: &str) -> IResult<&str, EclExpression> {
    // Try to parse a refined expression (focus : refinement)
    // The refinement must be checked before compound operators
    let (input, first) = refined_expression(input)?;
    compound_tail(input, first)
}

fn compound_tail(input: &str, left: EclExpression) -> IResult<&str, EclExpression> {
    // Try to parse more operators
    // First try with mandatory whitespace for word operators (AND, OR, MINUS)
    // Then try comma which doesn't need whitespace
    let result = alt((
        preceded(mws, word_compound_operator),
        preceded(ws, comma_operator),
    ))(input);

    match result {
        Ok((remaining, op)) => {
            let (remaining, right) = preceded(ws, sub_expression_constraint)(remaining)?;
            let combined = match op {
                CompoundOp::And => EclExpression::And(Box::new(left), Box::new(right)),
                CompoundOp::Or => EclExpression::Or(Box::new(left), Box::new(right)),
                CompoundOp::Minus => EclExpression::Minus(Box::new(left), Box::new(right)),
            };
            // Continue parsing for more operators (left associative)
            compound_tail(remaining, combined)
        }
        Err(_) => Ok((input, left)),
    }
}

#[derive(Debug, Clone, Copy)]
enum CompoundOp {
    And,
    Or,
    Minus,
}

fn word_compound_operator(input: &str) -> IResult<&str, CompoundOp> {
    alt((
        value(CompoundOp::And, tag_no_case("AND")),
        value(CompoundOp::Or, or_keyword),
        value(CompoundOp::Minus, minus_keyword),
    ))(input)
}

fn comma_operator(input: &str) -> IResult<&str, CompoundOp> {
    value(CompoundOp::And, tag(","))(input)
}

fn or_keyword(input: &str) -> IResult<&str, &str> {
    tag_no_case("OR")(input)
}

fn minus_keyword(input: &str) -> IResult<&str, &str> {
    tag_no_case("MINUS")(input)
}

// ============================================================================
// Sub-expression constraint
// ============================================================================

/// Parse a base sub-expression (without dot notation or filters).
/// This is used internally by constraint_expression to avoid infinite recursion.
fn base_sub_expression(input: &str) -> IResult<&str, EclExpression> {
    alt((
        // Top/bottom of set operators (must come first - longer prefix)
        top_of_set,
        bottom_of_set,
        // Parenthesized expression
        map(
            delimited(
                pair(char('('), ws),
                compound_or_simple_expression,
                pair(ws, char(')')),
            ),
            |inner| EclExpression::Nested(Box::new(inner)),
        ),
        // Constraint operator + focus concept
        constraint_expression,
        // Member of with nested
        member_of_expression,
        // Simple focus concept (self)
        focus_concept,
    ))(input)
}

fn sub_expression_constraint(input: &str) -> IResult<&str, EclExpression> {
    let (input, expr) = base_sub_expression(input)?;

    // Check for dot notation
    let (input, expr) = dot_notation_tail(input, expr)?;

    // Check for filters
    filtered_expression_tail(input, expr)
}

fn constraint_expression(input: &str) -> IResult<&str, EclExpression> {
    let (input, op) = constraint_operator(input)?;
    let (input, _) = ws(input)?;
    // Use base_sub_expression to avoid applying dot notation inside the constraint
    let (input, inner) = base_sub_expression(input)?;

    let expr = match op {
        ConstraintOp::DescendantOf => EclExpression::DescendantOf(Box::new(inner)),
        ConstraintOp::DescendantOrSelfOf => EclExpression::DescendantOrSelfOf(Box::new(inner)),
        ConstraintOp::ChildOf => EclExpression::ChildOf(Box::new(inner)),
        ConstraintOp::ChildOrSelfOf => EclExpression::ChildOrSelfOf(Box::new(inner)),
        ConstraintOp::AncestorOf => EclExpression::AncestorOf(Box::new(inner)),
        ConstraintOp::AncestorOrSelfOf => EclExpression::AncestorOrSelfOf(Box::new(inner)),
        ConstraintOp::ParentOf => EclExpression::ParentOf(Box::new(inner)),
        ConstraintOp::ParentOrSelfOf => EclExpression::ParentOrSelfOf(Box::new(inner)),
    };

    Ok((input, expr))
}

#[derive(Debug, Clone, Copy)]
#[allow(clippy::enum_variant_names)]
enum ConstraintOp {
    DescendantOf,
    DescendantOrSelfOf,
    ChildOf,
    ChildOrSelfOf,
    AncestorOf,
    AncestorOrSelfOf,
    ParentOf,
    ParentOrSelfOf,
}

fn constraint_operator(input: &str) -> IResult<&str, ConstraintOp> {
    alt((
        // Order matters - longer matches first
        value(ConstraintOp::ChildOrSelfOf, tag("<<!")),
        value(ConstraintOp::DescendantOrSelfOf, tag("<<")),
        value(ConstraintOp::ChildOf, tag("<!")),
        value(ConstraintOp::DescendantOf, tag("<")),
        value(ConstraintOp::ParentOrSelfOf, tag(">>!")),
        value(ConstraintOp::AncestorOrSelfOf, tag(">>")),
        value(ConstraintOp::ParentOf, tag(">!")),
        value(ConstraintOp::AncestorOf, tag(">")),
    ))(input)
}

fn member_of_expression(input: &str) -> IResult<&str, EclExpression> {
    let (input, _) = char('^')(input)?;
    let (input, _) = ws(input)?;
    let (input, inner) = focus_concept(input)?;

    // Extract the concept info for memberOf
    match inner {
        EclExpression::ConceptReference { concept_id, term } => {
            Ok((input, EclExpression::MemberOf { refset_id: concept_id, term }))
        }
        _ => {
            // If not a simple concept, we wrap it
            Ok((input, EclExpression::MemberOf { refset_id: 0, term: None }))
        }
    }
}

// ============================================================================
// Focus concept
// ============================================================================

fn focus_concept(input: &str) -> IResult<&str, EclExpression> {
    alt((
        wildcard,
        concept_reference,
    ))(input)
}

fn wildcard(input: &str) -> IResult<&str, EclExpression> {
    value(EclExpression::Any, char('*'))(input)
}

fn concept_reference(input: &str) -> IResult<&str, EclExpression> {
    let (input, id) = sct_id(input)?;
    let (input, term) = opt(preceded(ws, term_in_pipes))(input)?;

    Ok((
        input,
        EclExpression::ConceptReference {
            concept_id: id,
            term,
        },
    ))
}

fn sct_id(input: &str) -> IResult<&str, SctId> {
    let (input, digits) = digit1(input)?;
    match digits.parse::<SctId>() {
        Ok(id) => Ok((input, id)),
        Err(_) => Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        ))),
    }
}

fn term_in_pipes(input: &str) -> IResult<&str, String> {
    let (input, _) = char('|')(input)?;
    let (input, _) = opt(multispace0)(input)?;
    let (input, term) = take_while(|c| c != '|')(input)?;
    let (input, _) = char('|')(input)?;

    Ok((input, term.trim().to_string()))
}

// ============================================================================
// Whitespace handling
// ============================================================================

/// Optional whitespace
fn ws(input: &str) -> IResult<&str, &str> {
    multispace0(input)
}

/// Mandatory whitespace
fn mws(input: &str) -> IResult<&str, &str> {
    multispace1(input)
}

// =============================================================================
// Refinement Parsing (Story 10.9)
// =============================================================================

/// Parse a cardinality constraint: `[min..max]` or `[min..*]`
fn cardinality(input: &str) -> IResult<&str, Cardinality> {
    let (input, _) = char('[')(input)?;
    let (input, _) = ws(input)?;
    let (input, min_str) = digit1(input)?;
    let min = min_str.parse::<usize>().unwrap_or(0);
    let (input, _) = ws(input)?;
    let (input, _) = tag("..")(input)?;
    let (input, _) = ws(input)?;
    let (input, max) = alt((
        map(char('*'), |_| None),
        map(digit1, |s: &str| Some(s.parse::<usize>().unwrap_or(0))),
    ))(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char(']')(input)?;
    Ok((input, Cardinality::new(min, max)))
}

/// Parse a refinement operator: `=`, `!=`, etc.
fn refinement_operator(input: &str) -> IResult<&str, RefinementOperator> {
    alt((
        // Order matters - longer matches first
        value(RefinementOperator::DescendantOrSelfOf, preceded(ws, tag("<<"))),
        value(RefinementOperator::DescendantOf, preceded(ws, tag("<"))),
        value(RefinementOperator::AncestorOrSelfOf, preceded(ws, tag(">>"))),
        value(RefinementOperator::AncestorOf, preceded(ws, tag(">"))),
        value(RefinementOperator::NotEqual, tag("!=")),
        value(RefinementOperator::Equal, char('=')),
    ))(input)
}

/// Parse a single attribute constraint.
/// Format: `[cardinality] [R] attributeType operator value`
fn attribute_constraint(input: &str) -> IResult<&str, AttributeConstraint> {
    let (input, cardinality) = opt(preceded(ws, cardinality))(input)?;
    let (input, _) = ws(input)?;
    let (input, reverse) = opt(preceded(ws, tag_no_case("R")))(input)?;
    let (input, _) = ws(input)?;

    // Attribute type - can be a concept reference or wildcard
    let (input, attr_type) = alt((wildcard, concept_reference))(input)?;

    let (input, _) = ws(input)?;

    // Parse the operator with optional hierarchy prefix
    let (input, operator) = refinement_operator(input)?;

    let (input, _) = ws(input)?;

    // Value - can be an expression or concrete value
    let (input, value_expr) = alt((
        concrete_value_expression,
        sub_expression_constraint,
    ))(input)?;

    Ok((
        input,
        AttributeConstraint {
            cardinality,
            reverse: reverse.is_some(),
            attribute_type: Box::new(attr_type),
            operator,
            value: Box::new(value_expr),
        },
    ))
}

/// Parse an attribute group: `{ constraint, constraint, ... }`
fn attribute_group(input: &str) -> IResult<&str, AttributeGroup> {
    let (input, cardinality) = opt(preceded(ws, cardinality))(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('{')(input)?;
    let (input, _) = ws(input)?;

    let (input, constraints) = separated_list1(
        preceded(ws, char(',')),
        preceded(ws, attribute_constraint),
    )(input)?;

    let (input, _) = ws(input)?;
    let (input, _) = char('}')(input)?;

    Ok((input, AttributeGroup { cardinality, constraints }))
}

/// Parse a refinement clause.
fn refinement_clause(input: &str) -> IResult<&str, Refinement> {
    // A refinement is a mix of ungrouped attributes and groups
    // We need to parse both, separated by commas
    let mut ungrouped = Vec::new();
    let mut groups = Vec::new();

    let (mut remaining, _) = ws(input)?;

    // Parse first item (can be group or constraint)
    let result = alt((
        map(attribute_group, |g| (None, Some(g))),
        map(attribute_constraint, |c| (Some(c), None)),
    ))(remaining);

    if let Ok((rest, (constraint, group))) = result {
        if let Some(c) = constraint {
            ungrouped.push(c);
        }
        if let Some(g) = group {
            groups.push(g);
        }
        remaining = rest;

        // Parse additional items
        loop {
            let (rest, _) = ws(remaining)?;

            // Try to match comma followed by another item
            if let Ok((rest2, _)) = char::<&str, nom::error::Error<&str>>(',')(rest) {
                let (rest2, _) = ws(rest2)?;

                if let Ok((rest3, (constraint, group))) = alt((
                    map(attribute_group, |g| (None, Some(g))),
                    map(attribute_constraint, |c| (Some(c), None)),
                ))(rest2)
                {
                    if let Some(c) = constraint {
                        ungrouped.push(c);
                    }
                    if let Some(g) = group {
                        groups.push(g);
                    }
                    remaining = rest3;
                    continue;
                }
            }
            break;
        }
    }

    Ok((remaining, Refinement { ungrouped, groups }))
}

/// Parse a refined expression: `focusExpression : refinement`
fn refined_expression(input: &str) -> IResult<&str, EclExpression> {
    let (remaining, focus) = sub_expression_constraint(input)?;

    // Check for refinement without consuming whitespace if not present
    let trimmed = remaining.trim_start();
    if trimmed.starts_with(':') {
        let (rest, _) = ws(remaining)?;
        let (rest, _) = char(':')(rest)?;
        let (rest, _) = ws(rest)?;
        let (rest, refinement) = refinement_clause(rest)?;
        Ok((
            rest,
            EclExpression::Refined {
                focus: Box::new(focus),
                refinement,
            },
        ))
    } else {
        // No refinement, return original remaining input
        Ok((remaining, focus))
    }
}

// =============================================================================
// Dot Notation Parsing
// =============================================================================

/// Parse dot notation: `expression . attributeType`
fn dot_notation_tail(input: &str, left: EclExpression) -> IResult<&str, EclExpression> {
    // Try to parse whitespace followed by dot
    // If no dot, return original input (not after ws)
    let trimmed = input.trim_start();

    if trimmed.starts_with('.') {
        // Found a dot, now parse properly
        let (rest, _) = ws(input)?;
        let (rest, _) = char('.')(rest)?;
        let (rest, _) = ws(rest)?;
        let (rest, attr_type) = alt((wildcard, concept_reference))(rest)?;

        // Recursively check for more dots
        let expr = EclExpression::DotNotation {
            source: Box::new(left),
            attribute_type: Box::new(attr_type),
        };
        dot_notation_tail(rest, expr)
    } else {
        // No dot found, return original input unchanged
        Ok((input, left))
    }
}

// =============================================================================
// Concrete Value Parsing
// =============================================================================

/// Parse a concrete value: `#123`, `#3.14`, or `#"string"`
fn concrete_value(input: &str) -> IResult<&str, ConcreteValue> {
    let (input, _) = char('#')(input)?;

    alt((
        // String value
        map(
            delimited(char('"'), take_until("\""), char('"')),
            |s: &str| ConcreteValue::String(s.to_string()),
        ),
        // Decimal or integer
        map(
            recognize(tuple((
                opt(char('-')),
                digit1,
                opt(tuple((char('.'), digit1))),
            ))),
            |s: &str| {
                if s.contains('.') {
                    ConcreteValue::Decimal(s.parse().unwrap_or(0.0))
                } else {
                    ConcreteValue::Integer(s.parse().unwrap_or(0))
                }
            },
        ),
    ))(input)
}

/// Parse a concrete value expression with comparison operator.
fn concrete_value_expression(input: &str) -> IResult<&str, EclExpression> {
    let (input, value) = concrete_value(input)?;

    Ok((
        input,
        EclExpression::Concrete {
            value,
            operator: ComparisonOperator::Equal,
        },
    ))
}

// =============================================================================
// Filter Parsing
// =============================================================================

/// Parse a comparison operator for filters.
fn comparison_operator(input: &str) -> IResult<&str, ComparisonOperator> {
    alt((
        value(ComparisonOperator::LessThanOrEqual, tag("<=")),
        value(ComparisonOperator::GreaterThanOrEqual, tag(">=")),
        value(ComparisonOperator::NotEqual, tag("!=")),
        value(ComparisonOperator::LessThan, char('<')),
        value(ComparisonOperator::GreaterThan, char('>')),
        value(ComparisonOperator::Equal, char('=')),
    ))(input)
}

/// Parse a quoted string.
fn quoted_string(input: &str) -> IResult<&str, String> {
    let (input, _) = char('"')(input)?;
    let (input, content) = take_until("\"")(input)?;
    let (input, _) = char('"')(input)?;
    Ok((input, content.to_string()))
}

/// Parse a term filter: `term = "value"` or `term startsWith "value"`
fn term_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = tag_no_case("term")(input)?;
    let (input, _) = ws(input)?;

    let (input, match_type) = alt((
        value(TermMatchType::StartsWith, tag_no_case("startsWith")),
        value(TermMatchType::Regex, tag_no_case("regex")),
        value(TermMatchType::Exact, tag("==")),
        value(TermMatchType::Contains, char('=')),
    ))(input)?;

    let (input, _) = ws(input)?;
    let (input, value) = quoted_string(input)?;

    Ok((
        input,
        EclFilter::Term {
            match_type,
            value,
            language: None,
            type_id: None,
        },
    ))
}

/// Parse a member filter: `M fieldName = "value"`
fn member_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = alt((tag("M "), tag("m ")))(input)?;
    let (input, _) = ws(input)?;
    let (input, field) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    let (input, _) = ws(input)?;
    let (input, operator) = comparison_operator(input)?;
    let (input, _) = ws(input)?;
    let (input, value) = quoted_string(input)?;

    Ok((
        input,
        EclFilter::Member {
            field: field.to_string(),
            operator,
            value,
        },
    ))
}

/// Parse a history supplement filter: `+HISTORY`
fn history_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = tag("+")(input)?;
    let (input, _) = tag_no_case("HISTORY")(input)?;
    Ok((input, EclFilter::History))
}

/// Parse an active filter: `active = true/false`
fn active_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = tag_no_case("active")(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = ws(input)?;
    let (input, active) = alt((
        value(true, tag_no_case("true")),
        value(false, tag_no_case("false")),
        value(true, char('1')),
        value(false, char('0')),
    ))(input)?;

    Ok((input, EclFilter::Active(active)))
}

/// Parse a module filter: `moduleId = 900000000000207008`
fn module_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = tag_no_case("moduleId")(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = ws(input)?;
    let (input, id) = sct_id(input)?;

    Ok((input, EclFilter::Module(id)))
}

/// Parse a single filter.
fn single_filter(input: &str) -> IResult<&str, EclFilter> {
    alt((
        history_filter,
        member_filter,
        term_filter,
        active_filter,
        module_filter,
    ))(input)
}

/// Parse a filter block: `{{ filter, filter, ... }}`
fn filter_block(input: &str) -> IResult<&str, Vec<EclFilter>> {
    let (input, _) = tag("{{")(input)?;
    let (input, _) = ws(input)?;

    let (input, filters) = separated_list1(
        preceded(ws, char(',')),
        preceded(ws, single_filter),
    )(input)?;

    let (input, _) = ws(input)?;
    let (input, _) = tag("}}")(input)?;

    Ok((input, filters))
}

/// Parse filtered expression tail.
fn filtered_expression_tail(input: &str, expr: EclExpression) -> IResult<&str, EclExpression> {
    // Check for filter block without consuming input if not present
    let trimmed = input.trim_start();

    if trimmed.starts_with("{{") {
        // Found a filter block, now parse properly
        let (rest, _) = ws(input)?;
        let (rest, filters) = filter_block(rest)?;
        let filtered = EclExpression::Filtered {
            expression: Box::new(expr),
            filters,
        };
        // Check for more filters
        filtered_expression_tail(rest, filtered)
    } else {
        // No filter block found, return original input unchanged
        Ok((input, expr))
    }
}

// =============================================================================
// Top/Bottom of Set Operators
// =============================================================================

/// Parse top of set operator: `!!>`
fn top_of_set(input: &str) -> IResult<&str, EclExpression> {
    let (input, _) = tag("!!>")(input)?;
    let (input, _) = ws(input)?;
    let (input, inner) = sub_expression_constraint(input)?;
    Ok((input, EclExpression::TopOfSet(Box::new(inner))))
}

/// Parse bottom of set operator: `!!<`
fn bottom_of_set(input: &str) -> IResult<&str, EclExpression> {
    let (input, _) = tag("!!<")(input)?;
    let (input, _) = ws(input)?;
    let (input, inner) = sub_expression_constraint(input)?;
    Ok((input, EclExpression::BottomOfSet(Box::new(inner))))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // 1. Simple Expression Constraints (from IHTSDO examples)
    // ========================================================================

    mod simple_expressions {
        use super::*;

        /// 1.1 Self - Just the concept ID
        /// Example: 404684003 |clinical finding|
        #[test]
        fn test_1_1_self_concept_id_only() {
            let expr = parse("404684003").unwrap();
            assert!(matches!(
                expr,
                EclExpression::ConceptReference { concept_id: 404684003, term: None }
            ));
        }

        #[test]
        fn test_1_1_self_concept_with_term() {
            let expr = parse("404684003 |clinical finding|").unwrap();
            match expr {
                EclExpression::ConceptReference { concept_id, term } => {
                    assert_eq!(concept_id, 404684003);
                    assert_eq!(term.as_deref(), Some("clinical finding"));
                }
                _ => panic!("Expected ConceptReference"),
            }
        }

        /// 1.2 DescendantOf - Using < operator
        /// Example: < 404684003 |clinical finding|
        #[test]
        fn test_1_2_descendant_of() {
            let expr = parse("< 404684003 |clinical finding|").unwrap();
            match expr {
                EclExpression::DescendantOf(inner) => {
                    match inner.as_ref() {
                        EclExpression::ConceptReference { concept_id, term } => {
                            assert_eq!(*concept_id, 404684003);
                            assert_eq!(term.as_deref(), Some("clinical finding"));
                        }
                        _ => panic!("Expected ConceptReference inside DescendantOf"),
                    }
                }
                _ => panic!("Expected DescendantOf"),
            }
        }

        #[test]
        fn test_1_2_descendant_of_no_space() {
            let expr = parse("<404684003").unwrap();
            assert!(matches!(expr, EclExpression::DescendantOf(_)));
        }

        /// 1.3 DescendantOrSelfOf - Using << operator
        /// Example: << 73211009 |diabetes mellitus|
        #[test]
        fn test_1_3_descendant_or_self_of() {
            let expr = parse("<< 73211009 |diabetes mellitus|").unwrap();
            match expr {
                EclExpression::DescendantOrSelfOf(inner) => {
                    match inner.as_ref() {
                        EclExpression::ConceptReference { concept_id, term } => {
                            assert_eq!(*concept_id, 73211009);
                            assert_eq!(term.as_deref(), Some("diabetes mellitus"));
                        }
                        _ => panic!("Expected ConceptReference inside DescendantOrSelfOf"),
                    }
                }
                _ => panic!("Expected DescendantOrSelfOf"),
            }
        }

        #[test]
        fn test_1_3_descendant_or_self_of_no_space() {
            let expr = parse("<<73211009").unwrap();
            assert!(matches!(expr, EclExpression::DescendantOrSelfOf(_)));
        }

        /// 1.4 AncestorOf - Using > operator
        /// Example: > 40541001 |acute pulmonary edema|
        #[test]
        fn test_1_4_ancestor_of() {
            let expr = parse("> 40541001 |acute pulmonary edema|").unwrap();
            match expr {
                EclExpression::AncestorOf(inner) => {
                    match inner.as_ref() {
                        EclExpression::ConceptReference { concept_id, term } => {
                            assert_eq!(*concept_id, 40541001);
                            assert_eq!(term.as_deref(), Some("acute pulmonary edema"));
                        }
                        _ => panic!("Expected ConceptReference inside AncestorOf"),
                    }
                }
                _ => panic!("Expected AncestorOf"),
            }
        }

        /// 1.5 AncestorOrSelfOf - Using >> operator
        /// Example: >> 40541001 |acute pulmonary edema|
        #[test]
        fn test_1_5_ancestor_or_self_of() {
            let expr = parse(">> 40541001 |acute pulmonary edema|").unwrap();
            match expr {
                EclExpression::AncestorOrSelfOf(inner) => {
                    match inner.as_ref() {
                        EclExpression::ConceptReference { concept_id, term } => {
                            assert_eq!(*concept_id, 40541001);
                            assert_eq!(term.as_deref(), Some("acute pulmonary edema"));
                        }
                        _ => panic!("Expected ConceptReference inside AncestorOrSelfOf"),
                    }
                }
                _ => panic!("Expected AncestorOrSelfOf"),
            }
        }

        /// 1.6 MemberOf - Using ^ operator
        /// Example: ^ 700043003 |example problem list concepts reference set|
        #[test]
        fn test_1_6_member_of() {
            let expr = parse("^ 700043003 |example problem list concepts reference set|").unwrap();
            match expr {
                EclExpression::MemberOf { refset_id, term } => {
                    assert_eq!(refset_id, 700043003);
                    assert_eq!(
                        term.as_deref(),
                        Some("example problem list concepts reference set")
                    );
                }
                _ => panic!("Expected MemberOf"),
            }
        }

        #[test]
        fn test_1_6_member_of_no_term() {
            let expr = parse("^700043003").unwrap();
            match expr {
                EclExpression::MemberOf { refset_id, term } => {
                    assert_eq!(refset_id, 700043003);
                    assert!(term.is_none());
                }
                _ => panic!("Expected MemberOf"),
            }
        }

        /// 1.7 Any - Using * wildcard
        /// Example: *
        #[test]
        fn test_1_7_any_wildcard() {
            let expr = parse("*").unwrap();
            assert!(matches!(expr, EclExpression::Any));
        }

        /// 1.8 ChildOf - Using <! operator
        /// Example: <! 404684003 |clinical finding|
        #[test]
        fn test_1_8_child_of() {
            let expr = parse("<! 404684003 |clinical finding|").unwrap();
            assert!(matches!(expr, EclExpression::ChildOf(_)));
        }

        /// 1.9 ParentOf - Using >! operator
        /// Example: >! 40541001 |acute pulmonary edema|
        #[test]
        fn test_1_9_parent_of() {
            let expr = parse(">! 40541001 |acute pulmonary edema|").unwrap();
            assert!(matches!(expr, EclExpression::ParentOf(_)));
        }

        /// ChildOrSelfOf - Using <<! operator
        #[test]
        fn test_child_or_self_of() {
            let expr = parse("<<! 404684003").unwrap();
            assert!(matches!(expr, EclExpression::ChildOrSelfOf(_)));
        }

        /// ParentOrSelfOf - Using >>! operator
        #[test]
        fn test_parent_or_self_of() {
            let expr = parse(">>! 40541001").unwrap();
            assert!(matches!(expr, EclExpression::ParentOrSelfOf(_)));
        }
    }

    // ========================================================================
    // 4. Conjunction and Disjunction (from IHTSDO examples)
    // ========================================================================

    mod compound_expressions {
        use super::*;

        /// 4.1 Compound Expression - AND
        /// Example: < 19829001 |disorder of lung| AND < 301867009 |edema of trunk|
        #[test]
        fn test_4_1_and_expression() {
            let expr =
                parse("< 19829001 |disorder of lung| AND < 301867009 |edema of trunk|").unwrap();
            match expr {
                EclExpression::And(left, right) => {
                    assert!(matches!(left.as_ref(), EclExpression::DescendantOf(_)));
                    assert!(matches!(right.as_ref(), EclExpression::DescendantOf(_)));
                }
                _ => panic!("Expected And expression"),
            }
        }

        /// 4.2 Compound Expression - OR
        /// Example: < 19829001 |disorder of lung| OR < 301867009 |edema of trunk|
        #[test]
        fn test_4_2_or_expression() {
            let expr =
                parse("< 19829001 |disorder of lung| OR < 301867009 |edema of trunk|").unwrap();
            match expr {
                EclExpression::Or(left, right) => {
                    assert!(matches!(left.as_ref(), EclExpression::DescendantOf(_)));
                    assert!(matches!(right.as_ref(), EclExpression::DescendantOf(_)));
                }
                _ => panic!("Expected Or expression"),
            }
        }

        /// 4.3 Compound Expression - MemberOf with AND
        /// Example: < 19829001 |disorder of lung| AND ^ 700043003
        #[test]
        fn test_4_3_member_of_with_and() {
            let expr = parse("< 19829001 |disorder of lung| AND ^ 700043003").unwrap();
            match expr {
                EclExpression::And(left, right) => {
                    assert!(matches!(left.as_ref(), EclExpression::DescendantOf(_)));
                    assert!(matches!(right.as_ref(), EclExpression::MemberOf { .. }));
                }
                _ => panic!("Expected And expression with MemberOf"),
            }
        }

        /// Case-insensitive keywords
        #[test]
        fn test_and_case_insensitive() {
            let expr1 = parse("<< 1 AND << 2").unwrap();
            let expr2 = parse("<< 1 and << 2").unwrap();
            let expr3 = parse("<< 1 And << 2").unwrap();
            assert!(matches!(expr1, EclExpression::And(_, _)));
            assert!(matches!(expr2, EclExpression::And(_, _)));
            assert!(matches!(expr3, EclExpression::And(_, _)));
        }

        #[test]
        fn test_or_case_insensitive() {
            let expr1 = parse("<< 1 OR << 2").unwrap();
            let expr2 = parse("<< 1 or << 2").unwrap();
            assert!(matches!(expr1, EclExpression::Or(_, _)));
            assert!(matches!(expr2, EclExpression::Or(_, _)));
        }

        #[test]
        fn test_minus_case_insensitive() {
            let expr1 = parse("<< 1 MINUS << 2").unwrap();
            let expr2 = parse("<< 1 minus << 2").unwrap();
            assert!(matches!(expr1, EclExpression::Minus(_, _)));
            assert!(matches!(expr2, EclExpression::Minus(_, _)));
        }

        /// Comma as AND (ECL allows comma as conjunction)
        #[test]
        fn test_comma_as_and() {
            let expr = parse("<< 1, << 2").unwrap();
            assert!(matches!(expr, EclExpression::And(_, _)));
        }
    }

    // ========================================================================
    // 5. Exclusion (MINUS) (from IHTSDO examples)
    // ========================================================================

    mod exclusion {
        use super::*;

        /// 5.1 Simple Exclusion
        /// Example: 19829001 |disorder of lung| MINUS 301867009 |edema of trunk|
        #[test]
        fn test_5_1_simple_exclusion() {
            let expr =
                parse("19829001 |disorder of lung| MINUS 301867009 |edema of trunk|").unwrap();
            match expr {
                EclExpression::Minus(left, right) => {
                    match left.as_ref() {
                        EclExpression::ConceptReference { concept_id, .. } => {
                            assert_eq!(*concept_id, 19829001);
                        }
                        _ => panic!("Expected ConceptReference on left"),
                    }
                    match right.as_ref() {
                        EclExpression::ConceptReference { concept_id, .. } => {
                            assert_eq!(*concept_id, 301867009);
                        }
                        _ => panic!("Expected ConceptReference on right"),
                    }
                }
                _ => panic!("Expected Minus expression"),
            }
        }

        /// 5.2 Exclusion with hierarchy operators
        /// Example: << 19829001 |disorder of lung| MINUS << 301867009 |edema of trunk|
        #[test]
        fn test_5_2_exclusion_with_hierarchy() {
            let expr = parse(
                "<< 19829001 |disorder of lung| MINUS << 301867009 |edema of trunk|",
            )
            .unwrap();
            match expr {
                EclExpression::Minus(left, right) => {
                    assert!(matches!(left.as_ref(), EclExpression::DescendantOrSelfOf(_)));
                    assert!(matches!(right.as_ref(), EclExpression::DescendantOrSelfOf(_)));
                }
                _ => panic!("Expected Minus expression"),
            }
        }
    }

    // ========================================================================
    // 7. Nested Expression Constraints
    // ========================================================================

    mod nested_expressions {
        use super::*;

        #[test]
        fn test_nested_simple() {
            let expr = parse("(<< 404684003)").unwrap();
            match expr {
                EclExpression::Nested(inner) => {
                    assert!(matches!(inner.as_ref(), EclExpression::DescendantOrSelfOf(_)));
                }
                _ => panic!("Expected Nested expression"),
            }
        }

        #[test]
        fn test_nested_compound() {
            let expr = parse("(<< 1 OR << 2) AND << 3").unwrap();
            match expr {
                EclExpression::And(left, right) => {
                    match left.as_ref() {
                        EclExpression::Nested(inner) => {
                            assert!(matches!(inner.as_ref(), EclExpression::Or(_, _)));
                        }
                        _ => panic!("Expected Nested on left"),
                    }
                    assert!(matches!(right.as_ref(), EclExpression::DescendantOrSelfOf(_)));
                }
                _ => panic!("Expected And expression"),
            }
        }

        #[test]
        fn test_nested_member_of() {
            // From IHTSDO test: << ^700043003 |Example problem list concepts reference set|
            let expr = parse("<< (^700043003)").unwrap();
            match expr {
                EclExpression::DescendantOrSelfOf(inner) => {
                    match inner.as_ref() {
                        EclExpression::Nested(nested) => {
                            assert!(matches!(nested.as_ref(), EclExpression::MemberOf { .. }));
                        }
                        _ => panic!("Expected Nested inside DescendantOrSelfOf"),
                    }
                }
                _ => panic!("Expected DescendantOrSelfOf"),
            }
        }

        #[test]
        fn test_deep_nesting() {
            let expr = parse("((<< 1))").unwrap();
            // Double nested
            match expr {
                EclExpression::Nested(inner) => {
                    match inner.as_ref() {
                        EclExpression::Nested(inner2) => {
                            assert!(matches!(inner2.as_ref(), EclExpression::DescendantOrSelfOf(_)));
                        }
                        _ => panic!("Expected inner Nested"),
                    }
                }
                _ => panic!("Expected outer Nested"),
            }
        }
    }

    // ========================================================================
    // IHTSDO ECLQueryBuilderTest equivalent tests
    // ========================================================================

    mod ihtsdo_query_builder_tests {
        use super::*;

        /// Test from IHTSDO: parseMemberOfQuerySyntax
        #[test]
        fn test_parse_member_of_query_syntax() {
            let expr =
                parse("^700043003 |Example problem list concepts reference set|").unwrap();
            match expr {
                EclExpression::MemberOf { refset_id, term } => {
                    assert_eq!(refset_id, 700043003);
                    assert_eq!(
                        term.as_deref(),
                        Some("Example problem list concepts reference set")
                    );
                }
                _ => panic!("Expected MemberOf"),
            }
        }

        /// Test from IHTSDO: parseMemberOfNestedQuerySyntax
        #[test]
        fn test_parse_member_of_nested_query_syntax_1() {
            // << ^700043003 should be interpreted as descendants of member-of
            let expr = parse("<< ^700043003 |Example problem list concepts reference set|").unwrap();
            match expr {
                EclExpression::DescendantOrSelfOf(inner) => {
                    assert!(matches!(inner.as_ref(), EclExpression::MemberOf { .. }));
                }
                _ => panic!("Expected DescendantOrSelfOf(MemberOf)"),
            }
        }

        #[test]
        fn test_parse_member_of_nested_query_syntax_2() {
            // With parentheses
            let expr =
                parse("<< (^700043003 |Example problem list concepts reference set|)").unwrap();
            match expr {
                EclExpression::DescendantOrSelfOf(inner) => {
                    match inner.as_ref() {
                        EclExpression::Nested(nested) => {
                            assert!(matches!(nested.as_ref(), EclExpression::MemberOf { .. }));
                        }
                        _ => panic!("Expected Nested"),
                    }
                }
                _ => panic!("Expected DescendantOrSelfOf"),
            }
        }

        /// Test from IHTSDO: parseCommaWithoutSpace
        /// "<<404684003:363698007=<<123037004,116676008=<<415582006"
        /// Note: This includes refinements which we don't support yet, but we support
        /// comma as AND operator at the basic level
        #[test]
        fn test_comma_without_space_simple() {
            // Simplified version without refinements
            let expr = parse("<<404684003,<<123037004").unwrap();
            assert!(matches!(expr, EclExpression::And(_, _)));
        }
    }

    // ========================================================================
    // Error handling tests
    // ========================================================================

    mod error_handling {
        use super::*;

        #[test]
        fn test_empty_input() {
            let result = parse("");
            assert!(matches!(result, Err(EclError::EmptyExpression)));
        }

        #[test]
        fn test_whitespace_only() {
            let result = parse("   ");
            assert!(matches!(result, Err(EclError::EmptyExpression)));
        }

        #[test]
        fn test_invalid_syntax_trailing() {
            // Invalid trailing characters
            let result = parse("404684003 garbage");
            assert!(result.is_err());
        }

        #[test]
        fn test_unclosed_parenthesis() {
            let result = parse("(<< 404684003");
            assert!(result.is_err());
        }

        #[test]
        fn test_and_without_right_operand() {
            let result = parse("<< 404684003 AND");
            assert!(result.is_err());
        }

        #[test]
        fn test_double_operator() {
            // Invalid: two operators in a row
            let result = parse("<< << 404684003");
            // This should actually parse as << (<<404684003) - descendant of descendant
            assert!(result.is_ok());
        }
    }

    // ========================================================================
    // Real-world ECL expressions from TerminologyX Attribute Registry
    // ========================================================================

    mod attribute_registry_expressions {
        use super::*;

        /// Finding site domain: << 404684003 |Clinical finding|
        #[test]
        fn test_finding_site_domain() {
            let expr = parse("<< 404684003 |Clinical finding|").unwrap();
            match expr {
                EclExpression::DescendantOrSelfOf(inner) => {
                    match inner.as_ref() {
                        EclExpression::ConceptReference { concept_id, term } => {
                            assert_eq!(*concept_id, 404684003);
                            assert_eq!(term.as_deref(), Some("Clinical finding"));
                        }
                        _ => panic!("Expected ConceptReference"),
                    }
                }
                _ => panic!("Expected DescendantOrSelfOf"),
            }
        }

        /// Finding site range: << 123037004 |Body structure|
        #[test]
        fn test_finding_site_range() {
            let expr = parse("<< 123037004 |Body structure|").unwrap();
            match expr {
                EclExpression::DescendantOrSelfOf(inner) => {
                    match inner.as_ref() {
                        EclExpression::ConceptReference { concept_id, term } => {
                            assert_eq!(*concept_id, 123037004);
                            assert_eq!(term.as_deref(), Some("Body structure"));
                        }
                        _ => panic!("Expected ConceptReference"),
                    }
                }
                _ => panic!("Expected DescendantOrSelfOf"),
            }
        }

        /// Severity range: << 272141005 |Severities|
        #[test]
        fn test_severity_range() {
            let expr = parse("<< 272141005 |Severities|").unwrap();
            assert!(matches!(expr, EclExpression::DescendantOrSelfOf(_)));
        }

        /// Laterality range: << 182353008 |Side|
        #[test]
        fn test_laterality_range() {
            let expr = parse("<< 182353008 |Side|").unwrap();
            assert!(matches!(expr, EclExpression::DescendantOrSelfOf(_)));
        }

        /// Associated morphology range: << 49755003 |Morphologic abnormality|
        #[test]
        fn test_associated_morphology_range() {
            let expr = parse("<< 49755003 |Morphologic abnormality|").unwrap();
            assert!(matches!(expr, EclExpression::DescendantOrSelfOf(_)));
        }
    }

    // ========================================================================
    // Whitespace handling tests
    // ========================================================================

    mod whitespace_handling {
        use super::*;

        #[test]
        fn test_leading_whitespace() {
            let expr = parse("  << 404684003").unwrap();
            assert!(matches!(expr, EclExpression::DescendantOrSelfOf(_)));
        }

        #[test]
        fn test_trailing_whitespace() {
            let expr = parse("<< 404684003  ").unwrap();
            assert!(matches!(expr, EclExpression::DescendantOrSelfOf(_)));
        }

        #[test]
        fn test_whitespace_around_operators() {
            // Various whitespace patterns
            let expr1 = parse("<<404684003").unwrap();
            let expr2 = parse("<< 404684003").unwrap();
            let expr3 = parse("<<  404684003").unwrap();
            assert!(matches!(expr1, EclExpression::DescendantOrSelfOf(_)));
            assert!(matches!(expr2, EclExpression::DescendantOrSelfOf(_)));
            assert!(matches!(expr3, EclExpression::DescendantOrSelfOf(_)));
        }

        #[test]
        fn test_whitespace_in_term() {
            let expr = parse("404684003 | Clinical finding |").unwrap();
            match expr {
                EclExpression::ConceptReference { term, .. } => {
                    // Term should be trimmed
                    assert_eq!(term.as_deref(), Some("Clinical finding"));
                }
                _ => panic!("Expected ConceptReference"),
            }
        }

        #[test]
        fn test_newlines() {
            let expr = parse("<< 1\nAND\n<< 2").unwrap();
            assert!(matches!(expr, EclExpression::And(_, _)));
        }

        #[test]
        fn test_tabs() {
            let expr = parse("<<\t404684003").unwrap();
            assert!(matches!(expr, EclExpression::DescendantOrSelfOf(_)));
        }
    }

    // ========================================================================
    // Chain operators (left associativity)
    // ========================================================================

    mod chained_operators {
        use super::*;

        #[test]
        fn test_triple_and() {
            let expr = parse("<< 1 AND << 2 AND << 3").unwrap();
            // Should be ((1 AND 2) AND 3) - left associative
            match expr {
                EclExpression::And(left, right) => {
                    // Right should be << 3
                    assert!(matches!(right.as_ref(), EclExpression::DescendantOrSelfOf(_)));
                    // Left should be (1 AND 2)
                    assert!(matches!(left.as_ref(), EclExpression::And(_, _)));
                }
                _ => panic!("Expected And expression"),
            }
        }

        #[test]
        fn test_triple_or() {
            let expr = parse("<< 1 OR << 2 OR << 3").unwrap();
            match expr {
                EclExpression::Or(left, _) => {
                    assert!(matches!(left.as_ref(), EclExpression::Or(_, _)));
                }
                _ => panic!("Expected Or expression"),
            }
        }

        #[test]
        fn test_mixed_and_or() {
            // Without parentheses, operators are left-associative
            let expr = parse("<< 1 AND << 2 OR << 3").unwrap();
            // Should be ((1 AND 2) OR 3)
            match expr {
                EclExpression::Or(left, _) => {
                    assert!(matches!(left.as_ref(), EclExpression::And(_, _)));
                }
                _ => panic!("Expected Or at top level"),
            }
        }

        #[test]
        fn test_parentheses_change_precedence() {
            let expr = parse("<< 1 AND (<< 2 OR << 3)").unwrap();
            // Should be (1 AND (2 OR 3))
            match expr {
                EclExpression::And(_, right) => {
                    match right.as_ref() {
                        EclExpression::Nested(inner) => {
                            assert!(matches!(inner.as_ref(), EclExpression::Or(_, _)));
                        }
                        _ => panic!("Expected Nested on right"),
                    }
                }
                _ => panic!("Expected And at top level"),
            }
        }
    }

    // ========================================================================
    // Display/roundtrip tests
    // ========================================================================

    mod display_roundtrip {
        use super::*;

        fn roundtrip(input: &str) -> String {
            let expr = parse(input).unwrap();
            expr.to_string()
        }

        #[test]
        fn test_simple_roundtrip() {
            let output = roundtrip("404684003");
            assert_eq!(output, "404684003");
        }

        #[test]
        fn test_descendant_roundtrip() {
            let output = roundtrip("<< 404684003");
            assert_eq!(output, "<< 404684003");
        }

        #[test]
        fn test_with_term_roundtrip() {
            let output = roundtrip("404684003 |Clinical finding|");
            assert_eq!(output, "404684003 |Clinical finding|");
        }
    }

    // ========================================================================
    // Advanced ECL Features (Story 10.9)
    // ========================================================================

    mod advanced_ecl_features {
        use super::*;

        // Attribute Refinement Tests
        mod refinement {
            use super::*;

            /// Test: Simple refinement
            /// `< 404684003 : 363698007 = << 39057004`
            #[test]
            fn test_simple_refinement() {
                let expr = parse("< 404684003 : 363698007 = << 39057004").unwrap();
                match expr {
                    EclExpression::Refined { focus, refinement } => {
                        assert!(matches!(focus.as_ref(), EclExpression::DescendantOf(_)));
                        assert_eq!(refinement.ungrouped.len(), 1);
                        assert!(refinement.groups.is_empty());
                    }
                    _ => panic!("Expected Refined expression"),
                }
            }

            /// Test: Refinement with multiple attributes
            #[test]
            fn test_multiple_attribute_refinement() {
                let expr = parse("< 404684003 : 363698007 = << 39057004, 116676008 = << 79654002").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        assert_eq!(refinement.ungrouped.len(), 2);
                    }
                    _ => panic!("Expected Refined expression"),
                }
            }

            /// Test: Refinement with wildcard attribute
            #[test]
            fn test_wildcard_attribute() {
                let expr = parse("< 404684003 : * = << 39057004").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        assert_eq!(refinement.ungrouped.len(), 1);
                        assert!(matches!(refinement.ungrouped[0].attribute_type.as_ref(), EclExpression::Any));
                    }
                    _ => panic!("Expected Refined expression"),
                }
            }

            /// Test: Refinement with not equal operator
            #[test]
            fn test_not_equal_refinement() {
                let expr = parse("< 404684003 : 363698007 != 39057004").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        assert!(matches!(refinement.ungrouped[0].operator, RefinementOperator::NotEqual));
                    }
                    _ => panic!("Expected Refined expression"),
                }
            }
        }

        // Attribute Group Tests
        mod groups {
            use super::*;

            /// Test: Single attribute group
            #[test]
            fn test_single_attribute_group() {
                let expr = parse("< 404684003 : { 363698007 = << 39057004 }").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        assert!(refinement.ungrouped.is_empty());
                        assert_eq!(refinement.groups.len(), 1);
                        assert_eq!(refinement.groups[0].constraints.len(), 1);
                    }
                    _ => panic!("Expected Refined expression with group"),
                }
            }

            /// Test: Multiple attributes in a group
            #[test]
            fn test_multiple_attributes_in_group() {
                let expr = parse("< 404684003 : { 363698007 = << 39057004, 116676008 = << 79654002 }").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        assert_eq!(refinement.groups.len(), 1);
                        assert_eq!(refinement.groups[0].constraints.len(), 2);
                    }
                    _ => panic!("Expected Refined expression with group"),
                }
            }

            /// Test: Multiple attribute groups
            #[test]
            fn test_multiple_groups() {
                let expr = parse("< 404684003 : { 363698007 = << 39057004 }, { 116676008 = << 79654002 }").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        assert_eq!(refinement.groups.len(), 2);
                    }
                    _ => panic!("Expected Refined expression with multiple groups"),
                }
            }
        }

        // Cardinality Tests
        mod cardinality_tests {
            use super::*;

            /// Test: Cardinality on attribute
            #[test]
            fn test_attribute_cardinality() {
                let expr = parse("< 404684003 : [1..1] 363698007 = << 39057004").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        let attr = &refinement.ungrouped[0];
                        assert!(attr.cardinality.is_some());
                        let card = attr.cardinality.as_ref().unwrap();
                        assert_eq!(card.min, 1);
                        assert_eq!(card.max, Some(1));
                    }
                    _ => panic!("Expected Refined expression with cardinality"),
                }
            }

            /// Test: Unbounded cardinality
            #[test]
            fn test_unbounded_cardinality() {
                let expr = parse("< 404684003 : [1..*] 363698007 = << 39057004").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        let attr = &refinement.ungrouped[0];
                        let card = attr.cardinality.as_ref().unwrap();
                        assert_eq!(card.min, 1);
                        assert_eq!(card.max, None);
                    }
                    _ => panic!("Expected Refined expression"),
                }
            }

            /// Test: Zero cardinality (exclusion)
            #[test]
            fn test_zero_cardinality() {
                let expr = parse("< 404684003 : [0..0] 363698007 = *").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        let attr = &refinement.ungrouped[0];
                        let card = attr.cardinality.as_ref().unwrap();
                        assert_eq!(card.min, 0);
                        assert_eq!(card.max, Some(0));
                    }
                    _ => panic!("Expected Refined expression"),
                }
            }

            /// Test: Group cardinality
            #[test]
            fn test_group_cardinality() {
                let expr = parse("< 404684003 : [1..2] { 363698007 = << 39057004 }").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        let group = &refinement.groups[0];
                        assert!(group.cardinality.is_some());
                        let card = group.cardinality.as_ref().unwrap();
                        assert_eq!(card.min, 1);
                        assert_eq!(card.max, Some(2));
                    }
                    _ => panic!("Expected Refined expression with group cardinality"),
                }
            }
        }

        // Dot Notation Tests
        mod dot_notation {
            use super::*;

            /// Test: Simple dot notation
            #[test]
            fn test_simple_dot_notation() {
                let expr = parse("< 404684003 . 363698007").unwrap();
                match expr {
                    EclExpression::DotNotation { source, attribute_type } => {
                        assert!(matches!(source.as_ref(), EclExpression::DescendantOf(_)));
                        match attribute_type.as_ref() {
                            EclExpression::ConceptReference { concept_id, .. } => {
                                assert_eq!(*concept_id, 363698007);
                            }
                            _ => panic!("Expected concept reference for attribute type"),
                        }
                    }
                    _ => panic!("Expected DotNotation expression"),
                }
            }

            /// Test: Chained dot notation
            #[test]
            fn test_chained_dot_notation() {
                let expr = parse("< 404684003 . 363698007 . 116676008").unwrap();
                match expr {
                    EclExpression::DotNotation { source, .. } => {
                        // The source should also be a DotNotation
                        assert!(matches!(source.as_ref(), EclExpression::DotNotation { .. }));
                    }
                    _ => panic!("Expected chained DotNotation expression"),
                }
            }

            /// Test: Dot notation with wildcard attribute
            #[test]
            fn test_dot_notation_wildcard() {
                let expr = parse("< 404684003 . *").unwrap();
                match expr {
                    EclExpression::DotNotation { attribute_type, .. } => {
                        assert!(matches!(attribute_type.as_ref(), EclExpression::Any));
                    }
                    _ => panic!("Expected DotNotation with wildcard"),
                }
            }
        }

        // Concrete Value Tests
        mod concrete_values {
            use super::*;

            /// Test: Integer concrete value
            #[test]
            fn test_integer_concrete_value() {
                let expr = parse("< 404684003 : 363698007 = #250").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        match refinement.ungrouped[0].value.as_ref() {
                            EclExpression::Concrete { value, .. } => {
                                assert!(matches!(value, ConcreteValue::Integer(250)));
                            }
                            _ => panic!("Expected Concrete value"),
                        }
                    }
                    _ => panic!("Expected Refined expression"),
                }
            }

            /// Test: Decimal concrete value
            #[test]
            fn test_decimal_concrete_value() {
                let expr = parse("< 404684003 : 363698007 = #3.14").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        match refinement.ungrouped[0].value.as_ref() {
                            EclExpression::Concrete { value, .. } => {
                                match value {
                                    ConcreteValue::Decimal(v) => assert!((v - 3.14).abs() < 0.001),
                                    _ => panic!("Expected Decimal"),
                                }
                            }
                            _ => panic!("Expected Concrete value"),
                        }
                    }
                    _ => panic!("Expected Refined expression"),
                }
            }

            /// Test: String concrete value
            #[test]
            fn test_string_concrete_value() {
                let expr = parse(r#"< 404684003 : 363698007 = #"test value""#).unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        match refinement.ungrouped[0].value.as_ref() {
                            EclExpression::Concrete { value, .. } => {
                                assert!(matches!(value, ConcreteValue::String(s) if s == "test value"));
                            }
                            _ => panic!("Expected Concrete value"),
                        }
                    }
                    _ => panic!("Expected Refined expression"),
                }
            }

            /// Test: Negative integer
            #[test]
            fn test_negative_integer() {
                let expr = parse("< 404684003 : 363698007 = #-100").unwrap();
                match expr {
                    EclExpression::Refined { refinement, .. } => {
                        match refinement.ungrouped[0].value.as_ref() {
                            EclExpression::Concrete { value, .. } => {
                                assert!(matches!(value, ConcreteValue::Integer(-100)));
                            }
                            _ => panic!("Expected Concrete value"),
                        }
                    }
                    _ => panic!("Expected Refined expression"),
                }
            }
        }

        // Filter Tests
        mod filters {
            use super::*;

            /// Test: Term filter
            #[test]
            fn test_term_filter() {
                let expr = parse(r#"< 404684003 {{ term = "heart" }}"#).unwrap();
                match expr {
                    EclExpression::Filtered { expression, filters } => {
                        assert!(matches!(expression.as_ref(), EclExpression::DescendantOf(_)));
                        assert_eq!(filters.len(), 1);
                        match &filters[0] {
                            EclFilter::Term { match_type, value, .. } => {
                                assert!(matches!(match_type, TermMatchType::Contains));
                                assert_eq!(value, "heart");
                            }
                            _ => panic!("Expected Term filter"),
                        }
                    }
                    _ => panic!("Expected Filtered expression"),
                }
            }

            /// Test: Term startsWith filter
            #[test]
            fn test_term_starts_with_filter() {
                let expr = parse(r#"< 404684003 {{ term startsWith "card" }}"#).unwrap();
                match expr {
                    EclExpression::Filtered { filters, .. } => {
                        match &filters[0] {
                            EclFilter::Term { match_type, value, .. } => {
                                assert!(matches!(match_type, TermMatchType::StartsWith));
                                assert_eq!(value, "card");
                            }
                            _ => panic!("Expected Term filter"),
                        }
                    }
                    _ => panic!("Expected Filtered expression"),
                }
            }

            /// Test: Active filter
            #[test]
            fn test_active_filter() {
                let expr = parse("< 404684003 {{ active = true }}").unwrap();
                match expr {
                    EclExpression::Filtered { filters, .. } => {
                        assert!(matches!(&filters[0], EclFilter::Active(true)));
                    }
                    _ => panic!("Expected Filtered expression"),
                }
            }

            /// Test: History supplement
            #[test]
            fn test_history_supplement() {
                let expr = parse("< 404684003 {{ +HISTORY }}").unwrap();
                match expr {
                    EclExpression::Filtered { filters, .. } => {
                        assert!(matches!(&filters[0], EclFilter::History));
                    }
                    _ => panic!("Expected Filtered expression"),
                }
            }

            /// Test: Member filter
            #[test]
            fn test_member_filter() {
                let expr = parse(r#"^ 447562003 {{ M mapTarget = "J45.9" }}"#).unwrap();
                match expr {
                    EclExpression::Filtered { filters, .. } => {
                        match &filters[0] {
                            EclFilter::Member { field, operator, value } => {
                                assert_eq!(field, "mapTarget");
                                assert!(matches!(operator, ComparisonOperator::Equal));
                                assert_eq!(value, "J45.9");
                            }
                            _ => panic!("Expected Member filter"),
                        }
                    }
                    _ => panic!("Expected Filtered expression"),
                }
            }
        }

        // Top/Bottom of Set Tests
        mod set_operators {
            use super::*;

            /// Test: Top of set
            #[test]
            fn test_top_of_set() {
                let expr = parse("!!> < 404684003").unwrap();
                match expr {
                    EclExpression::TopOfSet(inner) => {
                        assert!(matches!(inner.as_ref(), EclExpression::DescendantOf(_)));
                    }
                    _ => panic!("Expected TopOfSet expression"),
                }
            }

            /// Test: Bottom of set
            #[test]
            fn test_bottom_of_set() {
                let expr = parse("!!< < 404684003").unwrap();
                match expr {
                    EclExpression::BottomOfSet(inner) => {
                        assert!(matches!(inner.as_ref(), EclExpression::DescendantOf(_)));
                    }
                    _ => panic!("Expected BottomOfSet expression"),
                }
            }
        }

        // Complex Expression Tests (combining features)
        mod complex_expressions {
            use super::*;

            /// Test: Refinement with compound operators
            #[test]
            fn test_refinement_with_and() {
                let expr = parse("< 404684003 : 363698007 = << 39057004 AND < 64572001").unwrap();
                match expr {
                    EclExpression::And(left, right) => {
                        assert!(matches!(left.as_ref(), EclExpression::Refined { .. }));
                        assert!(matches!(right.as_ref(), EclExpression::DescendantOf(_)));
                    }
                    _ => panic!("Expected And expression with refined left operand"),
                }
            }

            /// Test: Nested expression with refinement
            #[test]
            fn test_nested_refinement() {
                let expr = parse("(< 404684003 : 363698007 = << 39057004)").unwrap();
                match expr {
                    EclExpression::Nested(inner) => {
                        assert!(matches!(inner.as_ref(), EclExpression::Refined { .. }));
                    }
                    _ => panic!("Expected Nested expression"),
                }
            }
        }
    }
}
