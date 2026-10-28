//! Static floor geometry in declared metres. No renderer, visual scale or Engine.
use sf_domain::{CoordinateFrame, Pose, Vec2};

pub(super) const EPS: f64 = 1e-8;
pub(super) type Point = [f64; 2];
pub(super) type Polygon = Vec<Point>;

pub(super) fn polygon(points: &[Vec2]) -> Result<Polygon, &'static str> {
    let p: Polygon = points.iter().map(|p| [p.x.value(), p.y.value()]).collect();
    validate(&p)?;
    if triangles(&p).len() != p.len() - 2 {
        return Err("polygon triangulation is numerically unresolved");
    }
    Ok(p)
}
fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn area(p: &[Point]) -> f64 {
    let origin = p.first().copied().unwrap_or([0., 0.]);
    edges(p)
        .map(|(a, b)| {
            (a[0] - origin[0]) * (b[1] - origin[1]) - (a[1] - origin[1]) * (b[0] - origin[0])
        })
        .sum::<f64>()
        * 0.5
}
fn edges(p: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    p.iter()
        .copied()
        .zip(p.iter().copied().cycle().skip(1))
        .take(p.len())
}
fn on_segment(p: Point, a: Point, b: Point) -> bool {
    cross(a, b, p).abs() <= EPS
        && p[0] >= a[0].min(b[0]) - EPS
        && p[0] <= a[0].max(b[0]) + EPS
        && p[1] >= a[1].min(b[1]) - EPS
        && p[1] <= a[1].max(b[1]) + EPS
}
fn intersects(a: Point, b: Point, c: Point, d: Point) -> bool {
    let (x, y, z, w) = (
        cross(a, b, c),
        cross(a, b, d),
        cross(c, d, a),
        cross(c, d, b),
    );
    ((x > EPS && y < -EPS || x < -EPS && y > EPS) && (z > EPS && w < -EPS || z < -EPS && w > EPS))
        || on_segment(a, c, d)
        || on_segment(b, c, d)
        || on_segment(c, a, b)
        || on_segment(d, a, b)
}
pub(super) fn validate(p: &[Point]) -> Result<(), &'static str> {
    if p.len() < 3 {
        return Err("polygon requires at least three vertices");
    }
    if p.iter().flatten().any(|v| !v.is_finite()) {
        return Err("nonfinite polygon");
    }
    if area(p).abs() <= EPS {
        return Err("polygon has zero area");
    }
    for (i, (a, b)) in edges(p).enumerate() {
        if (a[0] - b[0]).hypot(a[1] - b[1]) <= EPS {
            return Err("zero length polygon edge");
        }
        for (j, (c, d)) in edges(p).enumerate().skip(i + 1) {
            if j == i + 1 || i == 0 && j == p.len() - 1 {
                continue;
            }
            if intersects(a, b, c, d) {
                return Err("polygon is not simple");
            }
        }
    }
    Ok(())
}
pub(super) fn rectangle(w: f64, d: f64) -> Result<Polygon, &'static str> {
    if !w.is_finite() || !d.is_finite() || w <= 0. || d <= 0. {
        return Err("positive dimensions required");
    }
    Ok(vec![[0., 0.], [w, 0.], [w, d], [0., d]])
}
pub(super) fn inside(p: Point, poly: &[Point]) -> bool {
    if edges(poly).any(|(a, b)| on_segment(p, a, b)) {
        return true;
    }
    let mut yes = false;
    for (a, b) in edges(poly) {
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            yes = !yes;
        }
    }
    yes
}
// Split each edge at every boundary intersection. Testing only vertices would
// accept a segment which crosses a concave notch or a gap between usable regions.
fn cuts(a: Point, b: Point, polygons: &[Polygon]) -> Vec<f64> {
    let mut cuts = vec![0., 1.];
    let r = [b[0] - a[0], b[1] - a[1]];
    for poly in polygons {
        for (c, d) in edges(poly) {
            let q = [c[0] - a[0], c[1] - a[1]];
            let tvec = [d[0] - c[0], d[1] - c[1]];
            let det = r[0] * tvec[1] - r[1] * tvec[0];
            if det.abs() > EPS {
                let t = (q[0] * tvec[1] - q[1] * tvec[0]) / det;
                let u = (q[0] * r[1] - q[1] * r[0]) / det;
                if (-EPS..=1. + EPS).contains(&t) && (-EPS..=1. + EPS).contains(&u) {
                    cuts.push(t.clamp(0., 1.));
                }
            } else {
                for q in [c, d] {
                    if on_segment(q, a, b) {
                        cuts.push(if r[0].abs() > r[1].abs() {
                            (q[0] - a[0]) / r[0]
                        } else {
                            (q[1] - a[1]) / r[1]
                        });
                    }
                }
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < EPS);
    cuts
}
pub(super) fn contained(p: &[Point], covers: &[Polygon]) -> bool {
    if covers.is_empty() {
        return false;
    }
    let covered = |q| covers.iter().any(|c| inside(q, c));
    if p.iter().any(|q| !covered(*q)) {
        return false;
    }
    for (a, b) in edges(p) {
        let cuts = cuts(a, b, covers);
        for ts in cuts.windows(2) {
            let t = (ts[0] + ts[1]) * 0.5;
            if !covered([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]) {
                return false;
            }
        }
    }
    // Check each face-side segment in the arrangement of ALL usable polygons
    // and the device boundary. This also detects holes wholly inside a device,
    // even when an overlap covers the midpoint of an unsplit region edge.
    let mut boundaries = covers.to_vec();
    boundaries.push(p.to_vec());
    for cover in covers {
        for (a, b) in edges(cover) {
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let len = dx.hypot(dy);
            let segments = cuts(a, b, &boundaries);
            for ts in segments.windows(2) {
                let t = (ts[0] + ts[1]) * 0.5;
                let mid = [a[0] + dx * t, a[1] + dy * t];
                for sign in [-1., 1.] {
                    let q = [
                        mid[0] - sign * dy / len * EPS * 8.,
                        mid[1] + sign * dx / len * EPS * 8.,
                    ];
                    if inside(q, p) && !covered(q) {
                        return false;
                    }
                }
            }
        }
    }
    true
}
fn triangle(p: &[Point], a: Point, b: Point, c: Point) -> bool {
    p.iter()
        .all(|q| cross(a, b, *q) >= -EPS && cross(b, c, *q) >= -EPS && cross(c, a, *q) >= -EPS)
}
fn triangles(p: &[Point]) -> Vec<[Point; 3]> {
    let mut rest = p.to_vec();
    if area(&rest) < 0. {
        rest.reverse();
    }
    let mut out = vec![];
    while rest.len() > 3 {
        let ear = (0..rest.len()).find(|&i| {
            let n = rest.len();
            let prev = (i + n - 1) % n;
            let next = (i + 1) % n;
            cross(rest[prev], rest[i], rest[next]) > EPS
                && !rest.iter().enumerate().any(|(j, q)| {
                    j != prev
                        && j != i
                        && j != next
                        && triangle(&[*q], rest[prev], rest[i], rest[next])
                })
        });
        let Some(i) = ear else {
            return vec![];
        };
        let n = rest.len();
        out.push([rest[(i + n - 1) % n], rest[i], rest[(i + 1) % n]]);
        rest.remove(i);
    }
    if rest.len() == 3 {
        out.push([rest[0], rest[1], rest[2]]);
    }
    out
}
pub(super) fn overlap(a: &[Point], b: &[Point]) -> bool {
    for ta in triangles(a) {
        for tb in triangles(b) {
            let mut clip = ta.to_vec();
            for (c, d) in edges(&tb) {
                let mut next = vec![];
                for (p, q) in edges(&clip) {
                    let dp = cross(c, d, p);
                    let dq = cross(c, d, q);
                    if dp >= -EPS {
                        next.push(p);
                    }
                    if (dp > EPS && dq < -EPS) || (dp < -EPS && dq > EPS) {
                        let t = dp / (dp - dq);
                        next.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
                    }
                }
                clip = next;
                if clip.is_empty() {
                    break;
                }
            }
            if clip.len() >= 3 && area(&clip).abs() > EPS {
                return true;
            }
        }
    }
    false
}
fn point_distance(p: Point, a: Point, b: Point) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let t =
        (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1])).clamp(0., 1.);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}
