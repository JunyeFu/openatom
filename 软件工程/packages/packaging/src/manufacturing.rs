use crate::{
    PackagingArtifactContext,
    collision::Tracker,
    exact::{self, ExactTf, P, Poly, R, area2, convex_intersection, convex_parts, n, q},
    kinematics::*,
};
use num_traits::{Signed, ToPrimitive, Zero};
use sf_domain::*;
use std::collections::BTreeMap;

pub(crate) struct Manufacturing {
    pub check: PackagingCheckResult,
    pub files: Vec<ManufacturingFile>,
    pub bytes: Vec<(FileRef, Vec<u8>)>,
    pub counts: Vec<MaterialCount>,
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn xy(e: &ExactTf, poly: &Poly) -> Result<Poly, &'static str> {
    let points: Vec<_> = poly
        .iter()
        .map(|p| e.point(&[p[0].clone(), p[1].clone(), n(0)]))
        .collect();
    if points.iter().any(|p| !p[2].is_zero()) {
        return Err("noncoplanar_flat_geometry");
    }
    exact::polygon_q(
        points
            .into_iter()
            .map(|p| [p[0].clone(), p[1].clone()])
            .collect(),
    )
}
fn area_intersection(a: &Poly, b: &Poly) -> Result<R, &'static str> {
    Ok(convex_parts(a)?
        .iter()
        .flat_map(|x| {
            convex_parts(b)
                .expect("validated polygon")
                .into_iter()
                .map(|y| convex_intersection(x, &y))
        })
        .map(|p| area2(&p) / n(2))
        .sum())
}
fn segment_overlap(a: &P, b: &P, c: &P, d: &P) -> bool {
    if !exact::cross(a, b, c).is_zero() || !exact::cross(a, b, d).is_zero() {
        return false;
    }
    let k = if a[0] != b[0] { 0 } else { 1 };
    let lo = a[k]
        .clone()
        .min(b[k].clone())
        .max(c[k].clone().min(d[k].clone()));
    let hi = a[k]
        .clone()
        .max(b[k].clone())
        .min(c[k].clone().max(d[k].clone()));
    lo < hi
}
fn shares_edge(a: &Poly, b: &Poly) -> bool {
    (0..a.len()).any(|i| {
        (0..b.len())
            .any(|j| segment_overlap(&a[i], &a[(i + 1) % a.len()], &b[j], &b[(j + 1) % b.len()]))
    })
}
fn path(edges: &[(P, P)]) -> Option<String> {
    let mut out = String::new();
    for (a, b) in edges {
        let [ax, ay, bx, by] =
            [&a[0], &a[1], &b[0], &b[1]].map(|x| x.to_f64().filter(|v| v.is_finite()));
        let (Some(ax), Some(ay), Some(bx), Some(by)) = (ax, ay, bx, by) else {
            return None;
        };
        out.push_str(&format!("M {ax} {ay} L {bx} {by} "));
    }
    Some(out)
}

