//! First-pass constraint solver.
//!
//! Takes a sketch, packs its entities' free variables into a DOF vector,
//! evaluates residuals for every recorded constraint, and runs
//! Levenberg-Marquardt iterations until residuals fall below tolerance or
//! the iteration budget is exhausted. Satisfied state is written back into
//! the sketch's entities.
//!
//! No external linear-algebra dependency - we use forward-difference
//! Jacobians and Gauss elimination with partial pivoting, which is fine for
//! the modest DOF counts typical of sketches.

use std::collections::HashMap;

use glam::DVec2;
use roncad_core::constraint::{Constraint, EntityPoint};
use roncad_core::ids::{ConstraintId, SketchEntityId};

use crate::sketch::Sketch;
use crate::sketch_entity::SketchEntity;

const DEFAULT_MAX_ITERS: usize = 40;
const DEFAULT_TOLERANCE: f64 = 1e-8;
const INITIAL_LAMBDA: f64 = 1e-3;
const JACOBIAN_H: f64 = 1e-7;
const CONSTRAINT_SATISFIED_TOLERANCE: f64 = 1e-4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveStatus {
    /// Constraints are satisfied but the sketch still appears to have free DOFs.
    Underdefined,
    /// Constraints are satisfied and the sketch appears fully constrained.
    Solved,
    /// One or more constraints remain unsatisfied.
    Conflicting,
    /// One or more constraints could not be evaluated.
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintDiagnosticKind {
    Unsatisfied,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ConstraintDiagnostic {
    pub id: ConstraintId,
    pub constraint: Constraint,
    pub kind: ConstraintDiagnosticKind,
    pub residual_norm: f64,
    pub referenced_entities: Vec<SketchEntityId>,
}

#[derive(Debug, Clone)]
pub struct SolveReport {
    pub status: SolveStatus,
    pub iterations: usize,
    pub final_residual_norm: f64,
    pub constraint_count: usize,
    pub unsatisfied_count: usize,
    pub failed_count: usize,
    pub estimated_free_dofs: usize,
    pub diagnostics: Vec<ConstraintDiagnostic>,
}

/// Run the solver on `sketch` and write the result back in place.
pub fn solve_sketch(sketch: &mut Sketch) -> SolveReport {
    solve_sketch_with(sketch, DEFAULT_MAX_ITERS, DEFAULT_TOLERANCE)
}

pub fn solve_sketch_with(sketch: &mut Sketch, max_iters: usize, tol: f64) -> SolveReport {
    let layout = DofLayout::build(sketch);
    let constraints: Vec<(ConstraintId, Constraint)> = sketch
        .constraints
        .iter()
        .map(|(id, constraint)| (id, *constraint))
        .collect();

    if constraints.is_empty() {
        return SolveReport {
            status: if layout.total_dofs > 0 {
                SolveStatus::Underdefined
            } else {
                SolveStatus::Solved
            },
            iterations: 0,
            final_residual_norm: 0.0,
            constraint_count: 0,
            unsatisfied_count: 0,
            failed_count: 0,
            estimated_free_dofs: layout.total_dofs,
            diagnostics: Vec::new(),
        };
    }

    let mut x = pack_dofs(sketch, &layout);
    let mut lambda = INITIAL_LAMBDA;

    let mut evaluations = evaluate_constraints(&x, &layout, &constraints);
    let mut residuals = residual_vector(&evaluations);
    let mut residual_norm = vec_norm(&residuals);
    let mut iter = 0;

    while iter < max_iters && residual_norm > tol {
        let j = numerical_jacobian(&x, &layout, &constraints, &residuals);

        // Normal equations with LM damping: (J^T J + lambda I) dx = -J^T r
        let jt_j = jtj(&j);
        let jt_r = jtv(&j, &residuals);
        let damped = damped_symmetric(&jt_j, lambda);
        let neg_jt_r: Vec<f64> = jt_r.iter().map(|v| -v).collect();

        let Some(dx) = solve_linear_system(damped, neg_jt_r) else {
            lambda *= 10.0;
            iter += 1;
            continue;
        };

        let x_trial: Vec<f64> = x.iter().zip(dx.iter()).map(|(a, b)| a + b).collect();
        let trial_residuals =
            residual_vector(&evaluate_constraints(&x_trial, &layout, &constraints));
        let trial_residual_norm = vec_norm(&trial_residuals);

        if trial_residual_norm < residual_norm {
            x = x_trial;
            residuals = trial_residuals;
            residual_norm = trial_residual_norm;
            lambda = (lambda * 0.5).max(1e-12);
        } else {
            lambda *= 4.0;
        }

        iter += 1;
    }

    unpack_dofs(sketch, &layout, &x);

    evaluations = evaluate_constraints(&x, &layout, &constraints);
    residuals = residual_vector(&evaluations);
    residual_norm = vec_norm(&residuals);

    let diagnostics: Vec<_> = evaluations
        .iter()
        .filter_map(ConstraintEvaluation::to_diagnostic)
        .collect();
    let failed_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.kind == ConstraintDiagnosticKind::Failed)
        .count();
    let unsatisfied_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.kind == ConstraintDiagnosticKind::Unsatisfied)
        .count();
    let estimated_free_dofs = if failed_count > 0 || residuals.is_empty() {
        layout.total_dofs
    } else {
        let j = numerical_jacobian(&x, &layout, &constraints, &residuals);
        layout.total_dofs.saturating_sub(matrix_rank(&j, 1e-7))
    };

    let status = if failed_count > 0 {
        SolveStatus::Failed
    } else if unsatisfied_count > 0 {
        SolveStatus::Conflicting
    } else if estimated_free_dofs > 0 {
        SolveStatus::Underdefined
    } else {
        SolveStatus::Solved
    };

    SolveReport {
        status,
        iterations: iter,
        final_residual_norm: residual_norm,
        constraint_count: constraints.len(),
        unsatisfied_count,
        failed_count,
        estimated_free_dofs,
        diagnostics,
    }
}

