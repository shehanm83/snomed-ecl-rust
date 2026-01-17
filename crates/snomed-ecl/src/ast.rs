//! Abstract Syntax Tree types for ECL expressions.

use crate::SctId;

// =============================================================================
// Refinement Types (Story 10.9)
// =============================================================================

/// Comparison operators for attribute refinement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RefinementOperator {
    /// Exact match: `=`
    Equal,
    /// Not equal: `!=`
    NotEqual,
    /// Descendant of target: `= <`
    DescendantOf,
    /// Descendant or self of target: `= <<`
    DescendantOrSelfOf,
    /// Ancestor of target: `= >`
    AncestorOf,
    /// Ancestor or self of target: `= >>`
    AncestorOrSelfOf,
}

impl std::fmt::Display for RefinementOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefinementOperator::Equal => write!(f, "="),
            RefinementOperator::NotEqual => write!(f, "!="),
            RefinementOperator::DescendantOf => write!(f, "= <"),
            RefinementOperator::DescendantOrSelfOf => write!(f, "= <<"),
            RefinementOperator::AncestorOf => write!(f, "= >"),
            RefinementOperator::AncestorOrSelfOf => write!(f, "= >>"),
        }
    }
}

/// Cardinality constraint for attributes: `[min..max]`
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cardinality {
    /// Minimum occurrences.
    pub min: usize,
    /// Maximum occurrences (None = unbounded `*`).
    pub max: Option<usize>,
}

impl Cardinality {
    /// Creates a cardinality constraint.
    pub fn new(min: usize, max: Option<usize>) -> Self {
        Self { min, max }
    }

    /// Cardinality of exactly zero: `[0..0]`
    pub fn zero() -> Self {
        Self { min: 0, max: Some(0) }
    }

    /// Cardinality of exactly one: `[1..1]`
    pub fn one() -> Self {
        Self { min: 1, max: Some(1) }
    }

    /// Cardinality of at least one: `[1..*]`
    pub fn at_least_one() -> Self {
        Self { min: 1, max: None }
    }

    /// Checks if a count satisfies this cardinality constraint.
    pub fn matches(&self, count: usize) -> bool {
        if count < self.min {
            return false;
        }
        if let Some(max) = self.max {
            count <= max
        } else {
            true
        }
    }
}

impl std::fmt::Display for Cardinality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.max {
            Some(max) => write!(f, "[{}..{}]", self.min, max),
            None => write!(f, "[{}..*]", self.min),
        }
    }
}

/// A single attribute constraint within a refinement.
///
/// Example: `363698007 |Finding site| = << 39057004 |Pulmonary structure|`
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AttributeConstraint {
    /// Optional cardinality constraint.
    pub cardinality: Option<Cardinality>,
    /// Whether the attribute constraint reverses the relationship direction.
    pub reverse: bool,
    /// The attribute type (relationship type ID).
    pub attribute_type: Box<EclExpression>,
    /// Comparison operator.
    pub operator: RefinementOperator,
    /// The target value (may be an expression or wildcard).
    pub value: Box<EclExpression>,
}

impl std::fmt::Display for AttributeConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref card) = self.cardinality {
            write!(f, "{} ", card)?;
        }
        if self.reverse {
            write!(f, "R ")?;
        }
        write!(f, "{} {} {}", self.attribute_type, self.operator, self.value)
    }
}

/// A group of attribute constraints that must all be satisfied within
/// the same relationship group.
///
/// Example: `{ 363698007 = << 39057004, 116676008 = << 415582006 }`
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AttributeGroup {
    /// Optional cardinality for the group itself.
    pub cardinality: Option<Cardinality>,
    /// The attribute constraints in this group.
    pub constraints: Vec<AttributeConstraint>,
}

impl std::fmt::Display for AttributeGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref card) = self.cardinality {
            write!(f, "{} ", card)?;
        }
        write!(f, "{{ ")?;
        for (i, c) in self.constraints.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", c)?;
        }
        write!(f, " }}")
    }
}

