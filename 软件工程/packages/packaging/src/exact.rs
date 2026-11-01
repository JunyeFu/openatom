//! Exact binary-input planar geometry. Concavities and patch boundaries are
//! retained; rational topology is never inferred from a bounding rectangle.
use crate::{collision::Tracker, numeric::Iv};
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
use sf_domain::{PanelFaceSide, Pose, Vec2};
use std::collections::BTreeSet;

pub(crate) type R = Q;
pub(crate) type P = [R; 2];
pub(crate) type Poly = Vec<P>;
pub(crate) fn q(v: f64) -> R {
    Q::from_float(v).expect("finite typed input")
}
pub(crate) fn n(v: i64) -> R {
    Q::from_integer(v.into())
}
pub(crate) fn enc(v: &R) -> Iv {
    let Some(f) = v.to_f64().filter(|x| x.is_finite()) else {
        return Iv::INVALID;
    };
    let a = q(f);
    Iv::new(
        if a > *v { f.next_down() } else { f },
        if a < *v { f.next_up() } else { f },
    )
}
pub(crate) fn cross(a: &P, b: &P, c: &P) -> R {
    (&b[0] - &a[0]) * (&c[1] - &a[1]) - (&b[1] - &a[1]) * (&c[0] - &a[0])
}
pub(crate) fn area2(p: &[P]) -> R {
    (0..p.len())
        .map(|i| &p[i][0] * &p[(i + 1) % p.len()][1] - &p[i][1] * &p[(i + 1) % p.len()][0])
        .sum()
}
pub(crate) fn on_segment(p: &P, a: &P, b: &P) -> bool {
    cross(a, b, p).is_zero()
        && (0..2).all(|k| {
            p[k] >= a[k].clone().min(b[k].clone()) && p[k] <= a[k].clone().max(b[k].clone())
        })
}
fn intersects(a: &P, b: &P, c: &P, d: &P) -> bool {
    let v = [
        cross(a, b, c),
        cross(a, b, d),
        cross(c, d, a),
        cross(c, d, b),
    ];
    (&v[0] * &v[1] < n(0) && &v[2] * &v[3] < n(0))
        || (v[0].is_zero() && on_segment(c, a, b))
        || (v[1].is_zero() && on_segment(d, a, b))
        || (v[2].is_zero() && on_segment(a, c, d))
        || (v[3].is_zero() && on_segment(b, c, d))
}
pub(crate) fn clean(mut p: Poly) -> Poly {
    p.dedup();
    if p.len() > 1 && p.first() == p.last() {
        p.pop();
    }
    loop {
        let found = (0..p.len()).find(|&i| {
            p.len() >= 3
                && cross(
                    &p[(i + p.len() - 1) % p.len()],
                    &p[i],
                    &p[(i + 1) % p.len()],
                )
                .is_zero()
        });
        if let Some(i) = found {
            p.remove(i);
        } else {
            break;
        }
    }
    p
}
pub(crate) fn polygon(points: &[Vec2]) -> Result<Poly, &'static str> {
    polygon_q(
        points
            .iter()
            .map(|p| [q(p.x.value()), q(p.y.value())])
            .collect(),
    )
}
pub(crate) fn polygon_q(raw: Poly) -> Result<Poly, &'static str> {
    if raw.len() < 3 || raw.iter().collect::<BTreeSet<_>>().len() != raw.len() {
        return Err("invalid_simple_polygon");
    }
    for i in 0..raw.len() {
        let (a, b, c) = (
            &raw[(i + raw.len() - 1) % raw.len()],
            &raw[i],
            &raw[(i + 1) % raw.len()],
        );
        if cross(a, b, c).is_zero()
            && (0..2).map(|k| (&b[k] - &a[k]) * (&c[k] - &b[k])).sum::<R>() <= n(0)
        {
            return Err("invalid_simple_polygon");
        }
        for j in i + 1..raw.len() {
            if j == i + 1 || (i == 0 && j == raw.len() - 1) {
                continue;
            }
            if intersects(
                &raw[i],
                &raw[(i + 1) % raw.len()],
                &raw[j],
                &raw[(j + 1) % raw.len()],
            ) {
                return Err("invalid_simple_polygon");
            }
        }
    }
    let mut p = clean(raw);
    if p.len() < 3 || area2(&p).is_zero() {
        return Err("invalid_simple_polygon");
    }
    if area2(&p) < n(0) {
        p.reverse();
    }
    Ok(p)
}
pub(crate) fn inside(p: &P, poly: &[P]) -> bool {
    let mut winding = 0;
    for i in 0..poly.len() {
        let (a, b) = (&poly[i], &poly[(i + 1) % poly.len()]);
        if on_segment(p, a, b) {
            return true;
        }
        if a[1] <= p[1] && p[1] < b[1] && cross(a, b, p) > n(0) {
            winding += 1;
        } else if b[1] <= p[1] && p[1] < a[1] && cross(a, b, p) < n(0) {
            winding -= 1;
        }
    }
    winding != 0
}
pub(crate) fn centroid(p: &[P]) -> P {
    std::array::from_fn(|k| p.iter().map(|v| v[k].clone()).sum::<R>() / n(p.len() as i64))
}
pub(crate) fn convex_parts(poly: &[P]) -> Result<Vec<Poly>, &'static str> {
    if (0..poly.len()).all(|i| {
        cross(
            &poly[(i + poly.len() - 1) % poly.len()],
            &poly[i],
            &poly[(i + 1) % poly.len()],
        ) >= n(0)
    }) {
        return Ok(vec![poly.to_vec()]);
    }
    let mut p = poly.to_vec();
    let mut out = vec![];
    while p.len() > 3 {
        let ear = (0..p.len()).find(|&i| {
            let (a, b, c) = (
                &p[(i + p.len() - 1) % p.len()],
                &p[i],
                &p[(i + 1) % p.len()],
            );
            cross(a, b, c) > n(0)
                && !p.iter().filter(|x| *x != a && *x != b && *x != c).any(|x| {
                    cross(a, b, x) >= n(0) && cross(b, c, x) >= n(0) && cross(c, a, x) >= n(0)
                })
        });
        let Some(i) = ear else {
            return Err("polygon_triangulation_unresolved");
        };
        out.push(vec![
            p[(i + p.len() - 1) % p.len()].clone(),
            p[i].clone(),
            p[(i + 1) % p.len()].clone(),
        ]);
        p.remove(i);
    }
    out.push(p);
    Ok(out)
}
pub(crate) fn clip(p: &[P], a: &P, b: &P, sign: i64) -> Poly {
    let mut out = vec![];
    for i in 0..p.len() {
        let (u, v) = (&p[i], &p[(i + 1) % p.len()]);
        let (du, dv) = (cross(a, b, u) * n(sign), cross(a, b, v) * n(sign));
        if du >= n(0) {
            out.push(u.clone());
        }
        if (&du * &dv) < n(0) {
            let r = &du / (&du - &dv);
            out.push(std::array::from_fn(|k| &u[k] + &r * (&v[k] - &u[k])));
        }
    }
    let out = clean(out);
    if out.len() >= 3 && area2(&out) > n(0) {
        out
    } else {
        vec![]
    }
}
pub(crate) fn convex_intersection(a: &[P], b: &[P]) -> Poly {
    let mut out = a.to_vec();
    for i in 0..b.len() {
        out = clip(&out, &b[i], &b[(i + 1) % b.len()], 1);
        if out.is_empty() {
            break;
        }
    }
    out
}
#[derive(Clone)]
pub(crate) struct Patch {
    pub id: usize,
    pub polygon: Poly,
    pub depth: R,
    pub front: bool,
}
#[derive(Clone)]
pub(crate) struct Cell {
    pub polygon: Poly,
    pub lo: R,
    pub hi: R,
    pub labels: BTreeSet<usize>,
}
pub(crate) fn partition(
    poly: &[P],
    thickness: R,
    patches: &[Patch],
) -> Result<Vec<Cell>, &'static str> {
    let mut regions = convex_parts(poly)?;
    for patch in patches {
        if patch.depth < n(0) || patch.depth >= thickness {
            return Err("invalid_contact_depth");
        }
        for i in 0..patch.polygon.len() {
            regions = regions
                .into_iter()
                .flat_map(|region| {
                    [
                        clip(
                            &region,
                            &patch.polygon[i],
                            &patch.polygon[(i + 1) % patch.polygon.len()],
                            1,
                        ),
                        clip(
                            &region,
                            &patch.polygon[i],
                            &patch.polygon[(i + 1) % patch.polygon.len()],
                            -1,
                        ),
                    ]
                })
                .filter(|r| !r.is_empty())
                .collect();
        }
    }
    let flags: Vec<Vec<bool>> = regions
        .iter()
        .map(|r| {
            patches
                .iter()
                .map(|p| inside(&centroid(r), &p.polygon))
                .collect()
        })
        .collect();
    for (j, p) in patches.iter().enumerate() {
        if regions
            .iter()
            .zip(&flags)
            .filter(|(_, f)| f[j])
            .map(|(r, _)| area2(r))
            .sum::<R>()
            != area2(&p.polygon)
        {
            return Err("patch_outside_panel");
        }
    }
    let z1 = thickness / n(2);
    let z0 = -&z1;
    let slabs: Vec<_> = patches
        .iter()
        .map(|p| {
            if p.front {
                (&z1 - &p.depth, z1.clone())
            } else {
                (z0.clone(), &z0 + &p.depth)
            }
        })
        .collect();
    let levels: BTreeSet<_> = std::iter::once(z0)
        .chain(std::iter::once(z1))
        .chain(slabs.iter().flat_map(|(a, b)| [a.clone(), b.clone()]))
        .collect();
    let levels: Vec<_> = levels.into_iter().collect();
    let mut cells = vec![];
    for (r, f) in regions.into_iter().zip(flags) {
        for pair in levels.windows(2) {
            let mid = (&pair[0] + &pair[1]) / n(2);
            let labels = patches
                .iter()
                .enumerate()
                .filter(|(j, _)| f[*j] && slabs[*j].0 < mid && mid < slabs[*j].1)
                .map(|(_, p)| p.id)
                .collect();
            cells.push(Cell {
                polygon: r.clone(),
                lo: pair[0].clone(),
                hi: pair[1].clone(),
                labels,
            });
        }
    }
    if !patches.is_empty() && !cells.iter().any(|c| c.labels.is_empty()) {
        return Err("pair_allowance_covers_entire_panel");
    }
    Ok(cells)
}
pub(crate) fn front(face: &PanelFaceSide) -> bool {
    matches!(face, PanelFaceSide::Front)
}

