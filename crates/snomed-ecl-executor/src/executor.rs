//! ECL executor implementation.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use snomed_ecl::EclExpression;
use snomed_ecl::SctId;

use crate::cache::{normalize_cache_key, QueryCache};
use crate::config::ExecutorConfig;
use crate::error::{EclExecutorError, EclResult};
use crate::planner::{QueryPlan, QueryPlanner};
use crate::result::{ExecutionStats, QueryResult};
use crate::traits::EclQueryable;
use crate::traverser::HierarchyTraverser;

/// Main ECL execution engine.
///
/// The executor bridges the ECL parser (`snomed-ecl`) and any SNOMED CT store
/// that implements [`EclQueryable`] to execute ECL queries.
///
/// # Example
///
/// ```ignore
/// use snomed_ecl_executor::{EclExecutor, EclQueryable};
///
/// // Assumes MyStore implements EclQueryable
/// let store = MyStore::new();
/// let executor = EclExecutor::new(&store);
///
/// // Execute a descendant query
/// let result = executor.execute("<< 73211009")?;
/// println!("Found {} diabetes-related concepts", result.count());
///
/// // Check if a concept matches
/// let is_diabetes = executor.matches(46635009, "<< 73211009")?;
/// ```
pub struct EclExecutor<'a> {
    /// Reference to the queryable store.
    store: &'a dyn EclQueryable,
    /// Executor configuration.
    config: ExecutorConfig,
    /// Query result cache (optional).
    cache: Option<Arc<QueryCache>>,
}

impl<'a> EclExecutor<'a> {
    /// Creates a new executor with default configuration.
    ///
    /// # Arguments
    ///
    /// * `store` - A reference to a SNOMED store implementing `EclQueryable`
    ///
    /// # Example
    ///
    /// ```ignore
    /// let store = SnomedStore::new();
    /// let executor = EclExecutor::new(&store);
    /// ```
    pub fn new(store: &'a dyn EclQueryable) -> Self {
        Self {
            store,
            config: ExecutorConfig::default(),
            cache: None,
        }
    }

    /// Creates an executor with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `store` - A reference to a SNOMED store implementing `EclQueryable`
    /// * `config` - Executor configuration
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = ExecutorConfig::builder()
    ///     .with_cache(CacheConfig::default())
    ///     .with_parallel(true)
    ///     .build();
    ///
    /// let executor = EclExecutor::with_config(&store, config);
    /// ```
    pub fn with_config(store: &'a dyn EclQueryable, config: ExecutorConfig) -> Self {
        let cache = config
            .cache
            .as_ref()
            .map(|c| Arc::new(QueryCache::new(c.clone())));
        Self {
            store,
            config,
            cache,
        }
    }

    /// Returns a reference to the cache if enabled.
    pub fn cache(&self) -> Option<&QueryCache> {
        self.cache.as_ref().map(|c| c.as_ref())
    }

    /// Returns a reference to the executor configuration.
    pub fn config(&self) -> &ExecutorConfig {
        &self.config
    }