// --- DOF layout --------------------------------------------------------

#[derive(Debug, Clone)]
struct DofLayout {
    ranges: HashMap<SketchEntityId, (usize, EntityKind)>,
    total_dofs: usize,
}

#[derive(Debug, Clone, Copy)]
enum EntityKind {
    Point,
    Line,
    Rectangle,
    Circle,
    Arc,
}

impl EntityKind {
    fn from(entity: &SketchEntity) -> Self {
        match entity {
            SketchEntity::Point { .. } => Self::Point,
            SketchEntity::Line { .. } => Self::Line,
            SketchEntity::Rectangle { .. } => Self::Rectangle,
            SketchEntity::Circle { .. } => Self::Circle,
            SketchEntity::Arc { .. } => Self::Arc,
        }
    }

    fn dof_count(self) -> usize {
        match self {
            Self::Point => 2,
            Self::Line => 4,
            Self::Rectangle => 4,
            Self::Circle => 3,
            Self::Arc => 5,
        }
    }
}

impl DofLayout {
    fn build(sketch: &Sketch) -> Self {
        let mut ranges = HashMap::new();
        let mut offset = 0;
        for (id, entity) in sketch.entities.iter() {
            let kind = EntityKind::from(entity);
            ranges.insert(id, (offset, kind));
            offset += kind.dof_count();
        }
        Self {
            ranges,
            total_dofs: offset,
        }
    }
}

fn pack_dofs(sketch: &Sketch, layout: &DofLayout) -> Vec<f64> {
    let mut x = vec![0.0; layout.total_dofs];
    for (id, entity) in sketch.entities.iter() {
        let (off, _) = layout.ranges[&id];
        match entity {
            SketchEntity::Point { p } => {
                x[off] = p.x;
                x[off + 1] = p.y;
            }
            SketchEntity::Line { a, b } => {
                x[off] = a.x;
                x[off + 1] = a.y;
                x[off + 2] = b.x;
                x[off + 3] = b.y;
            }
            SketchEntity::Rectangle { corner_a, corner_b } => {
                x[off] = corner_a.x;
                x[off + 1] = corner_a.y;
                x[off + 2] = corner_b.x;
                x[off + 3] = corner_b.y;
            }
            SketchEntity::Circle { center, radius } => {
                x[off] = center.x;
                x[off + 1] = center.y;
                x[off + 2] = *radius;
            }
            SketchEntity::Arc {
                center,
                radius,
                start_angle,
                sweep_angle,
            } => {
                x[off] = center.x;
                x[off + 1] = center.y;
                x[off + 2] = *radius;
                x[off + 3] = *start_angle;
                x[off + 4] = *sweep_angle;
            }
        }
    }
    x
}

