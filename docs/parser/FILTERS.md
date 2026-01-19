# ECL Filters Reference

Complete guide to ECL filter constraints supported by the `snomed-ecl` parser.

## What are Filters?

Filters are post-constraints that narrow query results based on concept or description metadata. They are applied AFTER the main expression is evaluated.

```
<< 73211009 {{ active = true }}
│            │
│            └── Filter: only active concepts
└── Main expression: Diabetes and subtypes
```

## Filter Syntax

Filters are enclosed in double curly braces `{{ }}`:

```
expression {{ filter }}
expression {{ filter1, filter2 }}
expression {{ filter1 }} {{ filter2 }}
```

---

## Concept Filters

### Active Filter

Filter by concept active/inactive status.

**Syntax:**
```
{{ active = true }}
{{ active = false }}
```

**Examples:**
```ecl
// Only active diabetes concepts
<< 73211009 {{ active = true }}

// Only inactive concepts (historical)
<< 73211009 {{ active = false }}
```

**Use Case:** Exclude retired/inactive concepts from clinical queries.

---

### Definition Status Filter

Filter by whether concept is primitive or fully defined.

**Syntax:**
```
{{ definitionStatus = primitive }}
{{ definitionStatus = defined }}
{{ definitionStatus = 900000000000074008 }}  // Primitive by ID
{{ definitionStatus = 900000000000073002 }}  // Defined by ID
```

**Examples:**
```ecl
// Only fully defined concepts
<< 73211009 {{ definitionStatus = defined }}

// Only primitive concepts
<< 73211009 {{ definitionStatus = primitive }}
```

**Background:**
- **Primitive:** Concept defined only by necessary conditions (may have more unspecified attributes)
- **Fully Defined:** Concept defined by necessary AND sufficient conditions (complete definition)

---

### Module Filter

Filter by SNOMED CT module.

**Syntax:**
```
{{ moduleId = 900000000000207008 }}
{{ moduleId = (900000000000207008 900000000000012004) }}
```

**Examples:**
```ecl
// Only International Edition concepts
<< 73211009 {{ moduleId = 900000000000207008 }}

// International or US Edition
<< 73211009 {{ moduleId = (900000000000207008 731000124108) }}
```

**Common Module IDs:**
| Module ID | Name |
|-----------|------|
| 900000000000207008 | SNOMED CT core module |
| 900000000000012004 | SNOMED CT model component |
| 731000124108 | US National Library of Medicine |

---

### Effective Time Filter

Filter by when the concept was created or last modified.

**Syntax:**
```
{{ effectiveTime = 20200101 }}
{{ effectiveTime > 20200101 }}
{{ effectiveTime >= 20200101 }}
{{ effectiveTime < 20200101 }}
{{ effectiveTime <= 20200101 }}
```

**Examples:**
```ecl
// Concepts added/modified in 2020 or later
<< 73211009 {{ effectiveTime >= 20200101 }}

// Concepts from exactly January 2020 release
<< 73211009 {{ effectiveTime = 20200131 }}
```

**Format:** YYYYMMDD (8 digits)

---

### ID Filter

Filter to specific concept IDs.

**Syntax:**
```
{{ id = 73211009 }}
{{ id = (73211009 46635009 44054006) }}
```

**Examples:**
```ecl
// From diabetes subtypes, get only these two
<< 73211009 {{ id = (46635009 44054006) }}
```

**Use Case:** Combine broad ECL with specific ID list for validation.

---

## Description Filters

### Term Filter

Filter by description text.

**Syntax:**
```
{{ term = "diabetes" }}
{{ term match "diabetes" }}
{{ term wild "diab*" }}
```

**Match Types:**
| Type | Syntax | Behavior |
|------|--------|----------|
| Contains | `term = "x"` | Term contains "x" (case-insensitive) |
| Match | `term match "x"` | Exact match |
| Wildcard | `term wild "x*"` | Wildcard pattern (* = any characters) |