/// Manufacture from the actual flat material and actually activated connections.
/// Nonplanar or unresolved projected contacts retain Undetermined and no invented
/// glue rectangle is emitted.
pub(crate) fn manufacture(
    model: &Model<'_>,
    activations: &BTreeMap<usize, Vec<Tf>>,
    context: &PackagingArtifactContext,
    tr: &mut Tracker<'_>,
) -> Manufacturing {
    let job = model.job;
    let tol = job.tolerance.as_ref().expect("explicit profile");
    let mut out = Manufacturing {
        check: PackagingCheckResult {
            purpose: PackagingStepPurpose::Manufacturing,
            outcome: CheckOutcome::Pass,
            witnesses: vec![],
            unresolved: vec![],
            queries_used: uint(0),
        },
        files: vec![],
        bytes: vec![],
        counts: vec![],
    };
    let unresolved = |code: &str, path: &str, message: &str| problem(job, code, path, message);
    for (part_index, part) in job.parts.iter().enumerate() {
        if !tr.alive() {
            out.check.unresolved.push(unresolved(
                "cancelled",
                "/parts",
                "制造输出被取消；已生成内容保留为部分成果。",
            ));
            break;
        }
        tr.phase(format!("manufacturing:{}", part.id.as_str()));
        let root = &job.panels[model.panels[&part.root_panel]];
        let plane = ExactTf::from_pose(&root.flat_pose).inverse();
        let mut polygons = vec![];
        let mut polygon_panels = vec![];
        let mut flat = BTreeMap::new();
        let mut material_areas: BTreeMap<ObjectId, R> = BTreeMap::new();
        let mut local_issues = vec![];
        let mut fail = false;
        for id in &part.panel_ids {
            if !tr.alive() {
                break;
            }
            let panel = &job.panels[model.panels[id]];
            let source = exact::polygon(&panel.polygon_mm).expect("input validated");
            match xy(
                &plane.compose(&ExactTf::from_pose(&panel.flat_pose)),
                &source,
            ) {
                Ok(p) => {
                    *material_areas
                        .entry(panel.material_ref.clone())
                        .or_insert(n(0)) += area2(&p) / n(2);
                    flat.insert(panel.id.clone(), p.clone());
                    polygons.push(p);
                    polygon_panels.push(panel.id.clone());
                }
                Err(e) => local_issues.push(unresolved(
                    e,
                    "/panels",
                    "展开板必须在声明的同一构件平面，不能删除Z坐标冒充刀线。",
                )),
            }
        }
        for i in 0..polygons.len() {
            for j in i + 1..polygons.len() {
                if !tr.alive() {
                    break;
                }
                let overlap = area_intersection(&polygons[i], &polygons[j])
                    .expect("validated simple polygons");
                if overlap > n(0) {
                    let max_side = polygons[i]
                        .iter()
                        .chain(&polygons[j])
                        .flat_map(|p| p.iter())
                        .map(|x| x.abs())
                        .max()
                        .unwrap_or(n(1));
                    let band = q(tol.constraint_mm.value()) * max_side.max(n(1));
                    if overlap > band {
                        fail = true;
                        out.check.witnesses.push(PackagingWitness {
                            step_id: job.id.clone(),
                            object_pair: Some([
                                polygon_panels[i].clone(),
                                polygon_panels[j].clone(),
                            ]),
                            interval: [number(0.0), number(0.0)],
                            contact_point_mm: None,
                            message: format!(
                                "同一展开blank存在已证实正面积覆盖：{} mm²。",
                                overlap.to_f64().unwrap_or(f64::MAX)
                            ),
                        });
                    } else {
                        local_issues.push(unresolved(
                            "manufacturing_boundary_deadband",
                            "/panels",
                            "展开覆盖位于明确比较容差内，不能认证无覆盖。",
                        ));
                    }
                }
            }
        }
        let mut folds = vec![];
        for (bi, band) in job
            .fold_bands
            .iter()
            .enumerate()
            .filter(|(_, b)| job.panels[model.panels[&b.panel_a]].part_id == part.id)
        {
            if !tr.alive() {
                break;
            }
            let parent = &job.panels[model.panels[&band.panel_a]];
            let e = plane.compose(&ExactTf::from_pose(&parent.flat_pose));
            let p3 = |v: &Vec3| [q(v.x.value()), q(v.y.value()), q(v.z.value())];
            let a = e.point(&p3(&band.axis_start_mm));
            let b = e.point(&p3(&band.axis_end_mm));
            if !a[2].is_zero() || !b[2].is_zero() {
                local_issues.push(unresolved(
                    "noncoplanar_fold_axis",
                    &format!("/fold_bands/{bi}"),
                    "折带有限轴不在实际展开平面。",
                ));
                continue;
            }
            let (a, b) = ([a[0].clone(), a[1].clone()], [b[0].clone(), b[1].clone()]);
            let (dx, dy) = (&b[0] - &a[0], &b[1] - &a[1]);
            let squared = &dx * &dx + &dy * &dy;
            let approx = exact::enc(&squared).midpoint().sqrt();
            if !approx.is_finite() || q(approx) * q(approx) != squared {
                local_issues.push(unresolved(
                    "fold_strip_exact_normal_unresolved",
                    &format!("/fold_bands/{bi}"),
                    "该斜轴单位法向量尚未取得精确平面制造表达；不替换为外接矩形。",
                ));
                continue;
            }
            let k = q(band.width_mm.value()) / n(2) / q(approx);
            let offset = [-&dy * &k, &dx * &k];
            let strip = exact::polygon_q(vec![
                [&a[0] - &offset[0], &a[1] - &offset[1]],
                [&b[0] - &offset[0], &b[1] - &offset[1]],
                [&b[0] + &offset[0], &b[1] + &offset[1]],
                [&a[0] + &offset[0], &a[1] + &offset[1]],
            ])
            .expect("nonzero finite strip");
            let Some(pa) = flat.get(&band.panel_a) else {
                continue;
            };
            let Some(pb) = flat.get(&band.panel_b) else {
                continue;
            };
            if !shares_edge(&strip, pa) || !shares_edge(&strip, pb) {
                local_issues.push(unresolved(
                    "fold_strip_attachment",
                    &format!("/fold_bands/{bi}"),
                    "压痕带须与声明父子板的实际局部边界相接。",
                ));
            }
            for p in &polygons {
                if area_intersection(p, &strip).expect("validated polygon") > n(0) {
                    local_issues.push(unresolved(
                        "fold_strip_overlap",
                        &format!("/fold_bands/{bi}"),
                        "压痕带与实际板片面积重叠，不能隐藏于制造并集。",
                    ));
                }
            }
            let child = &job.panels[model.panels[&band.panel_b]];
            if child.material_ref != parent.material_ref {
                local_issues.push(unresolved(
                    "fold_strip_material",
                    &format!("/fold_bands/{bi}"),
                    "折带两侧材料不同，当前输入没有声明带区材料。",
                ));
            } else {
                *material_areas
                    .entry(parent.material_ref.clone())
                    .or_insert(n(0)) += area2(&strip) / n(2);
            }
            polygons.push(strip);
            let steps = job
                .steps
                .iter()
                .filter(|s| matches!(&s.motion,PackagingMotion::Fold(m)if m.fold_band==band.id))
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            folds.push((vec![(a,b)],format!("data-fold-band-ref=\"{}\" data-panel-refs=\"{} {}\" data-active-step-ref=\"{}\" data-width-mm=\"{}\" data-target-angle-rad=\"{}\"",xml(band.id.as_str()),xml(band.panel_a.as_str()),xml(band.panel_b.as_str()),xml(&steps),band.width_mm.value(),band.target_angle_rad.value())));
        }
        let mut glues = vec![];
        for (ci, c) in job.connections.iter().enumerate().filter(|(_, c)| {
            matches!(c.kind, ConnectionMaterialKind::Glue)
                && job.panels[model.panels[&c.panel_a]].part_id == part.id
        }) {
            if !tr.alive() {
                break;
            }
            let Some(poses) = activations.get(&ci) else {
                local_issues.push(unresolved(
                    "glue_not_activated",
                    &format!("/connections/{ci}"),
                    "未实际完整通过激活端点的胶接不能生成应用胶层。",
                ));
                continue;
            };
            let ia = model.panels[&c.panel_a];
            let ib = model.panels[&c.panel_b];
            let relative = poses[ia].inverse().compose(poses[ib]);
            let mut regions = vec![];
            let mut allowance_ids = vec![];
            for aid in &c.contact_allowance_refs {
                if !tr.alive() {
                    break;
                }
                let allowance = job
                    .conditions
                    .allowed_contacts
                    .iter()
                    .find(|a| a.id == *aid)
                    .expect("validated reference");
                let direct = allowance.panel_a == c.panel_a;
                let (a, b, face_b) = if direct {
                    (
                        &allowance.polygon_a_mm,
                        &allowance.polygon_b_mm,
                        &allowance.face_b,
                    )
                } else {
                    (
                        &allowance.polygon_b_mm,
                        &allowance.polygon_a_mm,
                        &allowance.face_a,
                    )
                };
                let normal = relative.direction([
                    crate::numeric::Iv::ZERO,
                    crate::numeric::Iv::ZERO,
                    crate::numeric::Iv::ONE,
                ]);
                if normal[0].abs_upper() > tol.angle_rad.value()
                    || normal[1].abs_upper() > tol.angle_rad.value()
                {
                    local_issues.push(unresolved(
                        "glue_projection_orientation",
                        &format!("/connections/{ci}"),
                        "胶区两面非平行或法向区间未收敛，不猜测应用区域。",
                    ));
                    continue;
                }
                let z = job.panels[ib].thickness_mm.value() / 2.0
                    * if exact::front(face_b) { 1.0 } else { -1.0 };
                let mut projected = vec![];
                let mut precise = true;
                for v in b {
                    if !tr.alive() {
                        precise = false;
                        break;
                    }
                    let p = relative.point([
                        crate::numeric::Iv::point(v.x.value()),
                        crate::numeric::Iv::point(v.y.value()),
                        crate::numeric::Iv::point(z),
                    ]);
                    if p[..2]
                        .iter()
                        .any(|x| !x.valid() || x.width() > tol.distance_mm.value() / 8.0)
                    {
                        precise = false;
                        break;
                    }
                    projected.push([q(p[0].midpoint()), q(p[1].midpoint())]);
                }
                if !precise {
                    local_issues.push(unresolved(
                        "glue_projection_precision",
                        &format!("/connections/{ci}"),
                        "共同胶区的坐标包围未达显式制造精度。",
                    ));
                    continue;
                }
                let Ok(projected) = exact::polygon_q(projected) else {
                    local_issues.push(unresolved(
                        "glue_projection_topology",
                        &format!("/connections/{ci}"),
                        "共同胶区投影退化。",
                    ));
                    continue;
                };
                let a = exact::polygon(a).expect("validated allowance");
                for x in convex_parts(&a).expect("simple allowance") {
                    for y in convex_parts(&projected).expect("simple projection") {
                        let r = convex_intersection(&x, &y);
                        if !r.is_empty() {
                            regions.push(r);
                        }
                    }
                }
                allowance_ids.push(aid.as_str());
            }
            if regions.is_empty() {
                local_issues.push(unresolved(
                    "missing_common_glue_region",
                    &format!("/connections/{ci}"),
                    "未取得非空的实际双面共同胶区。",
                ));
                continue;
            }
            let union = match exact::union_cells(&regions, tr) {
                Ok(union) => union,
                Err(code) => {
                    local_issues.push(unresolved(code, "/connections", "共同胶区并集未完成。"));
                    continue;
                }
            };
            let e = plane.compose(&ExactTf::from_pose(&job.panels[ia].flat_pose));
            let mapped: Result<Vec<_>, _> = union.iter().map(|p| xy(&e, p)).collect();
            match mapped {
                Ok(regions) => {
                    let edges = match exact::boundary(&regions, tr) {
                        Ok(edges) => edges,
                        Err(code) => {
                            local_issues.push(unresolved(code, "/connections", "胶区边界未完成。"));
                            continue;
                        }
                    };
                    let area = regions.iter().map(|p| area2(p) / n(2)).sum::<R>().to_f64();
                    if let Some(area) = area.filter(|v| v.is_finite()) {
                        glues.push((edges,format!("data-panel-refs=\"{}\" data-partner-panel-ref=\"{}\" data-connection-ref=\"{}\" data-allowance-ref=\"{}\" data-active-step-ref=\"{}\" data-area-mm2=\"{area}\"",xml(c.panel_a.as_str()),xml(c.panel_b.as_str()),xml(c.id.as_str()),xml(&allowance_ids.join(" ")),xml(c.engage_after_step_id.as_ref().map(ObjectId::as_str).unwrap_or("initial")))));
                    } else {
                        local_issues.push(unresolved(
                            "manufacturing_numeric_range",
                            "/connections",
                            "胶区面积超出有限输出范围。",
                        ));
                    }
                }
                Err(code) => local_issues.push(unresolved(
                    code,
                    "/connections",
                    "胶区无法映回实际blank平面。",
                )),
            }
        }
        if !tr.alive() {
            local_issues.push(unresolved("cancelled", "/parts", "制造输出取消。"));
        }
        if fail {
            out.check.outcome = CheckOutcome::Fail;
        }
        let local_unresolved = !local_issues.is_empty();
        out.check.unresolved.extend(local_issues);
        // A partially described blank does not emit a complete manufacturing
        // asset or material area. Other complete parts may still be preserved.
        if fail || local_unresolved {
            continue;
        }
        let union = match exact::union_cells(&polygons, tr) {
            Ok(union) => union,
            Err(code) => {
                out.check
                    .unresolved
                    .push(unresolved(code, "/parts", "实际展开并集未完成。"));
                break;
            }
        };
        let edges = match exact::boundary(&union, tr) {
            Ok(edges) => edges,
            Err(code) => {
                out.check
                    .unresolved
                    .push(unresolved(code, "/parts", "实际刀线边界未完成。"));
                break;
            }
        };
        let Some(cut) = path(&edges) else {
            out.check.unresolved.push(unresolved(
                "manufacturing_numeric_range",
                "/parts",
                "刀线坐标超出有限输出范围。",
            ));
            continue;
        };
        let minmax = |axis: usize, minimum: bool| -> Option<f64> {
            let it = polygons.iter().flatten().map(|p| p[axis].clone());
            let r = if minimum { it.min() } else { it.max() }?;
            r.to_f64().filter(|x| x.is_finite())
        };
        let (Some(x0), Some(y0), Some(x1), Some(y1)) = (
            minmax(0, true),
            minmax(1, true),
            minmax(0, false),
            minmax(1, false),
        ) else {
            out.check.unresolved.push(unresolved(
                "manufacturing_numeric_range",
                "/parts",
                "展开范围未取得有限数值。",
            ));
            continue;
        };
        if !(x1 - x0).is_finite() || !(y1 - y0).is_finite() || x1 <= x0 || y1 <= y0 {
            out.check.unresolved.push(unresolved(
                "manufacturing_numeric_range",
                "/parts",
                "实际SVG的mm物理尺寸不能取得有限正值。",
            ));
            continue;
        }
        let panel_ids = part
            .panel_ids
            .iter()
            .map(ObjectId::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}mm\" height=\"{}mm\" viewBox=\"{x0} {y0} {} {}\" data-part-ref=\"{}\" data-job-ref=\"{}\" data-job-revision=\"{}\"><g id=\"cut\" fill=\"none\" stroke=\"black\"><path data-panel-refs=\"{}\" d=\"{cut}\"/></g><g id=\"fold\" fill=\"none\" stroke=\"blue\">",
            x1 - x0,
            y1 - y0,
            x1 - x0,
            y1 - y0,
            xml(part.id.as_str()),
            xml(job.id.as_str()),
            job.revision.0.value(),
            xml(&panel_ids)
        );
        for (e, attrs) in folds {
            if let Some(d) = path(&e) {
                svg.push_str(&format!("<path {attrs} d=\"{d}\"/>"));
            }
        }
        svg.push_str("</g><g id=\"glue\" fill=\"none\" stroke=\"green\">");
        for (e, attrs) in glues {
            if let Some(d) = path(&e) {
                svg.push_str(&format!("<path {attrs} d=\"{d}\"/>"));
            }
        }
        svg.push_str("</g></svg>");
        let file = FileRef {
            path: format!(
                "{}/part-{part_index}.svg",
                context.manufacturing_directory.as_str()
            )
            .try_into()
            .expect("safe host directory and index"),
            kind: DocumentKind::Asset,
        };
        for (material_id, area) in material_areas {
            let Some(area) = area.to_f64().filter(|x| x.is_finite()) else {
                out.check.unresolved.push(unresolved(
                    "manufacturing_numeric_range",
                    "/materials",
                    "材料面积超出有限输出范围。",
                ));
                continue;
            };
            out.counts.push(MaterialCount {
                material_id,
                part_id: part.id.clone(),
                quantity: uint(1),
                area_mm2: Some(number(area)),
            });
        }
        out.files.push(ManufacturingFile {
            part_id: part.id.clone(),
            file: file.clone(),
            layers: vec![SvgLayer::Cut, SvgLayer::Fold, SvgLayer::Glue],
            unit: LengthUnit::Millimeter,
        });
        out.bytes.push((file, svg.into_bytes()));
    }
    if !matches!(out.check.outcome, CheckOutcome::Fail) && !out.check.unresolved.is_empty() {
        out.check.outcome = CheckOutcome::Undetermined;
    }
    out
}
