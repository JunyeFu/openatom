use crate::{
    PackagingArtifactContext, PackagingProduct, PackagingProgress,
    collision::*,
    exact,
    kinematics::*,
    manufacturing::{Manufacturing, manufacture},
    numeric::Iv,
    search,
};
use sf_domain::*;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};

fn merge(a: &CheckOutcome, b: &CheckOutcome) -> CheckOutcome {
    if matches!(a, CheckOutcome::Fail) || matches!(b, CheckOutcome::Fail) {
        CheckOutcome::Fail
    } else if matches!(a, CheckOutcome::Undetermined) || matches!(b, CheckOutcome::Undetermined) {
        CheckOutcome::Undetermined
    } else {
        CheckOutcome::Pass
    }
}
fn purpose_name(p: &PackagingStepPurpose) -> &'static str {
    match p {
        PackagingStepPurpose::Forming => "forming",
        PackagingStepPurpose::Opening => "opening",
        PackagingStepPurpose::NetSpace => "net_space",
        PackagingStepPurpose::LinearExtraction => "linear_extraction",
        PackagingStepPurpose::Manufacturing => "manufacturing",
    }
}
fn empty(p: PackagingStepPurpose) -> PackagingCheckResult {
    PackagingCheckResult {
        purpose: p,
        outcome: CheckOutcome::Pass,
        witnesses: vec![],
        unresolved: vec![],
        queries_used: uint(0),
    }
}
fn unknown(
    job: &PackageStructureJob,
    p: PackagingStepPurpose,
    code: &str,
    path: &str,
    text: &str,
) -> PackagingCheckResult {
    PackagingCheckResult {
        outcome: CheckOutcome::Undetermined,
        unresolved: vec![problem(job, code, path, text)],
        ..empty(p)
    }
}
fn combine(a: &mut PackagingCheckResult, b: PackagingCheckResult) {
    a.outcome = merge(&a.outcome, &b.outcome);
    a.witnesses.extend(b.witnesses);
    a.unresolved.extend(b.unresolved);
    a.queries_used = uint(a.queries_used.value() + b.queries_used.value());
}
fn pair_into(
    job: &PackageStructureJob,
    step: &ObjectId,
    pair: Option<[ObjectId; 2]>,
    res: PairResult,
    check: &mut PackagingCheckResult,
) {
    check.outcome = merge(&check.outcome, &res.outcome);
    if matches!(res.outcome, CheckOutcome::Fail) {
        check.witnesses.push(PackagingWitness {
            step_id: step.clone(),
            object_pair: pair,
            interval: res.interval.map(number),
            contact_point_mm: res.point.map(|p| Vec3 {
                x: number(p[0]),
                y: number(p[1]),
                z: number(p[2]),
            }),
            message: format!("{}; certified_time={:?}", res.reason, res.time),
        });
    }
    if matches!(res.outcome, CheckOutcome::Undetermined) {
        check.unresolved.push(problem(
            job,
            &res.reason,
            "/steps",
            format!(
                "步骤{}，未完成区间[{},{}]。",
                step.as_str(),
                res.interval[0],
                res.interval[1]
            ),
        ));
    }
}

