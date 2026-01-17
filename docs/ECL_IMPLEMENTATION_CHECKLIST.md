# ECL Implementation Checklist

Track implementation progress for ECL specification compliance.

## Legend
- [ ] Not started
- [~] In progress
- [x] Completed
- [N/A] Not applicable

---

## Phase 1: Filter Types (High Priority)

### 1.1 LanguageFilter `{{ language = en }}`

- [ ] AST: Add `EclFilter::Language` variant
- [ ] Parser: Implement `language_filter()` function
- [ ] Parser: Support single language code
- [ ] Parser: Support multiple codes `(en es fr)`
- [ ] Traits: Add `get_descriptions()` to `EclQueryable`
- [ ] Traits: Add `DescriptionInfo` struct
- [ ] Executor: Implement language filter execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.2 TypeFilter `{{ typeId = 900000000000003001 }}`

- [ ] AST: Add `EclFilter::DescriptionType` variant
- [ ] Parser: Implement `type_filter()` function
- [ ] Parser: Support type IDs
- [ ] Parser: Support type aliases (`fsn`, `syn`, `def`)
- [ ] Parser: Support multiple types
- [ ] Executor: Implement type filter execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.3 DialectFilter `{{ dialect = en-US }}`

- [ ] AST: Add `EclFilter::Dialect` variant
- [ ] AST: Add `Acceptability` enum
- [ ] Parser: Implement `dialect_filter()` function
- [ ] Parser: Support dialect IDs
- [ ] Parser: Support dialect aliases
- [ ] Parser: Support multiple dialects
- [ ] Parser: Support acceptability constraint
- [ ] Executor: Implement dialect filter execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.4 DefinitionStatusFilter `{{ definitionStatus = primitive }}`

- [ ] AST: Add `EclFilter::DefinitionStatus` variant
- [ ] Parser: Implement `definition_status_filter()` function
- [ ] Parser: Support status keywords (`primitive`, `defined`)
- [ ] Parser: Support status IDs
- [ ] Traits: Add `get_concept_definition_status()` to `EclQueryable`
- [ ] Executor: Implement definition status filter execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.5 SemanticTagFilter `{{ semanticTag = "disorder" }}`

- [ ] AST: Add `EclFilter::SemanticTag` variant
- [ ] Parser: Implement `semantic_tag_filter()` function
- [ ] Parser: Support single tag
- [ ] Parser: Support multiple tags
- [ ] Traits: Add `get_semantic_tag()` to `EclQueryable`
- [ ] Executor: Implement semantic tag filter execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.6 EffectiveTimeFilter `{{ effectiveTime >= 20200101 }}`

- [ ] AST: Add `EclFilter::EffectiveTime` variant
- [ ] Parser: Implement `effective_time_filter()` function
- [ ] Parser: Support all comparison operators
- [ ] Traits: Add `get_concept_effective_time()` to `EclQueryable`
- [ ] Executor: Implement effective time filter execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.7 PreferredInFilter `{{ preferredIn = refsetId }}`

- [ ] AST: Add `EclFilter::PreferredIn` variant
- [ ] Parser: Implement `preferred_in_filter()` function
- [ ] Parser: Support single refset
- [ ] Parser: Support multiple refsets
- [ ] Executor: Implement preferred in filter execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.8 AcceptableInFilter `{{ acceptableIn = refsetId }}`

- [ ] AST: Add `EclFilter::AcceptableIn` variant
- [ ] Parser: Implement `acceptable_in_filter()` function
- [ ] Executor: Implement acceptable in filter execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.9 LanguageRefSetFilter `{{ languageRefSetId = refsetId }}`

- [ ] AST: Add `EclFilter::LanguageRefSet` variant
- [ ] Parser: Implement `language_refset_filter()` function
- [ ] Executor: Implement language refset filter execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.10 CaseSignificanceFilter `{{ caseSignificance = caseInsensitive }}`

- [ ] AST: Add `EclFilter::CaseSignificance` variant
- [ ] Parser: Implement `case_significance_filter()` function
- [ ] Parser: Support keywords and IDs
- [ ] Executor: Implement case significance filter execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.11 IdFilter `{{ id = 123456 }}`

- [ ] AST: Add `EclFilter::Id` variant
- [ ] Parser: Implement `id_filter()` function
- [ ] Parser: Support single ID
- [ ] Parser: Support multiple IDs
- [ ] Executor: Implement ID filter execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.12 EclQueryable Trait Extensions

- [ ] Add `get_descriptions(concept_id)` method
- [ ] Add `DescriptionInfo` struct with all fields
- [ ] Add `get_concept_definition_status(concept_id)` method
- [ ] Add `get_concept_module(concept_id)` method
- [ ] Add `get_concept_effective_time(concept_id)` method
- [ ] Add `get_semantic_tag(concept_id)` method
- [ ] Add `get_inbound_relationships(concept_id)` method
- [ ] Update default implementations
- [ ] Tests: Trait tests with mock store

---

## Phase 2: Syntax Features (Medium Priority)

### 2.1 HistorySupplement Profiles `{{ +HISTORY-MIN }}`