fn unpack_dofs(sketch: &mut Sketch, layout: &DofLayout, x: &[f64]) {
    for (id, entity) in sketch.entities.iter_mut() {
        let (off, _) = layout.ranges[&id];
        match entity {
            SketchEntity::Point { p } => {
                p.x = x[off];
                p.y = x[off + 1];
            }
            SketchEntity::Line { a, b } => {
                a.x = x[off];
                a.y = x[off + 1];
                b.x = x[off + 2];
                b.y = x[off + 3];
            }
            SketchEntity::Rectangle { corner_a, corner_b } => {
                corner_a.x = x[off];
                corner_a.y = x[off + 1];
                corner_b.x = x[off + 2];
                corner_b.y = x[off + 3];
            }
            SketchEntity::Circle { center, radius } => {
                center.x = x[off];
                center.y = x[off + 1];
                *radius = x[off + 2];
            }
            SketchEntity::Arc {
                center,
                radius,
                start_angle,
                sweep_angle,
            } => {
                center.x = x[off];
                center.y = x[off + 1];
                *radius = x[off + 2];
                *start_angle = x[off + 3];
                *sweep_angle = x[off + 4];
            }
        }
    }
}

// --- Residuals ---------------------------------------------------------

fn entity_point(x: &[f64], layout: &DofLayout, handle: EntityPoint) -> Option<DVec2> {
    let (off, kind) = layout.ranges.get(&handle.entity()).copied()?;
    match (handle, kind) {
        (EntityPoint::Point(_), EntityKind::Point) => Some(DVec2::new(x[off], x[off + 1])),
        (EntityPoint::Start(_), EntityKind::Line) => Some(DVec2::new(x[off], x[off + 1])),
        (EntityPoint::End(_), EntityKind::Line) => Some(DVec2::new(x[off + 2], x[off + 3])),
        (EntityPoint::Start(_), EntityKind::Arc) => {
            let center = DVec2::new(x[off], x[off + 1]);
            let radius = x[off + 2];
            let start_angle = x[off + 3];
            Some(center + DVec2::new(start_angle.cos(), start_angle.sin()) * radius)
        }
        (EntityPoint::End(_), EntityKind::Arc) => {
            let center = DVec2::new(x[off], x[off + 1]);
            let radius = x[off + 2];
            let start_angle = x[off + 3];
            let sweep_angle = x[off + 4];
            let end_angle = start_angle + sweep_angle;
            Some(center + DVec2::new(end_angle.cos(), end_angle.sin()) * radius)
        }
        (EntityPoint::Center(_), EntityKind::Circle | EntityKind::Arc) => {
            Some(DVec2::new(x[off], x[off + 1]))
        }
        (EntityPoint::CornerA(_), EntityKind::Rectangle) => Some(DVec2::new(x[off], x[off + 1])),
        (EntityPoint::CornerB(_), EntityKind::Rectangle) => {
            Some(DVec2::new(x[off + 2], x[off + 1]))
        }
        (EntityPoint::CornerC(_), EntityKind::Rectangle) => {
            Some(DVec2::new(x[off + 2], x[off + 3]))
        }
        (EntityPoint::CornerD(_), EntityKind::Rectangle) => Some(DVec2::new(x[off], x[off + 3])),
        _ => None,
    }
}

fn line_points(x: &[f64], layout: &DofLayout, id: SketchEntityId) -> Option<(DVec2, DVec2)> {
    let (off, kind) = layout.ranges.get(&id).copied()?;
    match kind {
        EntityKind::Line => Some((
            DVec2::new(x[off], x[off + 1]),
            DVec2::new(x[off + 2], x[off + 3]),
        )),
        _ => None,
    }
}