/// Refinement clause containing attribute constraints.
///
/// A refinement can have both ungrouped attributes and grouped attributes.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Refinement {
    /// Ungrouped attribute constraints (AND-combined).
    pub ungrouped: Vec<AttributeConstraint>,
    /// Grouped attribute constraints.
    pub groups: Vec<AttributeGroup>,
}

impl std::fmt::Display for Refinement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for c in &self.ungrouped {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", c)?;
            first = false;
        }
        for g in &self.groups {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", g)?;
            first = false;
        }
        Ok(())
    }
}

/// Concrete value types for ECL.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConcreteValue {
    /// Integer value: `#250`
    Integer(i64),
    /// Decimal value: `#3.14`
    Decimal(f64),
    /// String value: `#"text"`
    String(String),
    /// Boolean value: `#true` or `#false`
    Boolean(bool),
}

impl std::fmt::Display for ConcreteValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConcreteValue::Integer(n) => write!(f, "#{}", n),
            ConcreteValue::Decimal(n) => write!(f, "#{}", n),
            ConcreteValue::String(s) => write!(f, "#\"{}\"", s),
            ConcreteValue::Boolean(b) => write!(f, "#{}", b),
        }
    }
}

impl Eq for ConcreteValue {}

/// Comparison operators for concrete values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ComparisonOperator {
    /// Equal: `=`
    Equal,
    /// Not equal: `!=`
    NotEqual,
    /// Less than: `<`
    LessThan,
    /// Less than or equal: `<=`
    LessThanOrEqual,
    /// Greater than: `>`
    GreaterThan,
    /// Greater than or equal: `>=`
    GreaterThanOrEqual,
}

impl std::fmt::Display for ComparisonOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComparisonOperator::Equal => write!(f, "="),
            ComparisonOperator::NotEqual => write!(f, "!="),
            ComparisonOperator::LessThan => write!(f, "<"),
            ComparisonOperator::LessThanOrEqual => write!(f, "<="),
            ComparisonOperator::GreaterThan => write!(f, ">"),
            ComparisonOperator::GreaterThanOrEqual => write!(f, ">="),
        }
    }
}

/// Term match type for term filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TermMatchType {
    /// Contains the term (default): `term = "x"`
    Contains,
    /// Starts with the term: `term startsWith "x"`
    StartsWith,
    /// Matches regex pattern: `term regex "x.*"`
    Regex,
    /// Exact match: `term == "x"`
    Exact,
    /// Wildcard matching: `term wild "diab*"`
    Wildcard,
}

impl std::fmt::Display for TermMatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TermMatchType::Contains => write!(f, "term ="),
            TermMatchType::StartsWith => write!(f, "term startsWith"),
            TermMatchType::Regex => write!(f, "term regex"),
            TermMatchType::Exact => write!(f, "term =="),
            TermMatchType::Wildcard => write!(f, "term wild"),
        }
    }
}

/// History supplement profile for historical associations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HistoryProfile {
    /// Minimal: SAME_AS only
    Min,
    /// Moderate: SAME_AS, REPLACED_BY, POSSIBLY_EQUIVALENT_TO
    Mod,
    /// Maximum: All historical associations
    Max,
}

impl std::fmt::Display for HistoryProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoryProfile::Min => write!(f, "-MIN"),
            HistoryProfile::Mod => write!(f, "-MOD"),
            HistoryProfile::Max => write!(f, "-MAX"),
        }
    }
}

/// Acceptability value for dialect filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FilterAcceptability {
    /// Preferred term only.
    Preferred,
    /// Acceptable term only.
    Acceptable,
}

