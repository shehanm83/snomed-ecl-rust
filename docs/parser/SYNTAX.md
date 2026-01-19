# ECL Syntax Reference

Complete reference for all ECL syntax supported by the `snomed-ecl` parser.

## Table of Contents

1. [Concept References](#concept-references)
2. [Hierarchy Operators](#hierarchy-operators)
3. [Compound Expressions](#compound-expressions)
4. [Refinements](#refinements)
5. [Attribute Groups](#attribute-groups)
6. [Cardinality](#cardinality)
7. [Concrete Values](#concrete-values)
8. [Dot Notation](#dot-notation)
9. [Nested Expressions](#nested-expressions)
10. [Alternate Identifiers](#alternate-identifiers)

---

## Concept References

### Basic Concept ID

A SNOMED CT concept identified by its numeric ID (SCTID):

```
73211009
```

**Rust AST:**
```rust
EclExpression::ConceptReference {
    concept_id: 73211009,
    term: None
}
```

### Concept with Term

Include a human-readable term in pipes for clarity:

```
73211009 |Diabetes mellitus|
```

**Rust AST:**
```rust
EclExpression::ConceptReference {
    concept_id: 73211009,
    term: Some("Diabetes mellitus".to_string())
}
```

**Note:** The term is for documentation only and does not affect query semantics.

---

## Hierarchy Operators

### Descendant Of (`<`)

All concepts that are subtypes of the specified concept (not including itself):

```
< 73211009 |Diabetes mellitus|
```

**Returns:** Type 1 diabetes, Type 2 diabetes, Gestational diabetes, etc.
**Does NOT include:** 73211009 itself

**Rust AST:**
```rust
EclExpression::DescendantOf(Box::new(
    EclExpression::ConceptReference { concept_id: 73211009, term: Some("Diabetes mellitus") }
))
```

### Descendant or Self Of (`<<`)

The concept itself plus all its subtypes:

```
<< 73211009 |Diabetes mellitus|
```

**Returns:** 73211009 plus Type 1 diabetes, Type 2 diabetes, etc.

### Ancestor Of (`>`)

All concepts that the specified concept is a subtype of:

```
> 46635009 |Type 1 diabetes mellitus|
```

**Returns:** Diabetes mellitus, Disorder of glucose metabolism, Disease, Clinical finding, etc.

### Ancestor or Self Of (`>>`)

The concept itself plus all its supertypes:

```
>> 46635009 |Type 1 diabetes mellitus|
```

### Child Of (`<!`)

Direct subtypes only (one level down):

```
<! 73211009 |Diabetes mellitus|
```

**Returns:** Only immediate children like Type 1 diabetes, Type 2 diabetes
**Does NOT include:** Grandchildren like "Type 1 diabetes with ketoacidosis"

### Child or Self Of (`<<!`)

The concept plus its direct children:

```
<<! 73211009
```

### Parent Of (`>!`)

Direct supertypes only (one level up):

```
>! 46635009 |Type 1 diabetes mellitus|
```

**Returns:** Only immediate parent(s), typically just "Diabetes mellitus"

### Parent or Self Of (`>>!`)

The concept plus its direct parents:

```
>>! 46635009
```

### Wildcard / Any (`*`)

Matches all concepts:

```
*
```

**Use case:** Often combined with refinements to find "any concept with attribute X"

---

## Compound Expressions

### Conjunction (AND)

Intersection of two sets - concepts must be in BOTH:

```
<< 73211009 AND << 64572001
```

**Alternative syntax using comma:**
```
<< 73211009, << 64572001
```

**Rust AST:**
```rust
EclExpression::And(
    Box::new(EclExpression::DescendantOrSelfOf(...)),
    Box::new(EclExpression::DescendantOrSelfOf(...))
)
```

### Disjunction (OR)

Union of two sets - concepts in either or both:

```
<< 73211009 OR << 38341003
```

**Returns:** All descendants of Diabetes mellitus OR all descendants of Hypertension

### Exclusion (MINUS)

Set difference - concepts in first set but not second:

```
<< 73211009 MINUS << 46635009
```

**Returns:** Diabetes concepts EXCEPT Type 1 diabetes and its subtypes

### Operator Precedence

From highest to lowest:
1. Parentheses `()`
2. `MINUS`
3. `AND` / `,`
4. `OR`

**Example:**
```
A OR B AND C MINUS D
```

Parsed as:
```
A OR ((B AND C) MINUS D)
```

Use parentheses to override:
```
(A OR B) AND (C MINUS D)
```

---

## Refinements

Refinements constrain concepts based on their attribute relationships.

### Basic Attribute Constraint

```
< 404684003 : 363698007 = << 39057004
│             │           │
│             │           └── Value constraint
│             └── Attribute type
└── Focus concepts
```

**Meaning:** Clinical findings with Finding site = Pulmonary valve (or subtype)

### Attribute Name (Type)

The attribute type is a SNOMED concept:

| Concept ID | Attribute Name |
|------------|----------------|
| 363698007 | Finding site |
| 116676008 | Associated morphology |
| 246075003 | Causative agent |
| 370135005 | Pathological process |
| 255234002 | After |

### Value Constraints

The value can be any ECL expression:

```rust
// Exact concept
: 363698007 = 39057004

// Descendants
: 363698007 = < 39057004

// Descendants or self
: 363698007 = << 39057004

// Any value (wildcard)
: 363698007 = *

// Nested expression
: 363698007 = (<< 39057004 OR << 80891009)
```

### Comparison Operators

| Operator | Meaning |
|----------|---------|
| `=` | Equals |
| `!=` | Not equals |

### Multiple Attributes

Use comma to require multiple attributes:

```
< 404684003 : 363698007 = << 39057004, 116676008 = << 49755003
```

**Meaning:** Clinical findings with Finding site = Pulmonary valve AND Associated morphology = Morphologic abnormality

### Reverse Flag (`R`)

Query by attribute destination instead of source:

```
< 123037004 : R 363698007 = *
```

**Meaning:** Body structures that ARE the finding site of something

---

## Attribute Groups

Groups bundle related attributes that must occur together.

### Single Group

```
< 404684003 : { 363698007 = << 39057004, 116676008 = << 49755003 }
```

**Meaning:** Clinical findings where Finding site AND Associated morphology occur in the SAME relationship group

### Multiple Groups

```
< 404684003 : { 363698007 = << 39057004 } { 116676008 = << 49755003 }
```

**Meaning:** Must have one group with Finding site AND a (possibly different) group with Associated morphology

### Ungrouped vs Grouped

```
// Ungrouped - attributes don't need to be in same group
: 363698007 = X, 116676008 = Y

// Grouped - attributes MUST be in same group
: { 363698007 = X, 116676008 = Y }
```

---

## Cardinality

Specify how many times an attribute must/may occur.

### Syntax

```
[min..max]
```

### Examples

```
// Exactly one
: [1..1] 363698007 = *

// At least one
: [1..*] 363698007 = *

// Zero or one (optional)
: [0..1] 363698007 = *

// Zero or more (any number)
: [0..*] 363698007 = *

// Between 2 and 5
: [2..5] 363698007 = *
```

### Group Cardinality

```
// Exactly 2 groups with this attribute
: [2..2] { 363698007 = * }

// At least one group
: [1..*] { 363698007 = * }
```

---

## Concrete Values

ECL supports concrete (literal) values for data properties.

### Integer Values

```
: 3311482005 |Has strength numerator value| = #500
: 3311482005 >= #100
: 3311482005 < #1000
```

### Decimal Values

```
: 3311482005 = #3.14
: 3311482005 >= #0.5
```

### String Values

```
: 3311482005 = #"500mg"
```

### Boolean Values

```
: 3311482005 = #true
: 3311482005 = #false
```

### Comparison Operators for Concrete Values

| Operator | Meaning |
|----------|---------|
| `=` | Equals |
| `!=` | Not equals |
| `<` | Less than |
| `<=` | Less than or equal |
| `>` | Greater than |
| `>=` | Greater than or equal |

---

## Dot Notation

Navigate attribute chains to find related concepts.

### Basic Dot Notation

```
< 404684003 . 363698007
│             │
│             └── Get the Finding site attribute values
└── Start with Clinical findings
```

**Returns:** The finding sites (body structures) of all clinical findings

### Chained Dot Notation

```
< 404684003 . 363698007 . 272741003
```

**Meaning:** Get the laterality of the finding sites of clinical findings

### With Refinement

```
(< 404684003 . 363698007) : 272741003 = 7771000
```

---

## Nested Expressions

Use parentheses to group expressions:

### Grouping for Precedence

```
(<< 73211009 OR << 38341003) AND << 404684003
```

### Complex Nesting

```
((<< 73211009 MINUS << 46635009) OR << 38341003) AND << 64572001
```

### With Hierarchy Operators

```
<< (^ 700043003)
```

**Meaning:** Descendants of members of the reference set

---

## Alternate Identifiers

Reference concepts using URIs instead of SCTIDs.

### Fragment Syntax

```
http://snomed.info/sct#73211009
```

### Path Syntax

```
http://snomed.info/id/73211009
```

### Usage

```
<< http://snomed.info/id/73211009
```

---

## Member Of

Query reference set membership.

### Basic Member Of

```
^ 700043003
```

**Returns:** All concepts that are members of reference set 700043003

### With Term

```
^ 700043003 |Example problem list reference set|
```

### Nested Member Of

```
^ (<< 446609009)
```

**Meaning:** Members of any reference set that is a subtype of Simple type reference set

### Combined with Hierarchy

```
<< (^ 700043003)
```

**Meaning:** Descendants of refset members

---

## Complete Grammar Summary

```
expressionConstraint = subExpressionConstraint
                     | compoundExpressionConstraint

compoundExpressionConstraint = conjunctionExpressionConstraint
                             | disjunctionExpressionConstraint
                             | exclusionExpressionConstraint

conjunctionExpressionConstraint = subExpressionConstraint
                                  ("AND" | ",") subExpressionConstraint

subExpressionConstraint = [constraintOperator] focusConcept [refinement] [filter]

constraintOperator = "<" | "<<" | ">" | ">>" | "<!" | ">!" | "^" | "<<!" | ">>!"

focusConcept = conceptReference | wildcard | "(" expressionConstraint ")"

conceptReference = conceptId ["|" term "|"]

refinement = ":" refinementSet

filter = "{{" filterConstraint "}}"
```

---

## Next Steps

- See [FILTERS.md](FILTERS.md) for detailed filter documentation
- See [USAGE.md](USAGE.md) for code examples