/// Exact normalized quaternion rotation. Pose validation rejects zero/nonunit
/// quaternion input before this constructor; rational normalization removes only
/// representation drift and supplies a certified orthogonal matrix.
#[derive(Clone)]
pub(crate) struct ExactTf {
    pub r: [[R; 3]; 3],
    pub t: [R; 3],
}
impl ExactTf {
    pub fn from_pose(p: &Pose) -> Self {
        let [x, y, z, w] = [
            p.rotation.x.value(),
            p.rotation.y.value(),
            p.rotation.z.value(),
            p.rotation.w.value(),
        ]
        .map(q);
        let norm = &x * &x + &y * &y + &z * &z + &w * &w;
        let two = n(2) / norm;
        let r = [
            [
                n(1) - &two * (&y * &y + &z * &z),
                &two * (&x * &y - &z * &w),
                &two * (&x * &z + &y * &w),
            ],
            [
                &two * (&x * &y + &z * &w),
                n(1) - &two * (&x * &x + &z * &z),
                &two * (&y * &z - &x * &w),
            ],
            [
                &two * (&x * &z - &y * &w),
                &two * (&y * &z + &x * &w),
                n(1) - &two * (&x * &x + &y * &y),
            ],
        ];
        Self {
            r,
            t: [
                q(p.translation.x.value()),
                q(p.translation.y.value()),
                q(p.translation.z.value()),
            ],
        }
    }
    pub fn point(&self, p: &[R; 3]) -> [R; 3] {
        std::array::from_fn(|i| &self.t[i] + (0..3).map(|j| &self.r[i][j] * &p[j]).sum::<R>())
    }
    pub fn inverse(&self) -> Self {
        let r = std::array::from_fn(|i| std::array::from_fn(|j| self.r[j][i].clone()));
        let t = std::array::from_fn(|i| -(0..3).map(|j| &r[i][j] * &self.t[j]).sum::<R>());
        Self { r, t }
    }
    pub fn compose(&self, b: &Self) -> Self {
        Self {
            r: std::array::from_fn(|i| {
                std::array::from_fn(|j| (0..3).map(|k| &self.r[i][k] * &b.r[k][j]).sum())
            }),
            t: self.point(&b.t),
        }
    }
}