/// Filter types for ECL expressions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum EclFilter {
    // =========================================================================
    // Description Filters
    // =========================================================================

    /// Term filter: `{{ term = "heart" }}`
    Term {
        /// How to match the term.
        match_type: TermMatchType,
        /// The term value to match.
        value: String,
    },

    /// Language filter: `{{ language = en }}` or `{{ language = (en es) }}`
    Language {
        /// Language codes (ISO 639-1).
        codes: Vec<String>,
    },

    /// Type filter: `{{ typeId = 900000000000003001 }}` or `{{ type = syn }}`
    DescriptionType {
        /// Description type IDs.
        type_ids: Vec<SctId>,
    },

    /// Dialect filter: `{{ dialect = en-US }}` or `{{ dialectId = 900000000000509007 }}`
    Dialect {
        /// Dialect reference set IDs.
        dialect_ids: Vec<SctId>,
        /// Optional acceptability constraint.
        acceptability: Option<FilterAcceptability>,
    },

    /// Case significance filter: `{{ caseSignificance = caseInsensitive }}`
    CaseSignificance {
        /// Case significance ID.
        case_significance_id: SctId,
    },

    // =========================================================================
    // Concept Filters
    // =========================================================================

    /// Active filter: `{{ active = true }}`
    Active(bool),

    /// Module filter: `{{ moduleId = 900000000000207008 }}`
    Module {
        /// Module IDs to filter by.
        module_ids: Vec<SctId>,
    },

    /// Effective time filter: `{{ effectiveTime >= 20200101 }}`
    EffectiveTime {
        /// Comparison operator.
        operator: ComparisonOperator,
        /// Date in YYYYMMDD format.
        date: u32,
    },

    /// Definition status filter: `{{ definitionStatus = primitive }}`
    DefinitionStatus {
        /// True = primitive, False = defined.
        is_primitive: bool,
    },

    /// Semantic tag filter: `{{ semanticTag = "disorder" }}`
    SemanticTag {
        /// Semantic tags to match.
        tags: Vec<String>,
    },

    // =========================================================================
    // Language Reference Set Filters
    // =========================================================================

    /// Preferred in filter: `{{ preferredIn = 900000000000509007 }}`
    PreferredIn {
        /// Language reference set IDs.
        refset_ids: Vec<SctId>,
    },

    /// Acceptable in filter: `{{ acceptableIn = 900000000000509007 }}`
    AcceptableIn {
        /// Language reference set IDs.
        refset_ids: Vec<SctId>,
    },

    /// Language reference set filter: `{{ languageRefSetId = 900000000000509007 }}`
    LanguageRefSet {
        /// Language reference set IDs (either preferred or acceptable).
        refset_ids: Vec<SctId>,
    },

    // =========================================================================
    // Member Filters
    // =========================================================================

    /// Member field filter: `{{ M mapTarget = "J45.9" }}`
    Member {
        /// The refset field to filter on.
        field: String,
        /// The comparison operator.
        operator: ComparisonOperator,
        /// The value to compare against.
        value: MemberFieldValue,
    },

    // =========================================================================
    // Other Filters
    // =========================================================================

    /// ID filter: `{{ id = 123456 }}` or `{{ id = (123 456) }}`
    Id {
        /// Component IDs to match.
        ids: Vec<SctId>,
    },

    /// History supplement: `{{ +HISTORY }}` or `{{ +HISTORY-MIN }}`
    History {
        /// Optional profile (MIN, MOD, MAX).
        profile: Option<HistoryProfile>,
    },
}

/// Value types for member field filters.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MemberFieldValue {
    /// String value.
    String(String),
    /// Integer value.
    Integer(i64),
    /// Decimal value.
    Decimal(f64),
    /// Boolean value.
    Boolean(bool),
    /// SCT ID value.
    SctId(SctId),
}

impl Eq for MemberFieldValue {}

impl std::fmt::Display for MemberFieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberFieldValue::String(s) => write!(f, "\"{}\"", s),
            MemberFieldValue::Integer(n) => write!(f, "{}", n),
            MemberFieldValue::Decimal(n) => write!(f, "{}", n),
            MemberFieldValue::Boolean(b) => write!(f, "{}", b),
            MemberFieldValue::SctId(id) => write!(f, "{}", id),
        }
    }
}