fn curve_center_radius(x: &[f64], layout: &DofLayout, id: SketchEntityId) -> Option<(DVec2, f64)> {
    let (off, kind) = layout.ranges.get(&id).copied()?;
    match kind {
        EntityKind::Circle | EntityKind::Arc => Some((DVec2::new(x[off], x[off + 1]), x[off + 2])),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct ConstraintEvaluation {
    id: ConstraintId,
    constraint: Constraint,
    residuals: Vec<f64>,
    residual_norm: f64,
    valid: bool,
}

impl ConstraintEvaluation {
    fn diagnostic_kind(&self) -> Option<ConstraintDiagnosticKind> {
        if !self.valid {
            Some(ConstraintDiagnosticKind::Failed)
        } else if self.residual_norm > CONSTRAINT_SATISFIED_TOLERANCE {
            Some(ConstraintDiagnosticKind::Unsatisfied)
        } else {
            None
        }
    }

    fn to_diagnostic(&self) -> Option<ConstraintDiagnostic> {
        Some(ConstraintDiagnostic {
            id: self.id,
            constraint: self.constraint,
            kind: self.diagnostic_kind()?,
            residual_norm: self.residual_norm,
            referenced_entities: self.constraint.referenced_entities(),
        })
    }
}

fn evaluate_constraints(
    x: &[f64],
    layout: &DofLayout,
    constraints: &[(ConstraintId, Constraint)],
) -> Vec<ConstraintEvaluation> {
    constraints
        .iter()
        .map(|(id, constraint)| evaluate_constraint(x, layout, *id, constraint))
        .collect()
}

fn residual_vector(evaluations: &[ConstraintEvaluation]) -> Vec<f64> {
    evaluations
        .iter()
        .flat_map(|evaluation| evaluation.residuals.iter().copied())
        .collect()
}

fn evaluate_constraint(
    x: &[f64],
    layout: &DofLayout,
    id: ConstraintId,
    constraint: &Constraint,
) -> ConstraintEvaluation {
    let (residuals, valid) = match *constraint {
        Constraint::Coincident { a, b } => {
            if let (Some(pa), Some(pb)) = (entity_point(x, layout, a), entity_point(x, layout, b)) {
                (vec![pa.x - pb.x, pa.y - pb.y], true)
            } else {
                (Vec::new(), false)
            }
        }
        Constraint::FixPoint { point, target } => {
            if let Some(actual) = entity_point(x, layout, point) {
                (vec![actual.x - target.x, actual.y - target.y], true)
            } else {
                (Vec::new(), false)
            }
        }
        Constraint::PointOnEntity { point, entity } => {
            if let Some(point) = entity_point(x, layout, point) {
                if let Some((line_a, line_b)) = line_points(x, layout, entity) {
                    let delta = line_b - line_a;
                    let query = point - line_a;
                    (vec![delta.x * query.y - delta.y * query.x], true)
                } else if let Some((center, radius)) = curve_center_radius(x, layout, entity) {
                    (vec![(point - center).length() - radius], true)
                } else {
                    (Vec::new(), false)
                }
            } else {
                (Vec::new(), false)
            }
        }
        Constraint::Horizontal { entity } => {
            if let Some((a, b)) = line_points(x, layout, entity) {
                (vec![a.y - b.y], true)
            } else {
                (Vec::new(), false)
            }
        }
        Constraint::Vertical { entity } => {
            if let Some((a, b)) = line_points(x, layout, entity) {
                (vec![a.x - b.x], true)
            } else {
                (Vec::new(), false)
            }
        }
        Constraint::Parallel { a, b } => {
            if let (Some((a1, a2)), Some((b1, b2))) =
                (line_points(x, layout, a), line_points(x, layout, b))
            {
                let da = a2 - a1;
                let db = b2 - b1;
                (vec![da.x * db.y - da.y * db.x], true)
            } else {
                (Vec::new(), false)
            }
        }
        Constraint::Perpendicular { a, b } => {
            if let (Some((a1, a2)), Some((b1, b2))) =
                (line_points(x, layout, a), line_points(x, layout, b))
            {
                let da = a2 - a1;
                let db = b2 - b1;
                (vec![da.x * db.x + da.y * db.y], true)
            } else {
                (Vec::new(), false)
            }
        }
        Constraint::Tangent { line, curve } => {
            if let (Some((line_a, line_b)), Some((center, radius))) = (
                line_points(x, layout, line),
                curve_center_radius(x, layout, curve),
            ) {
                let delta = line_b - line_a;
                let query = center - line_a;
                let cross = delta.x * query.y - delta.y * query.x;
                let len_sq = delta.length_squared();
                (vec![cross * cross - radius * radius * len_sq], true)
            } else {
                (Vec::new(), false)
            }
        }
        Constraint::EqualLength { a, b } => {
            if let (Some((a1, a2)), Some((b1, b2))) =
                (line_points(x, layout, a), line_points(x, layout, b))
            {
                (
                    vec![(a2 - a1).length_squared() - (b2 - b1).length_squared()],
                    true,
                )
            } else {
                (Vec::new(), false)
            }
        }
        Constraint::EqualRadius { a, b } => {
            if let (Some((_, ra)), Some((_, rb))) = (
                curve_center_radius(x, layout, a),
                curve_center_radius(x, layout, b),
            ) {
                (vec![ra - rb], true)
            } else {
                (Vec::new(), false)
            }
        }
    };

    ConstraintEvaluation {
        id,
        constraint: *constraint,
        residual_norm: vec_norm(&residuals),
        residuals,
        valid,
    }
}

// --- Jacobian & linear algebra -----------------------------------------

fn numerical_jacobian(
    x: &[f64],
    layout: &DofLayout,
    constraints: &[(ConstraintId, Constraint)],
    r0: &[f64],
) -> Matrix {
    let m = r0.len();
    let n = x.len();
    let mut j = Matrix::zeros(m, n);
    let mut x_pert = x.to_vec();
    for k in 0..n {
        let original = x_pert[k];
        let h = JACOBIAN_H.max(original.abs() * JACOBIAN_H);
        x_pert[k] = original + h;
        let r_pert = residual_vector(&evaluate_constraints(&x_pert, layout, constraints));
        x_pert[k] = original;
        if r_pert.len() != m {
            continue;
        }
        for i in 0..m {
            j[(i, k)] = (r_pert[i] - r0[i]) / h;
        }
    }
    j
}

#[derive(Debug, Clone)]
struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

impl Matrix {
    fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }
}

impl std::ops::Index<(usize, usize)> for Matrix {
    type Output = f64;

    fn index(&self, (r, c): (usize, usize)) -> &f64 {
        &self.data[r * self.cols + c]
    }
}

impl std::ops::IndexMut<(usize, usize)> for Matrix {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut f64 {
        &mut self.data[r * self.cols + c]
    }
}

fn jtj(j: &Matrix) -> Matrix {
    let n = j.cols;
    let mut out = Matrix::zeros(n, n);
    for r in 0..n {
        for c in 0..n {
            let mut sum = 0.0;
            for k in 0..j.rows {
                sum += j[(k, r)] * j[(k, c)];
            }
            out[(r, c)] = sum;
        }
    }
    out
}

fn jtv(j: &Matrix, v: &[f64]) -> Vec<f64> {
    let n = j.cols;
    let mut out = vec![0.0; n];
    for c in 0..n {
        let mut sum = 0.0;
        for r in 0..j.rows {
            sum += j[(r, c)] * v[r];
        }
        out[c] = sum;
    }
    out
}

fn damped_symmetric(m: &Matrix, lambda: f64) -> Matrix {
    let mut out = m.clone();
    for i in 0..out.rows {
        out[(i, i)] += lambda;
    }
    out
}

fn solve_linear_system(mut a: Matrix, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = a.rows;
    if a.cols != n || b.len() != n {
        return None;
    }

    for i in 0..n {
        let mut pivot_row = i;
        let mut pivot_val = a[(i, i)].abs();
        for r in (i + 1)..n {
            let value = a[(r, i)].abs();
            if value > pivot_val {
                pivot_val = value;
                pivot_row = r;
            }
        }
        if pivot_val < 1e-18 {
            return None;
        }

        if pivot_row != i {
            for c in 0..n {
                let tmp = a[(i, c)];
                a[(i, c)] = a[(pivot_row, c)];
                a[(pivot_row, c)] = tmp;
            }
            b.swap(i, pivot_row);
        }

        for r in (i + 1)..n {
            let factor = a[(r, i)] / a[(i, i)];
            for c in i..n {
                a[(r, c)] -= factor * a[(i, c)];
            }
            b[r] -= factor * b[i];
        }
    }

    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = b[i];
        for c in (i + 1)..n {
            sum -= a[(i, c)] * x[c];
        }
        x[i] = sum / a[(i, i)];
    }

    Some(x)
}

