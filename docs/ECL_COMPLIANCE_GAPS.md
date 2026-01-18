# ECL Specification Compliance Gaps

This document details all gaps between the current `snomed-ecl-rust` implementation and the official SNOMED CT Expression Constraint Language specification (ECL 2.2).

**Reference:** [Official ECL Specification](https://docs.snomed.org/snomed-ct-specifications/snomed-ct-expression-constraint-language)

---

## Table of Contents

1. [Current Implementation Status](#current-implementation-status)
2. [Missing Filter Types](#missing-filter-types)
3. [Missing Syntax Features](#missing-syntax-features)
4. [Partially Implemented Features](#partially-implemented-features)
5. [Implementation Tasks](#implementation-tasks)

---

## Current Implementation Status

### Fully Implemented (Parser + Executor)

| Feature | Syntax | Location |
|---------|--------|----------|
| Self (concept reference) | `404684003` | `parser.rs:259-270` |
| Term in pipes | `404684003 \|Clinical finding\|` | `parser.rs:283-290` |
| DescendantOf | `< concept` | `parser.rs:219` |
| DescendantOrSelfOf | `<< concept` | `parser.rs:217` |
| AncestorOf | `> concept` | `parser.rs:223` |
| AncestorOrSelfOf | `>> concept` | `parser.rs:221` |
| ChildOf | `<! concept` | `parser.rs:218` |
| ChildOrSelfOf | `<<! concept` | `parser.rs:216` |
| ParentOf | `>! concept` | `parser.rs:222` |
| ParentOrSelfOf | `>>! concept` | `parser.rs:220` |
| MemberOf | `^ refsetId` | `parser.rs:227-242` |
| Wildcard (Any) | `*` | `parser.rs:255-257` |
| Conjunction | `AND`, `,` | `parser.rs:121-131` |
| Disjunction | `OR` | `parser.rs:124` |
| Exclusion | `MINUS` | `parser.rs:125` |
| Nested expressions | `( expression )` | `parser.rs:153-160` |
| Attribute refinement | `focus : attribute = value` | `parser.rs:449-471` |
| Attribute groups | `{ attr1, attr2 }` | `parser.rs:378-393` |
| Cardinality | `[min..max]` | `parser.rs:311-326` |
| Reverse flag | `R attribute` | `parser.rs:346` |
| Dot notation | `expression . attribute` | `parser.rs:478-500` |
| Concrete values | `#123`, `#3.14`, `#"string"` | `parser.rs:507-545` |
| Top of set | `!!> expression` | `parser.rs:704-709` |
| Bottom of set | `!!< expression` | `parser.rs:712-717` |

---

## Missing Filter Types

### 1. LanguageFilter

**Syntax:**
```
{{ language = en }}
{{ language = (en es fr) }}
```

**Specification:** Filters descriptions by ISO 639-1 language code.

**AST Addition Required (`ast.rs`):**
```rust
/// Language filter: `{{ language = en }}`
Language {
    /// Language codes to filter by (ISO 639-1)
    codes: Vec<String>,
},
```

**Parser Addition Required (`parser.rs`):**
```rust
fn language_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = tag_no_case("language")(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = ws(input)?;
    let (input, codes) = alt((
        // Multiple codes: (en es fr)
        map(
            delimited(char('('), separated_list1(ws, language_code), char(')')),
            |codes| codes,
        ),
        // Single code: en
        map(language_code, |code| vec![code]),
    ))(input)?;
    Ok((input, EclFilter::Language { codes }))
}

fn language_code(input: &str) -> IResult<&str, String> {
    map(
        take_while1(|c: char| c.is_ascii_lowercase()),
        |s: &str| s.to_string(),
    )(input)
}
```

**Executor Addition Required (`executor.rs`):**
- Requires `EclQueryable` trait extension for description language lookup
- Filter concepts whose descriptions match the specified language

---

### 2. TypeFilter (Description Type)

**Syntax:**
```
{{ typeId = 900000000000003001 }}
{{ type = syn }}
{{ type = (syn def) }}
```

**Specification:** Filters by description type (FSN, synonym, definition).

**Well-known type IDs:**
- `900000000000003001` - Fully specified name
- `900000000000013009` - Synonym
- `900000000000550004` - Definition

**Type aliases:**
- `fsn` or `fullname` = Fully specified name
- `syn` or `synonym` = Synonym
- `def` or `definition` = Definition

**AST Addition Required:**
```rust
/// Type filter: `{{ typeId = 900000000000003001 }}`
DescriptionType {
    /// Type IDs or type aliases
    type_ids: Vec<SctId>,
},
```

**Parser Addition Required:**
```rust
fn type_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = alt((tag_no_case("typeId"), tag_no_case("type")))(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = ws(input)?;
    let (input, type_ids) = alt((
        // Multiple: (syn def)
        delimited(char('('), separated_list1(ws, type_id_or_alias), char(')')),
        // Single
        map(type_id_or_alias, |id| vec![id]),
    ))(input)?;
    Ok((input, EclFilter::DescriptionType { type_ids }))
}

fn type_id_or_alias(input: &str) -> IResult<&str, SctId> {
    alt((
        sct_id,
        value(900000000000003001, alt((tag_no_case("fsn"), tag_no_case("fullname")))),
        value(900000000000013009, alt((tag_no_case("syn"), tag_no_case("synonym")))),
        value(900000000000550004, alt((tag_no_case("def"), tag_no_case("definition")))),
    ))(input)
}
```

---

### 3. DialectFilter

**Syntax:**
```
{{ dialect = en-US }}
{{ dialect = (en-US en-GB) }}
{{ dialectId = 900000000000509007 }}
```

**Specification:** Filters descriptions by dialect (language reference set).

**Well-known dialect aliases:**
- `en-US` = US English (`900000000000509007`)
- `en-GB` = GB English (`900000000000508004`)
- `en-AU` = Australian English
- `en-NZ` = New Zealand English
- `es` = Spanish
- etc.

**AST Addition Required:**
```rust
/// Dialect filter: `{{ dialect = en-US }}`
Dialect {
    /// Dialect reference set IDs
    dialect_ids: Vec<SctId>,
    /// Acceptability constraint (preferred, acceptable, or both)
    acceptability: Option<Acceptability>,
},

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acceptability {
    Preferred,
    Acceptable,
}
```

**Parser Addition Required:**
```rust
fn dialect_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = alt((tag_no_case("dialectId"), tag_no_case("dialect")))(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = ws(input)?;
    let (input, dialect_ids) = alt((
        delimited(char('('), separated_list1(ws, dialect_id_or_alias), char(')')),
        map(dialect_id_or_alias, |id| vec![id]),
    ))(input)?;
    // Optional acceptability
    let (input, acceptability) = opt(preceded(ws, acceptability_keyword))(input)?;
    Ok((input, EclFilter::Dialect { dialect_ids, acceptability }))
}

fn dialect_id_or_alias(input: &str) -> IResult<&str, SctId> {
    alt((
        sct_id,
        value(900000000000509007, tag_no_case("en-US")),
        value(900000000000508004, tag_no_case("en-GB")),
        // Add more aliases as needed
    ))(input)
}
```

---

### 4. DefinitionStatusFilter

**Syntax:**
```
{{ definitionStatus = primitive }}
{{ definitionStatus = defined }}
{{ definitionStatusId = 900000000000074008 }}
```

**Specification:** Filters concepts by their definition status.

**Well-known IDs:**
- `900000000000074008` - Primitive
- `900000000000073002` - Defined (sufficiently defined)

**AST Addition Required:**
```rust
/// Definition status filter: `{{ definitionStatus = primitive }}`
DefinitionStatus {
    /// True = primitive, False = defined
    is_primitive: bool,
},
```

**Parser Addition Required:**
```rust
fn definition_status_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = alt((
        tag_no_case("definitionStatusId"),
        tag_no_case("definitionStatus"),
    ))(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = ws(input)?;
    let (input, is_primitive) = alt((
        value(true, tag_no_case("primitive")),
        value(true, tag("900000000000074008")),
        value(false, tag_no_case("defined")),
        value(false, tag("900000000000073002")),
    ))(input)?;
    Ok((input, EclFilter::DefinitionStatus { is_primitive }))
}
```

---

### 5. EffectiveTimeFilter

**Syntax:**
```
{{ effectiveTime = 20200101 }}
{{ effectiveTime >= 20200101 }}
{{ effectiveTime < 20210601 }}
```

**Specification:** Filters components by their effective time (release date).

**AST Addition Required:**
```rust
/// Effective time filter: `{{ effectiveTime >= 20200101 }}`
EffectiveTime {
    /// Comparison operator
    operator: ComparisonOperator,
    /// Date in YYYYMMDD format
    date: u32,
},
```

**Parser Addition Required:**
```rust
fn effective_time_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = tag_no_case("effectiveTime")(input)?;
    let (input, _) = ws(input)?;
    let (input, operator) = comparison_operator(input)?;
    let (input, _) = ws(input)?;
    let (input, date_str) = digit1(input)?;
    let date = date_str.parse::<u32>().unwrap_or(0);
    Ok((input, EclFilter::EffectiveTime { operator, date }))
}
```

---

### 6. SemanticTagFilter

**Syntax:**
```
{{ semanticTag = "disorder" }}
{{ semanticTag = ("disorder" "finding") }}
```

**Specification:** Filters concepts by their semantic tag (from FSN).

**AST Addition Required:**
```rust
/// Semantic tag filter: `{{ semanticTag = "disorder" }}`
SemanticTag {
    /// Semantic tags to match
    tags: Vec<String>,
},
```

**Parser Addition Required:**
```rust
fn semantic_tag_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = tag_no_case("semanticTag")(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = ws(input)?;
    let (input, tags) = alt((
        delimited(char('('), separated_list1(ws, quoted_string), char(')')),
        map(quoted_string, |s| vec![s]),
    ))(input)?;
    Ok((input, EclFilter::SemanticTag { tags }))
}
```

---

### 7. PreferredInFilter

**Syntax:**
```
{{ preferredIn = 900000000000509007 }}
{{ preferredIn = (900000000000509007 900000000000508004) }}
```

**Specification:** Filters descriptions that are preferred in specified language reference sets.

**AST Addition Required:**
```rust
/// Preferred in filter: `{{ preferredIn = refsetId }}`
PreferredIn {
    /// Language reference set IDs
    refset_ids: Vec<SctId>,
},
```

---

### 8. AcceptableInFilter

**Syntax:**
```
{{ acceptableIn = 900000000000509007 }}
```

**Specification:** Filters descriptions that are acceptable in specified language reference sets.

**AST Addition Required:**
```rust
/// Acceptable in filter: `{{ acceptableIn = refsetId }}`
AcceptableIn {
    /// Language reference set IDs
    refset_ids: Vec<SctId>,
},
```

---

### 9. LanguageRefSetFilter

**Syntax:**
```
{{ languageRefSetId = 900000000000509007 }}
```

**Specification:** Filters by language reference set membership (either preferred or acceptable).

**AST Addition Required:**
```rust
/// Language reference set filter: `{{ languageRefSetId = refsetId }}`
LanguageRefSet {
    /// Language reference set IDs
    refset_ids: Vec<SctId>,
},
```

---

### 10. CaseSignificanceFilter

**Syntax:**
```
{{ caseSignificance = caseInsensitive }}
{{ caseSignificanceId = 900000000000448009 }}
```

**Specification:** Filters descriptions by case significance.

**Well-known IDs:**
- `900000000000448009` - Case insensitive
- `900000000000017005` - Case sensitive (initial character)
- `900000000000020002` - Case sensitive (entire term)

**AST Addition Required:**
```rust
/// Case significance filter: `{{ caseSignificance = caseInsensitive }}`
CaseSignificance {
    /// Case significance ID
    significance_id: SctId,
},
```

---

### 11. IdFilter

**Syntax:**
```
{{ id = 123456789 }}
{{ id = (123456 789012) }}
```

**Specification:** Filters components by their SNOMED CT identifier.

**AST Addition Required:**
```rust
/// ID filter: `{{ id = 123456 }}`
Id {
    /// Component IDs to match
    ids: Vec<SctId>,
},
```

---

## Missing Syntax Features

### 1. AlternateIdentifier

**Syntax:**
```
http://snomed.info/id#123456789
urn:oid:2.16.840.1.113883.6.96#123456789
```

**Specification:** Allows referencing concepts using alternate identifier schemes.

**AST Addition Required:**
```rust
/// Alternate identifier reference
AlternateIdentifier {
    /// The identifier scheme (URL or URN)
    scheme: String,
    /// The identifier value
    identifier: String,
},
```

**Parser Addition Required:**
```rust
fn alternate_identifier(input: &str) -> IResult<&str, EclExpression> {
    // Parse scheme (everything before #)
    let (input, scheme) = take_until("#")(input)?;
    let (input, _) = char('#')(input)?;
    let (input, identifier) = take_while1(|c: char| c.is_alphanumeric())(input)?;
    Ok((input, EclExpression::AlternateIdentifier {
        scheme: scheme.to_string(),
        identifier: identifier.to_string(),
    }))
}
```

---

### 2. HistorySupplement Profiles

**Syntax:**
```
{{ +HISTORY-MIN }}
{{ +HISTORY-MOD }}
{{ +HISTORY-MAX }}
```

**Specification:** Different levels of historical association inclusion:
- `MIN` - Only SAME_AS associations
- `MOD` - SAME_AS + REPLACED_BY + POSSIBLY_EQUIVALENT_TO
- `MAX` - All historical associations

**Current Implementation:** Only `+HISTORY` without profile support.

**AST Modification Required:**
```rust
/// History supplement: `{{+HISTORY}}` or `{{+HISTORY-MIN}}`
History {
    /// Optional profile (MIN, MOD, MAX). None = default behavior.
    profile: Option<HistoryProfile>,
},

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryProfile {
    /// Minimal: SAME_AS only
    Min,
    /// Moderate: SAME_AS, REPLACED_BY, POSSIBLY_EQUIVALENT_TO
    Mod,
    /// Maximum: All historical associations
    Max,
}
```

**Parser Modification Required:**
```rust
fn history_filter(input: &str) -> IResult<&str, EclFilter> {
    let (input, _) = tag("+")(input)?;
    let (input, _) = tag_no_case("HISTORY")(input)?;
    let (input, profile) = opt(preceded(
        char('-'),
        alt((
            value(HistoryProfile::Min, tag_no_case("MIN")),
            value(HistoryProfile::Mod, tag_no_case("MOD")),
            value(HistoryProfile::Max, tag_no_case("MAX")),
        )),
    ))(input)?;
    Ok((input, EclFilter::History { profile }))
}
```

---

### 3. EclConceptReferenceSet

**Syntax:**
```
(123456 789012 345678)
```

**Specification:** A parenthesized list of concept IDs treated as a set.

**AST Addition Required:**
```rust
/// Concept reference set: `(123 456 789)`
ConceptSet {
    /// The concept IDs in the set
    concept_ids: Vec<SctId>,
},
```

**Parser Addition Required:**
```rust
fn concept_reference_set(input: &str) -> IResult<&str, EclExpression> {
    let (input, _) = char('(')(input)?;
    let (input, _) = ws(input)?;
    let (input, ids) = separated_list1(multispace1, sct_id)(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char(')')(input)?;
    Ok((input, EclExpression::ConceptSet { concept_ids: ids }))
}
```

---

### 4. Short Domain Prefixes

**Syntax:**
```
{{ C definitionStatus = primitive }}  // Concept filter
{{ D term = "heart" }}                 // Description filter
{{ M mapTarget = "J45" }}              // Member filter (already partial)
```

**Specification:** Single-letter prefixes to specify filter domain.

**Current Implementation:** Only `M` for member filters is implemented.

**Parser Modification Required:**
```rust
fn filter_with_domain(input: &str) -> IResult<&str, EclFilter> {
    let (input, domain) = opt(alt((
        value(FilterDomain::Concept, char('C')),
        value(FilterDomain::Description, char('D')),
        value(FilterDomain::Member, char('M')),
    )))(input)?;
    let (input, _) = ws(input)?;
    // Parse the actual filter and attach domain
    // ...
}
```

---

### 5. Boolean Concrete Values

**Syntax:**
```
#true
#false
```

**Specification:** Boolean concrete values for attribute comparisons.

**AST Modification Required:**
```rust
pub enum ConcreteValue {
    Integer(i64),
    Decimal(f64),
    String(String),
    Boolean(bool),  // ADD THIS
}
```

**Parser Addition Required:**
```rust
fn concrete_value(input: &str) -> IResult<&str, ConcreteValue> {
    let (input, _) = char('#')(input)?;
    alt((
        // Boolean values (must come before string to avoid conflicts)
        value(ConcreteValue::Boolean(true), tag_no_case("true")),
        value(ConcreteValue::Boolean(false), tag_no_case("false")),
        // String value
        map(
            delimited(char('"'), take_until("\""), char('"')),
            |s: &str| ConcreteValue::String(s.to_string()),
        ),
        // Decimal or integer
        // ... existing code ...
    ))(input)
}
```

---

### 6. Numeric Comparison Operators in Refinements

**Syntax:**
```
< 404684003 : 363698007 >= #100
< 404684003 : 363698007 < #250
```

**Specification:** Numeric comparison operators for concrete value constraints.

**Current Implementation:** Only `=` is effectively used. The `ComparisonOperator` enum exists but isn't fully wired up in refinement parsing.

**Parser Fix Required:**
```rust
fn concrete_value_comparison(input: &str) -> IResult<&str, (ComparisonOperator, ConcreteValue)> {
    let (input, operator) = alt((
        value(ComparisonOperator::LessThanOrEqual, tag("<=")),
        value(ComparisonOperator::GreaterThanOrEqual, tag(">=")),
        value(ComparisonOperator::NotEqual, tag("!=")),
        value(ComparisonOperator::LessThan, char('<')),
        value(ComparisonOperator::GreaterThan, char('>')),
        value(ComparisonOperator::Equal, char('=')),
    ))(input)?;
    let (input, _) = ws(input)?;
    let (input, value) = concrete_value(input)?;
    Ok((input, (operator, value)))
}
```

---

### 7. Wildcard Term Matching

**Syntax:**
```
{{ term wild "diab*" }}
{{ term wild "*itis" }}
```

**Specification:** Wildcard pattern matching for term filters using `*` as wildcard.

**AST Modification Required:**
```rust
pub enum TermMatchType {
    Contains,     // term = "x"
    StartsWith,   // term startsWith "x"
    Regex,        // term regex "x"
    Exact,        // term == "x"
    Wildcard,     // term wild "x*"  // ADD THIS
}
```

**Parser Addition Required:**
```rust
fn term_filter(input: &str) -> IResult<&str, EclFilter> {
    // ... existing code ...
    let (input, match_type) = alt((
        value(TermMatchType::Wildcard, tag_no_case("wild")),  // ADD THIS
        value(TermMatchType::StartsWith, tag_no_case("startsWith")),
        value(TermMatchType::Regex, tag_no_case("regex")),
        value(TermMatchType::Exact, tag("==")),
        value(TermMatchType::Contains, char('=')),
    ))(input)?;
    // ...
}
```

---

### 8. Nested MemberOf with Expression

**Syntax:**
```
^ (<< 123456)
^ (<< 123456 {{ M mapTarget = "J45" }})
```

**Specification:** MemberOf can take a nested expression, not just a concept ID.

**Current Implementation:** Only parses simple concept reference after `^`.

**AST Modification Required:**
```rust
/// Reference set membership.
/// Syntax: `^ refsetExpression`
MemberOf {
    /// The reference set expression (can be nested)
    refset: Box<EclExpression>,
},
```

**Parser Modification Required:**
```rust
fn member_of_expression(input: &str) -> IResult<&str, EclExpression> {
    let (input, _) = char('^')(input)?;
    let (input, _) = ws(input)?;
    // Can be a nested expression or simple concept
    let (input, inner) = alt((
        // Nested expression
        delimited(
            pair(char('('), ws),
            compound_or_simple_expression,
            pair(ws, char(')')),
        ),
        // Simple concept reference
        focus_concept,
    ))(input)?;
    Ok((input, EclExpression::MemberOf { refset: Box::new(inner) }))
}
```

---

## Partially Implemented Features

### 1. Filter Execution

**Issue:** Parser supports filters but executor's `apply_filter` method has limited implementation.

**Current `executor.rs` filter handling:**
```rust
fn apply_filter(
    &self,
    concepts: &HashSet<SctId>,
    filter: &EclFilter,
) -> EclResult<HashSet<SctId>> {
    // Currently returns concepts unchanged for most filters
    // Needs EclQueryable extensions
}
```

**Required `EclQueryable` trait extensions (`traits.rs`):**
```rust
pub trait EclQueryable {
    // Existing methods...

    // NEW: Filter support methods
    fn get_descriptions(&self, concept_id: SctId) -> Vec<DescriptionInfo>;
    fn get_concept_definition_status(&self, concept_id: SctId) -> Option<bool>;
    fn get_concept_module(&self, concept_id: SctId) -> Option<SctId>;
    fn get_concept_effective_time(&self, concept_id: SctId) -> Option<u32>;
    fn get_semantic_tag(&self, concept_id: SctId) -> Option<String>;
}

#[derive(Debug, Clone)]
pub struct DescriptionInfo {
    pub description_id: SctId,
    pub term: String,
    pub type_id: SctId,
    pub language_code: String,
    pub case_significance_id: SctId,
    pub acceptability: HashMap<SctId, Acceptability>, // refset_id -> acceptability
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acceptability {
    Preferred,
    Acceptable,
}
```

---

### 2. Reverse Flag Execution

**Issue:** Parser parses `R` flag but executor doesn't fully handle reversed relationships.

**Current behavior:** The `reverse` flag is parsed into `AttributeConstraint.reverse` but execution needs verification.

**Required executor change:**
```rust
fn evaluate_attribute_constraint(...) -> EclResult<bool> {
    // When constraint.reverse is true, look for relationships
    // where the concept is the DESTINATION, not the source
    if constraint.reverse {
        // Get inbound relationships instead of outbound
        let inbound_rels = self.store.get_inbound_relationships(concept_id);
        // ... evaluate against inbound
    } else {
        let attributes = self.store.get_attributes(concept_id);
        // ... existing logic
    }
}
```

**Required trait extension:**
```rust
pub trait EclQueryable {
    // Existing...

    /// Get relationships where this concept is the destination (for reverse flag)
    fn get_inbound_relationships(&self, concept_id: SctId) -> Vec<RelationshipInfo>;
}
```

---

## Implementation Tasks

### Phase 1: Complete Filter Support (High Priority)

| Task | Files to Modify | Effort |
|------|-----------------|--------|
| 1.1 Add LanguageFilter | `ast.rs`, `parser.rs`, `executor.rs`, `traits.rs` | Medium |
| 1.2 Add TypeFilter | `ast.rs`, `parser.rs`, `executor.rs` | Medium |
| 1.3 Add DialectFilter | `ast.rs`, `parser.rs`, `executor.rs`, `traits.rs` | Medium |
| 1.4 Add DefinitionStatusFilter | `ast.rs`, `parser.rs`, `executor.rs`, `traits.rs` | Low |
| 1.5 Add SemanticTagFilter | `ast.rs`, `parser.rs`, `executor.rs`, `traits.rs` | Medium |
| 1.6 Add EffectiveTimeFilter | `ast.rs`, `parser.rs`, `executor.rs`, `traits.rs` | Low |
| 1.7 Add PreferredIn/AcceptableIn filters | `ast.rs`, `parser.rs`, `executor.rs` | Medium |
| 1.8 Add CaseSignificanceFilter | `ast.rs`, `parser.rs`, `executor.rs` | Low |
| 1.9 Add IdFilter | `ast.rs`, `parser.rs`, `executor.rs` | Low |
| 1.10 Add LanguageRefSetFilter | `ast.rs`, `parser.rs`, `executor.rs` | Low |
| 1.11 Extend EclQueryable trait | `traits.rs` | High |
| 1.12 Implement filter execution | `executor.rs` | High |

### Phase 2: Complete Syntax Support (Medium Priority)

| Task | Files to Modify | Effort |
|------|-----------------|--------|
| 2.1 Add HistorySupplement profiles | `ast.rs`, `parser.rs`, `executor.rs` | Low |
| 2.2 Add EclConceptReferenceSet | `ast.rs`, `parser.rs`, `executor.rs` | Medium |
| 2.3 Add Boolean concrete values | `ast.rs`, `parser.rs` | Low |
| 2.4 Fix numeric comparison in refinements | `parser.rs`, `executor.rs` | Medium |
| 2.5 Add wildcard term matching | `ast.rs`, `parser.rs`, `executor.rs` | Low |
| 2.6 Add short domain prefixes | `ast.rs`, `parser.rs` | Low |
| 2.7 Enhance MemberOf with nested expressions | `ast.rs`, `parser.rs`, `executor.rs` | Medium |

### Phase 3: Advanced Features (Low Priority)

| Task | Files to Modify | Effort |
|------|-----------------|--------|
| 3.1 Add AlternateIdentifier | `ast.rs`, `parser.rs`, `executor.rs`, `traits.rs` | Medium |
| 3.2 Implement reverse flag execution | `executor.rs`, `traits.rs` | Medium |
| 3.3 Add comprehensive filter tests | `parser.rs` tests, new test files | High |
| 3.4 Add ECL 2.2 version marker | `lib.rs` | Low |

---

## Testing Requirements

Each new feature requires:

1. **Parser tests** - Verify syntax is correctly parsed
2. **AST Display tests** - Verify expression can be serialized back
3. **Executor tests** - Verify correct execution with mock store
4. **Integration tests** - End-to-end with real SNOMED data (if available)

Example test structure:
```rust
#[cfg(test)]
mod language_filter_tests {
    use super::*;

    #[test]
    fn test_parse_single_language() {
        let expr = parse("<< 404684003 {{ language = en }}").unwrap();
        // Assert correct AST structure
    }

    #[test]
    fn test_parse_multiple_languages() {
        let expr = parse("<< 404684003 {{ language = (en es fr) }}").unwrap();
        // Assert correct AST structure
    }

    #[test]
    fn test_execute_language_filter() {
        let store = MockStoreWithDescriptions::new();
        let executor = EclExecutor::new(&store);
        let result = executor.execute("<< 404684003 {{ language = en }}").unwrap();
        // Assert only English descriptions included
    }
}
```

---

## References

- [ECL Specification (SNOMED International)](https://docs.snomed.org/snomed-ct-specifications/snomed-ct-expression-constraint-language)
- [ECL Quick Reference](https://confluence.ihtsdotools.org/display/DOCECL/Appendix+D+-+ECL+Quick+Reference)