pub(super) fn distance(a: &[Point], b: &[Point]) -> f64 {
    if overlap(a, b) {
        return 0.;
    }
    let mut distance = f64::INFINITY;
    for (p, q) in edges(a) {
        for (r, s) in edges(b) {
            if intersects(p, q, r, s) {
                return 0.;
            }
            distance = distance
                .min(point_distance(p, r, s))
                .min(point_distance(q, r, s))
                .min(point_distance(r, p, q))
                .min(point_distance(s, p, q));
        }
    }
    distance
}

/// Physical footprint is already declared in metres; presentation scale is
/// deliberately excluded. Tilts are named unsupported, never a projected Pass.
pub(super) fn transform(poly: &[Point], poses: &[&Pose]) -> Result<(Polygon, f64), &'static str> {
    let mut out = poly.to_vec();
    let mut z = 0.;
    for pose in poses.iter().rev() {
        if !matches!(pose.coordinate_frame, CoordinateFrame::RightHandedZUp) {
            return Err("unsupported noncanonical coordinate frame for static floor placement");
        }
        let q = &pose.rotation;
        let (x, y, rz, w) = (q.x.value(), q.y.value(), q.z.value(), q.w.value());
        if (x * x + y * y + rz * rz + w * w - 1.).abs() > EPS {
            return Err("rotation quaternion is not unit length");
        }
        if x.abs() > EPS || y.abs() > EPS {
            return Err("tilted floor placement is not supported by static planar check");
        }
        let (cos, sin) = (1. - 2. * rz * rz, 2. * w * rz);
        for p in &mut out {
            *p = [
                cos * p[0] - sin * p[1] + pose.translation.x.value(),
                sin * p[0] + cos * p[1] + pose.translation.y.value(),
            ];
        }
        z += pose.translation.z.value();
    }
    validate(&out)?;
    if !z.is_finite() {
        return Err("nonfinite composed elevation");
    }
    Ok((out, z))
}