fn matrix_rank(m: &Matrix, tol: f64) -> usize {
    let mut a = m.clone();
    let mut rank = 0;
    let scale = a
        .data
        .iter()
        .copied()
        .map(f64::abs)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let threshold = tol * scale;
    let mut row = 0;

    for col in 0..a.cols {
        if row >= a.rows {
            break;
        }

        let mut pivot_row = row;
        let mut pivot_val = a[(row, col)].abs();
        for candidate in (row + 1)..a.rows {
            let value = a[(candidate, col)].abs();
            if value > pivot_val {
                pivot_val = value;
                pivot_row = candidate;
            }
        }
        if pivot_val <= threshold {
            continue;
        }

        if pivot_row != row {
            for c in 0..a.cols {
                let tmp = a[(row, c)];
                a[(row, c)] = a[(pivot_row, c)];
                a[(pivot_row, c)] = tmp;
            }
        }

        for candidate in (row + 1)..a.rows {
            let factor = a[(candidate, col)] / a[(row, col)];
            if factor.abs() <= threshold {
                continue;
            }
            for c in col..a.cols {
                a[(candidate, c)] -= factor * a[(row, c)];
            }
        }

        rank += 1;
        row += 1;
    }

    rank
}

fn vec_norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

// --- Tests -------------------------------------------------------------