impl std::fmt::Display for EclFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EclFilter::Term { match_type, value } => {
                write!(f, "{} \"{}\"", match_type, value)
            }
            EclFilter::Language { codes } => {
                if codes.len() == 1 {
                    write!(f, "language = {}", codes[0])
                } else {
                    write!(f, "language = ({})", codes.join(" "))
                }
            }
            EclFilter::DescriptionType { type_ids } => {
                if type_ids.len() == 1 {
                    write!(f, "typeId = {}", type_ids[0])
                } else {
                    let ids: Vec<String> = type_ids.iter().map(|id| id.to_string()).collect();
                    write!(f, "typeId = ({})", ids.join(" "))
                }
            }
            EclFilter::Dialect { dialect_ids, acceptability } => {
                if dialect_ids.len() == 1 {
                    write!(f, "dialectId = {}", dialect_ids[0])?;
                } else {
                    let ids: Vec<String> = dialect_ids.iter().map(|id| id.to_string()).collect();
                    write!(f, "dialectId = ({})", ids.join(" "))?;
                }
                if let Some(acc) = acceptability {
                    match acc {
                        FilterAcceptability::Preferred => write!(f, " prefer")?,
                        FilterAcceptability::Acceptable => write!(f, " accept")?,
                    }
                }
                Ok(())
            }
            EclFilter::CaseSignificance { case_significance_id } => {
                write!(f, "caseSignificanceId = {}", case_significance_id)
            }
            EclFilter::Active(active) => {
                write!(f, "active = {}", active)
            }
            EclFilter::Module { module_ids } => {
                if module_ids.len() == 1 {
                    write!(f, "moduleId = {}", module_ids[0])
                } else {
                    let ids: Vec<String> = module_ids.iter().map(|id| id.to_string()).collect();
                    write!(f, "moduleId = ({})", ids.join(" "))
                }
            }
            EclFilter::EffectiveTime { operator, date } => {
                write!(f, "effectiveTime {} {}", operator, date)
            }
            EclFilter::DefinitionStatus { is_primitive } => {
                if *is_primitive {
                    write!(f, "definitionStatus = primitive")
                } else {
                    write!(f, "definitionStatus = defined")
                }
            }
            EclFilter::SemanticTag { tags } => {
                if tags.len() == 1 {
                    write!(f, "semanticTag = \"{}\"", tags[0])
                } else {
                    let quoted: Vec<String> = tags.iter().map(|t| format!("\"{}\"", t)).collect();
                    write!(f, "semanticTag = ({})", quoted.join(" "))
                }
            }
            EclFilter::PreferredIn { refset_ids } => {
                if refset_ids.len() == 1 {
                    write!(f, "preferredIn = {}", refset_ids[0])
                } else {
                    let ids: Vec<String> = refset_ids.iter().map(|id| id.to_string()).collect();
                    write!(f, "preferredIn = ({})", ids.join(" "))
                }
            }
            EclFilter::AcceptableIn { refset_ids } => {
                if refset_ids.len() == 1 {
                    write!(f, "acceptableIn = {}", refset_ids[0])
                } else {
                    let ids: Vec<String> = refset_ids.iter().map(|id| id.to_string()).collect();
                    write!(f, "acceptableIn = ({})", ids.join(" "))
                }
            }
            EclFilter::LanguageRefSet { refset_ids } => {
                if refset_ids.len() == 1 {
                    write!(f, "languageRefSetId = {}", refset_ids[0])
                } else {
                    let ids: Vec<String> = refset_ids.iter().map(|id| id.to_string()).collect();
                    write!(f, "languageRefSetId = ({})", ids.join(" "))
                }
            }
            EclFilter::Member { field, operator, value } => {
                write!(f, "M {} {} {}", field, operator, value)
            }
            EclFilter::Id { ids } => {
                if ids.len() == 1 {
                    write!(f, "id = {}", ids[0])
                } else {
                    let id_strs: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
                    write!(f, "id = ({})", id_strs.join(" "))
                }
            }
            EclFilter::History { profile } => {
                write!(f, "+HISTORY")?;
                if let Some(p) = profile {
                    write!(f, "{}", p)?;
                }
                Ok(())
            }
        }
    }
}

