//! Query planner for ECL execution.
//!
//! Provides query planning and optimization hints for ECL expressions.
//! The planner analyzes ECL ASTs to generate execution plans with
//! cardinality estimates and optimization suggestions.

use std::fmt;

use snomed_ecl::EclExpression;
use snomed_ecl::SctId;

use crate::statistics::{heuristics, StatisticsService};

/// A complete execution plan for an ECL query.
///
/// Contains the steps to execute the query, estimated total cardinality,
/// and optimization hints for improving query performance.
///
/// # Example
///
/// ```ignore
/// let plan = executor.explain("<< 404684003 AND << 39057004")?;
///
/// println!("Estimated result size: {}", plan.estimated_total);
///
/// for hint in &plan.optimization_hints {
///     println!("Hint: {}", hint);
/// }
///
/// for step in &plan.steps {
///     println!("{} - estimated {} concepts", step.operation, step.estimated_cardinality);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// The original ECL expression.
    pub ecl: String,
    /// Ordered execution steps.
    pub steps: Vec<QueryStep>,
    /// Estimated total result size.
    pub estimated_total: usize,
    /// Optimization hints and suggestions.
    pub optimization_hints: Vec<String>,
}

impl QueryPlan {
    /// Creates a new empty query plan.
    pub fn new(ecl: impl Into<String>) -> Self {
        Self {
            ecl: ecl.into(),
            steps: Vec::new(),
            estimated_total: 0,
            optimization_hints: Vec::new(),
        }
    }

    /// Adds a step to the plan.
    pub fn add_step(&mut self, step: QueryStep) {
        self.steps.push(step);
    }

    /// Adds an optimization hint.
    pub fn add_hint(&mut self, hint: impl Into<String>) {
        self.optimization_hints.push(hint.into());
    }

    /// Returns true if the plan has optimization hints.
    pub fn has_hints(&self) -> bool {
        !self.optimization_hints.is_empty()
    }

    /// Calculates total estimated cost from all steps.
    pub fn total_cost(&self) -> f64 {
        self.steps.iter().map(|s| s.cost_estimate).sum()
    }
}

impl fmt::Display for QueryPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Query Plan for: {}", self.ecl)?;
        writeln!(f, "Estimated total: {} concepts", self.estimated_total)?;
        writeln!(f, "Estimated cost: {:.4}ms", self.total_cost())?;
        writeln!(f)?;

        writeln!(f, "Steps:")?;
        for (i, step) in self.steps.iter().enumerate() {
            writeln!(f, "  {}. {}", i + 1, step)?;
        }

        if !self.optimization_hints.is_empty() {
            writeln!(f)?;
            writeln!(f, "Optimization Hints:")?;
            for hint in &self.optimization_hints {
                writeln!(f, "  - {}", hint)?;
            }
        }

        Ok(())
    }
}

/// A single step in the query execution plan.
#[derive(Debug, Clone)]
pub struct QueryStep {
    /// Type of operation (e.g., "Descendants", "Intersect", "Union").
    pub operation: String,
    /// The ECL subexpression for this step.
    pub expression: String,
    /// Estimated number of concepts in the result.
    pub estimated_cardinality: usize,
    /// Estimated execution cost (in milliseconds).
    pub cost_estimate: f64,
}

impl QueryStep {
    /// Creates a new query step.
    pub fn new(
        operation: impl Into<String>,
        expression: impl Into<String>,
        estimated_cardinality: usize,
        cost_estimate: f64,
    ) -> Self {
        Self {
            operation: operation.into(),
            expression: expression.into(),
            estimated_cardinality,
            cost_estimate,
        }
    }
}

impl fmt::Display for QueryStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} (est. {} concepts, {:.4}ms)",
            self.operation, self.expression, self.estimated_cardinality, self.cost_estimate
        )
    }
}