**Examples:**
```ecl
// Concepts with "heart" in any description
<< 404684003 {{ term = "heart" }}

// Wildcard search
<< 404684003 {{ term wild "card*" }}

// Multiple terms (OR logic)
<< 404684003 {{ term = ("heart" "cardiac") }}
```

---

### Language Filter

Filter descriptions by language code.

**Syntax:**
```
{{ language = en }}
{{ language = (en es) }}
```

**Examples:**
```ecl
// Only match English descriptions
<< 73211009 {{ language = en }}

// English or Spanish
<< 73211009 {{ language = (en es) }}
```

**Common Language Codes:**
| Code | Language |
|------|----------|
| en | English |
| es | Spanish |
| fr | French |
| de | German |
| nl | Dutch |
| sv | Swedish |

---

### Description Type Filter

Filter by description type (FSN, synonym, definition).

**Syntax:**
```
{{ typeId = 900000000000003001 }}     // FSN
{{ typeId = 900000000000013009 }}     // Synonym
{{ typeId = 900000000000550004 }}     // Definition
{{ type = fsn }}                       // Alias
{{ type = syn }}                       // Alias
{{ type = def }}                       // Alias
```

**Examples:**
```ecl
// Only match Fully Specified Names
<< 73211009 {{ type = fsn }}

// Only synonyms containing "diabetes"
<< 73211009 {{ type = syn, term = "diabetes" }}
```

**Description Types:**
| ID | Alias | Name |
|----|-------|------|
| 900000000000003001 | fsn | Fully Specified Name |
| 900000000000013009 | syn | Synonym |
| 900000000000550004 | def | Definition |

---

### Dialect Filter

Filter by language reference set (dialect).

**Syntax:**
```
{{ dialect = en-US }}
{{ dialect = en-GB }}
{{ dialect = 900000000000509007 }}  // US English by ID
{{ dialect = 900000000000508004 }}  // GB English by ID
```

**With Acceptability:**
```
{{ dialect = en-US (preferred) }}
{{ dialect = en-US (acceptable) }}
```

**Examples:**
```ecl
// Concepts with US English preferred term
<< 73211009 {{ dialect = en-US (preferred) }}

// US or GB English
<< 73211009 {{ dialect = (en-US en-GB) }}
```

**Common Dialect IDs:**
| ID | Alias | Name |
|----|-------|------|
| 900000000000509007 | en-US | US English |
| 900000000000508004 | en-GB | GB English |

---

### Case Significance Filter

Filter by description case sensitivity.

**Syntax:**
```
{{ caseSignificance = caseInsensitive }}
{{ caseSignificance = caseSensitive }}
{{ caseSignificance = 900000000000448009 }}  // Case insensitive
{{ caseSignificance = 900000000000017005 }}  // Initial character case sensitive
```

**Examples:**
```ecl
// Only case-insensitive descriptions
<< 73211009 {{ caseSignificance = caseInsensitive }}
```

---

### Acceptability Filters

#### Preferred In

Concepts with preferred term in specific language reference set.

**Syntax:**
```
{{ preferredIn = 900000000000509007 }}  // US English
{{ preferredIn = (900000000000509007 900000000000508004) }}
```

**Examples:**
```ecl
// Has preferred term in US English
<< 73211009 {{ preferredIn = 900000000000509007 }}
```

#### Acceptable In

Concepts with acceptable term in specific language reference set.

**Syntax:**
```
{{ acceptableIn = 900000000000509007 }}
```

---

### Language Reference Set Filter

Filter by language reference set membership.

**Syntax:**
```
{{ languageRefSetId = 900000000000509007 }}
```

---

### Semantic Tag Filter

Filter by semantic tag (from Fully Specified Name).

**Syntax:**
```
{{ semanticTag = "disorder" }}
{{ semanticTag = ("disorder" "finding") }}
```

**Examples:**
```ecl
// Only disorders
<< 404684003 {{ semanticTag = "disorder" }}

// Disorders or findings
<< 404684003 {{ semanticTag = ("disorder" "finding") }}
```