/// Abstract Syntax Tree for ECL expressions.
///
/// Represents the parsed structure of an ECL (Expression Constraint Language) expression.
/// This follows the [official ECL specification](https://confluence.ihtsdotools.org/display/DOCECL).
///
/// # Examples
///
/// ```rust
/// use snomed_ecl::{parse, EclExpression};
///
/// // Simple concept reference
/// let expr = parse("404684003").unwrap();
/// assert!(matches!(expr, EclExpression::ConceptReference { .. }));
///
/// // Descendants of a concept
/// let expr = parse("<< 404684003").unwrap();
/// assert!(matches!(expr, EclExpression::DescendantOrSelfOf(_)));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EclExpression {
    /// A single concept reference (self).
    /// Example: `404684003` or `404684003 |Clinical finding|`
    ConceptReference {
        /// The SNOMED CT concept ID.
        concept_id: SctId,
        /// Optional term/label in pipe notation.
        term: Option<String>,
    },

    /// Descendants of a concept (exclusive, not including self).
    /// Syntax: `< conceptReference`
    /// Example: `< 404684003 |Clinical finding|`
    DescendantOf(Box<EclExpression>),

    /// Descendants of a concept or the concept itself (inclusive).
    /// Syntax: `<< conceptReference`
    /// Example: `<< 404684003 |Clinical finding|`
    DescendantOrSelfOf(Box<EclExpression>),

    /// Child of a concept (direct descendants only, one level).
    /// Syntax: `<! conceptReference`
    /// Example: `<! 404684003 |Clinical finding|`
    ChildOf(Box<EclExpression>),

    /// Child or self of a concept (direct descendants + self).
    /// Syntax: `<<! conceptReference`
    /// Example: `<<! 404684003 |Clinical finding|`
    ChildOrSelfOf(Box<EclExpression>),

    /// Ancestors of a concept (exclusive, not including self).
    /// Syntax: `> conceptReference`
    /// Example: `> 40541001 |Acute pulmonary edema|`
    AncestorOf(Box<EclExpression>),

    /// Ancestors of a concept or the concept itself (inclusive).
    /// Syntax: `>> conceptReference`
    /// Example: `>> 40541001 |Acute pulmonary edema|`
    AncestorOrSelfOf(Box<EclExpression>),

    /// Parent of a concept (direct ancestors only, one level).
    /// Syntax: `>! conceptReference`
    /// Example: `>! 40541001 |Acute pulmonary edema|`
    ParentOf(Box<EclExpression>),

    /// Parent or self of a concept (direct ancestors + self).
    /// Syntax: `>>! conceptReference`
    /// Example: `>>! 40541001 |Acute pulmonary edema|`
    ParentOrSelfOf(Box<EclExpression>),

    /// Conjunction (AND) of two expressions.
    /// Syntax: `expression AND expression`
    /// Example: `<< 404684003 AND << 123037004`
    And(Box<EclExpression>, Box<EclExpression>),

    /// Disjunction (OR) of two expressions.
    /// Syntax: `expression OR expression`
    /// Example: `<< 404684003 OR << 71388002`
    Or(Box<EclExpression>, Box<EclExpression>),

    /// Set difference/exclusion (MINUS) of two expressions.
    /// Syntax: `expression MINUS expression`
    /// Example: `<< 404684003 MINUS << 64572001`
    Minus(Box<EclExpression>, Box<EclExpression>),

    /// Reference set membership.
    /// Syntax: `^ refsetId`
    /// Example: `^ 700043003 |Example problem list concepts reference set|`
    MemberOf {
        /// The reference set concept ID.
        refset_id: SctId,
        /// Optional term/label in pipe notation.
        term: Option<String>,
    },

    /// Wildcard matching any concept.
    /// Syntax: `*`
    Any,

    /// Nested expression in parentheses.
    /// Used to control precedence.
    /// Example: `(<< 404684003 OR << 71388002) AND << 123037004`
    Nested(Box<EclExpression>),

    // =========================================================================
    // Advanced ECL Features (Story 10.9)
    // =========================================================================

    /// Refined expression with attribute constraints.
    /// Syntax: `focusExpression : refinement`
    /// Example: `< 19829001 : 116676008 = << 79654002`
    Refined {
        /// The focus expression.
        focus: Box<EclExpression>,
        /// The refinement clause.
        refinement: Refinement,
    },

    /// Dot notation for attribute value extraction.
    /// Syntax: `expression . attributeType`
    /// Example: `< 125605004 . 363698007`
    DotNotation {
        /// The source expression.
        source: Box<EclExpression>,
        /// The attribute type to extract values for.
        attribute_type: Box<EclExpression>,
    },

    /// Concrete value for numeric/string attribute matching.
    /// Syntax: `#250` or `#"text"`
    Concrete {
        /// The concrete value.
        value: ConcreteValue,
        /// Comparison operator.
        operator: ComparisonOperator,
    },

    /// Filtered expression with term/member/history filters.
    /// Syntax: `expression {{ filter }}`
    /// Example: `< 64572001 {{ term = "heart" }}`
    Filtered {
        /// The source expression.
        expression: Box<EclExpression>,
        /// The filters to apply.
        filters: Vec<EclFilter>,
    },

    /// Top of set operator - most general concepts in a set.
    /// Syntax: `!!> expression`
    /// Example: `!!> (< 386617003 . 363698007)`
    TopOfSet(Box<EclExpression>),

    /// Bottom of set operator - most specific concepts in a set.
    /// Syntax: `!!< expression`
    /// Example: `!!< (>> 45133009 AND ^ 991411000000109)`
    BottomOfSet(Box<EclExpression>),
}