fn frame_relation(a: Tf, b: Tf, distance: f64, angle: f64) -> CheckOutcome {
    if !a.finite() || !b.finite() {
        return CheckOutcome::Undetermined;
    }
    let d = length(minus(a.t, b.t));
    let mut r = Iv::ZERO;
    for i in 0..3 {
        for j in 0..3 {
            r = r + (a.r[i][j] - b.r[i][j]).square();
        }
    }
    let threshold = Iv::point(angle) * Iv::point(0.5);
    let chord = Iv::point(8.0) * threshold.sin().square();
    if d.valid() && chord.valid() && r.valid() {
        if d.lo > distance || r.lo > chord.hi {
            return CheckOutcome::Fail;
        }
        if d.hi <= distance && r.hi <= chord.lo {
            return CheckOutcome::Pass;
        }
    }
    CheckOutcome::Undetermined
}
fn connection_check(
    model: &Model<'_>,
    state: &State,
    step: Option<&PackagingStep>,
    index: usize,
    point: bool,
    tr: &mut Tracker<'_>,
) -> PairResult {
    let c = &model.job.connections[index];
    let a = model.panels[&c.panel_a];
    let b = model.panels[&c.panel_b];
    let tol = model.job.tolerance.as_ref().expect("explicit tolerance");
    let fixed = step.is_none_or(|s| model.moving(s, a) == model.moving(s, b));
    let mut stack = vec![[0.0, if point { 0.0 } else { 1.0 }]];
    while let Some([lo, hi]) = stack.pop() {
        if !tr.query() {
            return PairResult::unknown(
                [lo, hi],
                tr.usage.stopped_by.clone().unwrap_or("cancelled".into()),
            );
        }
        let time = if fixed { Iv::ZERO } else { Iv::new(lo, hi) };
        let ps = model.poses(state, step, time);
        let fa = ps[a].compose(Tf::pose(&c.frame_a));
        let fb = ps[b].compose(Tf::pose(&c.frame_b));
        match frame_relation(fa, fb, tol.constraint_mm.value(), tol.angle_rad.value()) {
            CheckOutcome::Pass => continue,
            CheckOutcome::Fail if point || fixed => {
                return PairResult {
                    outcome: CheckOutcome::Fail,
                    interval: [lo, hi],
                    time: Some(lo),
                    point: None,
                    reason: "fixed_connection_frame_violation".into(),
                };
            }
            _ => {}
        }
        let mid = lo / 2.0 + hi / 2.0;
        for t in [lo, mid, hi] {
            let ps = model.poses(state, step, Iv::point(t));
            if matches!(
                frame_relation(
                    ps[a].compose(Tf::pose(&c.frame_a)),
                    ps[b].compose(Tf::pose(&c.frame_b)),
                    tol.constraint_mm.value(),
                    tol.angle_rad.value()
                ),
                CheckOutcome::Fail
            ) {
                return PairResult {
                    outcome: CheckOutcome::Fail,
                    interval: [lo, hi],
                    time: Some(t),
                    point: None,
                    reason: "fixed_connection_frame_violation".into(),
                };
            }
        }
        if fixed || point || mid <= lo || mid >= hi {
            return PairResult::unknown([lo, hi], "connection_closure_precision_or_boundary");
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

fn panel_checks(
    model: &Model<'_>,
    state: &State,
    step: Option<&PackagingStep>,
    purpose: PackagingStepPurpose,
    tr: &mut Tracker<'_>,
) -> Result<PackagingCheckResult, Vec<Issue>> {
    let mut out = empty(purpose);
    let before = tr.usage.interval_queries.value();
    let sid = step.map(|s| &s.id).unwrap_or(&model.job.id);
    let tol = model.job.tolerance.as_ref().expect("explicit tolerance");
    'pairs: for i in 0..model.job.panels.len() {
        for j in i + 1..model.job.panels.len() {
            if !tr.alive() {
                let purpose = out.purpose.clone();
                combine(
                    &mut out,
                    unknown(
                        model.job,
                        purpose,
                        "cancelled",
                        "/steps",
                        "几何计算已取消。",
                    ),
                );
                break 'pairs;
            }
            let (a, b) = panel_cells(model.job, i, j, step.map(|s| &s.id)).map_err(|code| {
                vec![problem(
                    model.job,
                    code,
                    "/conditions/allowed_contacts",
                    "实际许可分割不满足区域/深度/残余体要求。",
                )]
            })?;
            let common = step.is_none_or(|s| model.moving(s, i) == model.moving(s, j));
            let pose = |u| {
                let ps = model.poses(state, step, if common { Iv::ZERO } else { u });
                (ps[i], ps[j])
            };
            for ca in &a {
                for cb in &b {
                    let result =
                        certify_pair(ca, cb, &pose, 0.0, tol.distance_mm.value(), common, tr);
                    pair_into(
                        model.job,
                        sid,
                        Some([
                            model.job.panels[i].id.clone(),
                            model.job.panels[j].id.clone(),
                        ]),
                        result,
                        &mut out,
                    );
                    if matches!(out.outcome, CheckOutcome::Fail) || tr.usage.stopped_by.is_some() {
                        break 'pairs;
                    }
                }
            }
        }
    }
    out.queries_used = uint(tr.usage.interval_queries.value() - before);
    Ok(out)
}
fn product_checks(
    model: &Model<'_>,
    state: &State,
    step: Option<&PackagingStep>,
    pose: &Pose,
    extraction: bool,
    purpose: PackagingStepPurpose,
    tr: &mut Tracker<'_>,
) -> Result<PackagingCheckResult, Vec<Issue>> {
    let job = model.job;
    let Some(envelope) = &job.conditions.product_envelope else {
        return Ok(unknown(
            job,
            purpose,
            "missing_product_envelope",
            "/conditions/product_envelope",
            "本用途需要实际产品保守凸包和认可姿态。",
        ));
    };
    let solid = match Solid::hull(&envelope.vertices_mm, tr) {
        Ok(solid) => solid,
        Err("cancelled") => {
            return Ok(unknown(
                job,
                purpose,
                "cancelled",
                "/conditions/product_envelope/vertices_mm",
                "产品凸包顶点/支持面遍历期间计算被取消，未完成部分保持Undetermined。",
            ));
        }
        Err(code) => {
            return Err(vec![problem(
                job,
                code,
                "/conditions/product_envelope/vertices_mm",
                "产品包络须含有三维非退化凸包。",
            )]);
        }
    };
    let tol = job.tolerance.as_ref().expect("explicit tolerance");
    let before = tr.usage.interval_queries.value();
    let mut out = empty(purpose);
    let sid = step.map(|s| &s.id).unwrap_or(&job.id);
    let product_tf = Tf::pose(pose);
    let direction = job.conditions.extraction.as_ref().map(|e| {
        let d = vector(&e.direction);
        scale(d, Iv::point(e.distance_mm.value()) / length(d))
    });
    'panels: for (i, panel) in job.panels.iter().enumerate() {
        let cells = exact::partition(
            &exact::polygon(&panel.polygon_mm).expect("validated polygon"),
            exact::q(panel.thickness_mm.value()),
            &[],
        )
        .map_err(|code| vec![problem(job, code, "/panels", "产品碰撞板体无法建立。")])?;
        for cell in &cells {
            let panel_solid = Solid::cell(cell);
            let poses = |u| {
                let ps = model.poses(state, step, u);
                let mut product = product_tf;
                if extraction {
                    product.t = plus(product.t, scale(direction.expect("explicit extraction"), u));
                }
                (ps[i], product)
            };
            let fixed = !extraction && step.is_none_or(|s| !model.moving(s, i));
            let res = certify_pair(
                &panel_solid,
                &solid,
                &poses,
                envelope.clearance_mm.value(),
                tol.distance_mm.value(),
                fixed,
                tr,
            );
            pair_into(
                job,
                sid,
                Some([panel.id.clone(), envelope.subject.id.clone()]),
                res,
                &mut out,
            );
            if matches!(out.outcome, CheckOutcome::Fail) || tr.usage.stopped_by.is_some() {
                break 'panels;
            }
        }
    }
    out.queries_used = uint(tr.usage.interval_queries.value() - before);
    Ok(out)
}

fn display_pose(t: Tf) -> Option<Pose> {
    if !t.finite() {
        return None;
    }
    let r = t.r.map(|row| row.map(Iv::midpoint));
    let trace = r[0][0] + r[1][1] + r[2][2];
    let [x, y, z, w] = if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        [
            (r[2][1] - r[1][2]) / s,
            (r[0][2] - r[2][0]) / s,
            (r[1][0] - r[0][1]) / s,
            s / 4.0,
        ]
    } else {
        let i = (0..3).max_by(|&i, &j| r[i][i].total_cmp(&r[j][j]))?;
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        let s = (1.0 + r[i][i] - r[j][j] - r[k][k]).max(0.0).sqrt() * 2.0;
        if s == 0.0 {
            return None;
        }
        let mut q = [0.0; 4];
        q[i] = s / 4.0;
        q[j] = (r[j][i] + r[i][j]) / s;
        q[k] = (r[k][i] + r[i][k]) / s;
        q[3] = (r[k][j] - r[j][k]) / s;
        q
    };
    let norm = (x * x + y * y + z * z + w * w).sqrt();
    if !norm.is_finite() || norm == 0.0 {
        return None;
    }
    Some(Pose {
        translation: Vec3 {
            x: number(t.t[0].midpoint()),
            y: number(t.t[1].midpoint()),
            z: number(t.t[2].midpoint()),
        },
        rotation: Quaternion {
            x: number(x / norm),
            y: number(y / norm),
            z: number(z / norm),
            w: number(w / norm),
        },
        scale: Vec3 {
            x: number(1.0),
            y: number(1.0),
            z: number(1.0),
        },
        unit: LengthUnit::Millimeter,
        coordinate_frame: CoordinateFrame::RightHandedZUp,
    })
}
fn sample(
    model: &Model<'_>,
    state: &State,
    step: &PackagingStep,
    u: f64,
) -> Option<PackagingProcessSample> {
    let poses = model.poses(state, Some(step), Iv::point(u));
    let panel_poses = model
        .job
        .panels
        .iter()
        .zip(poses)
        .map(|(p, t)| {
            Some(PackagingPanelPose {
                panel_id: p.id.clone(),
                pose: display_pose(t)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(PackagingProcessSample {
        step_id: step.id.clone(),
        progress: number(u),
        panel_poses,
    })
}

struct CandidateResult {
    checks: Vec<PackagingCheckResult>,
    samples: Vec<PackagingProcessSample>,
    manufacturing: Manufacturing,
    limitations: Vec<Issue>,
}
fn evaluate(
    job: &PackageStructureJob,
    ctx: &PackagingArtifactContext,
    tr: &mut Tracker<'_>,
) -> Result<CandidateResult, Vec<Issue>> {
    let model = Model::build(job)?;
    let mut state = model.initial();
    let mut checks = vec![];
    let mut samples = vec![];
    let mut activations = BTreeMap::new();
    let mut active = vec![];
    let mut limitations = job.issues.clone();
    let tol = job.tolerance.as_ref().expect("explicit profile");
    for (i, c) in job
        .connections
        .iter()
        .enumerate()
        .filter(|(_, c)| c.engage_after_step_id.is_none())
    {
        let before_connection = tr.usage.interval_queries.value();
        let res = connection_check(&model, &state, None, i, true, tr);
        let pass = matches!(res.outcome, CheckOutcome::Pass);
        let mut check = empty(PackagingStepPurpose::Forming);
        pair_into(
            job,
            &job.id,
            Some([c.panel_a.clone(), c.panel_b.clone()]),
            res,
            &mut check,
        );
        check.queries_used = uint(tr.usage.interval_queries.value() - before_connection);
        checks.push(check);
        if pass {
            active.push(i);
            activations.insert(i, model.poses(&state, None, Iv::ZERO));
        }
    }
    let mut forming = empty(PackagingStepPurpose::Forming);
    let mut opening = empty(PackagingStepPurpose::Opening);
    let mut forming_count = 0;
    let mut opening_count = 0;
    let mut net_done = false;
    let mut selected_pose: Option<Pose> = None;
    let do_net = |state: &State,
                  tr: &mut Tracker<'_>|
     -> Result<(PackagingCheckResult, Option<Pose>), Vec<Issue>> {
        let Some(p) = &job.conditions.product_envelope else {
            return Ok((
                unknown(
                    job,
                    PackagingStepPurpose::NetSpace,
                    "missing_product_envelope",
                    "/conditions/product_envelope",
                    "净空间需要明确产品包络。",
                ),
                None,
            ));
        };
        let choices: Vec<_> = if job.conditions.extraction_required {
            if let Some(e) = &job.conditions.extraction {
                vec![&e.product_pose]
            } else {
                vec![]
            }
        } else {
            p.allowed_poses.iter().collect()
        };
        if choices.is_empty() {
            return Ok((
                unknown(
                    job,
                    PackagingStepPurpose::NetSpace,
                    "missing_allowed_pose",
                    "/conditions/product_envelope/allowed_poses",
                    "需要至少一个明确认可的产品姿态。",
                ),
                None,
            ));
        }
        let mut failures = empty(PackagingStepPurpose::NetSpace);
        let mut unknown_pose = false;
        for pose in choices {
            if !p.allowed_poses.iter().any(|p| {
                matches!(
                    frame_relation(
                        Tf::pose(p),
                        Tf::pose(pose),
                        tol.constraint_mm.value(),
                        tol.angle_rad.value()
                    ),
                    CheckOutcome::Pass
                )
            }) {
                return Ok((
                    unknown(
                        job,
                        PackagingStepPurpose::NetSpace,
                        "extraction_pose_not_approved",
                        "/conditions/extraction/product_pose",
                        "固定取物姿态必须属于当前认可姿态。",
                    ),
                    None,
                ));
            }
            let result = product_checks(
                &model,
                state,
                None,
                pose,
                false,
                PackagingStepPurpose::NetSpace,
                tr,
            )?;
            if matches!(result.outcome, CheckOutcome::Pass) {
                return Ok((result, Some(pose.clone())));
            }
            unknown_pose |= matches!(result.outcome, CheckOutcome::Undetermined);
            combine(&mut failures, result);
            if tr.usage.stopped_by.is_some() {
                break;
            }
        }
        if unknown_pose {
            failures.outcome = CheckOutcome::Undetermined;
        }
        Ok((failures, None))
    };
    let mut prefix_pass = checks
        .iter()
        .all(|c| matches!(c.outcome, CheckOutcome::Pass));
    for &si in &model.order {
        if !tr.alive() || tr.usage.stopped_by.is_some() {
            limitations.push(problem(
                job,
                "unfinished_process",
                "/steps",
                "过程前缀之后尚未执行，不补成完整终态。",
            ));
            break;
        }
        let step = &job.steps[si];
        tr.phase(format!(
            "{}:{}",
            purpose_name(&step.purpose),
            step.id.as_str()
        ));
        if matches!(step.purpose, PackagingStepPurpose::Opening)
            && !net_done
            && (job.conditions.net_space_required || job.conditions.extraction_required)
        {
            let (c, p) = do_net(&state, tr)?;
            checks.push(c);
            selected_pose = p;
            net_done = true;
        }
        let before_step = tr.usage.interval_queries.value();
        let continuity = match &step.motion {
            PackagingMotion::Fold(m) => {
                let bi = model.bands[&m.fold_band];
                let b = &job.fold_bands[bi];
                if (state.angles[bi] - m.from_angle_rad.value()).abs() > tol.angle_rad.value()
                    || m.from_angle_rad.value().min(m.to_angle_rad.value())
                        < b.allowed_range_rad[0].value()
                    || m.from_angle_rad.value().max(m.to_angle_rad.value())
                        > b.allowed_range_rad[1].value()
                {
                    return Err(vec![problem(
                        job,
                        "fold_step_continuity",
                        &format!("/steps/{si}/motion"),
                        "from角须衔接当前关节，完整路径须在认可范围内。",
                    )]);
                }
                CheckOutcome::Pass
            }
            PackagingMotion::RigidTranslation(m) => frame_relation(
                state.roots[model.parts[&m.part_id]],
                Tf::pose(&m.start_pose),
                tol.constraint_mm.value(),
                tol.angle_rad.value(),
            ),
            PackagingMotion::FixedPose(m) => frame_relation(
                state.roots[model.parts[&m.part_id]],
                Tf::pose(&m.pose),
                tol.constraint_mm.value(),
                tol.angle_rad.value(),
            ),
            PackagingMotion::RigidRotation(_) => CheckOutcome::Pass,
        };
        let mut result = panel_checks(&model, &state, Some(step), step.purpose.clone(), tr)?;
        if matches!(continuity, CheckOutcome::Fail) {
            pair_into(
                job,
                &step.id,
                None,
                PairResult {
                    outcome: CheckOutcome::Fail,
                    interval: [0.0, 0.0],
                    time: Some(0.0),
                    point: None,
                    reason: "root_pose_continuity_constraint_violation_no_teleport".into(),
                },
                &mut result,
            );
        }
        if matches!(continuity, CheckOutcome::Undetermined) {
            combine(
                &mut result,
                unknown(
                    job,
                    step.purpose.clone(),
                    "root_pose_precision",
                    &format!("/steps/{si}/motion"),
                    "姿态衔接的定点包围未取得证书。",
                ),
            );
        }
        for &ci in &active {
            let c = &job.connections[ci];
            let res = connection_check(&model, &state, Some(step), ci, false, tr);
            pair_into(
                job,
                &step.id,
                Some([c.panel_a.clone(), c.panel_b.clone()]),
                res,
                &mut result,
            );
            if tr.usage.stopped_by.is_some() {
                break;
            }
        }
        if matches!(step.motion, PackagingMotion::FixedPose(_))
            && job.conditions.product_envelope.is_some()
            && !(matches!(step.purpose, PackagingStepPurpose::Opening) && selected_pose.is_some())
        {
            let envelope = job
                .conditions
                .product_envelope
                .as_ref()
                .expect("explicit envelope");
            let mut held_product: Option<PackagingCheckResult> = None;
            let mut unknown_held_pose = false;
            for pose in &envelope.allowed_poses {
                let check = product_checks(
                    &model,
                    &state,
                    Some(step),
                    pose,
                    false,
                    step.purpose.clone(),
                    tr,
                )?;
                if matches!(check.outcome, CheckOutcome::Pass) {
                    held_product = Some(check);
                    break;
                }
                unknown_held_pose |= matches!(check.outcome, CheckOutcome::Undetermined);
                if let Some(previous) = &mut held_product {
                    combine(previous, check);
                } else {
                    held_product = Some(check);
                }
                if tr.usage.stopped_by.is_some() {
                    break;
                }
            }
            if unknown_held_pose {
                if let Some(check) = &mut held_product {
                    if !matches!(check.outcome, CheckOutcome::Pass) {
                        check.outcome = CheckOutcome::Undetermined;
                    }
                }
            }
            combine(
                &mut result,
                held_product.unwrap_or_else(|| {
                    unknown(
                        job,
                        step.purpose.clone(),
                        "missing_allowed_pose",
                        "/conditions/product_envelope/allowed_poses",
                        "保持步骤保留明确产品障碍，需要实际认可姿态。",
                    )
                }),
            );
        }
        if matches!(step.purpose, PackagingStepPurpose::Opening) {
            if let Some(pose) = &selected_pose {
                combine(
                    &mut result,
                    product_checks(
                        &model,
                        &state,
                        Some(step),
                        pose,
                        false,
                        PackagingStepPurpose::Opening,
                        tr,
                    )?,
                );
            } else if job.conditions.product_envelope.is_some() {
                combine(
                    &mut result,
                    unknown(
                        job,
                        PackagingStepPurpose::Opening,
                        "missing_verified_product_pose",
                        "/conditions/product_envelope/allowed_poses",
                        "未取得净空间通过的产品姿态，开合不能声称对产品无干涉。",
                    ),
                );
            }
        }
        let fully = tr.usage.stopped_by.is_none();
        if fully {
            for u in [0.0, 0.5, 1.0] {
                if let Some(s) = sample(&model, &state, step, u) {
                    samples.push(s);
                } else {
                    limitations.push(problem(
                        job,
                        "process_pose_range",
                        "/steps",
                        "实际过程位姿超出有限输出范围，未伪造样本。",
                    ));
                }
            }
        } else if let Some(s) = sample(&model, &state, step, 0.0) {
            samples.push(s);
        }
        let step_pass = matches!(result.outcome, CheckOutcome::Pass);
        prefix_pass &= step_pass;
        model.advance(&mut state, step);
        for (ci, c) in job
            .connections
            .iter()
            .enumerate()
            .filter(|(_, c)| c.engage_after_step_id.as_ref() == Some(&step.id))
        {
            if prefix_pass && fully {
                let res = connection_check(&model, &state, None, ci, true, tr);
                let pass = matches!(res.outcome, CheckOutcome::Pass);
                pair_into(
                    job,
                    &step.id,
                    Some([c.panel_a.clone(), c.panel_b.clone()]),
                    res,
                    &mut result,
                );
                if pass {
                    active.push(ci);
                    activations.insert(ci, model.poses(&state, None, Iv::ZERO));
                } else {
                    prefix_pass = false;
                }
            } else {
                combine(
                    &mut result,
                    unknown(
                        job,
                        step.purpose.clone(),
                        "connection_activation_unverified",
                        &format!("/connections/{ci}"),
                        "连接只能在整个此前过程与本步骤通过且端点闭合后激活。",
                    ),
                );
            }
        }
        result.queries_used = uint(tr.usage.interval_queries.value() - before_step);
        match step.purpose {
            PackagingStepPurpose::Forming => {
                forming_count += 1;
                combine(&mut forming, result);
            }
            PackagingStepPurpose::Opening => {
                opening_count += 1;
                combine(&mut opening, result);
            }
            _ => checks.push(result),
        }
        if !fully {
            break;
        }
    }
    if job.conditions.forming_required && forming_count == 0 {
        combine(
            &mut forming,
            unknown(
                job,
                PackagingStepPurpose::Forming,
                "missing_forming_steps",
                "/steps",
                "请求成型但没有实际成型步骤。",
            ),
        );
    }
    if job.conditions.opening_required && opening_count == 0 {
        combine(
            &mut opening,
            unknown(
                job,
                PackagingStepPurpose::Opening,
                "missing_opening_steps",
                "/steps",
                "请求开合但没有实际开合步骤。",
            ),
        );
    }
    if forming_count > 0 || job.conditions.forming_required {
        checks.push(forming);
    }
    if opening_count > 0 || job.conditions.opening_required {
        checks.push(opening);
    }
    if !net_done && (job.conditions.net_space_required || job.conditions.extraction_required) {
        let (c, p) = do_net(&state, tr)?;
        checks.push(c);
        selected_pose = p;
    }
    if job.conditions.extraction_required {
        if let Some(e) = &job.conditions.extraction {
            if tr.usage.stopped_by.is_none() {
                checks.push(product_checks(
                    &model,
                    &state,
                    None,
                    &e.product_pose,
                    true,
                    PackagingStepPurpose::LinearExtraction,
                    tr,
                )?);
            } else {
                checks.push(unknown(
                    job,
                    PackagingStepPurpose::LinearExtraction,
                    "unfinished_extraction",
                    "/conditions/extraction",
                    "预算/取消已停止后续取物过程。",
                ));
            }
        } else {
            checks.push(unknown(
                job,
                PackagingStepPurpose::LinearExtraction,
                "missing_extraction_plan",
                "/conditions/extraction",
                "固定姿态直线取物需要明确完整计划。",
            ));
        }
    }
    // The product pose is a particular approved possibility; it is not an
    // automatically optimized orientation or arbitrary extraction path.
    if let Some(p) = selected_pose {
        limitations.push(Issue {
            code: "packaging.approved_pose_scope".into(),
            object_ref: Some(ObjectRef {
                kind: ObjectKind::Job,
                id: job.id.clone(),
            }),
            field_path: Some("/conditions/product_envelope/allowed_poses".into()),
            message: format!(
                "净空间/开合使用认可姿态平移({}, {}, {})mm；不是任意姿态保证。",
                p.translation.x.value(),
                p.translation.y.value(),
                p.translation.z.value()
            ),
            blocks: vec![],
        });
    }
    let manufacturing = manufacture(&model, &activations, ctx, tr);
    checks.push(manufacturing.check.clone());
    let mut merged: Vec<PackagingCheckResult> = vec![];
    for c in checks {
        if let Some(old) = merged
            .iter_mut()
            .find(|x| purpose_name(&x.purpose) == purpose_name(&c.purpose))
        {
            combine(old, c);
        } else {
            merged.push(c);
        }
    }
    Ok(CandidateResult {
        checks: merged,
        samples,
        manufacturing,
        limitations,
    })
}

fn finish(
    selected_job: PackageStructureJob,
    ctx: PackagingArtifactContext,
    checks: Vec<PackagingCheckResult>,
    samples: Vec<PackagingProcessSample>,
    files: Vec<ManufacturingFile>,
    counts: Vec<MaterialCount>,
    bytes: Vec<(FileRef, Vec<u8>)>,
    usage: BudgetUsage,
    limitations: Vec<Issue>,
    cancelled: bool,
    unresolved_search: bool,
) -> PackagingProduct {
    let mut outcome = if checks.is_empty() {
        CheckOutcome::Undetermined
    } else {
        CheckOutcome::Pass
    };
    for c in &checks {
        outcome = merge(&outcome, &c.outcome);
    }
    if limitations.iter().any(|i| {
        i.blocks
            .iter()
            .any(|b| matches!(b, CapabilityPurpose::CompleteConclusion))
    }) && !matches!(outcome, CheckOutcome::Fail)
    {
        outcome = CheckOutcome::Undetermined;
    }
    // A failed concrete candidate is not a proof that an unexplored structural
    // range is infeasible. Its check-level witnesses remain unchanged.
    if unresolved_search {
        outcome = CheckOutcome::Undetermined;
    }
    let mut scope = vec!["editable_job".into()];
    scope.extend(
        checks
            .iter()
            .filter(|c| matches!(c.outcome, CheckOutcome::Pass))
            .map(|c| purpose_name(&c.purpose).into()),
    );
    let result = PackagingResult {
        evidence_origin: EvidenceOrigin::ActualSoftware,
        artifact_id: ctx.artifact_id.clone(),
        job_ref: JobInputRevisionRef {
            id: selected_job.id.clone(),
            revision: selected_job.revision.clone(),
        },
        subject_ref: selected_job.subject_ref.clone(),
        outcome: outcome.clone(),
        checks,
        process_poses: samples,
        manufacturing_files: files,
        material_counts: counts,
        budget_used: usage,
        supported_scope: scope.clone(),
        limitations,
    };
    let artifact = ArtifactRecord {
        evidence_origin: EvidenceOrigin::ActualSoftware,
        id: ctx.artifact_id,
        artifact_type: "paper_packaging".into(),
        schema_version: SchemaVersion::V1,
        subject_ref: selected_job.subject_ref.clone(),
        input_ref: ctx.input_ref,
        result_ref: ctx.result_ref,
        unit: "millimeter".to_string().try_into().expect("literal unit"),
        coordinate_frame: CoordinateFrame::RightHandedZUp,
        supported_scope: scope,
        actual_outcome: outcome,
    };
    PackagingProduct {
        selected_job,
        result,
        artifact,
        manufacturing_bytes: bytes,
        provider: ProviderCapability {
            provider_id: "superfactory.paper-packaging"
                .to_string()
                .try_into()
                .expect("literal provider"),
            available_modes: vec![ProfessionalMode::CalculatedNominal],
            producer_status: ProviderStatus::Available,
            supported_input_schema: "PackageStructureJob@1.0".into(),
            supported_scope: vec![
                "editable_job".into(),
                "forming".into(),
                "opening".into(),
                "linear_extraction".into(),
                "net_space".into(),
                "manufacturing".into(),
                "finite_simple_planar_panels_with_declared_thickness".into(),
                "rooted_fold_tree_and_explicit_rigid_process".into(),
                "interval_certified_pass_strict_witness_fail_and_undetermined".into(),
                "actual_flat_cut_fold_glue_and_explicit_part_material_counts".into(),
            ],
        },
        cancelled,
    }
}

/// Compute an editable frozen candidate and its real three-state result. No
/// filesystem writes, approval, consumer mutation, stock or factory time here.
pub fn compute_packaging(
    job: &PackageStructureJob,
    budget: &CalculationBudget,
    context: PackagingArtifactContext,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(PackagingProgress),
) -> Result<PackagingProduct, Vec<Issue>> {
    if !matches!(context.input_ref.kind, DocumentKind::PackagingJob)
        || !matches!(context.result_ref.kind, DocumentKind::PackagingResult)
    {
        return Err(vec![problem(
            job,
            "artifact_reference_kind",
            "/",
            "Host必须明确提供PackagingJob输入和PackagingResult输出文件引用。",
        )]);
    }
    if let Some(b) = &job.budget {
        if b.candidate_limit != budget.candidate_limit
            || b.interval_query_limit != budget.interval_query_limit
            || b.subdivision_limit != budget.subdivision_limit
            || b.basis != budget.basis
        {
            return Err(vec![problem(
                job,
                "budget_mismatch",
                "/budget",
                "冻结Job预算与显式请求预算必须一致。",
            )]);
        }
    }
    let mut selected = job.clone();
    selected.budget = Some(budget.clone());
    let mut tr = Tracker::new(budget, cancelled, progress);
    tr.phase("validate_input");
    let early = if selected.panels.is_empty() || selected.parts.is_empty() {
        Some((
            "incomplete_structure",
            "/panels",
            "空白可编辑结构尚无完整板片/构件，不能产生完整Pass。",
        ))
    } else if selected.tolerance.is_none() {
        Some((
            "missing_tolerance",
            "/tolerance",
            "需要显式采用数值容差；不静默补默认值。",
        ))
    } else if selected.issues.iter().any(|i| {
        i.blocks
            .iter()
            .any(|p| matches!(p, CapabilityPurpose::Execution))
    }) {
        Some((
            "unresolved_execution_input",
            "/issues",
            "输入仍有明确阻止执行的条件，不能用计算抹去。",
        ))
    } else {
        None
    };
    if let Some((code, path, text)) = early {
        let check = unknown(
            &selected,
            PackagingStepPurpose::Manufacturing,
            code,
            path,
            text,
        );
        let limitations = selected.issues.clone();
        return Ok(finish(
            selected,
            context,
            vec![check],
            vec![],
            vec![],
            vec![],
            vec![],
            tr.usage.clone(),
            limitations,
            cancelled.load(Ordering::Relaxed),
            false,
        ));
    }
    let tolerance = selected.tolerance.as_ref().expect("checked");
    if [
        tolerance.distance_mm.value(),
        tolerance.angle_rad.value(),
        tolerance.constraint_mm.value(),
    ]
    .iter()
    .any(|&x| x <= 0.0)
        || tolerance.basis.trim().is_empty()
    {
        return Err(vec![problem(
            job,
            "invalid_tolerance",
            "/tolerance",
            "比较容差必须显式为正并说明依据。",
        )]);
    }
    Model::build(&selected)?;
    let mut candidates = search::Candidates::new(&selected)?;
    let mut last: Option<(PackageStructureJob, CandidateResult)> = None;
    let mut last_failure: Option<(PackageStructureJob, CandidateResult)> = None;
    let mut found_complete = false;
    let mut had_unknown = false;
    let mut rejected = vec![];
    while let Some(candidate) = candidates.next() {
        if !tr.alive() {
            break;
        }
        if let Err(issues) = search::constraints(&candidate) {
            rejected.extend(issues);
            continue;
        }
        if !tr.candidate() {
            break;
        }
        tr.phase("candidate");
        let evaluation = match evaluate(&candidate, &context, &mut tr) {
            Ok(r) => r,
            Err(issues) if !job.variables.is_empty() => {
                had_unknown = true;
                rejected.extend(issues);
                continue;
            }
            Err(issues) => return Err(issues),
        };
        let pass = evaluation
            .checks
            .iter()
            .all(|c| matches!(c.outcome, CheckOutcome::Pass))
            && !evaluation.limitations.iter().any(|i| {
                i.blocks
                    .iter()
                    .any(|p| matches!(p, CapabilityPurpose::CompleteConclusion))
            });
        had_unknown |= evaluation
            .checks
            .iter()
            .any(|c| matches!(c.outcome, CheckOutcome::Undetermined));
        if pass {
            found_complete = true;
            last = Some((candidate, evaluation));
        } else if evaluation
            .checks
            .iter()
            .any(|c| matches!(c.outcome, CheckOutcome::Fail))
        {
            last_failure = Some((candidate, evaluation));
        } else {
            last = Some((candidate, evaluation));
        }
        if pass || job.variables.is_empty() || tr.usage.stopped_by.is_some() {
            break;
        }
    }
    let selected_result = if found_complete {
        last
    } else if tr.usage.stopped_by.is_some() {
        last_failure.or(last)
    } else {
        last.or(last_failure)
    };
    let (selected, mut evaluation) = if let Some(last) = selected_result {
        last
    } else {
        let reason = tr
            .usage
            .stopped_by
            .clone()
            .unwrap_or("no_admissible_candidate".into());
        let check = unknown(
            &selected,
            PackagingStepPurpose::Manufacturing,
            &reason,
            "/variables",
            "没有取得一个完整可检查候选；不能宣称几何全局无解。",
        );
        let m = Manufacturing {
            check: check.clone(),
            files: vec![],
            bytes: vec![],
            counts: vec![],
        };
        (
            selected.clone(),
            CandidateResult {
                checks: vec![check],
                samples: vec![],
                manufacturing: m,
                limitations: vec![problem(
                    &selected,
                    &reason,
                    "/variables",
                    "没有满足显式范围和硬约束的已执行候选。",
                )],
            },
        )
    };
    let unresolved_search = !job.variables.is_empty()
        && !found_complete
        && !cancelled.load(Ordering::Relaxed)
        && (candidates.continuous || !candidates.exhausted() || had_unknown);
    if unresolved_search {
        evaluation.limitations.push(problem(&selected,"search_unresolved","/variables","逐目的反例属于当前交付的具体候选；仍有连续范围/未执行候选/未决结果，整个搜索请求保持Undetermined，不宣称全域无解。"));
    }
    if !job.variables.is_empty() {
        evaluation.limitations.push(Issue {
            code: "packaging.finite_search_scope".into(),
            object_ref: Some(ObjectRef {
                kind: ObjectKind::Job,
                id: job.id.clone(),
            }),
            field_path: Some("/variables".into()),
            message: format!(
                "实际检查{}个候选；{}。被硬条件排除的候选诊断{}项，结论只对交付的具体候选成立。",
                tr.usage.candidates_evaluated.value(),
                if candidates.continuous {
                    "连续范围未穷尽，不声明整个结构域无解"
                } else {
                    "整数范围按惰性升序枚举，预算停止保留未完成"
                },
                rejected.len()
            ),
            blocks: vec![],
        });
    }
    if tr.usage.stopped_by.is_some() {
        evaluation.limitations.push(problem(
            &selected,
            "calculation_stopped",
            "/budget",
            "全Job共用预算/取消已停止；已证反例保留，未执行目的不能升级Pass。",
        ));
    }
    tr.phase("result");
    let m = evaluation.manufacturing;
    Ok(finish(
        selected,
        context,
        evaluation.checks,
        evaluation.samples,
        m.files,
        m.counts,
        m.bytes,
        tr.usage.clone(),
        evaluation.limitations,
        cancelled.load(Ordering::Relaxed),
        unresolved_search,
    ))
}
