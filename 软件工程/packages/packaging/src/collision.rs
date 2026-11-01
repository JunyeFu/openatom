//! Conservative continuous checks. Approximate axes and intersection points are
//! proposals only; every certificate/witness is checked with outward intervals.
use crate::{
    PackagingProgress,
    exact::{self, Cell, R, enc, n, q},
    kinematics::*,
    numeric::Iv,
};
use num_traits::{Signed, Zero};
use sf_domain::*;
use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) struct Tracker<'a> {
    pub limits: &'a CalculationBudget,
    pub usage: BudgetUsage,
    pub cancel: &'a AtomicBool,
    pub progress: &'a mut dyn FnMut(PackagingProgress),
}
impl<'a> Tracker<'a> {
    pub fn new(
        limits: &'a CalculationBudget,
        cancel: &'a AtomicBool,
        progress: &'a mut dyn FnMut(PackagingProgress),
    ) -> Self {
        Self {
            limits,
            usage: BudgetUsage {
                candidates_evaluated: uint(0),
                interval_queries: uint(0),
                subdivisions: uint(0),
                stopped_by: None,
            },
            cancel,
            progress,
        }
    }
    pub fn alive(&mut self) -> bool {
        if self.cancel.load(Ordering::Relaxed) {
            self.usage.stopped_by = Some("cancelled".into());
            false
        } else {
            true
        }
    }
    pub fn phase(&mut self, phase: impl Into<String>) {
        (self.progress)(PackagingProgress {
            phase: phase.into(),
            budget_used: self.usage.clone(),
        });
    }
    pub fn candidate(&mut self) -> bool {
        if !self.alive() {
            return false;
        }
        if self.usage.candidates_evaluated.value() >= self.limits.candidate_limit.value() {
            self.usage.stopped_by = Some("candidate_limit".into());
            return false;
        }
        self.usage.candidates_evaluated = uint(self.usage.candidates_evaluated.value() + 1);
        true
    }
    pub fn query(&mut self) -> bool {
        if !self.alive() {
            return false;
        }
        if self.usage.interval_queries.value() >= self.limits.interval_query_limit.value() {
            self.usage.stopped_by = Some("interval_query_limit".into());
            return false;
        }
        self.usage.interval_queries = uint(self.usage.interval_queries.value() + 1);
        if self.usage.interval_queries.value() % 64 == 0 {
            self.phase("continuous_intervals");
        }
        true
    }
    pub fn subdivide(&mut self) -> bool {
        if !self.alive() {
            return false;
        }
        if self.usage.subdivisions.value() >= self.limits.subdivision_limit.value() {
            self.usage.stopped_by = Some("subdivision_limit".into());
            return false;
        }
        self.usage.subdivisions = uint(self.usage.subdivisions.value() + 1);
        true
    }
}
#[derive(Clone)]
pub(crate) struct Solid {
    pub vertices: Vec<V>,
    pub planes: Vec<(V, Iv)>,
    pub edges: Vec<V>,
    pub labels: BTreeSet<usize>,
}
impl Solid {
    pub fn cell(c: &Cell) -> Self {
        let vertices = c
            .polygon
            .iter()
            .flat_map(|p| [c.lo.clone(), c.hi.clone()].map(|z| [enc(&p[0]), enc(&p[1]), enc(&z)]))
            .collect();
        let mut planes = vec![];
        let mut edges = vec![[Iv::ZERO, Iv::ZERO, Iv::ONE]];
        for i in 0..c.polygon.len() {
            let (a, b) = (&c.polygon[i], &c.polygon[(i + 1) % c.polygon.len()]);
            let dx = &b[0] - &a[0];
            let dy = &b[1] - &a[1];
            let normal = [enc(&(-&dy)), enc(&dx), Iv::ZERO];
            planes.push((normal, enc(&(-&dy * &a[0] + &dx * &a[1]))));
            edges.push([enc(&dx), enc(&dy), Iv::ZERO]);
        }
        planes.extend([
            ([Iv::ZERO, Iv::ZERO, Iv::ONE], enc(&c.lo)),
            ([Iv::ZERO, Iv::ZERO, -Iv::ONE], enc(&(-&c.hi))),
        ]);
        Self {
            vertices,
            planes,
            edges,
            labels: c.labels.clone(),
        }
    }
    pub fn hull(vertices: &[Vec3], tr: &mut Tracker<'_>) -> Result<Self, &'static str> {
        let mut p = Vec::with_capacity(vertices.len());
        let mut distinct = BTreeSet::new();
        for vertex in vertices {
            if !tr.alive() {
                return Err("cancelled");
            }
            let point = [
                q(vertex.x.value()),
                q(vertex.y.value()),
                q(vertex.z.value()),
            ];
            distinct.insert(point.clone());
            p.push(point);
        }
        if distinct.len() < 4 {
            return Err("degenerate_product_hull");
        }
        fn sub(a: &[R; 3], b: &[R; 3]) -> [R; 3] {
            std::array::from_fn(|i| &a[i] - &b[i])
        }
        fn cr(a: &[R; 3], b: &[R; 3]) -> [R; 3] {
            [
                &a[1] * &b[2] - &a[2] * &b[1],
                &a[2] * &b[0] - &a[0] * &b[2],
                &a[0] * &b[1] - &a[1] * &b[0],
            ]
        }
        fn dt(a: &[R; 3], b: &[R; 3]) -> R {
            (0..3).map(|i| &a[i] * &b[i]).sum()
        }
        let mut planes = BTreeSet::new();
        let mut volume = false;
        let mut support_reported = false;
        for i in 0..p.len() {
            if !tr.alive() {
                return Err("cancelled");
            }
            for j in i + 1..p.len() {
                if !tr.alive() {
                    return Err("cancelled");
                }
                for k in j + 1..p.len() {
                    if !tr.alive() {
                        return Err("cancelled");
                    }
                    let mut normal = cr(&sub(&p[j], &p[i]), &sub(&p[k], &p[i]));
                    let Some(first) = normal.iter().find(|x| !x.is_zero()).map(R::abs) else {
                        continue;
                    };
                    let mut d = dt(&normal, &p[i]);
                    let (mut pos, mut neg) = (false, false);
                    for vertex in &p {
                        if !tr.alive() {
                            return Err("cancelled");
                        }
                        let sign = dt(&normal, vertex) - &d;
                        pos |= sign > n(0);
                        neg |= sign < n(0);
                        if !support_reported {
                            support_reported = true;
                            tr.phase("product_hull_support_vertices");
                        }
                        if !tr.alive() {
                            return Err("cancelled");
                        }
                        if pos && neg {
                            break;
                        }
                    }
                    if pos && neg {
                        continue;
                    }
                    if !pos && !neg {
                        continue;
                    }
                    volume = true;
                    if neg {
                        normal = normal.map(|v| -v);
                        d = -d;
                    }
                    normal = normal.map(|v| v / &first);
                    d /= &first;
                    planes.insert((normal, d));
                }
            }
        }
        if !volume {
            return Err("degenerate_product_hull");
        }
        let mut edges = vec![];
        for i in 0..p.len() {
            if !tr.alive() {
                return Err("cancelled");
            }
            for j in i + 1..p.len() {
                if !tr.alive() {
                    return Err("cancelled");
                }
                edges.push(std::array::from_fn(|k| enc(&(&p[j][k] - &p[i][k]))));
            }
        }
        let mut output_vertices = Vec::with_capacity(p.len());
        for vertex in p {
            if !tr.alive() {
                return Err("cancelled");
            }
            output_vertices.push(std::array::from_fn(|i| enc(&vertex[i])));
        }
        let mut output_planes = Vec::with_capacity(planes.len());
        for (normal, distance) in planes {
            if !tr.alive() {
                return Err("cancelled");
            }
            output_planes.push((normal.map(|x| enc(&x)), enc(&distance)));
        }
        Ok(Self {
            vertices: output_vertices,
            planes: output_planes,
            edges,
            labels: BTreeSet::new(),
        })
    }
    fn vertices_at(&self, t: Tf, tr: &mut Tracker<'_>) -> Option<Vec<V>> {
        let mut out = Vec::with_capacity(self.vertices.len());
        for v in &self.vertices {
            if !tr.alive() {
                return None;
            }
            let w = t.point(*v);
            if w.iter().any(|x| !x.valid()) {
                return None;
            }
            out.push(w);
        }
        Some(out)
    }
}
#[derive(Clone)]
pub(crate) struct PairResult {
    pub outcome: CheckOutcome,
    pub interval: [f64; 2],
    pub time: Option<f64>,
    pub point: Option<[f64; 3]>,
    pub reason: String,
}
impl PairResult {
    pub fn pass() -> Self {
        Self {
            outcome: CheckOutcome::Pass,
            interval: [0.0, 1.0],
            time: None,
            point: None,
            reason: "all_closed_intervals_separated_or_same_local_allowance".into(),
        }
    }
    pub fn unknown(interval: [f64; 2], reason: impl Into<String>) -> Self {
        Self {
            outcome: CheckOutcome::Undetermined,
            interval,
            time: None,
            point: None,
            reason: reason.into(),
        }
    }
}
fn mid(v: V) -> [f64; 3] {
    v.map(Iv::midpoint)
}
fn f_dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn f_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn axes(a: &Solid, b: &Solid, ta: Tf, tb: Tf, tr: &mut Tracker<'_>) -> Option<Vec<[f64; 3]>> {
    if !tr.alive() {
        return None;
    }
    let mut set = BTreeSet::new();
    let mut out = vec![];
    let mut add = |v: [f64; 3]| {
        let s = v.into_iter().map(f64::abs).fold(0.0, f64::max);
        if !s.is_finite() || s == 0.0 {
            return;
        }
        let mut w = v.map(|x| x / s);
        if w.iter().find(|&&x| x != 0.0).is_some_and(|&x| x < 0.0) {
            w = w.map(|x| -x);
        }
        let key = w.map(|x| if x == 0.0 { 0 } else { x.to_bits() });
        if set.insert(key) {
            out.push(w);
        }
    };
    add([1.0, 0.0, 0.0]);
    add([0.0, 1.0, 0.0]);
    add([0.0, 0.0, 1.0]);
    for (s, t) in [(a, ta), (b, tb)] {
        for (p, _) in &s.planes {
            if !tr.alive() {
                return None;
            }
            add(mid(t.direction(*p)));
        }
    }
    for (i, ea) in a.edges.iter().enumerate() {
        if !tr.alive() {
            return None;
        }
        for (j, eb) in b.edges.iter().enumerate() {
            if !tr.alive() {
                return None;
            }
            add(f_cross(mid(ta.direction(*ea)), mid(tb.direction(*eb))));
            if i == 0 && j == 0 {
                tr.phase("separating_axes_edge_pairs");
            }
            if !tr.alive() {
                return None;
            }
        }
    }
    Some(out)
}
fn separated(
    a: &Solid,
    b: &Solid,
    ta: Tf,
    tb: Tf,
    ma: Tf,
    mb: Tf,
    clearance: f64,
    deadband: f64,
    tr: &mut Tracker<'_>,
) -> Option<bool> {
    let va = a.vertices_at(ta, tr)?;
    let vb = b.vertices_at(tb, tr)?;
    for axis in axes(a, b, ma, mb, tr)? {
        if !tr.alive() {
            return None;
        }
        let v = axis.map(Iv::point);
        let norm = axis
            .into_iter()
            .fold(Iv::ZERO, |s, x| s + Iv::point(x.abs()));
        let (mut alo, mut ahi, mut blo, mut bhi) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for p in &va {
            if !tr.alive() {
                return None;
            }
            let z = dot(*p, v);
            if !z.valid() {
                return None;
            }
            alo = alo.min(z.lo);
            ahi = ahi.max(z.hi);
        }
        for p in &vb {
            if !tr.alive() {
                return None;
            }
            let z = dot(*p, v);
            if !z.valid() {
                return None;
            }
            blo = blo.min(z.lo);
            bhi = bhi.max(z.hi);
        }
        let gap = (Iv::point(blo) - Iv::point(ahi))
            .lo
            .max((Iv::point(alo) - Iv::point(bhi)).lo);
        let margin = (Iv::point(clearance) + Iv::point(deadband)) * norm;
        if margin.valid() && gap > margin.hi {
            return Some(true);
        }
    }
    Some(false)
}
fn contains(s: &Solid, t: Tf, p: [f64; 3], eps: f64) -> bool {
    let local = t.inverse().point(p.map(Iv::point));
    s.planes.iter().all(|(normal, d)| {
        let v = dot(*normal, local) - *d;
        let norm = normal
            .iter()
            .fold(Iv::ZERO, |sum, x| sum + Iv::point(x.abs_upper()));
        v.valid() && v.lo > (norm * Iv::point(eps)).hi
    })
}
fn witness(
    a: &Solid,
    b: &Solid,
    ta: Tf,
    tb: Tf,
    eps: f64,
    tr: &mut Tracker<'_>,
) -> Option<[f64; 3]> {
    let va = a.vertices_at(ta, tr)?;
    let vb = b.vertices_at(tb, tr)?;
    let mut candidates = vec![];
    let mut boxpoint = [0.0; 3];
    for k in 0..3 {
        let lo = va
            .iter()
            .map(|p| p[k].lo)
            .fold(f64::INFINITY, f64::min)
            .max(vb.iter().map(|p| p[k].lo).fold(f64::INFINITY, f64::min));
        let hi = va
            .iter()
            .map(|p| p[k].hi)
            .fold(f64::NEG_INFINITY, f64::max)
            .min(vb.iter().map(|p| p[k].hi).fold(f64::NEG_INFINITY, f64::max));
        if lo >= hi {
            return None;
        }
        boxpoint[k] = lo / 2.0 + hi / 2.0;
    }
    candidates.push(boxpoint);
    for vs in [&va, &vb] {
        candidates.push(std::array::from_fn(|k| {
            vs.iter().map(|v| v[k].midpoint() / vs.len() as f64).sum()
        }));
    }
    let valid = |p: [f64; 3]| {
        p.into_iter().all(f64::is_finite) && contains(a, ta, p, eps) && contains(b, tb, p, eps)
    };
    if let Some(p) = candidates.into_iter().find(|&p| valid(p)) {
        return Some(p);
    }
    let mut planes = vec![];
    for (s, t) in [(a, ta), (b, tb)] {
        for (normal, d) in &s.planes {
            let v = mid(t.direction(*normal));
            planes.push((v, d.midpoint() + f_dot(v, mid(t.t))));
        }
    }
    let mut intersections = vec![];
    for i in 0..planes.len() {
        for j in i + 1..planes.len() {
            for k in j + 1..planes.len() {
                if !tr.alive() {
                    return None;
                }
                let (a, da) = planes[i];
                let (b, db) = planes[j];
                let (c, dc) = planes[k];
                let (x, y, z) = (f_cross(b, c), f_cross(c, a), f_cross(a, b));
                let det = f_dot(a, x);
                if det == 0.0 || !det.is_finite() {
                    continue;
                }
                let p = std::array::from_fn(|i| (da * x[i] + db * y[i] + dc * z[i]) / det);
                if p.iter().all(|x| x.is_finite())
                    && planes.iter().all(|(normal, d)| f_dot(*normal, p) >= *d)
                {
                    intersections.push(p);
                }
            }
        }
    }
    if intersections.is_empty() {
        return None;
    }
    let p = std::array::from_fn(|k| {
        intersections
            .iter()
            .map(|v| v[k] / intersections.len() as f64)
            .sum()
    });
    valid(p).then_some(p)
}

fn clearance_witness(
    a: &Solid,
    b: &Solid,
    ta: Tf,
    tb: Tf,
    clearance: f64,
    eps: f64,
    tr: &mut Tracker<'_>,
) -> Option<([f64; 3], [f64; 3])> {
    if clearance <= eps {
        return None;
    }
    let centre = |s: &Solid| -> [f64; 3] {
        std::array::from_fn(|k| {
            s.vertices
                .iter()
                .map(|v| v[k].midpoint() / s.vertices.len() as f64)
                .sum()
        })
    };
    let inward = |s: &Solid, p: [f64; 3], c: [f64; 3]| {
        let mut t: f64 = 0.0;
        for (normal, offset) in &s.planes {
            let normal = mid(*normal);
            let margin = f_dot(normal, p) - offset.midpoint();
            let centre_margin = f_dot(normal, c) - offset.midpoint();
            let target = eps * 4.0 * normal.into_iter().map(f64::abs).sum::<f64>();
            if margin < target {
                let difference = centre_margin - margin;
                t = t.max(if difference > 0.0 {
                    (target - margin) / difference
                } else {
                    1.0
                });
            }
        }
        let t = t.min(1.0);
        std::array::from_fn(|k| p[k] + (c[k] - p[k]) * t)
    };
    for (s0, s1, t0, t1) in [(a, b, ta, tb), (b, a, tb, ta)] {
        let (c0, c1) = (centre(s0), centre(s1));
        for vertex in &s0.vertices {
            if !tr.alive() {
                return None;
            }
            let local0 = inward(s0, mid(*vertex), c0);
            if !contains(s0, Tf::ID, local0, eps) {
                continue;
            }
            let world0 = t0.point(local0.map(Iv::point));
            let proposal = mid(world0);
            for (normal, offset) in &s1.planes {
                if !tr.alive() {
                    return None;
                }
                let wn = mid(t1.direction(*normal));
                let norm2 = f_dot(wn, wn);
                if norm2 == 0.0 || !norm2.is_finite() {
                    continue;
                }
                let distance =
                    (offset.midpoint() + f_dot(wn, mid(t1.t)) - f_dot(wn, proposal)) / norm2;
                let projected = std::array::from_fn(|k| proposal[k] + wn[k] * distance);
                let local1 = inward(s1, mid(t1.inverse().point(projected.map(Iv::point))), c1);
                // Fixed local points are certified inside the exact source
                // halfspaces. Approximate projection is not itself evidence.
                if !contains(s1, Tf::ID, local1, eps) {
                    continue;
                }
                let world1 = t1.point(local1.map(Iv::point));
                let bound = length(minus(world0, world1));
                let threshold = Iv::point(clearance) - Iv::point(eps);
                if bound.valid() && threshold.valid() && bound.hi < threshold.lo {
                    return Some((mid(world0), mid(world1)));
                }
            }
        }
    }
    None
}

pub(crate) fn certify_pair(
    a: &Solid,
    b: &Solid,
    pose: &dyn Fn(Iv) -> (Tf, Tf),
    clearance: f64,
    deadband: f64,
    static_relative: bool,
    tr: &mut Tracker<'_>,
) -> PairResult {
    let mut stack = vec![[0.0, 1.0]];
    while let Some([lo, hi]) = stack.pop() {
        if !tr.query() {
            return PairResult::unknown(
                [lo, hi],
                tr.usage.stopped_by.clone().unwrap_or("cancelled".into()),
            );
        }
        if !a.labels.is_disjoint(&b.labels) {
            continue;
        }
        let (t0, t1) = pose(Iv::new(lo, hi));
        let mid = lo / 2.0 + hi / 2.0;
        let (m0, m1) = pose(Iv::point(mid));
        match separated(a, b, t0, t1, m0, m1, clearance, deadband, tr) {
            Some(true) => continue,
            None => {
                return PairResult::unknown(
                    [lo, hi],
                    if tr.cancel.load(Ordering::Relaxed) {
                        "cancelled"
                    } else {
                        "nonfinite_transform_or_projection"
                    },
                );
            }
            Some(false) => {}
        }
        for time in [lo, mid, hi] {
            let (pa, pb) = pose(Iv::point(time));
            if let Some(point) = witness(a, b, pa, pb, deadband, tr) {
                return PairResult {
                    outcome: CheckOutcome::Fail,
                    interval: [lo, hi],
                    time: Some(time),
                    point: Some(point),
                    reason: "interval_verified_strict_interior_intersection".into(),
                };
            }
            if let Some((p, q)) = clearance_witness(a, b, pa, pb, clearance, deadband, tr) {
                return PairResult {
                    outcome: CheckOutcome::Fail,
                    interval: [lo, hi],
                    time: Some(time),
                    point: None,
                    reason: format!(
                        "interval_verified_clearance_deficit_between_interior_points:{p:?}:{q:?}"
                    ),
                };
            }
        }
        // A positive clearance deficit is a hard violation too, but only an
        // actual certified distance witness could establish it. SAT failure
        // alone never becomes a physical Fail.
        if static_relative {
            return PairResult::unknown([lo, hi], "static_boundary_or_clearance_not_certified");
        }
        if mid <= lo || mid >= hi {
            return PairResult::unknown([lo, hi], "binary64_time_resolution");
        }
        if !tr.subdivide() {
            return PairResult::unknown(
                [lo, hi],
                tr.usage.stopped_by.clone().unwrap_or("cancelled".into()),
            );
        }
        stack.push([mid, hi]);
        stack.push([lo, mid]);
    }
    PairResult::pass()
}

pub(crate) fn panel_cells(
    job: &PackageStructureJob,
    a: usize,
    b: usize,
    step: Option<&ObjectId>,
) -> Result<(Vec<Solid>, Vec<Solid>), &'static str> {
    let pa = &job.panels[a];
    let pb = &job.panels[b];
    let mut patches_a = vec![];
    let mut patches_b = vec![];
    for (i, p) in job.conditions.allowed_contacts.iter().enumerate() {
        if !step.is_some_and(|s| p.active_step_ids.contains(s)) {
            continue;
        }
        let matched = p.panel_a == pa.id && p.panel_b == pb.id;
        let reverse = p.panel_b == pa.id && p.panel_a == pb.id;
        if !matched && !reverse {
            continue;
        }
        if p.normal_overlap_limit_mm.value() < 0.0
            || p.normal_overlap_limit_mm.value()
                >= pa.thickness_mm.value().min(pb.thickness_mm.value())
        {
            return Err("invalid_contact_depth");
        }
        let (poly_a, face_a, poly_b, face_b) = if matched {
            (&p.polygon_a_mm, &p.face_a, &p.polygon_b_mm, &p.face_b)
        } else {
            (&p.polygon_b_mm, &p.face_b, &p.polygon_a_mm, &p.face_a)
        };
        patches_a.push(exact::Patch {
            id: i,
            polygon: exact::polygon(poly_a)?,
            depth: q(p.normal_overlap_limit_mm.value()),
            front: exact::front(face_a),
        });
        patches_b.push(exact::Patch {
            id: i,
            polygon: exact::polygon(poly_b)?,
            depth: q(p.normal_overlap_limit_mm.value()),
            front: exact::front(face_b),
        });
    }
    Ok((
        exact::partition(
            &exact::polygon(&pa.polygon_mm)?,
            q(pa.thickness_mm.value()),
            &patches_a,
        )?
        .iter()
        .map(Solid::cell)
        .collect(),
        exact::partition(
            &exact::polygon(&pb.polygon_mm)?,
            q(pb.thickness_mm.value()),
            &patches_b,
        )?
        .iter()
        .map(Solid::cell)
        .collect(),
    ))
}