impl EclExpression {
    /// Creates a new concept reference expression.
    pub fn concept(id: SctId) -> Self {
        EclExpression::ConceptReference {
            concept_id: id,
            term: None,
        }
    }

    /// Creates a new concept reference expression with a term.
    pub fn concept_with_term(id: SctId, term: impl Into<String>) -> Self {
        EclExpression::ConceptReference {
            concept_id: id,
            term: Some(term.into()),
        }
    }

    /// Creates a descendant-of expression.
    pub fn descendant_of(inner: EclExpression) -> Self {
        EclExpression::DescendantOf(Box::new(inner))
    }

    /// Creates a descendant-or-self-of expression.
    pub fn descendant_or_self_of(inner: EclExpression) -> Self {
        EclExpression::DescendantOrSelfOf(Box::new(inner))
    }

    /// Creates an ancestor-of expression.
    pub fn ancestor_of(inner: EclExpression) -> Self {
        EclExpression::AncestorOf(Box::new(inner))
    }

    /// Creates an ancestor-or-self-of expression.
    pub fn ancestor_or_self_of(inner: EclExpression) -> Self {
        EclExpression::AncestorOrSelfOf(Box::new(inner))
    }

    /// Creates an AND expression.
    pub fn and(left: EclExpression, right: EclExpression) -> Self {
        EclExpression::And(Box::new(left), Box::new(right))
    }

    /// Creates an OR expression.
    pub fn or(left: EclExpression, right: EclExpression) -> Self {
        EclExpression::Or(Box::new(left), Box::new(right))
    }