/// Exact union boundary arrangement. Shared edges are split at all endpoints;
/// opposite internal pieces cancel. Caller verifies no positive-area overlap.
pub(crate) fn boundary(polys: &[Poly], tr: &mut Tracker<'_>) -> Result<Vec<(P, P)>, &'static str> {
    let vertices: BTreeSet<_> = polys.iter().flatten().cloned().collect();
    let mut edges = BTreeSet::new();
    let mut reported = false;
    for p in polys {
        for i in 0..p.len() {
            let (a, b) = (&p[i], &p[(i + 1) % p.len()]);
            let mut points = vec![];
            for x in &vertices {
                if !tr.alive() {
                    return Err("cancelled");
                }
                if on_segment(x, a, b) {
                    points.push(x.clone());
                }
                if !reported {
                    tr.phase("manufacturing_boundary_edges");
                    reported = true;
                }
                if !tr.alive() {
                    return Err("cancelled");
                }
            }
            points.sort_by(|x, y| {
                let axis = if a[0] != b[0] { 0 } else { 1 };
                if a[axis] < b[axis] {
                    x[axis].cmp(&y[axis])
                } else {
                    y[axis].cmp(&x[axis])
                }
            });
            for w in points.windows(2) {
                if !tr.alive() {
                    return Err("cancelled");
                }
                if !edges.remove(&(w[1].clone(), w[0].clone())) {
                    edges.insert((w[0].clone(), w[1].clone()));
                }
            }
        }
    }
    Ok(edges.into_iter().collect())
}

pub(crate) fn union_cells(polys: &[Poly], tr: &mut Tracker<'_>) -> Result<Vec<Poly>, &'static str> {
    let mut triangles = vec![];
    for p in polys {
        if !tr.alive() {
            return Err("cancelled");
        }
        triangles.extend(convex_parts(p)?);
    }
    let lines: Vec<_> = triangles
        .iter()
        .flat_map(|p| (0..p.len()).map(|i| (p[i].clone(), p[(i + 1) % p.len()].clone())))
        .collect();
    let mut unique = BTreeSet::new();
    let mut reported = false;
    for poly in triangles {
        let mut regions = vec![poly];
        for (a, b) in &lines {
            let mut next = vec![];
            for p in regions {
                if !tr.alive() {
                    return Err("cancelled");
                }
                next.extend(
                    [clip(&p, a, b, 1), clip(&p, a, b, -1)]
                        .into_iter()
                        .filter(|p| !p.is_empty()),
                );
                if !reported {
                    tr.phase("manufacturing_union_cells");
                    reported = true;
                }
                if !tr.alive() {
                    return Err("cancelled");
                }
            }
            regions = next;
        }
        for mut p in regions {
            if !tr.alive() {
                return Err("cancelled");
            }
            let start = (0..p.len())
                .min_by_key(|&i| p[i].clone())
                .expect("nonempty polygon");
            p.rotate_left(start);
            unique.insert(p);
        }
    }
    Ok(unique.into_iter().collect())
}