#[cfg(test)]
mod tests {
    use glam::dvec2;

    use super::*;
    use crate::Sketch;

    fn new_sketch() -> Sketch {
        Sketch::new("S", slotmap::KeyData::default().into())
    }

    fn line(a: DVec2, b: DVec2) -> SketchEntity {
        SketchEntity::Line { a, b }
    }

    #[test]
    fn solver_on_already_horizontal_line_is_underdefined() {
        let mut sketch = new_sketch();
        let id = sketch.add(line(dvec2(0.0, 2.0), dvec2(10.0, 2.0)));
        sketch.add_constraint(Constraint::Horizontal { entity: id });

        let report = solve_sketch(&mut sketch);

        assert_eq!(report.status, SolveStatus::Underdefined);
        let SketchEntity::Line { a, b } = *sketch.entities.get(id).unwrap() else {
            panic!()
        };
        assert!((a.y - b.y).abs() < 1e-6);
    }

    #[test]
    fn solver_pulls_diagonal_line_horizontal_when_constrained() {
        let mut sketch = new_sketch();
        let id = sketch.add(line(dvec2(0.0, 0.0), dvec2(10.0, 5.0)));
        sketch.add_constraint(Constraint::Horizontal { entity: id });

        let report = solve_sketch(&mut sketch);

        assert_eq!(report.status, SolveStatus::Underdefined);
        let SketchEntity::Line { a, b } = *sketch.entities.get(id).unwrap() else {
            panic!()
        };
        assert!((a.y - b.y).abs() < 1e-6, "y mismatch: a={} b={}", a.y, b.y);
    }

    #[test]
    fn solver_enforces_coincident_endpoints() {
        let mut sketch = new_sketch();
        let l1 = sketch.add(line(dvec2(0.0, 0.0), dvec2(10.0, 0.0)));
        let l2 = sketch.add(line(dvec2(10.1, 0.2), dvec2(20.0, 5.0)));
        sketch.add_constraint(Constraint::Coincident {
            a: EntityPoint::End(l1),
            b: EntityPoint::Start(l2),
        });

        solve_sketch(&mut sketch);

        let SketchEntity::Line { b: end_l1, .. } = *sketch.entities.get(l1).unwrap() else {
            panic!()
        };
        let SketchEntity::Line { a: start_l2, .. } = *sketch.entities.get(l2).unwrap() else {
            panic!()
        };
        assert!(
            end_l1.distance(start_l2) < 1e-6,
            "endpoints should coincide, got {end_l1:?} vs {start_l2:?}"
        );
    }

    #[test]
    fn solver_makes_perpendicular() {
        let mut sketch = new_sketch();
        let l1 = sketch.add(line(dvec2(0.0, 0.0), dvec2(10.0, 0.0)));
        let l2 = sketch.add(line(dvec2(0.0, 0.0), dvec2(0.5, 10.0)));
        sketch.add_constraint(Constraint::Perpendicular { a: l1, b: l2 });

        solve_sketch(&mut sketch);

        let SketchEntity::Line { a: a1, b: b1 } = *sketch.entities.get(l1).unwrap() else {
            panic!()
        };
        let SketchEntity::Line { a: a2, b: b2 } = *sketch.entities.get(l2).unwrap() else {
            panic!()
        };
        let d1 = b1 - a1;
        let d2 = b2 - a2;
        assert!(
            d1.dot(d2).abs() < 1e-4,
            "dot product should be ~0, got {}",
            d1.dot(d2)
        );
    }