    /// Creates a MINUS expression.
    pub fn minus(left: EclExpression, right: EclExpression) -> Self {
        EclExpression::Minus(Box::new(left), Box::new(right))
    }

    /// Creates a member-of expression.
    pub fn member_of(refset_id: SctId) -> Self {
        EclExpression::MemberOf {
            refset_id,
            term: None,
        }
    }

    /// Returns true if this is a simple concept reference.
    pub fn is_concept_reference(&self) -> bool {
        matches!(self, EclExpression::ConceptReference { .. })
    }

    /// Returns true if this expression contains a hierarchy operator.
    pub fn has_hierarchy_operator(&self) -> bool {
        matches!(
            self,
            EclExpression::DescendantOf(_)
                | EclExpression::DescendantOrSelfOf(_)
                | EclExpression::ChildOf(_)
                | EclExpression::ChildOrSelfOf(_)
                | EclExpression::AncestorOf(_)
                | EclExpression::AncestorOrSelfOf(_)
                | EclExpression::ParentOf(_)
                | EclExpression::ParentOrSelfOf(_)
        )
    }

    /// Returns true if this is a compound expression (AND, OR, MINUS).
    pub fn is_compound(&self) -> bool {
        matches!(
            self,
            EclExpression::And(_, _) | EclExpression::Or(_, _) | EclExpression::Minus(_, _)
        )
    }

    /// Returns the concept ID if this is a simple concept reference.
    pub fn as_concept_id(&self) -> Option<SctId> {
        match self {
            EclExpression::ConceptReference { concept_id, .. } => Some(*concept_id),
            _ => None,
        }
    }

    /// Unwraps nested expressions to get the inner expression.
    pub fn unwrap_nested(&self) -> &EclExpression {
        match self {
            EclExpression::Nested(inner) => inner.unwrap_nested(),
            other => other,
        }
    }
}

