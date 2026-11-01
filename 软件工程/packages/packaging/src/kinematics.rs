use crate::{
    exact::{ExactTf, enc},
    numeric::Iv,
};
use sf_domain::*;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type V = [Iv; 3];
pub(crate) fn dot(a: V, b: V) -> Iv {
    (0..3).fold(Iv::ZERO, |s, i| s + a[i] * b[i])
}
pub(crate) fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn plus(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
pub(crate) fn minus(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(crate) fn vector(v: &Vec3) -> V {
    [v.x.value(), v.y.value(), v.z.value()].map(Iv::point)
}
pub(crate) fn length(a: V) -> Iv {
    a.into_iter()
        .fold(Iv::ZERO, |s, x| s + x.square())
        .sqrt_nonnegative()
}
pub(crate) fn scale(a: V, k: Iv) -> V {
    a.map(|v| v * k)
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tf {
    pub r: [[Iv; 3]; 3],
    pub t: V,
}
impl Tf {
    pub const ID: Self = Self {
        r: [
            [Iv::ONE, Iv::ZERO, Iv::ZERO],
            [Iv::ZERO, Iv::ONE, Iv::ZERO],
            [Iv::ZERO, Iv::ZERO, Iv::ONE],
        ],
        t: [Iv::ZERO; 3],
    };
    pub fn from_exact(e: &ExactTf) -> Self {
        Self {
            r: std::array::from_fn(|i| std::array::from_fn(|j| enc(&e.r[i][j]))),
            t: std::array::from_fn(|i| enc(&e.t[i])),
        }
    }
    pub fn pose(p: &Pose) -> Self {
        Self::from_exact(&ExactTf::from_pose(p))
    }
    pub fn direction(self, v: V) -> V {
        std::array::from_fn(|i| dot(self.r[i], v))
    }
    pub fn point(self, p: V) -> V {
        plus(self.direction(p), self.t)
    }
    pub fn compose(self, b: Self) -> Self {
        Self {
            r: std::array::from_fn(|i| {
                std::array::from_fn(|j| (0..3).fold(Iv::ZERO, |s, k| s + self.r[i][k] * b.r[k][j]))
            }),
            t: self.point(b.t),
        }
    }
    pub fn inverse(self) -> Self {
        let r = std::array::from_fn(|i| std::array::from_fn(|j| self.r[j][i]));
        Self {
            r,
            t: std::array::from_fn(|i| -dot(r[i], self.t)),
        }
    }
    pub fn translation(t: V) -> Self {
        Self { t, ..Self::ID }
    }
    pub fn rotation(axis: V, angle: Iv) -> Self {
        let nonzero: Vec<_> = (0..3).filter(|&i| !axis[i].exact_zero()).collect();
        let (c, s) = (angle.cos(), angle.sin());
        if nonzero.len() == 1 {
            let i = nonzero[0];
            let s = if axis[i].hi < 0.0 { -s } else { s };
            let mut r = Self::ID.r;
            let (j, k) = ((i + 1) % 3, (i + 2) % 3);
            r[j][j] = c;
            r[k][k] = c;
            r[j][k] = -s;
            r[k][j] = s;
            return Self { r, ..Self::ID };
        }
        let len = length(axis);
        let a = axis.map(|x| x / len);
        let one = Iv::ONE - c;
        let mut r = std::array::from_fn(|i| {
            std::array::from_fn(|j| one * a[i] * a[j] + if i == j { c } else { Iv::ZERO })
        });
        for i in 0..3 {
            let (j, k) = ((i + 1) % 3, (i + 2) % 3);
            r[j][k] = r[j][k] - s * a[i];
            r[k][j] = r[k][j] + s * a[i];
        }
        Self { r, ..Self::ID }
    }
    pub fn about(pivot: V, axis: V, angle: Iv) -> Self {
        Self::translation(pivot)
            .compose(Self::rotation(axis, angle))
            .compose(Self::translation(pivot.map(|v| -v)))
    }
    pub fn finite(self) -> bool {
        self.r.into_iter().flatten().chain(self.t).all(Iv::valid)
    }
}
#[derive(Clone)]
pub(crate) struct State {
    pub roots: Vec<Tf>,
    pub angles: Vec<f64>,
}
pub(crate) struct Model<'a> {
    pub job: &'a PackageStructureJob,
    pub panels: BTreeMap<ObjectId, usize>,
    pub parts: BTreeMap<ObjectId, usize>,
    pub bands: BTreeMap<ObjectId, usize>,
    pub parent: Vec<Option<(usize, usize)>>,
    pub part_for: Vec<usize>,
    pub flat: Vec<Tf>,
    pub relative: Vec<Tf>,
    pub order: Vec<usize>,
}
pub(crate) fn problem(
    job: &PackageStructureJob,
    code: &str,
    path: &str,
    message: impl Into<String>,
) -> Issue {
    Issue {
        code: format!("packaging.{code}"),
        object_ref: Some(ObjectRef {
            kind: ObjectKind::Job,
            id: job.id.clone(),
        }),
        field_path: Some(path.into()),
        message: message.into(),
        blocks: vec![
            CapabilityPurpose::Execution,
            CapabilityPurpose::CompleteConclusion,
        ],
    }
}
pub(crate) fn number(x: f64) -> FiniteNumber {
    x.try_into().expect("finite kernel output")
}
pub(crate) fn uint(x: u64) -> SafeUint {
    x.try_into().expect("bounded by explicit safe budget")
}
pub(crate) fn pose_input_valid(p: &Pose, angle_tol: f64) -> bool {
    matches!(p.unit, LengthUnit::Millimeter)
        && matches!(p.coordinate_frame, CoordinateFrame::RightHandedZUp)
        && [p.scale.x.value(), p.scale.y.value(), p.scale.z.value()] == [1.0; 3]
        && {
            let q = [
                p.rotation.x.value(),
                p.rotation.y.value(),
                p.rotation.z.value(),
                p.rotation.w.value(),
            ];
            let norm = q.into_iter().map(|x| x * x).sum::<f64>();
            norm.is_finite() && norm > 0.0 && (norm - 1.0).abs() <= angle_tol * 2.0
        }
}
impl<'a> Model<'a> {
    pub fn build(job: &'a PackageStructureJob) -> Result<Self, Vec<Issue>> {
        let mut issues = vec![];
        let angle = job
            .tolerance
            .as_ref()
            .map(|x| x.angle_rad.value())
            .unwrap_or(0.0);
        let mut unique = |ids: Vec<ObjectId>, path: &str| {
            let mut out = BTreeMap::new();
            for (i, id) in ids.into_iter().enumerate() {
                if out.insert(id, i).is_some() {
                    issues.push(problem(job, "duplicate_id", path, "同类别标识必须唯一。"));
                }
            }
            out
        };
        let panels = unique(job.panels.iter().map(|p| p.id.clone()).collect(), "/panels");
        let parts = unique(job.parts.iter().map(|p| p.id.clone()).collect(), "/parts");
        let bands = unique(
            job.fold_bands.iter().map(|p| p.id.clone()).collect(),
            "/fold_bands",
        );
        let steps = unique(job.steps.iter().map(|p| p.id.clone()).collect(), "/steps");
        let materials = unique(
            job.materials.iter().map(|p| p.id.clone()).collect(),
            "/materials",
        );
        unique(
            job.connections.iter().map(|p| p.id.clone()).collect(),
            "/connections",
        );
        let allowances = unique(
            job.conditions
                .allowed_contacts
                .iter()
                .map(|p| p.id.clone())
                .collect(),
            "/conditions/allowed_contacts",
        );
        if !matches!(job.unit, LengthUnit::Millimeter)
            || !matches!(job.coordinate_frame, CoordinateFrame::RightHandedZUp)
        {
            issues.push(problem(
                job,
                "coordinate_contract",
                "/unit",
                "纸包装几何必须使用mm及right_handed_z_up。",
            ));
        }
        for (i, p) in job.panels.iter().enumerate() {
            if !parts.contains_key(&p.part_id) {
                issues.push(problem(
                    job,
                    "missing_part",
                    &format!("/panels/{i}/part_id"),
                    "板片所属构件不存在。",
                ));
            }
            if p.thickness_mm.value() <= 0.0 {
                issues.push(problem(
                    job,
                    "nonpositive_thickness",
                    &format!("/panels/{i}/thickness_mm"),
                    "板厚必须为正。",
                ));
            }
            if !materials
                .get(&p.material_ref)
                .is_some_and(|&m| job.materials[m].thickness_mm == p.thickness_mm)
            {
                issues.push(problem(
                    job,
                    "material_thickness",
                    &format!("/panels/{i}/material_ref"),
                    "必须存在厚度一致的显式认可材料。",
                ));
            }
            if let Err(e) = crate::exact::polygon(&p.polygon_mm) {
                issues.push(problem(
                    job,
                    e,
                    &format!("/panels/{i}/polygon_mm"),
                    "需要有限简单非退化平面多边形。",
                ));
            }
            if !pose_input_valid(&p.flat_pose, angle) {
                issues.push(problem(
                    job,
                    "invalid_pose",
                    &format!("/panels/{i}/flat_pose"),
                    "姿态须使用正确单位、坐标、单位缩放及单位四元数。",
                ));
            }
        }
        let mut part_for = vec![0; job.panels.len()];
        let mut occurrences = vec![0; job.panels.len()];
        for (i, p) in job.parts.iter().enumerate() {
            if !pose_input_valid(&p.placement, angle) {
                issues.push(problem(
                    job,
                    "invalid_pose",
                    &format!("/parts/{i}/placement"),
                    "构件姿态无效。",
                ));
            }
            if !p.panel_ids.contains(&p.root_panel) {
                issues.push(problem(
                    job,
                    "missing_root",
                    &format!("/parts/{i}/root_panel"),
                    "根板必须属于实际构件。",
                ));
            }
            for id in &p.panel_ids {
                if let Some(&j) = panels.get(id) {
                    occurrences[j] += 1;
                    part_for[j] = i;
                    if job.panels[j].part_id != p.id {
                        issues.push(problem(
                            job,
                            "part_membership",
                            &format!("/parts/{i}/panel_ids"),
                            "板所属构件与构件清单不一致。",
                        ));
                    }
                } else {
                    issues.push(problem(
                        job,
                        "missing_panel",
                        &format!("/parts/{i}/panel_ids"),
                        "构件引用未知板片。",
                    ));
                }
            }
        }
        if occurrences.iter().any(|&n| n != 1) {
            issues.push(problem(
                job,
                "part_membership",
                "/parts",
                "每个实际板片须恰属一个构件，重复件须显式建板并重绑引用。",
            ));
        }
        let mut parent = vec![None; job.panels.len()];
        for (i, b) in job.fold_bands.iter().enumerate() {
            let path = format!("/fold_bands/{i}");
            let (Some(&a), Some(&c)) = (panels.get(&b.panel_a), panels.get(&b.panel_b)) else {
                issues.push(problem(job, "missing_panel", &path, "折带引用未知板片。"));
                continue;
            };
            if a == c
                || job.panels[a].part_id != job.panels[c].part_id
                || parent[c].replace((a, i)).is_some()
            {
                issues.push(problem(
                    job,
                    "fold_tree",
                    &path,
                    "折带必须是同构件单父有向树。",
                ));
            }
            let d = minus(vector(&b.axis_end_mm), vector(&b.axis_start_mm));
            if length(d).lo <= 0.0
                || b.width_mm.value() <= 0.0
                || b.allowed_range_rad[0].value() > b.allowed_range_rad[1].value()
                || b.target_angle_rad.value() < b.allowed_range_rad[0].value()
                || b.target_angle_rad.value() > b.allowed_range_rad[1].value()
            {
                issues.push(problem(
                    job,
                    "invalid_fold_band",
                    &path,
                    "轴须非零，带宽为正，目标角在有序认可范围内。",
                ));
            }
        }
        for (i, p) in job.parts.iter().enumerate() {
            let Some(&root) = panels.get(&p.root_panel) else {
                continue;
            };
            if parent[root].is_some() {
                issues.push(problem(
                    job,
                    "fold_tree",
                    &format!("/parts/{i}/root_panel"),
                    "根板不能有父折带。",
                ));
            }
            for id in &p.panel_ids {
                let Some(&j) = panels.get(id) else {
                    continue;
                };
                let mut seen = BTreeSet::new();
                let mut cur = j;
                while let Some((a, _)) = parent[cur] {
                    if !seen.insert(cur) {
                        break;
                    }
                    cur = a;
                }
                if cur != root || seen.contains(&cur) {
                    issues.push(problem(
                        job,
                        "fold_tree",
                        &format!("/parts/{i}/panel_ids"),
                        "每板须经唯一无环路径到声明根板。",
                    ));
                }
            }
        }
        let mut order = vec![];
        let mut done = BTreeSet::new();
        while order.len() < job.steps.len() {
            let next = (0..job.steps.len()).find(|i| {
                !done.contains(i)
                    && job.steps[*i]
                        .prerequisites
                        .iter()
                        .all(|r| steps.get(r).is_some_and(|n| done.contains(n)))
            });
            let Some(i) = next else {
                issues.push(problem(
                    job,
                    "step_graph",
                    "/steps",
                    "步骤前置须存在且无环；同层按输入顺序执行。",
                ));
                break;
            };
            done.insert(i);
            order.push(i);
        }
        for (i, s) in job.steps.iter().enumerate() {
            let valid = match &s.motion {
                PackagingMotion::Fold(m) => bands.contains_key(&m.fold_band),
                PackagingMotion::RigidTranslation(m) => {
                    parts.contains_key(&m.part_id) && pose_input_valid(&m.start_pose, angle)
                }
                PackagingMotion::RigidRotation(m) => {
                    parts.contains_key(&m.part_id) && length(vector(&m.axis)).lo > 0.0
                }
                PackagingMotion::FixedPose(m) => {
                    parts.contains_key(&m.part_id) && pose_input_valid(&m.pose, angle)
                }
            };
            if !valid {
                issues.push(problem(
                    job,
                    "invalid_motion",
                    &format!("/steps/{i}/motion"),
                    "运动引用、轴或起始/保持姿态无效。",
                ));
            }
        }
        for (i, a) in job.conditions.allowed_contacts.iter().enumerate() {
            let path = format!("/conditions/allowed_contacts/{i}");
            if a.panel_a == a.panel_b
                || !panels.contains_key(&a.panel_a)
                || !panels.contains_key(&a.panel_b)
                || a.active_step_ids.is_empty()
                || a.active_step_ids.iter().any(|id| !steps.contains_key(id))
                || a.basis.trim().is_empty()
            {
                issues.push(problem(
                    job,
                    "invalid_allowance",
                    &path,
                    "局部许可须配对不同实际板、非空有效步骤和明确依据。",
                ));
            }
            for poly in [&a.polygon_a_mm, &a.polygon_b_mm] {
                if let Err(e) = crate::exact::polygon(poly) {
                    issues.push(problem(job, e, &path, "许可边界须是实际简单多边形。"));
                }
            }
        }
        for (i, c) in job.connections.iter().enumerate() {
            if c.allow_contact && c.contact_allowance_refs.is_empty()
                || !c.allow_contact && !c.contact_allowance_refs.is_empty()
            {
                issues.push(problem(
                    job,
                    "contact_permission_contract",
                    &format!("/connections/{i}/contact_allowance_refs"),
                    "allow_contact=false必须没有许可引用；true必须明确引用该连接板对的真实局部许可。",
                ));
            }
            if c.panel_a == c.panel_b
                || !panels.contains_key(&c.panel_a)
                || !panels.contains_key(&c.panel_b)
                || !pose_input_valid(&c.frame_a, angle)
                || !pose_input_valid(&c.frame_b, angle)
                || c.engage_after_step_id.as_ref().is_some_and(|x| {
                    !steps.get(x).is_some_and(|&j| {
                        matches!(job.steps[j].purpose, PackagingStepPurpose::Forming)
                    })
                })
                || c.contact_allowance_refs.iter().any(|id| {
                    !allowances.get(id).is_some_and(|&j| {
                        let a = &job.conditions.allowed_contacts[j];
                        a.panel_a == c.panel_a && a.panel_b == c.panel_b
                            || a.panel_a == c.panel_b && a.panel_b == c.panel_a
                    })
                })
            {
                issues.push(problem(
                    job,
                    "invalid_connection",
                    &format!("/connections/{i}"),
                    "固定连接的双面框架、板对、成型激活步骤或许可引用无效。",
                ));
            }
        }
        if let Some(p) = &job.conditions.product_envelope {
            if p.clearance_mm.value() < 0.0
                || p.allowed_poses.iter().any(|x| !pose_input_valid(x, angle))
                || p.subject.id != job.subject_ref.id
                || p.subject.revision != job.subject_ref.revision
            {
                issues.push(problem(
                    job,
                    "invalid_product_envelope",
                    "/conditions/product_envelope",
                    "包络须与当前产品修订一致，余量非负且认可姿态有效。",
                ));
            }
        }
        if let Some(e) = &job.conditions.extraction {
            if !parts.contains_key(&e.part_ref)
                || !pose_input_valid(&e.product_pose, angle)
                || length(vector(&e.direction)).lo <= 0.0
                || e.distance_mm.value() < 0.0
            {
                issues.push(problem(
                    job,
                    "invalid_extraction",
                    "/conditions/extraction",
                    "取物须指向实际构件、明确姿态、非零方向及非负距离。",
                ));
            }
        }
        if !issues.is_empty() {
            return Err(issues);
        }
        let mut allowance_groups = BTreeSet::new();
        for a in &job.conditions.allowed_contacts {
            let i = panels[&a.panel_a];
            let j = panels[&a.panel_b];
            for step in &a.active_step_ids {
                allowance_groups.insert((i.min(j), i.max(j), step.clone()));
            }
        }
        for (i, j, step) in allowance_groups {
            if let Err(code) = crate::collision::panel_cells(job, i, j, Some(&step)) {
                issues.push(problem(
                    job,
                    code,
                    "/conditions/allowed_contacts",
                    "局部许可须真实位于板面、深度合法，且合并后两板都保留正体积未许可残余。",
                ));
            }
        }
        if !issues.is_empty() {
            return Err(issues);
        }
        let flat_exact: Vec<_> = job
            .panels
            .iter()
            .map(|p| ExactTf::from_pose(&p.flat_pose))
            .collect();
        let flat = flat_exact.iter().map(Tf::from_exact).collect();
        let relative = parent
            .iter()
            .enumerate()
            .map(|(i, p)| {
                p.map(|(a, _)| Tf::from_exact(&flat_exact[a].inverse().compose(&flat_exact[i])))
                    .unwrap_or(Tf::ID)
            })
            .collect();
        Ok(Self {
            job,
            panels,
            parts,
            bands,
            parent,
            part_for,
            flat,
            relative,
            order,
        })
    }
    pub fn initial(&self) -> State {
        State {
            roots: self
                .job
                .parts
                .iter()
                .map(|p| Tf::pose(&p.placement))
                .collect(),
            angles: vec![0.0; self.job.fold_bands.len()],
        }
    }
    pub fn poses(&self, state: &State, step: Option<&PackagingStep>, u: Iv) -> Vec<Tf> {
        let mut roots = state.roots.clone();
        let mut angles: Vec<_> = state.angles.iter().copied().map(Iv::point).collect();
        if let Some(step) = step {
            match &step.motion {
                PackagingMotion::Fold(m) => {
                    angles[self.bands[&m.fold_band]] = Iv::point(m.from_angle_rad.value())
                        + (Iv::point(m.to_angle_rad.value()) - Iv::point(m.from_angle_rad.value()))
                            * u;
                }
                PackagingMotion::RigidTranslation(m) => {
                    let i = self.parts[&m.part_id];
                    roots[i].t = plus(roots[i].t, scale(vector(&m.displacement_mm), u));
                }
                PackagingMotion::RigidRotation(m) => {
                    let i = self.parts[&m.part_id];
                    let delta = (Iv::point(m.to_angle_rad.value())
                        - Iv::point(m.from_angle_rad.value()))
                        * u;
                    roots[i] =
                        Tf::about(vector(&m.pivot_mm), vector(&m.axis), delta).compose(roots[i]);
                }
                PackagingMotion::FixedPose(_) => {}
            }
        }
        fn panel(
            i: usize,
            m: &Model<'_>,
            roots: &[Tf],
            angles: &[Iv],
            out: &mut [Option<Tf>],
        ) -> Tf {
            if let Some(p) = out[i] {
                return p;
            }
            let p = if let Some((parent, b)) = m.parent[i] {
                let band = &m.job.fold_bands[b];
                panel(parent, m, roots, angles, out)
                    .compose(Tf::about(
                        vector(&band.axis_start_mm),
                        minus(vector(&band.axis_end_mm), vector(&band.axis_start_mm)),
                        angles[b],
                    ))
                    .compose(m.relative[i])
            } else {
                roots[m.part_for[i]].compose(m.flat[i])
            };
            out[i] = Some(p);
            p
        }
        let mut out = vec![None; self.job.panels.len()];
        (0..out.len())
            .map(|i| panel(i, self, &roots, &angles, &mut out))
            .collect()
    }
    pub fn moving(&self, step: &PackagingStep, i: usize) -> bool {
        match &step.motion {
            PackagingMotion::Fold(m) => {
                let child = self.panels[&self.job.fold_bands[self.bands[&m.fold_band]].panel_b];
                let mut c = i;
                loop {
                    if c == child {
                        return true;
                    }
                    let Some((p, _)) = self.parent[c] else {
                        return false;
                    };
                    c = p;
                }
            }
            PackagingMotion::RigidTranslation(m) => self.part_for[i] == self.parts[&m.part_id],
            PackagingMotion::RigidRotation(m) => self.part_for[i] == self.parts[&m.part_id],
            PackagingMotion::FixedPose(_) => false,
        }
    }
    pub fn advance(&self, state: &mut State, step: &PackagingStep) {
        match &step.motion {
            PackagingMotion::Fold(m) => {
                state.angles[self.bands[&m.fold_band]] = m.to_angle_rad.value()
            }
            PackagingMotion::RigidTranslation(m) => {
                let i = self.parts[&m.part_id];
                state.roots[i].t = plus(state.roots[i].t, vector(&m.displacement_mm));
            }
            PackagingMotion::RigidRotation(m) => {
                let i = self.parts[&m.part_id];
                state.roots[i] = Tf::about(
                    vector(&m.pivot_mm),
                    vector(&m.axis),
                    Iv::point(m.to_angle_rad.value()) - Iv::point(m.from_angle_rad.value()),
                )
                .compose(state.roots[i]);
            }
            PackagingMotion::FixedPose(_) => {}
        }
    }
}