/// Query planner for generating execution plans from ECL expressions.
///
/// The planner walks the ECL AST and generates a plan with:
/// - Execution steps in order
/// - Cardinality estimates for each step
/// - Optimization hints for improving performance
#[derive(Debug)]
pub struct QueryPlanner {
    /// Statistics service for cardinality estimation.
    statistics: StatisticsService,
}

impl Default for QueryPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryPlanner {
    /// Creates a new query planner with default statistics.
    pub fn new() -> Self {
        Self {
            statistics: StatisticsService::new(),
        }
    }

    /// Creates a query planner with custom statistics.
    pub fn with_statistics(statistics: StatisticsService) -> Self {
        Self { statistics }
    }

    /// Returns a reference to the statistics service.
    pub fn statistics(&self) -> &StatisticsService {
        &self.statistics
    }

    /// Returns a mutable reference to the statistics service.
    pub fn statistics_mut(&mut self) -> &mut StatisticsService {
        &mut self.statistics
    }

    /// Generates a query plan for an ECL expression.
    ///
    /// Walks the AST to build execution steps and generates
    /// optimization hints based on the query structure.
    pub fn plan(&self, ecl: &str, expr: &EclExpression) -> QueryPlan {
        let mut plan = QueryPlan::new(ecl);

        // Recursively plan the expression
        let (estimate, _cost) = self.plan_expression(expr, &mut plan);
        plan.estimated_total = estimate;

        // Generate optimization hints based on the full plan
        self.generate_hints(&mut plan, expr);

        plan
    }

    /// Plans a single expression and returns (estimated_cardinality, cost).
    fn plan_expression(&self, expr: &EclExpression, plan: &mut QueryPlan) -> (usize, f64) {
        let expr = expr.unwrap_nested();

        match expr {
            EclExpression::ConceptReference { concept_id, term } => {
                let estimate = self.statistics.estimated_self(*concept_id);
                let cost = self.statistics.cost_lookup();

                let expr_str = if let Some(t) = term {
                    format!("{} |{}|", concept_id, t)
                } else {
                    concept_id.to_string()
                };

                plan.add_step(QueryStep::new("Self", expr_str, estimate, cost));
                (estimate, cost)
            }

            EclExpression::DescendantOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                let estimate = self.statistics.estimated_descendants(concept_id);
                let cost = self.statistics.cost_descendants(estimate);

                plan.add_step(QueryStep::new(
                    "Descendants",
                    format!("< {}", concept_id),
                    estimate,
                    cost,
                ));

                if self.statistics.is_large_traversal(estimate) {
                    plan.add_hint(format!(
                        "Large descendant traversal for concept {} (est. {} concepts)",
                        concept_id, estimate
                    ));
                }

                (estimate, cost)
            }

            EclExpression::DescendantOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                let estimate = self.statistics.estimated_descendants(concept_id) + 1;
                let cost = self.statistics.cost_descendants(estimate);

                plan.add_step(QueryStep::new(
                    "DescendantsOrSelf",
                    format!("<< {}", concept_id),
                    estimate,
                    cost,
                ));

                if self.statistics.is_large_traversal(estimate) {
                    plan.add_hint(format!(
                        "Large descendant traversal for concept {} (est. {} concepts)",
                        concept_id, estimate
                    ));
                }

                (estimate, cost)
            }

            EclExpression::AncestorOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                let estimate = self.statistics.estimated_ancestors(concept_id);
                let cost = self.statistics.cost_ancestors(estimate);

                plan.add_step(QueryStep::new(
                    "Ancestors",
                    format!("> {}", concept_id),
                    estimate,
                    cost,
                ));