    /// Executes an ECL expression string.
    ///
    /// Parses the ECL string and executes it against the store.
    /// If caching is enabled, results are cached for subsequent calls.
    ///
    /// # Arguments
    ///
    /// * `ecl` - ECL expression string (e.g., `"<< 73211009"`)
    ///
    /// # Returns
    ///
    /// * `Ok(QueryResult)` - The matching concept IDs and execution stats
    /// * `Err(EclExecutorError)` - If parsing or execution fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = executor.execute("<< 73211009 |Diabetes mellitus|")?;
    /// println!("Found {} concepts", result.count());
    /// ```
    pub fn execute(&self, ecl: &str) -> EclResult<QueryResult> {
        let start = Instant::now();

        // Normalize the ECL for cache key
        let cache_key = normalize_cache_key(ecl);

        // Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(cached_result) = cache.get(&cache_key) {
                let stats = ExecutionStats::new(start.elapsed(), 0, true);
                return Ok(QueryResult::new(cached_result, stats));
            }
        }

        // Parse the ECL expression
        let expr = snomed_ecl::parse(ecl)?;

        // Execute the parsed AST (without caching, since we handle it here)
        let traverser = HierarchyTraverser::new(self.store);
        let (concept_ids, concepts_traversed) = self.execute_expression(&expr, &traverser)?;

        // Store in cache if enabled
        if let Some(ref cache) = self.cache {
            cache.set(cache_key, concept_ids.clone());
        }

        let stats = ExecutionStats::new(start.elapsed(), concepts_traversed, false);
        Ok(QueryResult::new(concept_ids, stats))
    }

    /// Executes a pre-parsed ECL expression.
    ///
    /// Use this when you have already parsed the ECL expression and want to
    /// execute it multiple times or against different stores.
    ///
    /// Note: This method uses the expression's string representation as the
    /// cache key. For optimal caching, prefer using `execute()` with the
    /// original ECL string.
    ///
    /// # Arguments
    ///
    /// * `expr` - Pre-parsed ECL expression AST
    ///
    /// # Returns
    ///
    /// * `Ok(QueryResult)` - The matching concept IDs and execution stats
    /// * `Err(EclExecutorError)` - If execution fails
    pub fn execute_ast(&self, expr: &EclExpression) -> EclResult<QueryResult> {
        let start = Instant::now();

        // Use expression's display representation as cache key
        let cache_key = normalize_cache_key(&expr.to_string());

        // Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(cached_result) = cache.get(&cache_key) {
                let stats = ExecutionStats::new(start.elapsed(), 0, true);
                return Ok(QueryResult::new(cached_result, stats));
            }
        }

        let traverser = HierarchyTraverser::new(self.store);
        let (concept_ids, concepts_traversed) = self.execute_expression(expr, &traverser)?;

        // Store in cache if enabled
        if let Some(ref cache) = self.cache {
            cache.set(cache_key, concept_ids.clone());
        }

        let stats = ExecutionStats::new(start.elapsed(), concepts_traversed, false);
        Ok(QueryResult::new(concept_ids, stats))
    }

    /// Internal method to execute an ECL expression recursively.
    ///
    /// This method handles caching of intermediate results when cache_intermediates
    /// is enabled in the configuration.
    fn execute_expression(
        &self,
        expr: &EclExpression,
        traverser: &HierarchyTraverser<'_>,
    ) -> EclResult<(HashSet<SctId>, usize)> {
        // Handle nested expressions by unwrapping
        let expr = expr.unwrap_nested();

        match expr {
            // Self constraint: single concept reference
            EclExpression::ConceptReference { concept_id, .. } => {
                // Verify concept exists
                if !self.store.has_concept(*concept_id) {
                    return Err(EclExecutorError::ConceptNotFound(*concept_id));
                }
                let mut result = HashSet::with_capacity(1);
                result.insert(*concept_id);
                Ok((result, 1))
            }

            // Descendant of: < concept
            EclExpression::DescendantOf(inner) => {
                let concept_id = self.get_focus_concept(inner)?;
                let descendants = traverser.get_descendants(concept_id);
                let count = descendants.len();
                Ok((descendants, count))
            }

            // Descendant or self of: << concept
            EclExpression::DescendantOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept(inner)?;
                let result = traverser.get_descendants_or_self(concept_id);
                let count = result.len();
                Ok((result, count))
            }

            // Ancestor of: > concept
            EclExpression::AncestorOf(inner) => {
                let concept_id = self.get_focus_concept(inner)?;
                let ancestors = traverser.get_ancestors(concept_id);
                let count = ancestors.len();
                Ok((ancestors, count))
            }

            // Ancestor or self of: >> concept
            EclExpression::AncestorOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept(inner)?;
                let result = traverser.get_ancestors_or_self(concept_id);
                let count = result.len();
                Ok((result, count))
            }

            // Child of: <! concept (direct children only)
            EclExpression::ChildOf(inner) => {
                let concept_id = self.get_focus_concept(inner)?;
                let children = traverser.get_direct_children(concept_id);
                let count = children.len();
                Ok((children, count))
            }

            // Child or self of: <<! concept
            EclExpression::ChildOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept(inner)?;
                let mut result = traverser.get_direct_children(concept_id);
                result.insert(concept_id);
                let count = result.len();
                Ok((result, count))
            }

            // Parent of: >! concept (direct parents only)
            EclExpression::ParentOf(inner) => {
                let concept_id = self.get_focus_concept(inner)?;
                let parents = traverser.get_direct_parents(concept_id);
                let count = parents.len();
                Ok((parents, count))
            }

            // Parent or self of: >>! concept
            EclExpression::ParentOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept(inner)?;
                let mut result = traverser.get_direct_parents(concept_id);
                result.insert(concept_id);
                let count = result.len();
                Ok((result, count))
            }

            // AND: intersection of two expressions
            EclExpression::And(left, right) => {
                let (left_result, left_count) =
                    self.execute_with_intermediate_cache(left, traverser)?;
                let (right_result, right_count) =
                    self.execute_with_intermediate_cache(right, traverser)?;
                let result: HashSet<SctId> =
                    left_result.intersection(&right_result).copied().collect();
                Ok((result, left_count + right_count))
            }

            // OR: union of two expressions
            EclExpression::Or(left, right) => {
                let (left_result, left_count) =
                    self.execute_with_intermediate_cache(left, traverser)?;
                let (right_result, right_count) =
                    self.execute_with_intermediate_cache(right, traverser)?;
                let result: HashSet<SctId> = left_result.union(&right_result).copied().collect();
                Ok((result, left_count + right_count))
            }

            // MINUS: difference of two expressions
            EclExpression::Minus(left, right) => {
                let (left_result, left_count) =
                    self.execute_with_intermediate_cache(left, traverser)?;
                let (right_result, right_count) =
                    self.execute_with_intermediate_cache(right, traverser)?;
                let result: HashSet<SctId> =
                    left_result.difference(&right_result).copied().collect();
                Ok((result, left_count + right_count))
            }

            // Member of: ^ refset_id or ^ (expression)
            EclExpression::MemberOf { refset } => {
                // First, evaluate the refset expression to get the set of refset IDs
                let (refset_ids, refset_count) = self.execute_expression(refset, traverser)?;

                // Get members from all matching refsets
                let mut members: HashSet<SctId> = HashSet::new();
                let mut found_any = false;
                for refset_id in &refset_ids {
                    let refset_members = self.store.get_refset_members(*refset_id);
                    if !refset_members.is_empty() {
                        found_any = true;
                        members.extend(refset_members);
                    }
                }

                if !found_any && refset_ids.len() == 1 {
                    // If we had a single refset and found no members, report error
                    if let Some(&single_id) = refset_ids.iter().next() {
                        return Err(EclExecutorError::RefsetNotFound(single_id));
                    }
                }

                let count = members.len() + refset_count;
                Ok((members, count))
            }

            // Any: * (all concepts)
            EclExpression::Any => {
                let all: HashSet<SctId> = self.store.all_concept_ids().collect();
                let count = all.len();
                Ok((all, count))
            }

            // Alternate identifier: http://snomed.info/id/73211009
            EclExpression::AlternateIdentifier { scheme, identifier } => {
                if let Some(concept_id) = self.store.resolve_alternate_identifier(scheme, identifier) {
                    let mut result = HashSet::new();
                    result.insert(concept_id);
                    Ok((result, 1))
                } else {
                    // Identifier could not be resolved - return empty set
                    Ok((HashSet::new(), 0))
                }
            }

            // Nested expressions are unwrapped at the start
            EclExpression::Nested(_) => {
                unreachable!("Nested expressions are unwrapped at the start")
            }

            // =========================================================================
            // Advanced ECL Features (Story 10.9)
            // =========================================================================

            // Refined expression: focus : refinement
            EclExpression::Refined { focus, refinement } => {
                // Execute the focus expression first
                let (focus_concepts, focus_count) =
                    self.execute_with_intermediate_cache(focus, traverser)?;

                // Filter concepts based on refinement
                let mut result = HashSet::new();
                let mut total_count = focus_count;

                for concept_id in focus_concepts {
                    // Get both outbound and inbound relationships (lazily, only if needed)
                    let outbound_attrs = self.store.get_attributes(concept_id);
                    let mut inbound_attrs: Option<Vec<_>> = None;
                    let mut matches = true;

                    // Check ungrouped attribute constraints
                    for constraint in &refinement.ungrouped {
                        // Get appropriate relationships based on reverse flag
                        let attrs: &[crate::traits::RelationshipInfo] = if constraint.reverse {
                            // Lazy initialization of inbound relationships
                            if inbound_attrs.is_none() {
                                inbound_attrs = Some(self.store.get_inbound_relationships(concept_id));
                            }
                            inbound_attrs.as_ref().unwrap()
                        } else {
                            &outbound_attrs
                        };

                        if !self.evaluate_attribute_constraint(
                            concept_id,
                            constraint,
                            attrs,
                            traverser,
                        )? {
                            matches = false;
                            break;
                        }
                    }

                    // Check grouped attribute constraints
                    if matches {
                        for group in &refinement.groups {
                            // For groups, check if any constraint has reverse flag
                            let has_reverse = group.constraints.iter().any(|c| c.reverse);
                            let attrs: &[crate::traits::RelationshipInfo] = if has_reverse {
                                if inbound_attrs.is_none() {
                                    inbound_attrs = Some(self.store.get_inbound_relationships(concept_id));
                                }
                                inbound_attrs.as_ref().unwrap()
                            } else {
                                &outbound_attrs
                            };

                            if !self.evaluate_attribute_group(
                                concept_id,
                                group,
                                attrs,
                                traverser,
                            )? {
                                matches = false;
                                break;
                            }
                        }
                    }

                    if matches {
                        result.insert(concept_id);
                    }
                    total_count += outbound_attrs.len();
                }

                Ok((result, total_count))
            }

            // Dot notation: expression.attributeType
            EclExpression::DotNotation {
                source,
                attribute_type,
            } => {
                // Execute the source expression
                let (source_concepts, source_count) =
                    self.execute_with_intermediate_cache(source, traverser)?;

                // Get the attribute type ID(s)
                let (attr_type_concepts, attr_count) =
                    self.execute_with_intermediate_cache(attribute_type, traverser)?;

                let mut result = HashSet::new();
                let mut total_count = source_count + attr_count;

                // For each source concept, get attribute values
                for concept_id in source_concepts {
                    let attributes = self.store.get_attributes(concept_id);
                    total_count += attributes.len();

                    for rel in &attributes {
                        // Check if this relationship's type matches any of our target types
                        if attr_type_concepts.contains(&rel.type_id) {
                            result.insert(rel.destination_id);
                        }
                    }
                }

                Ok((result, total_count))
            }

            // Concrete value: #value
            EclExpression::Concrete { .. } => {
                // Concrete values are typically used within refinements
                // When used standalone, they represent no concepts
                Ok((HashSet::new(), 0))
            }

            // Filtered expression: expression {{ filter }}
            EclExpression::Filtered { expression, filters } => {
                // Execute the base expression
                let (mut concepts, mut count) =
                    self.execute_with_intermediate_cache(expression, traverser)?;

                // Apply each filter
                for filter in filters {
                    let filtered = self.apply_filter(&concepts, filter)?;
                    count += concepts.len(); // Count filter evaluations
                    concepts = filtered;
                }

                Ok((concepts, count))
            }

            // Top of set: !!> expression (most general concepts)
            EclExpression::TopOfSet(inner) => {
                let (concepts, count) = self.execute_with_intermediate_cache(inner, traverser)?;

                // Find concepts that have no ancestors within the set
                let mut result = HashSet::new();
                for &concept_id in &concepts {
                    let ancestors = traverser.get_ancestors(concept_id);
                    let has_ancestor_in_set = ancestors.iter().any(|a| concepts.contains(a));
                    if !has_ancestor_in_set {
                        result.insert(concept_id);
                    }
                }

                Ok((result, count + concepts.len()))
            }

            // Bottom of set: !!< expression (most specific concepts)
            EclExpression::BottomOfSet(inner) => {
                let (concepts, count) = self.execute_with_intermediate_cache(inner, traverser)?;

                // Find concepts that have no descendants within the set
                let mut result = HashSet::new();
                for &concept_id in &concepts {
                    let descendants = traverser.get_descendants(concept_id);
                    let has_descendant_in_set = descendants.iter().any(|d| concepts.contains(d));
                    if !has_descendant_in_set {
                        result.insert(concept_id);
                    }
                }

                Ok((result, count + concepts.len()))
            }

            EclExpression::ConceptSet(ids) => {
                // Return all valid concept IDs from the set
                let result: HashSet<SctId> = ids
                    .iter()
                    .copied()
                    .filter(|&id| self.store.has_concept(id))
                    .collect();
                Ok((result, ids.len()))
            }
        }
    }

    /// Evaluates a single attribute constraint against a concept's attributes.
    fn evaluate_attribute_constraint(
        &self,
        concept_id: SctId,
        constraint: &snomed_ecl::AttributeConstraint,
        attributes: &[crate::traits::RelationshipInfo],
        traverser: &HierarchyTraverser<'_>,
    ) -> EclResult<bool> {
        use snomed_ecl::RefinementOperator;

        // Get the set of acceptable attribute types
        let attr_types = self.execute_expression(&constraint.attribute_type, traverser)?;

        // Check if this is a concrete value constraint
        if let snomed_ecl::EclExpression::Concrete { value, operator } = constraint.value.as_ref() {
            return self.evaluate_concrete_constraint(
                concept_id,
                constraint,
                &attr_types.0,
                value,
                *operator,
            );
        }

        // Get the set of acceptable values for concept-to-concept relationships
        let acceptable_values = self.execute_expression(&constraint.value, traverser)?;

        // Find matching attributes
        let matching_count = attributes
            .iter()
            .filter(|rel| {
                // Check if attribute type matches (or wildcard)
                let type_matches = matches!(
                    constraint.attribute_type.as_ref(),
                    snomed_ecl::EclExpression::Any
                ) || attr_types.0.contains(&rel.type_id);

                if !type_matches {
                    return false;
                }

                // Check if value matches based on operator
                match constraint.operator {
                    RefinementOperator::Equal | RefinementOperator::DescendantOrSelfOf => {
                        acceptable_values.0.contains(&rel.destination_id)
                    }
                    RefinementOperator::NotEqual => {
                        !acceptable_values.0.contains(&rel.destination_id)
                    }
                    RefinementOperator::DescendantOf => {
                        // Value must be a proper descendant
                        let value_ancestors = traverser.get_ancestors(rel.destination_id);
                        acceptable_values
                            .0
                            .iter()
                            .any(|v| value_ancestors.contains(v))
                    }
                    RefinementOperator::AncestorOf | RefinementOperator::AncestorOrSelfOf => {
                        // Value must be an ancestor
                        acceptable_values.0.iter().any(|v| {
                            let v_ancestors = traverser.get_ancestors(*v);
                            v_ancestors.contains(&rel.destination_id)
                                || (matches!(
                                    constraint.operator,
                                    RefinementOperator::AncestorOrSelfOf
                                ) && *v == rel.destination_id)
                        })
                    }
                }
            })
            .count();

        // Check cardinality
        if let Some(ref card) = constraint.cardinality {
            Ok(card.matches(matching_count))
        } else {
            // No cardinality means at least one match required
            Ok(matching_count > 0)
        }
    }

    /// Evaluates a concrete value constraint against a concept's concrete relationships.
    fn evaluate_concrete_constraint(
        &self,
        concept_id: SctId,
        constraint: &snomed_ecl::AttributeConstraint,
        attr_types: &HashSet<SctId>,
        target_value: &snomed_ecl::ConcreteValue,
        operator: snomed_ecl::ComparisonOperator,
    ) -> EclResult<bool> {
        // Get concrete relationships for this concept
        let concrete_rels = self.store.get_concrete_values(concept_id);

        // Find matching concrete relationships
        let matching_count = concrete_rels
            .iter()
            .filter(|rel| {
                // Check if attribute type matches (or wildcard)
                let type_matches = matches!(
                    constraint.attribute_type.as_ref(),
                    snomed_ecl::EclExpression::Any
                ) || attr_types.contains(&rel.type_id);

                if !type_matches {
                    return false;
                }

                // Compare the concrete value
                Self::compare_concrete_values(&rel.value, target_value, operator)
            })
            .count();

        // Check cardinality
        if let Some(ref card) = constraint.cardinality {
            Ok(card.matches(matching_count))
        } else {
            // No cardinality means at least one match required
            Ok(matching_count > 0)
        }
    }

    /// Compares a concrete relationship value against a target value using the given operator.
    fn compare_concrete_values(
        actual: &crate::traits::ConcreteValueRef,
        target: &snomed_ecl::ConcreteValue,
        operator: snomed_ecl::ComparisonOperator,
    ) -> bool {
        use snomed_ecl::ComparisonOperator;
        use snomed_ecl::ConcreteValue;
        use crate::traits::ConcreteValueRef;

        match (actual, target) {
            // Integer comparisons
            (ConcreteValueRef::Integer(a), ConcreteValue::Integer(t)) => {
                match operator {
                    ComparisonOperator::Equal => *a == *t,
                    ComparisonOperator::NotEqual => *a != *t,
                    ComparisonOperator::LessThan => *a < *t,
                    ComparisonOperator::LessThanOrEqual => *a <= *t,
                    ComparisonOperator::GreaterThan => *a > *t,
                    ComparisonOperator::GreaterThanOrEqual => *a >= *t,
                }
            }
            // Decimal comparisons
            (ConcreteValueRef::Decimal(a), ConcreteValue::Decimal(t)) => {
                match operator {
                    ComparisonOperator::Equal => (*a - *t).abs() < f64::EPSILON,
                    ComparisonOperator::NotEqual => (*a - *t).abs() >= f64::EPSILON,
                    ComparisonOperator::LessThan => *a < *t,
                    ComparisonOperator::LessThanOrEqual => *a <= *t,
                    ComparisonOperator::GreaterThan => *a > *t,
                    ComparisonOperator::GreaterThanOrEqual => *a >= *t,
                }
            }
            // Integer vs Decimal (promote integer to decimal)
            (ConcreteValueRef::Integer(a), ConcreteValue::Decimal(t)) => {
                let a_f = *a as f64;
                match operator {
                    ComparisonOperator::Equal => (a_f - *t).abs() < f64::EPSILON,
                    ComparisonOperator::NotEqual => (a_f - *t).abs() >= f64::EPSILON,
                    ComparisonOperator::LessThan => a_f < *t,
                    ComparisonOperator::LessThanOrEqual => a_f <= *t,
                    ComparisonOperator::GreaterThan => a_f > *t,
                    ComparisonOperator::GreaterThanOrEqual => a_f >= *t,
                }
            }
            (ConcreteValueRef::Decimal(a), ConcreteValue::Integer(t)) => {
                let t_f = *t as f64;
                match operator {
                    ComparisonOperator::Equal => (*a - t_f).abs() < f64::EPSILON,
                    ComparisonOperator::NotEqual => (*a - t_f).abs() >= f64::EPSILON,
                    ComparisonOperator::LessThan => *a < t_f,
                    ComparisonOperator::LessThanOrEqual => *a <= t_f,
                    ComparisonOperator::GreaterThan => *a > t_f,
                    ComparisonOperator::GreaterThanOrEqual => *a >= t_f,
                }
            }
            // String comparisons (only = and != make sense)
            (ConcreteValueRef::String(a), ConcreteValue::String(t)) => {
                match operator {
                    ComparisonOperator::Equal => a == t,
                    ComparisonOperator::NotEqual => a != t,
                    // Lexicographic comparison for strings
                    ComparisonOperator::LessThan => a < t,
                    ComparisonOperator::LessThanOrEqual => a <= t,
                    ComparisonOperator::GreaterThan => a > t,
                    ComparisonOperator::GreaterThanOrEqual => a >= t,
                }
            }
            // Boolean comparisons (only = and != make sense)
            (ConcreteValueRef::Integer(a), ConcreteValue::Boolean(t)) => {
                // Treat 0 as false, non-zero as true
                let a_bool = *a != 0;
                match operator {
                    ComparisonOperator::Equal => a_bool == *t,
                    ComparisonOperator::NotEqual => a_bool != *t,
                    _ => false, // Other comparisons don't make sense for booleans
                }
            }
            // Type mismatches
            _ => false,
        }
    }

    /// Evaluates an attribute group against a concept's attributes.
    fn evaluate_attribute_group(
        &self,
        concept_id: SctId,
        group: &snomed_ecl::AttributeGroup,
        attributes: &[crate::traits::RelationshipInfo],
        traverser: &HierarchyTraverser<'_>,
    ) -> EclResult<bool> {
        // Group constraints must be satisfied within the same relationship group
        // Get unique group numbers (excluding 0 which is ungrouped)
        let group_numbers: HashSet<u16> = attributes
            .iter()
            .filter(|r| r.group > 0)
            .map(|r| r.group)
            .collect();

        if group_numbers.is_empty() && !group.constraints.is_empty() {
            // No groups and we have constraints - check if cardinality allows zero
            if let Some(ref card) = group.cardinality {
                return Ok(card.matches(0));
            }
            return Ok(false);
        }

        let mut matching_groups = 0;

        for group_num in group_numbers {
            // Get attributes in this group
            let group_attrs: Vec<_> = attributes
                .iter()
                .filter(|r| r.group == group_num)
                .cloned()
                .collect();

            // Check if all constraints are satisfied within this group
            let mut all_constraints_met = true;
            for constraint in &group.constraints {
                if !self.evaluate_attribute_constraint(
                    concept_id,
                    constraint,
                    &group_attrs,
                    traverser,
                )? {
                    all_constraints_met = false;
                    break;
                }
            }

            if all_constraints_met {
                matching_groups += 1;
            }
        }

        // Check group cardinality
        if let Some(ref card) = group.cardinality {
            Ok(card.matches(matching_groups))
        } else {
            // No cardinality means at least one matching group required
            Ok(matching_groups > 0)
        }
    }

    /// Applies a filter to a set of concepts.
    fn apply_filter(
        &self,
        concepts: &HashSet<SctId>,
        filter: &snomed_ecl::EclFilter,
    ) -> EclResult<HashSet<SctId>> {
        use crate::traits::{Acceptability, HistoryAssociationType};
        use snomed_ecl::{EclFilter, HistoryProfile, TermMatchType};

        match filter {
            EclFilter::Term { match_type, value } => {
                let mut result = HashSet::new();
                let search_term = value.to_lowercase();

                for &concept_id in concepts {
                    let descriptions = self.store.get_descriptions(concept_id);

                    let matches = descriptions.iter().any(|desc| {
                        let term_lower = desc.term.to_lowercase();
                        match match_type {
                            TermMatchType::Contains => term_lower.contains(&search_term),
                            TermMatchType::StartsWith => term_lower.starts_with(&search_term),
                            TermMatchType::Exact => term_lower == search_term,
                            TermMatchType::Regex => {
                                // Basic regex support - could use regex crate for full support
                                term_lower.contains(&search_term)
                            }
                            TermMatchType::Wildcard => {
                                // Convert wildcard pattern (* and ?) to simple matching
                                let pattern = search_term
                                    .replace('*', "")
                                    .replace('?', "");
                                term_lower.contains(&pattern)
                            }
                        }
                    });

                    if matches {
                        result.insert(concept_id);
                    }
                }

                Ok(result)
            }

            EclFilter::Language { codes } => {
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    let descriptions = self.store.get_descriptions(concept_id);
                    let matches = descriptions.iter().any(|desc| {
                        codes.iter().any(|code| desc.language_code.to_lowercase() == *code)
                    });
                    if matches {
                        result.insert(concept_id);
                    }
                }
                Ok(result)
            }

            EclFilter::DescriptionType { type_ids } => {
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    let descriptions = self.store.get_descriptions(concept_id);
                    let matches = descriptions.iter().any(|desc| {
                        type_ids.contains(&desc.type_id)
                    });
                    if matches {
                        result.insert(concept_id);
                    }
                }
                Ok(result)
            }

            EclFilter::Dialect { dialect_ids, acceptability } => {
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    let descriptions = self.store.get_descriptions(concept_id);
                    let matches = descriptions.iter().any(|desc| {
                        let refsets = self.store.get_description_language_refsets(desc.description_id);
                        refsets.iter().any(|membership| {
                            let dialect_match = dialect_ids.contains(&membership.refset_id);
                            let acc_match = acceptability.as_ref().map_or(true, |acc| {
                                match acc {
                                    snomed_ecl::FilterAcceptability::Preferred => {
                                        membership.acceptability == Acceptability::Preferred
                                    }
                                    snomed_ecl::FilterAcceptability::Acceptable => {
                                        membership.acceptability == Acceptability::Acceptable
                                    }
                                }
                            });
                            dialect_match && acc_match
                        })
                    });
                    if matches {
                        result.insert(concept_id);
                    }
                }
                Ok(result)
            }

            EclFilter::CaseSignificance { case_significance_id } => {
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    let descriptions = self.store.get_descriptions(concept_id);
                    let matches = descriptions.iter().any(|desc| {
                        desc.case_significance_id == *case_significance_id
                    });
                    if matches {
                        result.insert(concept_id);
                    }
                }
                Ok(result)
            }

            EclFilter::Active(active) => {
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    if self.store.is_concept_active(concept_id) == *active {
                        result.insert(concept_id);
                    }
                }
                Ok(result)
            }

            EclFilter::Module { module_ids } => {
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    if let Some(module_id) = self.store.get_concept_module(concept_id) {
                        if module_ids.contains(&module_id) {
                            result.insert(concept_id);
                        }
                    }
                }
                Ok(result)
            }

            EclFilter::EffectiveTime { operator, date } => {
                use snomed_ecl::ComparisonOperator;
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    if let Some(effective_time) = self.store.get_concept_effective_time(concept_id) {
                        let matches = match operator {
                            ComparisonOperator::Equal => effective_time == *date,
                            ComparisonOperator::NotEqual => effective_time != *date,
                            ComparisonOperator::LessThan => effective_time < *date,
                            ComparisonOperator::LessThanOrEqual => effective_time <= *date,
                            ComparisonOperator::GreaterThan => effective_time > *date,
                            ComparisonOperator::GreaterThanOrEqual => effective_time >= *date,
                        };
                        if matches {
                            result.insert(concept_id);
                        }
                    }
                }
                Ok(result)
            }

            EclFilter::DefinitionStatus { is_primitive } => {
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    if let Some(primitive) = self.store.is_concept_primitive(concept_id) {
                        if primitive == *is_primitive {
                            result.insert(concept_id);
                        }
                    }
                }
                Ok(result)
            }

            EclFilter::SemanticTag { tags } => {
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    if let Some(tag) = self.store.get_semantic_tag(concept_id) {
                        let tag_lower = tag.to_lowercase();
                        if tags.iter().any(|t| t.to_lowercase() == tag_lower) {
                            result.insert(concept_id);
                        }
                    }
                }
                Ok(result)
            }

            EclFilter::PreferredIn { refset_ids } => {
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    let descriptions = self.store.get_descriptions(concept_id);
                    let matches = descriptions.iter().any(|desc| {
                        let refsets = self.store.get_description_language_refsets(desc.description_id);
                        refsets.iter().any(|membership| {
                            refset_ids.contains(&membership.refset_id)
                                && membership.acceptability == Acceptability::Preferred
                        })
                    });
                    if matches {
                        result.insert(concept_id);
                    }
                }
                Ok(result)
            }

            EclFilter::AcceptableIn { refset_ids } => {
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    let descriptions = self.store.get_descriptions(concept_id);
                    let matches = descriptions.iter().any(|desc| {
                        let refsets = self.store.get_description_language_refsets(desc.description_id);
                        refsets.iter().any(|membership| {
                            refset_ids.contains(&membership.refset_id)
                                && membership.acceptability == Acceptability::Acceptable
                        })
                    });
                    if matches {
                        result.insert(concept_id);
                    }
                }
                Ok(result)
            }

            EclFilter::LanguageRefSet { refset_ids } => {
                let mut result = HashSet::new();
                for &concept_id in concepts {
                    let descriptions = self.store.get_descriptions(concept_id);
                    let matches = descriptions.iter().any(|desc| {
                        let refsets = self.store.get_description_language_refsets(desc.description_id);
                        refsets.iter().any(|membership| {
                            refset_ids.contains(&membership.refset_id)
                        })
                    });
                    if matches {
                        result.insert(concept_id);
                    }
                }
                Ok(result)
            }

            EclFilter::Id { ids } => {
                // Filter to only concepts that are in the ID list
                let id_set: HashSet<_> = ids.iter().copied().collect();
                Ok(concepts.intersection(&id_set).copied().collect())
            }

            EclFilter::History { profile } => {
                // Include historical associations based on profile
                let mut result = concepts.clone();
                for &concept_id in concepts {
                    let historical = match profile {
                        None | Some(HistoryProfile::Max) => {
                            // All historical associations
                            self.store.get_historical_associations(concept_id)
                        }
                        Some(HistoryProfile::Min) => {
                            // SAME_AS only
                            self.store.get_historical_associations_by_type(
                                concept_id,
                                HistoryAssociationType::SameAs,
                            )
                        }
                        Some(HistoryProfile::Mod) => {
                            // SAME_AS, REPLACED_BY, POSSIBLY_EQUIVALENT_TO
                            let mut assocs = self.store.get_historical_associations_by_type(
                                concept_id,
                                HistoryAssociationType::SameAs,
                            );
                            assocs.extend(self.store.get_historical_associations_by_type(
                                concept_id,
                                HistoryAssociationType::ReplacedBy,
                            ));
                            assocs.extend(self.store.get_historical_associations_by_type(
                                concept_id,
                                HistoryAssociationType::PossiblyEquivalentTo,
                            ));
                            assocs
                        }
                    };
                    result.extend(historical);
                }
                Ok(result)
            }

            EclFilter::Member { .. } => {
                // Member filters require refset access - return as-is for now
                // Full implementation would filter based on refset member fields
                Ok(concepts.clone())
            }

            EclFilter::DomainQualified { domain: _, filter } => {
                // Domain-qualified filters: the domain indicates which component type to filter
                // For now, we delegate to the inner filter since most filters already have
                // implicit domain semantics (e.g., term filters are always description-based)
                // Full implementation would use domain to disambiguate when filters can
                // apply to multiple domains (like active, effectiveTime, moduleId)
                self.apply_filter(concepts, filter)
            }
        }
    }

    /// Executes a subexpression with intermediate caching for compound queries.
    ///
    /// This is used for the left and right operands of AND, OR, and MINUS operations.
    /// When cache_intermediates is enabled, subexpression results are cached.
    fn execute_with_intermediate_cache(
        &self,
        expr: &EclExpression,
        traverser: &HierarchyTraverser<'_>,
    ) -> EclResult<(HashSet<SctId>, usize)> {
        // Check if we should cache intermediates
        let should_cache = self
            .cache
            .as_ref()
            .map(|c| c.should_cache_intermediates())
            .unwrap_or(false);

        if !should_cache {
            return self.execute_expression(expr, traverser);
        }

        // Generate cache key from expression
        let cache_key = normalize_cache_key(&expr.to_string());

        // Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(cached_result) = cache.get(&cache_key) {
                return Ok((cached_result, 0)); // 0 traversed since it's cached
            }
        }

        // Execute the expression
        let (result, count) = self.execute_expression(expr, traverser)?;

        // Cache the result
        if let Some(ref cache) = self.cache {
            cache.set(cache_key, result.clone());
        }

        Ok((result, count))
    }

    /// Extracts the focus concept ID from an expression.
    ///
    /// For hierarchy operations like `< concept`, `<< concept`, etc.,
    /// the inner expression must resolve to a single concept.
    fn get_focus_concept(&self, expr: &EclExpression) -> EclResult<SctId> {
        let expr = expr.unwrap_nested();

        match expr {
            EclExpression::ConceptReference { concept_id, .. } => {
                // Verify concept exists
                if !self.store.has_concept(*concept_id) {
                    return Err(EclExecutorError::ConceptNotFound(*concept_id));
                }
                Ok(*concept_id)
            }
            _ => Err(EclExecutorError::UnsupportedFeature(
                "Hierarchy operators require a concept reference".to_string(),
            )),
        }
    }

    /// Returns a query plan without executing.
    ///
    /// Analyzes the ECL expression to generate an execution plan with:
    /// - Ordered execution steps
    /// - Cardinality estimates for each step
    /// - Optimization hints for improving performance
    ///
    /// This method does NOT execute the query - it only plans it.
    ///
    /// # Arguments
    ///
    /// * `ecl` - ECL expression string
    ///
    /// # Returns
    ///
    /// * `Ok(QueryPlan)` - The execution plan with estimates and hints
    /// * `Err(EclExecutorError)` - If parsing fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let plan = executor.explain("<< 404684003 AND << 39057004")?;
    ///
    /// println!("Estimated result: {} concepts", plan.estimated_total);
    /// println!("Estimated cost: {:.2}ms", plan.total_cost());
    ///
    /// for hint in &plan.optimization_hints {
    ///     println!("Optimization: {}", hint);
    /// }
    /// ```
    pub fn explain(&self, ecl: &str) -> EclResult<QueryPlan> {
        // Parse the ECL expression
        let expr = snomed_ecl::parse(ecl)?;

        // Use the query planner to generate the plan
        let planner = QueryPlanner::new();
        Ok(planner.plan(ecl, &expr))
    }

    /// Checks if a concept matches an ECL constraint.
    ///
    /// This is optimized for single-concept checking and may be
    /// faster than executing the full query when only checking
    /// membership.
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept ID to check
    /// * `ecl` - ECL constraint expression
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - If the concept matches the constraint
    /// * `Ok(false)` - If the concept does not match
    /// * `Err(EclExecutorError)` - If parsing or execution fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Check if Type 2 diabetes is a type of diabetes
    /// let is_diabetes = executor.matches(46635009, "<< 73211009")?;
    /// assert!(is_diabetes);
    /// ```
    pub fn matches(&self, concept_id: SctId, ecl: &str) -> EclResult<bool> {
        // For now, execute the full query and check membership
        // TODO: Optimize in Story 10.2 with early termination
        let result = self.execute(ecl)?;
        Ok(result.contains(concept_id))
    }

    /// Tests subsumption: is `child` a descendant of `parent`?
    ///
    /// This is a specialized method that directly traverses the
    /// hierarchy without parsing ECL.
    ///
    /// # Arguments
    ///
    /// * `child` - The potential descendant concept ID
    /// * `parent` - The potential ancestor concept ID
    ///
    /// # Returns
    ///
    /// `true` if `child` is a descendant of `parent`, `false` otherwise.
    pub fn is_subsumed_by(&self, child: SctId, parent: SctId) -> bool {
        if child == parent {
            return true;
        }

        // BFS to check if parent is an ancestor of child
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back(child);
        visited.insert(child);

        while let Some(current) = queue.pop_front() {
            for ancestor in self.store.get_parents(current) {
                if ancestor == parent {
                    return true;
                }
                if visited.insert(ancestor) {
                    queue.push_back(ancestor);
                }
            }
        }

        false
    }

    // ==================== Convenience Methods (Story 10.7) ====================

    /// Gets all ancestors of a concept.
    ///
    /// Equivalent to executing `> concept_id` ECL.
    /// Returns ancestors sorted by concept ID for consistent ordering.
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept to get ancestors of
    ///
    /// # Returns
    ///
    /// A sorted vector of all ancestor concept IDs.
    /// Returns empty vector if concept not found or has no ancestors.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ancestors = executor.get_ancestors(46635009.into());
    /// // [73211009, 126877002, 64572001, ...]
    /// ```
    pub fn get_ancestors(&self, concept_id: SctId) -> Vec<SctId> {
        let traverser = HierarchyTraverser::new(self.store);
        let mut ancestors: Vec<SctId> = traverser.get_ancestors(concept_id).into_iter().collect();
        ancestors.sort_unstable();
        ancestors
    }

    /// Gets all descendants of a concept.
    ///
    /// Equivalent to executing `< concept_id` ECL.
    /// Returns descendants sorted by concept ID for consistent ordering.
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept to get descendants of
    ///
    /// # Returns
    ///
    /// A sorted vector of all descendant concept IDs.
    /// Returns empty vector if concept not found or has no descendants.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let descendants = executor.get_descendants(73211009.into());
    /// // [46635009, 44054006, ...]
    /// ```
    pub fn get_descendants(&self, concept_id: SctId) -> Vec<SctId> {
        let traverser = HierarchyTraverser::new(self.store);
        let mut descendants: Vec<SctId> =
            traverser.get_descendants(concept_id).into_iter().collect();
        descendants.sort_unstable();
        descendants
    }

    /// Gets descendants of a concept with a limit.
    ///
    /// Uses BFS traversal to get the "closest" descendants first,
    /// returning up to `limit` concepts. Useful for large hierarchies
    /// where you only need a sample.
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept to get descendants of
    /// * `limit` - Maximum number of descendants to return
    ///
    /// # Returns
    ///
    /// A vector of descendant concept IDs, up to `limit` in size.
    /// Descendants closer to the concept appear first (BFS order).
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Get only the first 100 descendants
    /// let sample = executor.get_descendants_limited(73211009.into(), 100);
    /// assert!(sample.len() <= 100);
    /// ```
    pub fn get_descendants_limited(&self, concept_id: SctId, limit: usize) -> Vec<SctId> {
        use std::collections::VecDeque;

        // Early return for limit of 0
        if limit == 0 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(limit.min(1000));
        let mut visited = HashSet::with_capacity(limit.min(1000));
        let mut queue = VecDeque::with_capacity(100);

        // Start with direct children
        for child in self.store.get_children(concept_id) {
            if visited.insert(child) {
                queue.push_back(child);
            }
        }

        // BFS traversal with limit
        while let Some(current) = queue.pop_front() {
            result.push(current);

            if result.len() >= limit {
                break;
            }

            for child in self.store.get_children(current) {
                if visited.insert(child) {
                    queue.push_back(child);
                }
            }
        }

        result
    }

    /// Gets direct parents of a concept.
    ///
    /// Equivalent to executing `>! concept_id` ECL.
    /// Returns only immediate parents, not all ancestors.
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept to get parents of
    ///
    /// # Returns
    ///
    /// A sorted vector of direct parent concept IDs.
    /// Returns empty vector if concept is root or not found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let parents = executor.get_parents(46635009.into());
    /// // Direct parents only, not grandparents
    /// ```
    pub fn get_parents(&self, concept_id: SctId) -> Vec<SctId> {
        let mut parents = self.store.get_parents(concept_id);
        parents.sort_unstable();
        parents
    }

    /// Gets direct children of a concept.
    ///
    /// Equivalent to executing `<! concept_id` ECL.
    /// Returns only immediate children, not all descendants.
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept to get children of
    ///
    /// # Returns
    ///
    /// A sorted vector of direct child concept IDs.
    /// Returns empty vector if concept is leaf or not found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let children = executor.get_children(73211009.into());
    /// // Direct children only, not grandchildren
    /// ```
    pub fn get_children(&self, concept_id: SctId) -> Vec<SctId> {
        let mut children = self.store.get_children(concept_id);
        children.sort_unstable();
        children
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Mock store for testing.
    struct MockStore {
        concepts: HashSet<SctId>,
        children: HashMap<SctId, Vec<SctId>>,
        parents: HashMap<SctId, Vec<SctId>>,
        refsets: HashMap<SctId, Vec<SctId>>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                concepts: HashSet::new(),
                children: HashMap::new(),
                parents: HashMap::new(),
                refsets: HashMap::new(),
            }
        }

        fn add_concept(&mut self, id: SctId) {
            self.concepts.insert(id);
        }

        fn add_is_a(&mut self, child: SctId, parent: SctId) {
            self.children.entry(parent).or_default().push(child);
            self.parents.entry(child).or_default().push(parent);
        }

        #[allow(dead_code)]
        fn add_refset_member(&mut self, refset_id: SctId, member: SctId) {
            self.refsets.entry(refset_id).or_default().push(member);
        }
    }

    impl EclQueryable for MockStore {
        fn get_children(&self, concept_id: SctId) -> Vec<SctId> {
            self.children.get(&concept_id).cloned().unwrap_or_default()
        }

        fn get_parents(&self, concept_id: SctId) -> Vec<SctId> {
            self.parents.get(&concept_id).cloned().unwrap_or_default()
        }

        fn has_concept(&self, concept_id: SctId) -> bool {
            self.concepts.contains(&concept_id)
        }

        fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_> {
            Box::new(self.concepts.iter().copied())
        }

        fn get_refset_members(&self, refset_id: SctId) -> Vec<SctId> {
            self.refsets.get(&refset_id).cloned().unwrap_or_default()
        }
    }

    /// Creates a test hierarchy:
    /// ```text
    /// 100 (root)
    ///  |-- 200
    ///  |    |-- 400
    ///  |    |-- 500
    ///  |-- 300
    ///       |-- 600
    /// ```
    fn create_test_store() -> MockStore {
        let mut store = MockStore::new();

        for id in [100, 200, 300, 400, 500, 600] {
            store.add_concept(id);
        }

        store.add_is_a(200, 100);
        store.add_is_a(300, 100);
        store.add_is_a(400, 200);
        store.add_is_a(500, 200);
        store.add_is_a(600, 300);

        store
    }

    /// Creates a diamond inheritance hierarchy:
    /// ```text
    ///     100
    ///    /   \
    ///  200   300
    ///    \   /
    ///     400
    /// ```
    fn create_diamond_store() -> MockStore {
        let mut store = MockStore::new();

        for id in [100, 200, 300, 400] {
            store.add_concept(id);
        }

        store.add_is_a(200, 100);
        store.add_is_a(300, 100);
        store.add_is_a(400, 200);
        store.add_is_a(400, 300);

        store
    }

    // Basic executor tests

    #[test]
    fn test_executor_new() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);
        assert!(!executor.config().parallel);
        assert!(executor.config().cache.is_none());
    }

    #[test]
    fn test_executor_with_config() {
        let store = create_test_store();
        let config = ExecutorConfig::builder().with_parallel(true).build();
        let executor = EclExecutor::with_config(&store, config);
        assert!(executor.config().parallel);
    }

    // Self constraint tests (Story 10.2 Task 2)

    #[test]
    fn test_self_constraint_valid_concept() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("100").unwrap();
        assert_eq!(result.count(), 1);
        assert!(result.contains(100));
    }

    #[test]
    fn test_self_constraint_invalid_concept() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("999");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EclExecutorError::ConceptNotFound(999)
        ));
    }

    // Descendant of tests (Story 10.2 Task 3)

    #[test]
    fn test_descendant_of_root() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("< 100").unwrap();

        // All descendants of 100: 200, 300, 400, 500, 600
        assert_eq!(result.count(), 5);
        assert!(result.contains(200));
        assert!(result.contains(300));
        assert!(result.contains(400));
        assert!(result.contains(500));
        assert!(result.contains(600));
        // Should NOT include self
        assert!(!result.contains(100));
    }

    #[test]
    fn test_descendant_of_intermediate() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("< 200").unwrap();

        // Descendants of 200: 400, 500
        assert_eq!(result.count(), 2);
        assert!(result.contains(400));
        assert!(result.contains(500));
        assert!(!result.contains(200));
    }

    #[test]
    fn test_descendant_of_leaf() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("< 400").unwrap();

        // 400 is a leaf, no descendants
        assert!(result.is_empty());
    }

    // Descendant or self tests (Story 10.2 Task 4)

    #[test]
    fn test_descendant_or_self() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("<< 200").unwrap();

        // 200 + descendants: 200, 400, 500
        assert_eq!(result.count(), 3);
        assert!(result.contains(200)); // Self
        assert!(result.contains(400));
        assert!(result.contains(500));
    }

    #[test]
    fn test_descendant_or_self_leaf() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("<< 400").unwrap();

        // 400 is leaf, only self
        assert_eq!(result.count(), 1);
        assert!(result.contains(400));
    }

    // Ancestor of tests (Story 10.3 Task 2)

    #[test]
    fn test_ancestor_of() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("> 400").unwrap();

        // Ancestors of 400: 200, 100
        assert_eq!(result.count(), 2);
        assert!(result.contains(200));
        assert!(result.contains(100));
        // Should NOT include self
        assert!(!result.contains(400));
    }

    #[test]
    fn test_ancestor_of_root() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("> 100").unwrap();

        // 100 is root, no ancestors
        assert!(result.is_empty());
    }

    // Ancestor or self tests (Story 10.3 Task 3)

    #[test]
    fn test_ancestor_or_self() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute(">> 400").unwrap();

        // 400 + ancestors: 400, 200, 100
        assert_eq!(result.count(), 3);
        assert!(result.contains(400)); // Self
        assert!(result.contains(200));
        assert!(result.contains(100));
    }

    // Child of tests (Story 10.3 Task 4)

    #[test]
    fn test_child_of() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("<! 100").unwrap();

        // Direct children of 100: 200, 300
        assert_eq!(result.count(), 2);
        assert!(result.contains(200));
        assert!(result.contains(300));
        // Should NOT include grandchildren
        assert!(!result.contains(400));
    }

    #[test]
    fn test_child_of_leaf() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("<! 400").unwrap();

        // 400 is leaf, no children
        assert!(result.is_empty());
    }

    // Parent of tests (Story 10.3 Task 5)

    #[test]
    fn test_parent_of() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute(">! 400").unwrap();

        // Direct parent of 400: 200
        assert_eq!(result.count(), 1);
        assert!(result.contains(200));
        // Should NOT include grandparent
        assert!(!result.contains(100));
    }

    #[test]
    fn test_parent_of_root() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute(">! 100").unwrap();

        // 100 is root, no parents
        assert!(result.is_empty());
    }

    #[test]
    fn test_parent_of_multiple_parents() {
        let store = create_diamond_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute(">! 400").unwrap();

        // 400 has two parents: 200, 300
        assert_eq!(result.count(), 2);
        assert!(result.contains(200));
        assert!(result.contains(300));
    }

    // Diamond inheritance tests

    #[test]
    fn test_diamond_ancestors_no_duplicates() {
        let store = create_diamond_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("> 400").unwrap();

        // Ancestors of 400: 200, 300, 100 (no duplicates)
        assert_eq!(result.count(), 3);
        assert!(result.contains(200));
        assert!(result.contains(300));
        assert!(result.contains(100));
    }

    // Compound constraint tests (AND, OR, MINUS)

    #[test]
    fn test_and_constraint() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // Descendants of 100 AND descendants of 200
        // (200, 300, 400, 500, 600) AND (400, 500) = (400, 500)
        let result = executor.execute("< 100 AND < 200").unwrap();

        assert_eq!(result.count(), 2);
        assert!(result.contains(400));
        assert!(result.contains(500));
    }

    #[test]
    fn test_or_constraint() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // Direct children of 200 OR direct children of 300
        // (400, 500) OR (600) = (400, 500, 600)
        let result = executor.execute("<! 200 OR <! 300").unwrap();

        assert_eq!(result.count(), 3);
        assert!(result.contains(400));
        assert!(result.contains(500));
        assert!(result.contains(600));
    }

    #[test]
    fn test_minus_constraint() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // Descendants of 100 MINUS descendants of 200
        // (200, 300, 400, 500, 600) MINUS (400, 500) = (200, 300, 600)
        let result = executor.execute("< 100 MINUS < 200").unwrap();

        assert_eq!(result.count(), 3);
        assert!(result.contains(200));
        assert!(result.contains(300));
        assert!(result.contains(600));
    }

    // Subsumption tests

    #[test]
    fn test_is_subsumed_by_self() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        assert!(executor.is_subsumed_by(100, 100));
        assert!(executor.is_subsumed_by(400, 400));
    }

    #[test]
    fn test_is_subsumed_by_direct_parent() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        assert!(executor.is_subsumed_by(200, 100));
        assert!(executor.is_subsumed_by(400, 200));
    }

    #[test]
    fn test_is_subsumed_by_indirect_ancestor() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        assert!(executor.is_subsumed_by(400, 100));
        assert!(executor.is_subsumed_by(600, 100));
    }

    #[test]
    fn test_is_subsumed_by_not_related() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        assert!(!executor.is_subsumed_by(400, 300));
        assert!(!executor.is_subsumed_by(100, 200));
    }

    // Query plan tests (Story 10.6)

    #[test]
    fn test_explain_returns_plan() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let plan = executor.explain("<< 100").unwrap();
        assert_eq!(plan.ecl, "<< 100");
        assert!(!plan.steps.is_empty());
        assert_eq!(plan.steps[0].operation, "DescendantsOrSelf");
    }

    #[test]
    fn test_explain_does_not_execute() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // explain() should not execute - verify by checking it doesn't error
        // on expressions that would fail execution due to concept not found
        let plan = executor.explain("<< 999999999");
        // Should succeed (parsing works) even though concept doesn't exist
        assert!(plan.is_ok());
    }

    #[test]
    fn test_explain_compound_expressions() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let plan = executor.explain("<< 100 AND << 200").unwrap();

        // Should have: DescendantsOrSelf, DescendantsOrSelf, Intersect
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[2].operation, "Intersect");
    }

    #[test]
    fn test_explain_has_estimates() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let plan = executor.explain("<< 100").unwrap();

        // Should have cardinality estimate
        assert!(plan.estimated_total > 0);

        // Steps should have cost estimates
        assert!(plan.steps[0].cost_estimate >= 0.0);
    }

    // Error handling tests

    #[test]
    fn test_execute_invalid_ecl() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("invalid ecl !!!");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EclExecutorError::ParseError(_)
        ));
    }

    #[test]
    fn test_execute_concept_not_found() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("<< 999");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EclExecutorError::ConceptNotFound(999)
        ));
    }

    // Matches method tests

    #[test]
    fn test_matches_positive() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 400 is a descendant of 100
        assert!(executor.matches(400, "<< 100").unwrap());
    }

    #[test]
    fn test_matches_negative() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 100 is NOT a descendant of 200
        assert!(!executor.matches(100, "<< 200").unwrap());
    }

    // Execution stats tests

    #[test]
    fn test_execution_stats_populated() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("<< 100").unwrap();

        assert!(result.stats.duration.as_nanos() > 0);
        assert!(result.stats.concepts_traversed > 0);
        assert!(!result.stats.cache_hit);
    }

    // ==================== Convenience Methods Tests (Story 10.7) ====================

    #[test]
    fn test_get_ancestors() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 400's ancestors: 200, 100
        let ancestors = executor.get_ancestors(400);
        assert_eq!(ancestors.len(), 2);
        assert!(ancestors.contains(&200));
        assert!(ancestors.contains(&100));
        // Should be sorted
        assert!(ancestors[0] < ancestors[1] || ancestors.len() == 1);
    }

    #[test]
    fn test_get_ancestors_root() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 100 is root, no ancestors
        let ancestors = executor.get_ancestors(100);
        assert!(ancestors.is_empty());
    }

    #[test]
    fn test_get_descendants() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 200's descendants: 400, 500
        let descendants = executor.get_descendants(200);
        assert_eq!(descendants.len(), 2);
        assert!(descendants.contains(&400));
        assert!(descendants.contains(&500));
    }

    #[test]
    fn test_get_descendants_leaf() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 400 is a leaf, no descendants
        let descendants = executor.get_descendants(400);
        assert!(descendants.is_empty());
    }

    #[test]
    fn test_get_descendants_limited() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 100 has 5 descendants, limit to 2
        let descendants = executor.get_descendants_limited(100, 2);
        assert_eq!(descendants.len(), 2);

        // All returned should be descendants of 100
        for id in &descendants {
            assert!(executor.is_subsumed_by(*id, 100));
        }
    }

    #[test]
    fn test_get_descendants_limited_larger_than_actual() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 200 has 2 descendants, limit to 100
        let descendants = executor.get_descendants_limited(200, 100);
        assert_eq!(descendants.len(), 2);
    }

    #[test]
    fn test_get_parents() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 400's direct parent: 200
        let parents = executor.get_parents(400);
        assert_eq!(parents.len(), 1);
        assert!(parents.contains(&200));
        // Should NOT contain grandparent
        assert!(!parents.contains(&100));
    }

    #[test]
    fn test_get_parents_root() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 100 is root, no parents
        let parents = executor.get_parents(100);
        assert!(parents.is_empty());
    }

    #[test]
    fn test_get_parents_multiple() {
        let store = create_diamond_store();
        let executor = EclExecutor::new(&store);

        // 400 has two parents: 200, 300
        let parents = executor.get_parents(400);
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&200));
        assert!(parents.contains(&300));
    }

    #[test]
    fn test_get_children() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 100's direct children: 200, 300
        let children = executor.get_children(100);
        assert_eq!(children.len(), 2);
        assert!(children.contains(&200));
        assert!(children.contains(&300));
        // Should NOT contain grandchildren
        assert!(!children.contains(&400));
    }

    #[test]
    fn test_get_children_leaf() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // 400 is a leaf, no children
        let children = executor.get_children(400);
        assert!(children.is_empty());
    }

    #[test]
    fn test_convenience_methods_return_sorted() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        // Test that all convenience methods return sorted results
        let ancestors = executor.get_ancestors(400);
        for i in 1..ancestors.len() {
            assert!(ancestors[i - 1] <= ancestors[i]);
        }

        let descendants = executor.get_descendants(100);
        for i in 1..descendants.len() {
            assert!(descendants[i - 1] <= descendants[i]);
        }

        let parents = executor.get_parents(400);
        for i in 1..parents.len() {
            assert!(parents[i - 1] <= parents[i]);
        }

        let children = executor.get_children(100);
        for i in 1..children.len() {
            assert!(children[i - 1] <= children[i]);
        }
    }
}