impl std::fmt::Display for EclExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EclExpression::ConceptReference { concept_id, term } => {
                if let Some(t) = term {
                    write!(f, "{} |{}|", concept_id, t)
                } else {
                    write!(f, "{}", concept_id)
                }
            }
            EclExpression::DescendantOf(inner) => write!(f, "< {}", inner),
            EclExpression::DescendantOrSelfOf(inner) => write!(f, "<< {}", inner),
            EclExpression::ChildOf(inner) => write!(f, "<! {}", inner),
            EclExpression::ChildOrSelfOf(inner) => write!(f, "<<! {}", inner),
            EclExpression::AncestorOf(inner) => write!(f, "> {}", inner),
            EclExpression::AncestorOrSelfOf(inner) => write!(f, ">> {}", inner),
            EclExpression::ParentOf(inner) => write!(f, ">! {}", inner),
            EclExpression::ParentOrSelfOf(inner) => write!(f, ">>! {}", inner),
            EclExpression::And(left, right) => write!(f, "{} AND {}", left, right),
            EclExpression::Or(left, right) => write!(f, "{} OR {}", left, right),
            EclExpression::Minus(left, right) => write!(f, "{} MINUS {}", left, right),
            EclExpression::MemberOf { refset_id, term } => {
                if let Some(t) = term {
                    write!(f, "^ {} |{}|", refset_id, t)
                } else {
                    write!(f, "^ {}", refset_id)
                }
            }
            EclExpression::Any => write!(f, "*"),
            EclExpression::Nested(inner) => write!(f, "({})", inner),
            EclExpression::Refined { focus, refinement } => {
                write!(f, "{} : {}", focus, refinement)
            }
            EclExpression::DotNotation { source, attribute_type } => {
                write!(f, "{} . {}", source, attribute_type)
            }
            EclExpression::Concrete { value, operator } => {
                write!(f, "{} {}", operator, value)
            }
            EclExpression::Filtered { expression, filters } => {
                write!(f, "{} {{{{ ", expression)?;
                for (i, filter) in filters.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", filter)?;
                }
                write!(f, " }}}}")
            }
            EclExpression::TopOfSet(inner) => write!(f, "!!> {}", inner),
            EclExpression::BottomOfSet(inner) => write!(f, "!!< {}", inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concept_display() {
        let expr = EclExpression::concept(404684003);
        assert_eq!(expr.to_string(), "404684003");
    }

    #[test]
    fn test_concept_with_term_display() {
        let expr = EclExpression::concept_with_term(404684003, "Clinical finding");
        assert_eq!(expr.to_string(), "404684003 |Clinical finding|");
    }

    #[test]
    fn test_descendant_of_display() {
        let expr = EclExpression::descendant_of(EclExpression::concept(404684003));
        assert_eq!(expr.to_string(), "< 404684003");
    }

    #[test]
    fn test_descendant_or_self_of_display() {
        let expr = EclExpression::descendant_or_self_of(EclExpression::concept(404684003));
        assert_eq!(expr.to_string(), "<< 404684003");
    }

    #[test]
    fn test_ancestor_of_display() {
        let expr = EclExpression::ancestor_of(EclExpression::concept(40541001));
        assert_eq!(expr.to_string(), "> 40541001");
    }

    #[test]
    fn test_ancestor_or_self_of_display() {
        let expr = EclExpression::ancestor_or_self_of(EclExpression::concept(40541001));
        assert_eq!(expr.to_string(), ">> 40541001");
    }

    #[test]
    fn test_and_display() {
        let expr = EclExpression::and(
            EclExpression::descendant_or_self_of(EclExpression::concept(404684003)),
            EclExpression::descendant_or_self_of(EclExpression::concept(123037004)),
        );
        assert_eq!(expr.to_string(), "<< 404684003 AND << 123037004");
    }

    #[test]
    fn test_or_display() {
        let expr = EclExpression::or(
            EclExpression::descendant_of(EclExpression::concept(19829001)),
            EclExpression::descendant_of(EclExpression::concept(301867009)),
        );
        assert_eq!(expr.to_string(), "< 19829001 OR < 301867009");
    }

    #[test]
    fn test_minus_display() {
        let expr = EclExpression::minus(
            EclExpression::concept(19829001),
            EclExpression::concept(301867009),
        );
        assert_eq!(expr.to_string(), "19829001 MINUS 301867009");
    }

    #[test]
    fn test_member_of_display() {
        let expr = EclExpression::member_of(700043003);
        assert_eq!(expr.to_string(), "^ 700043003");
    }

    #[test]
    fn test_any_display() {
        let expr = EclExpression::Any;
        assert_eq!(expr.to_string(), "*");
    }

    #[test]
    fn test_is_concept_reference() {
        let expr = EclExpression::concept(404684003);
        assert!(expr.is_concept_reference());

        let expr2 = EclExpression::descendant_of(EclExpression::concept(404684003));
        assert!(!expr2.is_concept_reference());
    }

    #[test]
    fn test_has_hierarchy_operator() {
        let expr = EclExpression::concept(404684003);
        assert!(!expr.has_hierarchy_operator());

        let expr2 = EclExpression::descendant_of(EclExpression::concept(404684003));
        assert!(expr2.has_hierarchy_operator());

        let expr3 = EclExpression::ancestor_or_self_of(EclExpression::concept(404684003));
        assert!(expr3.has_hierarchy_operator());
    }

    #[test]
    fn test_is_compound() {
        let expr = EclExpression::concept(404684003);
        assert!(!expr.is_compound());

        let expr2 = EclExpression::and(
            EclExpression::concept(1),
            EclExpression::concept(2),
        );
        assert!(expr2.is_compound());
    }

    #[test]
    fn test_as_concept_id() {
        let expr = EclExpression::concept(404684003);
        assert_eq!(expr.as_concept_id(), Some(404684003));

        let expr2 = EclExpression::descendant_of(EclExpression::concept(404684003));
        assert_eq!(expr2.as_concept_id(), None);
    }
}