                (estimate, cost)
            }

            EclExpression::AncestorOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                let estimate = self.statistics.estimated_ancestors(concept_id) + 1;
                let cost = self.statistics.cost_ancestors(estimate);

                plan.add_step(QueryStep::new(
                    "AncestorsOrSelf",
                    format!(">> {}", concept_id),
                    estimate,
                    cost,
                ));

                (estimate, cost)
            }

            EclExpression::ChildOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                let estimate = self.statistics.estimated_children(concept_id);
                let cost = self.statistics.cost_lookup();

                plan.add_step(QueryStep::new(
                    "Children",
                    format!("<! {}", concept_id),
                    estimate,
                    cost,
                ));

                (estimate, cost)
            }

            EclExpression::ChildOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                let estimate = self.statistics.estimated_children(concept_id) + 1;
                let cost = self.statistics.cost_lookup();

                plan.add_step(QueryStep::new(
                    "ChildrenOrSelf",
                    format!("<<! {}", concept_id),
                    estimate,
                    cost,
                ));

                (estimate, cost)
            }

            EclExpression::ParentOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                let estimate = self.statistics.estimated_parents(concept_id);
                let cost = self.statistics.cost_lookup();

                plan.add_step(QueryStep::new(
                    "Parents",
                    format!(">! {}", concept_id),
                    estimate,
                    cost,
                ));

                (estimate, cost)
            }

            EclExpression::ParentOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                let estimate = self.statistics.estimated_parents(concept_id) + 1;
                let cost = self.statistics.cost_lookup();

                plan.add_step(QueryStep::new(
                    "ParentsOrSelf",
                    format!(">>! {}", concept_id),
                    estimate,
                    cost,
                ));

                (estimate, cost)
            }

            EclExpression::And(left, right) => {
                let (left_estimate, left_cost) = self.plan_expression(left, plan);
                let (right_estimate, right_cost) = self.plan_expression(right, plan);

                let result_estimate = self.statistics.estimated_and(left_estimate, right_estimate);
                let intersect_cost = self
                    .statistics
                    .cost_intersection(left_estimate.min(right_estimate));

                plan.add_step(QueryStep::new(
                    "Intersect",
                    "AND",
                    result_estimate,
                    intersect_cost,
                ));

                let total_cost = left_cost + right_cost + intersect_cost;
                (result_estimate, total_cost)
            }

            EclExpression::Or(left, right) => {
                let (left_estimate, left_cost) = self.plan_expression(left, plan);
                let (right_estimate, right_cost) = self.plan_expression(right, plan);

                let result_estimate = self.statistics.estimated_or(left_estimate, right_estimate);
                let union_cost = self.statistics.cost_union(left_estimate + right_estimate);

                plan.add_step(QueryStep::new("Union", "OR", result_estimate, union_cost));

                let total_cost = left_cost + right_cost + union_cost;
                (result_estimate, total_cost)
            }

            EclExpression::Minus(left, right) => {
                let (left_estimate, left_cost) = self.plan_expression(left, plan);
                let (right_estimate, right_cost) = self.plan_expression(right, plan);

                let result_estimate = self
                    .statistics
                    .estimated_minus(left_estimate, right_estimate);
                let diff_cost = self.statistics.cost_difference(left_estimate);

                plan.add_step(QueryStep::new(
                    "Difference",
                    "MINUS",
                    result_estimate,
                    diff_cost,
                ));

                let total_cost = left_cost + right_cost + diff_cost;
                (result_estimate, total_cost)
            }

            EclExpression::MemberOf { refset } => {
                // Reference sets have variable size; use a conservative estimate
                let estimate = heuristics::DEFAULT_DESCENDANT_ESTIMATE;
                let cost = self.statistics.cost_lookup();

                plan.add_step(QueryStep::new(
                    "MemberOf",
                    format!("^ ({})", refset),
                    estimate,
                    cost,
                ));

                (estimate, cost)
            }

            EclExpression::Any => {
                // All concepts - this is very large
                let estimate = 500_000; // Approximate total SNOMED concepts
                let cost = self.statistics.cost_descendants(estimate);

                plan.add_step(QueryStep::new("Any", "*", estimate, cost));
                plan.add_hint(
                    "Wildcard (*) query returns all concepts - consider adding constraints"
                        .to_string(),
                );

                (estimate, cost)
            }

            EclExpression::AlternateIdentifier { scheme, identifier } => {
                // Alternate identifier resolves to a single concept (or none)
                let estimate = 1;
                let cost = self.statistics.cost_lookup();

                plan.add_step(QueryStep::new(
                    "AlternateId",
                    format!("{}#{}", scheme, identifier),
                    estimate,
                    cost,
                ));

                (estimate, cost)
            }

            EclExpression::Nested(_) => {
                unreachable!("Nested expressions are unwrapped at the start")
            }

            // =========================================================================
            // Advanced ECL Features (Story 10.9)
            // =========================================================================
            EclExpression::Refined { focus, .. } => {
                let (focus_estimate, focus_cost) = self.plan_expression(focus, plan);
                // Refinement typically filters to a fraction of the focus
                let estimate = (focus_estimate as f64 * 0.1).max(1.0) as usize;
                let cost =
                    focus_cost + self.statistics.cost_lookup() * (focus_estimate as f64).sqrt();

                plan.add_step(QueryStep::new("Refined", "with refinement", estimate, cost));

                (estimate, focus_cost + cost)
            }

            EclExpression::DotNotation { source, .. } => {
                let (source_estimate, source_cost) = self.plan_expression(source, plan);
                // Dot notation returns attribute destinations, typically similar size
                let estimate = source_estimate;
                let cost = source_cost + self.statistics.cost_lookup() * source_estimate as f64;

                plan.add_step(QueryStep::new(
                    "DotNotation",
                    "attribute navigation",
                    estimate,
                    cost,
                ));

                (estimate, source_cost + cost)
            }

            EclExpression::Concrete { .. } => {
                // Concrete values alone don't return concepts
                (0, 0.0)
            }

            EclExpression::Filtered { expression, .. } => {
                let (inner_estimate, inner_cost) = self.plan_expression(expression, plan);
                // Filters typically reduce results
                let estimate = (inner_estimate as f64 * 0.5).max(1.0) as usize;
                let cost = self.statistics.cost_lookup() * inner_estimate as f64;

                plan.add_step(QueryStep::new("Filtered", "with filter", estimate, cost));

                (estimate, inner_cost + cost)
            }

            EclExpression::TopOfSet(inner) => {
                let (inner_estimate, inner_cost) = self.plan_expression(inner, plan);
                // Top of set returns much fewer - the most general
                let estimate = (inner_estimate as f64 * 0.1).max(1.0) as usize;
                let cost = self.statistics.cost_ancestors(inner_estimate);

                plan.add_step(QueryStep::new("TopOfSet", "!!>", estimate, cost));

                (estimate, inner_cost + cost)
            }

            EclExpression::BottomOfSet(inner) => {
                let (inner_estimate, inner_cost) = self.plan_expression(inner, plan);
                // Bottom of set returns the leaves - often larger than top
                let estimate = (inner_estimate as f64 * 0.3).max(1.0) as usize;
                let cost = self.statistics.cost_descendants(inner_estimate);

                plan.add_step(QueryStep::new("BottomOfSet", "!!<", estimate, cost));

                (estimate, inner_cost + cost)
            }

            EclExpression::ConceptSet(ids) => {
                let estimate = ids.len();
                let cost = self.statistics.cost_lookup() * ids.len() as f64;

                plan.add_step(QueryStep::new(
                    "ConceptSet",
                    format!("({} concepts)", ids.len()),
                    estimate,
                    cost,
                ));

                (estimate, cost)
            }
        }
    }

    /// Extracts the focus concept ID from an expression.
    fn get_focus_concept_id(&self, expr: &EclExpression) -> SctId {
        let expr = expr.unwrap_nested();
        match expr {
            EclExpression::ConceptReference { concept_id, .. } => *concept_id,
            _ => 0, // Unknown, will use default estimates
        }
    }

    /// Generates optimization hints based on query analysis.
    fn generate_hints(&self, plan: &mut QueryPlan, expr: &EclExpression) {
        // Analyze AND expressions for reordering opportunities
        self.analyze_and_ordering(plan, expr);

        // Check for repeated subexpressions (candidates for caching)
        self.analyze_subexpression_reuse(plan, expr);
    }

    /// Analyzes AND expressions to suggest optimal operand ordering.
    fn analyze_and_ordering(&self, plan: &mut QueryPlan, expr: &EclExpression) {
        let expr = expr.unwrap_nested();

        if let EclExpression::And(left, right) = expr {
            let left_estimate = self.estimate_cardinality(left);
            let right_estimate = self.estimate_cardinality(right);

            // If right operand is smaller, suggest reordering
            if right_estimate < left_estimate {
                let savings =
                    ((left_estimate - right_estimate) as f64) * heuristics::AND_SELECTIVITY_FACTOR;
                if savings > 1000.0 {
                    plan.add_hint(format!(
                        "Consider reordering AND operands: right operand ({} est.) is smaller than left ({} est.)",
                        right_estimate, left_estimate
                    ));
                }
            }

            // Recursively analyze nested ANDs
            self.analyze_and_ordering(plan, left);
            self.analyze_and_ordering(plan, right);
        }
    }

    /// Analyzes for repeated subexpressions that could benefit from caching.
    fn analyze_subexpression_reuse(&self, plan: &mut QueryPlan, expr: &EclExpression) {
        // Count occurrences of concept references
        let mut concept_counts = std::collections::HashMap::new();
        self.count_concept_refs(expr, &mut concept_counts);

        // Suggest caching for concepts appearing multiple times
        for (concept_id, count) in concept_counts {
            if count > 1 {
                let estimate = self.statistics.estimated_descendants(concept_id);
                if estimate > 1000 {
                    plan.add_hint(format!(
                        "Concept {} appears {} times - enable intermediate caching for better performance",
                        concept_id, count
                    ));
                }
            }
        }
    }

    /// Counts concept references in an expression.
    #[allow(clippy::only_used_in_recursion)]
    fn count_concept_refs(
        &self,
        expr: &EclExpression,
        counts: &mut std::collections::HashMap<SctId, usize>,
    ) {
        let expr = expr.unwrap_nested();

        match expr {
            EclExpression::ConceptReference { concept_id, .. } => {
                *counts.entry(*concept_id).or_insert(0) += 1;
            }
            EclExpression::DescendantOf(inner)
            | EclExpression::DescendantOrSelfOf(inner)
            | EclExpression::AncestorOf(inner)
            | EclExpression::AncestorOrSelfOf(inner)
            | EclExpression::ChildOf(inner)
            | EclExpression::ChildOrSelfOf(inner)
            | EclExpression::ParentOf(inner)
            | EclExpression::ParentOrSelfOf(inner) => {
                self.count_concept_refs(inner, counts);
            }
            EclExpression::And(left, right)
            | EclExpression::Or(left, right)
            | EclExpression::Minus(left, right) => {
                self.count_concept_refs(left, counts);
                self.count_concept_refs(right, counts);
            }
            EclExpression::MemberOf { .. }
            | EclExpression::Any
            | EclExpression::AlternateIdentifier { .. }
            | EclExpression::Nested(_) => {}

            // Advanced ECL features
            EclExpression::Refined { focus, .. } => {
                self.count_concept_refs(focus, counts);
            }
            EclExpression::DotNotation {
                source,
                attribute_type,
            } => {
                self.count_concept_refs(source, counts);
                self.count_concept_refs(attribute_type, counts);
            }
            EclExpression::Concrete { .. } => {}
            EclExpression::Filtered { expression, .. } => {
                self.count_concept_refs(expression, counts);
            }
            EclExpression::TopOfSet(inner) | EclExpression::BottomOfSet(inner) => {
                self.count_concept_refs(inner, counts);
            }
            EclExpression::ConceptSet(ids) => {
                for id in ids {
                    *counts.entry(*id).or_insert(0) += 1;
                }
            }
        }
    }

    /// Estimates cardinality for an expression without creating a full plan.
    pub fn estimate_cardinality(&self, expr: &EclExpression) -> usize {
        let expr = expr.unwrap_nested();

        match expr {
            EclExpression::ConceptReference { concept_id, .. } => {
                self.statistics.estimated_self(*concept_id)
            }
            EclExpression::DescendantOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                self.statistics.estimated_descendants(concept_id)
            }
            EclExpression::DescendantOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                self.statistics.estimated_descendants(concept_id) + 1
            }
            EclExpression::AncestorOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                self.statistics.estimated_ancestors(concept_id)
            }
            EclExpression::AncestorOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                self.statistics.estimated_ancestors(concept_id) + 1
            }
            EclExpression::ChildOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                self.statistics.estimated_children(concept_id)
            }
            EclExpression::ChildOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                self.statistics.estimated_children(concept_id) + 1
            }
            EclExpression::ParentOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                self.statistics.estimated_parents(concept_id)
            }
            EclExpression::ParentOrSelfOf(inner) => {
                let concept_id = self.get_focus_concept_id(inner);
                self.statistics.estimated_parents(concept_id) + 1
            }
            EclExpression::And(left, right) => {
                let left_est = self.estimate_cardinality(left);
                let right_est = self.estimate_cardinality(right);
                self.statistics.estimated_and(left_est, right_est)
            }
            EclExpression::Or(left, right) => {
                let left_est = self.estimate_cardinality(left);
                let right_est = self.estimate_cardinality(right);
                self.statistics.estimated_or(left_est, right_est)
            }
            EclExpression::Minus(left, right) => {
                let left_est = self.estimate_cardinality(left);
                let right_est = self.estimate_cardinality(right);
                self.statistics.estimated_minus(left_est, right_est)
            }
            EclExpression::MemberOf { .. } => heuristics::DEFAULT_DESCENDANT_ESTIMATE,
            EclExpression::Any => 500_000,
            EclExpression::AlternateIdentifier { .. } => 1,
            EclExpression::Nested(_) => unreachable!("Nested expressions are unwrapped"),

            // Advanced ECL features
            EclExpression::Refined { focus, .. } => {
                let focus_est = self.estimate_cardinality(focus);
                (focus_est as f64 * 0.1).max(1.0) as usize
            }
            EclExpression::DotNotation { source, .. } => self.estimate_cardinality(source),
            EclExpression::Concrete { .. } => 0,
            EclExpression::Filtered { expression, .. } => {
                let inner_est = self.estimate_cardinality(expression);
                (inner_est as f64 * 0.5).max(1.0) as usize
            }
            EclExpression::TopOfSet(inner) => {
                let inner_est = self.estimate_cardinality(inner);
                (inner_est as f64 * 0.1).max(1.0) as usize
            }
            EclExpression::BottomOfSet(inner) => {
                let inner_est = self.estimate_cardinality(inner);
                (inner_est as f64 * 0.3).max(1.0) as usize
            }
            EclExpression::ConceptSet(ids) => ids.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ecl(ecl: &str) -> EclExpression {
        snomed_ecl::parse(ecl).expect("Failed to parse ECL")
    }

    #[test]
    fn test_query_planner_new() {
        let planner = QueryPlanner::new();
        assert!(planner.statistics().estimated_descendants(404684003) > 0);
    }

    #[test]
    fn test_plan_self_constraint() {
        let planner = QueryPlanner::new();
        let expr = parse_ecl("73211009");
        let plan = planner.plan("73211009", &expr);

        assert_eq!(plan.ecl, "73211009");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].operation, "Self");
        assert_eq!(plan.estimated_total, 1);
    }

    #[test]
    fn test_plan_descendant_of() {
        let planner = QueryPlanner::new();
        let expr = parse_ecl("< 404684003");
        let plan = planner.plan("< 404684003", &expr);

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].operation, "Descendants");
        assert!(plan.estimated_total > 0);
        // Clinical finding has large traversal hint
        assert!(plan.has_hints());
    }

    #[test]
    fn test_plan_descendant_or_self() {
        let planner = QueryPlanner::new();
        let expr = parse_ecl("<< 73211009");
        let plan = planner.plan("<< 73211009", &expr);

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].operation, "DescendantsOrSelf");
    }

    #[test]
    fn test_plan_ancestor_of() {
        let planner = QueryPlanner::new();
        let expr = parse_ecl("> 73211009");
        let plan = planner.plan("> 73211009", &expr);

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].operation, "Ancestors");
    }

    #[test]
    fn test_plan_and_constraint() {
        let planner = QueryPlanner::new();
        let expr = parse_ecl("<< 404684003 AND << 123037004");
        let plan = planner.plan("<< 404684003 AND << 123037004", &expr);

        // Two descendant steps + one intersect step
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[0].operation, "DescendantsOrSelf");
        assert_eq!(plan.steps[1].operation, "DescendantsOrSelf");
        assert_eq!(plan.steps[2].operation, "Intersect");
    }

    #[test]
    fn test_plan_or_constraint() {
        let planner = QueryPlanner::new();
        let expr = parse_ecl("<! 100 OR <! 200");
        let plan = planner.plan("<! 100 OR <! 200", &expr);

        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[2].operation, "Union");
    }

    #[test]
    fn test_plan_minus_constraint() {
        let planner = QueryPlanner::new();
        let expr = parse_ecl("<< 100 MINUS << 200");
        let plan = planner.plan("<< 100 MINUS << 200", &expr);

        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[2].operation, "Difference");
    }

    #[test]
    fn test_plan_wildcard() {
        let planner = QueryPlanner::new();
        let expr = parse_ecl("*");
        let plan = planner.plan("*", &expr);

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].operation, "Any");
        assert!(plan.has_hints()); // Should warn about wildcard
    }

    #[test]
    fn test_plan_display() {
        let planner = QueryPlanner::new();
        let expr = parse_ecl("<< 73211009");
        let plan = planner.plan("<< 73211009", &expr);

        let display = format!("{}", plan);
        assert!(display.contains("Query Plan for:"));
        assert!(display.contains("Estimated total:"));
        assert!(display.contains("Steps:"));
    }

    #[test]
    fn test_estimate_cardinality() {
        let planner = QueryPlanner::new();

        // Self constraint
        let expr = parse_ecl("73211009");
        assert_eq!(planner.estimate_cardinality(&expr), 1);

        // AND should be less than min
        let and_expr = parse_ecl("<< 404684003 AND << 123037004");
        let and_est = planner.estimate_cardinality(&and_expr);
        let left_est = planner.estimate_cardinality(&parse_ecl("<< 404684003"));
        let right_est = planner.estimate_cardinality(&parse_ecl("<< 123037004"));
        assert!(and_est <= left_est.min(right_est));
    }

    #[test]
    fn test_child_and_parent_planning() {
        let planner = QueryPlanner::new();

        let child_expr = parse_ecl("<! 73211009");
        let plan = planner.plan("<! 73211009", &child_expr);
        assert_eq!(plan.steps[0].operation, "Children");

        let parent_expr = parse_ecl(">! 73211009");
        let plan = planner.plan(">! 73211009", &parent_expr);
        assert_eq!(plan.steps[0].operation, "Parents");
    }

    #[test]
    fn test_query_step_display() {
        let step = QueryStep::new("Descendants", "< 73211009", 1000, 0.5);
        let display = format!("{}", step);

        assert!(display.contains("Descendants"));
        assert!(display.contains("< 73211009"));
        assert!(display.contains("1000"));
    }

    #[test]
    fn test_total_cost() {
        let planner = QueryPlanner::new();
        let expr = parse_ecl("<< 73211009 AND << 46635009");
        let plan = planner.plan("<< 73211009 AND << 46635009", &expr);

        let total = plan.total_cost();
        let step_sum: f64 = plan.steps.iter().map(|s| s.cost_estimate).sum();
        assert!((total - step_sum).abs() < 0.0001);
    }
}
