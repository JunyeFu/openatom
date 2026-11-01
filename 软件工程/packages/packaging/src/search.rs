use crate::{
    exact::{n, q},
    kinematics::{number, problem},
};
use sf_domain::*;
use std::collections::BTreeSet;

#[derive(Clone)]
enum Leaf {
    Vertex(usize, usize, bool),
    Thickness(usize),
    Width(usize),
    Angle(usize),
    Placement(usize, usize),
    Clearance,
    Extraction,
}
impl Leaf {
    fn resolve(job: &PackageStructureJob, path: &str) -> Option<Self> {
        let p: Vec<_> = path.trim_start_matches('/').split('/').collect();
        match p.as_slice() {
            ["panels", id, "polygon_mm", index, axis] => {
                let i = job.panels.iter().position(|x| x.id.as_str() == *id)?;
                let j = index.parse::<usize>().ok()?;
                if j >= job.panels[i].polygon_mm.len() || !matches!(*axis, "x" | "y") {
                    None
                } else {
                    Some(Self::Vertex(i, j, *axis == "x"))
                }
            }
            ["panels", id, "thickness_mm"] => Some(Self::Thickness(
                job.panels.iter().position(|x| x.id.as_str() == *id)?,
            )),
            ["fold_bands", id, "width_mm"] => Some(Self::Width(
                job.fold_bands.iter().position(|x| x.id.as_str() == *id)?,
            )),
            ["fold_bands", id, "target_angle_rad"] => Some(Self::Angle(
                job.fold_bands.iter().position(|x| x.id.as_str() == *id)?,
            )),
            ["parts", id, "placement", "translation", axis] => Some(Self::Placement(
                job.parts.iter().position(|x| x.id.as_str() == *id)?,
                match *axis {
                    "x" => 0,
                    "y" => 1,
                    "z" => 2,
                    _ => return None,
                },
            )),
            ["conditions", "product_envelope", "clearance_mm"]
                if job.conditions.product_envelope.is_some() =>
            {
                Some(Self::Clearance)
            }
            ["conditions", "extraction", "distance_mm"] if job.conditions.extraction.is_some() => {
                Some(Self::Extraction)
            }
            _ => None,
        }
    }
    fn value_mut<'a>(&self, j: &'a mut PackageStructureJob) -> &'a mut FiniteNumber {
        match *self {
            Self::Vertex(i, k, true) => &mut j.panels[i].polygon_mm[k].x,
            Self::Vertex(i, k, false) => &mut j.panels[i].polygon_mm[k].y,
            Self::Thickness(i) => &mut j.panels[i].thickness_mm,
            Self::Width(i) => &mut j.fold_bands[i].width_mm,
            Self::Angle(i) => &mut j.fold_bands[i].target_angle_rad,
            Self::Placement(i, k) => {
                let p = &mut j.parts[i].placement.translation;
                match k {
                    0 => &mut p.x,
                    1 => &mut p.y,
                    _ => &mut p.z,
                }
            }
            Self::Clearance => {
                &mut j
                    .conditions
                    .product_envelope
                    .as_mut()
                    .expect("resolved leaf")
                    .clearance_mm
            }
            Self::Extraction => {
                &mut j
                    .conditions
                    .extraction
                    .as_mut()
                    .expect("resolved leaf")
                    .distance_mm
            }
        }
    }
}
enum Domain {
    Real(Vec<f64>),
    Integer { lo: i64, hi: i64 },
}
impl Domain {
    fn at(&self, index: u64) -> Option<f64> {
        match self {
            Self::Real(v) => usize::try_from(index).ok().and_then(|i| v.get(i)).copied(),
            Self::Integer { lo, hi } => {
                let v = i128::from(*lo) + i128::from(index);
                (v <= i128::from(*hi)).then_some(v as f64)
            }
        }
    }
}
pub(crate) struct Candidates {
    base: PackageStructureJob,
    leaves: Vec<Leaf>,
    domains: Vec<Domain>,
    indices: Vec<u64>,
    seed: bool,
    finished: bool,
    pub continuous: bool,
}
impl Candidates {
    pub fn exhausted(&self) -> bool {
        self.finished
    }
    pub fn new(job: &PackageStructureJob) -> Result<Self, Vec<Issue>> {
        let mut vars: Vec<_> = job.variables.iter().enumerate().collect();
        vars.sort_by_key(|(_, v)| v.id.clone());
        let mut ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        let mut leaves = vec![];
        let mut domains = vec![];
        let mut issues = vec![];
        let mut base = job.clone();
        let mut continuous = false;
        for (i, v) in vars {
            let path = format!("/variables/{i}");
            if !ids.insert(v.id.clone()) || !paths.insert(v.field_path.clone()) {
                issues.push(problem(
                    job,
                    "duplicate_variable",
                    &path,
                    "变量标识与绑定叶子必须唯一。",
                ));
                continue;
            }
            let Some(leaf) = Leaf::resolve(job, &v.field_path) else {
                issues.push(problem(
                    job,
                    "variable_binding",
                    &path,
                    "变量须绑定白名单中已存在的数值叶子。",
                ));
                continue;
            };
            let unit = if matches!(leaf, Leaf::Angle(_)) {
                "rad"
            } else {
                "mm"
            };
            if v.unit.as_str() != unit || v.minimum.value() > v.maximum.value() {
                issues.push(problem(
                    job,
                    "variable_range_or_unit",
                    &path,
                    "变量须使用mm/rad、非空有序范围。",
                ));
                continue;
            }
            let min = v.minimum.value();
            let max = v.maximum.value();
            let seed = leaf.value_mut(&mut base).value();
            let domain = if v.integer {
                if min < -9_007_199_254_740_991.0 || max > 9_007_199_254_740_991.0 {
                    issues.push(problem(
                        job,
                        "integer_enumeration_range",
                        &path,
                        "整数范围须在可精确表示的安全整数域内，不能跳过不可表示的整数。",
                    ));
                    continue;
                }
                let (mut lo, mut hi) = (min.ceil() as i64, max.floor() as i64);
                for c in job
                    .dimension_constraints
                    .iter()
                    .filter(|c| c.variable_id == v.id)
                {
                    let t = c.target.value();
                    match c.relation {
                        ComparisonOperator::Equal => {
                            if t.fract() != 0.0 || t < min || t > max {
                                lo = 1;
                                hi = 0;
                            } else {
                                lo = lo.max(t as i64);
                                hi = hi.min(t as i64);
                            }
                        }
                        ComparisonOperator::LessOrEqual => {
                            if t < min {
                                lo = 1;
                                hi = 0;
                            } else if t < max {
                                hi = hi.min(t.floor() as i64);
                            }
                        }
                        ComparisonOperator::GreaterOrEqual => {
                            if t > max {
                                lo = 1;
                                hi = 0;
                            } else if t > min {
                                lo = lo.max(t.ceil() as i64);
                            }
                        }
                    }
                }
                Domain::Integer { lo, hi }
            } else {
                continuous |= min < max;
                let mut values = vec![seed, min, max, min / 2.0 + max / 2.0];
                for c in job
                    .dimension_constraints
                    .iter()
                    .filter(|c| c.variable_id == v.id)
                {
                    let t = c.target.value();
                    values.extend([t, t.next_down(), t.next_up()]);
                }
                values.retain(|x| {
                    x.is_finite()
                        && *x >= min
                        && *x <= max
                        && job
                            .dimension_constraints
                            .iter()
                            .filter(|c| c.variable_id == v.id)
                            .all(|c| match c.relation {
                                ComparisonOperator::Equal => *x == c.target.value(),
                                ComparisonOperator::LessOrEqual => *x <= c.target.value(),
                                ComparisonOperator::GreaterOrEqual => *x >= c.target.value(),
                            })
                });
                values.sort_by(f64::total_cmp);
                values.dedup();
                Domain::Real(values)
            };
            leaves.push(leaf);
            domains.push(domain);
        }
        for (i, c) in job.dimension_constraints.iter().enumerate() {
            if !ids.contains(&c.variable_id) {
                issues.push(problem(
                    job,
                    "constraint_variable",
                    &format!("/dimension_constraints/{i}/variable_id"),
                    "硬约束引用未知变量。",
                ));
            }
        }
        if !issues.is_empty() {
            return Err(issues);
        }
        let indices = vec![0; leaves.len()];
        Ok(Self {
            base: job.clone(),
            leaves,
            domains,
            indices,
            seed: true,
            finished: false,
            continuous,
        })
    }
    pub fn next(&mut self) -> Option<PackageStructureJob> {
        if self.finished {
            return None;
        }
        if self.seed {
            self.seed = false;
            return Some(self.base.clone());
        }
        if self.leaves.is_empty() {
            self.finished = true;
            return None;
        }
        if self.domains.iter().any(|d| d.at(0).is_none()) {
            self.finished = true;
            return None;
        }
        let mut result = self.base.clone();
        for (i, leaf) in self.leaves.iter().enumerate() {
            *leaf.value_mut(&mut result) = number(
                self.domains[i]
                    .at(self.indices[i])
                    .expect("live lazy position"),
            );
        }
        let mut i = self.indices.len();
        loop {
            if i == 0 {
                self.finished = true;
                break;
            }
            i -= 1;
            self.indices[i] += 1;
            if self.domains[i].at(self.indices[i]).is_some() {
                break;
            }
            self.indices[i] = 0;
        }
        Some(result)
    }
}
pub(crate) fn constraints(job: &PackageStructureJob) -> Result<(), Vec<Issue>> {
    let mut clone = job.clone();
    let mut issues = vec![];
    for (i, v) in job.variables.iter().enumerate() {
        let leaf = Leaf::resolve(job, &v.field_path).expect("validated binding");
        let value = leaf.value_mut(&mut clone).value();
        if value < v.minimum.value()
            || value > v.maximum.value()
            || v.integer && q(value).denom() != n(1).numer()
        {
            issues.push(problem(
                job,
                "candidate_outside_variable",
                &format!("/variables/{i}"),
                "候选数值不满足显式变量范围/整数要求。",
            ));
        }
    }
    for (i, c) in job.dimension_constraints.iter().enumerate() {
        let v = job
            .variables
            .iter()
            .find(|v| v.id == c.variable_id)
            .expect("validated constraint");
        let value = Leaf::resolve(job, &v.field_path)
            .expect("validated binding")
            .value_mut(&mut clone)
            .value();
        let target = c.target.value();
        let ok = match c.relation {
            ComparisonOperator::Equal => q(value) == q(target),
            ComparisonOperator::LessOrEqual => q(value) <= q(target),
            ComparisonOperator::GreaterOrEqual => q(value) >= q(target),
        };
        if !ok {
            issues.push(problem(
                job,
                "dimension_constraint",
                &format!("/dimension_constraints/{i}"),
                "该候选被明确尺寸硬条件排除；不代表整个结构域无解。",
            ));
        }
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}
