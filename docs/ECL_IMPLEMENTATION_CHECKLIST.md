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

- [x] AST: Add `EclFilter::Language` variant
- [x] Parser: Implement `language_filter()` function
- [x] Parser: Support single language code
- [x] Parser: Support multiple codes `(en es fr)`
- [x] Traits: Add `get_descriptions()` to `EclQueryable`
- [x] Traits: Add `DescriptionInfo` struct
- [x] Executor: Implement language filter execution
- [x] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.2 TypeFilter `{{ typeId = 900000000000003001 }}`

- [x] AST: Add `EclFilter::DescriptionType` variant
- [x] Parser: Implement `type_filter()` function
- [x] Parser: Support type IDs
- [x] Parser: Support type aliases (`fsn`, `syn`, `def`)
- [x] Parser: Support multiple types
- [x] Executor: Implement type filter execution
- [x] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.3 DialectFilter `{{ dialect = en-US }}`

- [x] AST: Add `EclFilter::Dialect` variant
- [x] AST: Add `FilterAcceptability` enum
- [x] Parser: Implement `dialect_filter()` function
- [x] Parser: Support dialect IDs
- [x] Parser: Support dialect aliases
- [x] Parser: Support multiple dialects
- [x] Parser: Support acceptability constraint
- [x] Executor: Implement dialect filter execution
- [x] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.4 DefinitionStatusFilter `{{ definitionStatus = primitive }}`

- [x] AST: Add `EclFilter::DefinitionStatus` variant
- [x] Parser: Implement `definition_status_filter()` function
- [x] Parser: Support status keywords (`primitive`, `defined`)
- [x] Parser: Support status IDs
- [x] Traits: Add `is_concept_primitive()` to `EclQueryable`
- [x] Executor: Implement definition status filter execution
- [x] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.5 SemanticTagFilter `{{ semanticTag = "disorder" }}`

- [x] AST: Add `EclFilter::SemanticTag` variant
- [x] Parser: Implement `semantic_tag_filter()` function
- [x] Parser: Support single tag
- [x] Parser: Support multiple tags
- [x] Traits: Add `get_semantic_tag()` to `EclQueryable`
- [x] Executor: Implement semantic tag filter execution
- [x] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.6 EffectiveTimeFilter `{{ effectiveTime >= 20200101 }}`

- [x] AST: Add `EclFilter::EffectiveTime` variant
- [x] Parser: Implement `effective_time_filter()` function
- [x] Parser: Support all comparison operators
- [x] Traits: Add `get_concept_effective_time()` to `EclQueryable`
- [x] Executor: Implement effective time filter execution
- [x] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.7 PreferredInFilter `{{ preferredIn = refsetId }}`

- [x] AST: Add `EclFilter::PreferredIn` variant
- [x] Parser: Implement `preferred_in_filter()` function
- [x] Parser: Support single refset
- [x] Parser: Support multiple refsets
- [x] Executor: Implement preferred in filter execution
- [x] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.8 AcceptableInFilter `{{ acceptableIn = refsetId }}`

- [x] AST: Add `EclFilter::AcceptableIn` variant
- [x] Parser: Implement `acceptable_in_filter()` function
- [x] Executor: Implement acceptable in filter execution
- [x] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.9 LanguageRefSetFilter `{{ languageRefSetId = refsetId }}`

- [x] AST: Add `EclFilter::LanguageRefSet` variant
- [x] Parser: Implement `language_refset_filter()` function
- [x] Executor: Implement language refset filter execution
- [x] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.10 CaseSignificanceFilter `{{ caseSignificance = caseInsensitive }}`

- [x] AST: Add `EclFilter::CaseSignificance` variant
- [x] Parser: Implement `case_significance_filter()` function
- [x] Parser: Support keywords and IDs
- [x] Executor: Implement case significance filter execution
- [x] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.11 IdFilter `{{ id = 123456 }}`

- [x] AST: Add `EclFilter::Id` variant
- [x] Parser: Implement `id_filter()` function
- [x] Parser: Support single ID
- [x] Parser: Support multiple IDs
- [x] Executor: Implement ID filter execution
- [x] Tests: Parser tests
- [ ] Tests: Executor tests

### 1.12 EclQueryable Trait Extensions