- [ ] AST: Add `HistoryProfile` enum (Min, Mod, Max)
- [ ] AST: Modify `EclFilter::History` to include profile
- [ ] Parser: Update `history_filter()` to parse profiles
- [ ] Executor: Implement profile-aware history supplement
- [ ] Tests: Parser tests for each profile
- [ ] Tests: Executor tests

### 2.2 EclConceptReferenceSet `(123 456 789)`

- [ ] AST: Add `EclExpression::ConceptSet` variant
- [ ] Parser: Implement `concept_reference_set()` function
- [ ] Parser: Handle ambiguity with nested expressions
- [ ] Executor: Implement concept set execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 2.3 Boolean Concrete Values `#true`, `#false`

- [ ] AST: Add `ConcreteValue::Boolean` variant
- [ ] Parser: Update `concrete_value()` to parse booleans
- [ ] Executor: Handle boolean concrete values in refinements
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 2.4 Numeric Comparison in Refinements

- [ ] Parser: Wire up comparison operators in `attribute_constraint()`
- [ ] Executor: Implement numeric comparisons in `evaluate_attribute_constraint()`
- [ ] Executor: Handle `<`, `>`, `<=`, `>=` for concrete values
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 2.5 Wildcard Term Matching `{{ term wild "diab*" }}`

- [ ] AST: Add `TermMatchType::Wildcard` variant
- [ ] Parser: Update `term_filter()` to parse `wild` keyword
- [ ] Executor: Implement wildcard pattern matching
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests with patterns

### 2.6 Short Domain Prefixes `{{ C ... }}`, `{{ D ... }}`

- [ ] AST: Add `FilterDomain` enum (Concept, Description, Member)
- [ ] AST: Add domain field to relevant filters
- [ ] Parser: Update `single_filter()` to parse domain prefix
- [ ] Executor: Apply domain-specific filtering
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 2.7 Enhanced MemberOf `^ (expression)`

- [ ] AST: Change `MemberOf.refset_id` to `MemberOf.refset: Box<EclExpression>`
- [ ] Parser: Update `member_of_expression()` to handle nested
- [ ] Executor: Update member-of execution for nested expressions
- [ ] Tests: Parser tests with nested expressions
- [ ] Tests: Executor tests

---

## Phase 3: Advanced Features (Low Priority)

### 3.1 AlternateIdentifier `http://snomed.info#123`

- [ ] AST: Add `EclExpression::AlternateIdentifier` variant
- [ ] Parser: Implement `alternate_identifier()` function
- [ ] Traits: Add `resolve_alternate_identifier()` to `EclQueryable`
- [ ] Executor: Implement alternate ID resolution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 3.2 Reverse Flag Execution

- [ ] Traits: Add `get_inbound_relationships()` to `EclQueryable`
- [ ] Executor: Update `evaluate_attribute_constraint()` for reverse
- [ ] Tests: Executor tests with reverse attributes

### 3.3 Comprehensive Test Suite

- [ ] Create test file: `tests/filter_tests.rs`
- [ ] Create test file: `tests/syntax_tests.rs`
- [ ] Create test file: `tests/integration_tests.rs`
- [ ] Add ECL 2.2 compliance test vectors
- [ ] Add b2ihealthcare test case equivalents
- [ ] Add IHTSDO test case equivalents

### 3.4 Documentation

- [ ] Update crate-level documentation
- [ ] Add examples for all new features
- [ ] Update README with feature matrix
- [ ] Add ECL version support note

---

## Summary

| Phase | Total Tasks | Completed | Remaining |
|-------|-------------|-----------|-----------|
| Phase 1 (Filters) | ~60 | 0 | ~60 |
| Phase 2 (Syntax) | ~30 | 0 | ~30 |
| Phase 3 (Advanced) | ~15 | 0 | ~15 |
| **Total** | **~105** | **0** | **~105** |

---

## Implementation Notes

### File Modification Summary

**`crates/snomed-ecl/src/ast.rs`:**
- Add new `EclFilter` variants
- Add new `EclExpression` variants
- Add supporting enums (`HistoryProfile`, `Acceptability`, etc.)
- Add `ConcreteValue::Boolean`
- Add `TermMatchType::Wildcard`
- Modify `MemberOf` structure

**`crates/snomed-ecl/src/parser.rs`:**
- Add filter parser functions
- Update `single_filter()` with new filters
- Add domain prefix parsing
- Update `member_of_expression()`
- Add `concept_reference_set()`
- Add `alternate_identifier()`

**`crates/snomed-ecl-executor/src/traits.rs`:**
- Extend `EclQueryable` trait
- Add `DescriptionInfo` struct
- Add `Acceptability` enum
- Add default implementations

**`crates/snomed-ecl-executor/src/executor.rs`:**
- Implement `apply_filter()` fully
- Add filter-specific execution methods
- Update `evaluate_attribute_constraint()` for reverse
- Handle new expression types

### Dependencies (if needed)

Consider adding:
- `regex` crate for wildcard/regex term matching (optional)
- No other new dependencies expected

### Breaking Changes

The following may be breaking changes:
1. `EclFilter::History` changing to include profile (if existing code destructures)
2. `MemberOf` changing from `refset_id: SctId` to `refset: Box<EclExpression>`
3. `EclQueryable` trait getting new methods (implementors must update)

Mitigation:
- Add default implementations for new trait methods
- Use `#[non_exhaustive]` on enums if not already
- Document migration path in CHANGELOG