    #[test]
    fn solver_equalizes_line_lengths() {
        let mut sketch = new_sketch();
        let l1 = sketch.add(line(dvec2(0.0, 0.0), dvec2(10.0, 0.0)));
        let l2 = sketch.add(line(dvec2(0.0, 5.0), dvec2(8.0, 5.0)));
        sketch.add_constraint(Constraint::EqualLength { a: l1, b: l2 });

        solve_sketch(&mut sketch);

        let SketchEntity::Line { a: a1, b: b1 } = *sketch.entities.get(l1).unwrap() else {
            panic!()
        };
        let SketchEntity::Line { a: a2, b: b2 } = *sketch.entities.get(l2).unwrap() else {
            panic!()
        };
        let len1 = (b1 - a1).length();
        let len2 = (b2 - a2).length();
        assert!((len1 - len2).abs() < 1e-4, "{len1} vs {len2}");
    }

    #[test]
    fn solver_equalizes_circle_radii() {
        let mut sketch = new_sketch();
        let c1 = sketch.add(SketchEntity::Circle {
            center: dvec2(0.0, 0.0),
            radius: 3.0,
        });
        let c2 = sketch.add(SketchEntity::Circle {
            center: dvec2(10.0, 0.0),
            radius: 5.0,
        });
        sketch.add_constraint(Constraint::EqualRadius { a: c1, b: c2 });

        solve_sketch(&mut sketch);

        let SketchEntity::Circle { radius: r1, .. } = *sketch.entities.get(c1).unwrap() else {
            panic!()
        };
        let SketchEntity::Circle { radius: r2, .. } = *sketch.entities.get(c2).unwrap() else {
            panic!()
        };
        assert!((r1 - r2).abs() < 1e-6);
    }

    #[test]
    fn solver_returns_underdefined_when_no_constraints() {
        let mut sketch = new_sketch();
        sketch.add(line(dvec2(0.0, 0.0), dvec2(10.0, 5.0)));

        let report = solve_sketch(&mut sketch);

        assert_eq!(report.status, SolveStatus::Underdefined);
    }

    #[test]
    fn fix_point_fully_constrains_single_point() {
        let mut sketch = new_sketch();
        let point = sketch.add(SketchEntity::Point {
            p: dvec2(3.0, -1.0),
        });
        sketch.add_constraint(Constraint::FixPoint {
            point: EntityPoint::Point(point),
            target: dvec2(3.0, -1.0),
        });

        let report = solve_sketch(&mut sketch);

        assert_eq!(report.status, SolveStatus::Solved);
        assert_eq!(report.estimated_free_dofs, 0);
    }

    #[test]
    fn conflicting_constraints_are_reported() {
        let mut sketch = new_sketch();
        let id = sketch.add(line(dvec2(0.0, 0.0), dvec2(10.0, 5.0)));
        sketch.add_constraint(Constraint::Horizontal { entity: id });

        let report = solve_sketch_with(&mut sketch, 0, DEFAULT_TOLERANCE);

        assert_eq!(report.status, SolveStatus::Conflicting);
        assert!(report.unsatisfied_count >= 1);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == ConstraintDiagnosticKind::Unsatisfied));
    }

    #[test]
    fn invalid_constraint_is_reported_as_failed() {
        let mut sketch = new_sketch();
        let circle = sketch.add(SketchEntity::Circle {
            center: dvec2(0.0, 0.0),
            radius: 3.0,
        });
        sketch.add_constraint(Constraint::Horizontal { entity: circle });

        let report = solve_sketch(&mut sketch);

        assert_eq!(report.status, SolveStatus::Failed);
        assert_eq!(report.failed_count, 1);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == ConstraintDiagnosticKind::Failed));
    }

    #[test]
    fn solver_point_on_line_pulls_arc_center_onto_line() {
        let mut sketch = new_sketch();
        let l = sketch.add(line(dvec2(0.0, 0.0), dvec2(10.0, 0.0)));
        let c = sketch.add(SketchEntity::Circle {
            center: dvec2(3.0, 1.0),
            radius: 1.0,
        });
        sketch.add_constraint(Constraint::PointOnEntity {
            point: EntityPoint::Center(c),
            entity: l,
        });

        solve_sketch(&mut sketch);

        let SketchEntity::Line { a, b } = *sketch.entities.get(l).unwrap() else {
            panic!()
        };
        let SketchEntity::Circle { center, .. } = *sketch.entities.get(c).unwrap() else {
            panic!()
        };
        let d = b - a;
        let q = center - a;
        let cross = d.x * q.y - d.y * q.x;
        assert!(
            cross.abs() < 1e-4,
            "center should lie on line, cross={cross}"
        );
    }
}