- [x] Add `get_descriptions(concept_id)` method
- [x] Add `DescriptionInfo` struct with all fields
- [x] Add `get_concept_info(concept_id)` method (includes definition status)
- [x] Add `get_concept_module(concept_id)` method
- [x] Add `get_concept_effective_time(concept_id)` method
- [x] Add `get_semantic_tag(concept_id)` method
- [x] Add `get_inbound_relationships(concept_id)` method
- [x] Add `get_description_language_refsets(description_id)` method
- [x] Add `LanguageRefsetMember` struct
- [x] Add `ConceptInfo` struct
- [x] Add `Acceptability` enum
- [x] Add `HistoryAssociationType` enum
- [x] Add `get_historical_associations_by_type()` method
- [x] Update default implementations
- [x] Tests: Trait tests with mock store

---

## Phase 2: Syntax Features (Medium Priority)

### 2.1 HistorySupplement Profiles `{{ +HISTORY-MIN }}`

- [x] AST: Add `HistoryProfile` enum (Min, Mod, Max)
- [x] AST: Modify `EclFilter::History` to include profile
- [x] Parser: Update `history_filter()` to parse profiles
- [x] Executor: Implement profile-aware history supplement
- [x] Tests: Parser tests for each profile
- [ ] Tests: Executor tests

### 2.2 EclConceptReferenceSet `(123 456 789)`

- [ ] AST: Add `EclExpression::ConceptSet` variant
- [ ] Parser: Implement `concept_reference_set()` function
- [ ] Parser: Handle ambiguity with nested expressions
- [ ] Executor: Implement concept set execution
- [ ] Tests: Parser tests
- [ ] Tests: Executor tests

### 2.3 Boolean Concrete Values `#true`, `#false`

- [x] AST: Add `ConcreteValue::Boolean` variant
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

- [x] AST: Add `TermMatchType::Wildcard` variant
- [x] Parser: Update `term_filter()` to parse `wild` keyword
- [x] Executor: Implement wildcard pattern matching (basic)
- [x] Tests: Parser tests
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

- [x] Traits: Add `get_inbound_relationships()` to `EclQueryable`
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
| Phase 1 (Filters) | ~60 | ~55 | ~5 |
| Phase 2 (Syntax) | ~30 | ~10 | ~20 |
| Phase 3 (Advanced) | ~15 | ~1 | ~14 |
| **Total** | **~105** | **~66** | **~39** |

---

## Implementation Notes

### Completed Work

**`crates/snomed-ecl/src/ast.rs`:**
- Added all 16 `EclFilter` variants (Term, Language, DescriptionType, Dialect, CaseSignificance, Active, Module, EffectiveTime, DefinitionStatus, SemanticTag, PreferredIn, AcceptableIn, LanguageRefSet, Member, Id, History)
- Added `HistoryProfile` enum (Min, Mod, Max)
- Added `FilterAcceptability` enum (Preferred, Acceptable)
- Added `MemberFieldValue` enum for member filter values
- Added `ConcreteValue::Boolean` variant
- Added `TermMatchType::Wildcard` variant

**`crates/snomed-ecl/src/parser.rs`:**
- Implemented all filter parser functions
- Added `language_filter()`, `description_type_filter()`, `dialect_filter()`, `definition_status_filter()`, `semantic_tag_filter()`, `effective_time_filter()`, `preferred_in_filter()`, `acceptable_in_filter()`, `language_refset_filter()`, `case_significance_filter()`, `id_filter()`
- Updated `history_filter()` to parse profiles
- Updated `term_filter()` for wildcard matching
- Updated `single_filter()` to include all new filters
- Added 25+ new parser tests

**`crates/snomed-ecl-executor/src/traits.rs`:**
- Extended `EclQueryable` trait with filter support methods
- Added `DescriptionInfo` struct with all fields
- Added `ConceptInfo` struct for concept metadata
- Added `LanguageRefsetMember` struct
- Added `Acceptability` enum
- Added `HistoryAssociationType` enum
- Added default implementations for all new methods

**`crates/snomed-ecl-executor/src/executor.rs`:**
- Implemented complete `apply_filter()` method
- Added filter execution for all 16 filter types
- Added profile-aware history supplement execution

### Dependencies

No new dependencies were required.

### Breaking Changes

The following are breaking changes:
1. `EclFilter` enum has been completely restructured with new variants
2. `EclFilter::Term` no longer has `language` and `type_id` fields (moved to separate filters)
3. `EclFilter::History` is now a struct variant with `profile: Option<HistoryProfile>`
4. `EclFilter::Module` is now a struct variant with `module_ids: Vec<SctId>`
5. `EclFilter::Member` value is now `MemberFieldValue` instead of `String`
6. `EclQueryable` trait has many new methods (all have default implementations)
7. Removed `Eq` derive from types containing `EclExpression` or `EclFilter`

Mitigation:
- All new trait methods have default implementations
- Use pattern matching with `..` to ignore new fields