**Common Semantic Tags:**
| Tag | Meaning |
|-----|---------|
| disorder | Disease/condition |
| finding | Clinical finding |
| procedure | Medical procedure |
| substance | Chemical/drug |
| body structure | Anatomical structure |
| organism | Living organism |
| product | Medicinal product |

---

## Member Filters

Filter reference set members by member properties.

### Member Filter

**Syntax:**
```
{{ M moduleId = 900000000000207008 }}
{{ M effectiveTime >= 20200101 }}
{{ M active = true }}
```

**Examples:**
```ecl
// Refset members from international module
^ 700043003 {{ M moduleId = 900000000000207008 }}

// Active refset members only
^ 700043003 {{ M active = true }}
```

---

## Domain Prefixes

Specify which domain a filter applies to.

| Prefix | Domain | Applies To |
|--------|--------|------------|
| C | Concept | Concept-level properties |
| D | Description | Description-level properties |
| M | Member | Reference set member properties |

**Examples:**
```ecl
// Filter on concept active status
<< 73211009 {{ C active = true }}

// Filter on description active status
<< 73211009 {{ D active = true }}

// Filter on member active status
^ 700043003 {{ M active = true }}
```

---

## History Supplement

Include historical associations for inactive concepts.

**Syntax:**
```
{{ +HISTORY }}
{{ +HISTORY-MIN }}
{{ +HISTORY-MOD }}
{{ +HISTORY-MAX }}
```

**Profiles:**
| Profile | Includes |
|---------|----------|
| MIN | SAME_AS associations only |
| MOD | SAME_AS + REPLACED_BY |
| MAX | All historical association types |

**Examples:**
```ecl
// Include SAME_AS replacements for inactive concepts
<< 73211009 {{ +HISTORY-MIN }}

// Include all historical associations
<< 73211009 {{ +HISTORY-MAX }}
```

---

## Combining Filters

### Multiple Filters (AND)

All filters must match:

```ecl
<< 73211009 {{ active = true, definitionStatus = primitive }}
```

### Separate Filter Clauses

```ecl
<< 73211009 {{ active = true }} {{ term = "insulin" }}
```

### Complex Example

```ecl
<< 404684003 {{
    active = true,
    definitionStatus = primitive
}} {{
    term = "heart",
    type = syn,
    language = en
}}
```

---

## Filter AST Representation

```rust
pub enum EclFilter {
    Active(bool),
    DefinitionStatus { is_primitive: bool },
    ModuleId { module_ids: Vec<SctId> },
    EffectiveTime { operator: ComparisonOperator, time: u32 },
    Id { ids: Vec<SctId> },
    Term { match_type: TermMatchType, terms: Vec<String> },
    Language { codes: Vec<String> },
    DescriptionType { type_ids: Vec<SctId> },
    Dialect { dialect_ids: Vec<SctId>, acceptability: Option<FilterAcceptability> },
    CaseSignificance { case_significance_ids: Vec<SctId> },
    PreferredIn { refset_ids: Vec<SctId> },
    AcceptableIn { refset_ids: Vec<SctId> },
    LanguageRefSet { refset_ids: Vec<SctId> },
    SemanticTag { tags: Vec<String> },
    Member { field: String, value: MemberFieldValue },
    History { profile: Option<HistoryProfile> },
    DomainQualified { domain: FilterDomain, filter: Box<EclFilter> },
}
```

---

## Best Practices

1. **Use `active = true`** for clinical queries to exclude retired concepts
2. **Combine filters** to narrow results efficiently
3. **Use semantic tags** to filter by concept type
4. **Consider dialect** when matching terms for international systems
5. **Use history supplement** when maintaining mappings to inactive concepts

---

## Next Steps

- See [SYNTAX.md](SYNTAX.md) for core ECL syntax
- See [USAGE.md](USAGE.md) for code examples
