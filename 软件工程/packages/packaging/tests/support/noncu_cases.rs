//! Explicit test edits of the frozen public templates. No solver-derived oracle.
#![allow(dead_code)]
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const MECHANISMS: [&str; 3] = ["container", "outer_liner", "cover"];
pub const CHANGES: [&str; 3] = ["product_function", "material_approved", "part_relations_or_repeat_order"];

pub fn seed(mechanism: &str) -> Value {
    let text = match mechanism {
        "container" => include_str!("../../../../modules/professional/paper-packaging/templates/restricted-folded-container.job.json"),
        "outer_liner" => include_str!("../../../../modules/professional/paper-packaging/templates/outer-box-inner-liner.job.json"),
        "cover" => include_str!("../../../../modules/professional/paper-packaging/templates/cover.job.json"),
        "sleeve" => include_str!("../../../../modules/professional/paper-packaging/templates/sliding-sleeve-explicit-candidate.job.json"),
        _ => panic!("unknown explicit mechanism {mechanism}"),
    };
    serde_json::from_str::<Value>(text).unwrap()["data"].clone()
}

fn rename(value: &mut Value, ids: &BTreeMap<String, String>) {
    match value {
        Value::String(s) => if let Some(next) = ids.get(s) { *s = next.clone(); },
        Value::Array(a) => for child in a { rename(child, ids); },
        Value::Object(o) => for child in o.values_mut() { rename(child, ids); },
        _ => {},
    }
}

pub fn matrix_job(mechanism: &str, change: &str) -> Value {
    let mut job = seed(mechanism);
    job["id"] = json!(format!("NC-{mechanism}-{change}"));
    job["revision"] = json!(2);
    job["changes"] = json!([change]);
    match change {
        "product_function" => {
            job["subject_ref"]["revision"] = json!("r2");
            job["conditions"]["product_envelope"]["subject"] = job["subject_ref"].clone();
            // The explicit new product is 2mm wider. All original cavity/path
            // margins exceed this 1mm/side edit; no panel or condition is removed.
            for vertex in job["conditions"]["product_envelope"]["vertices_mm"].as_array_mut().unwrap() {
                let x = vertex["x"].as_f64().unwrap();
                vertex["x"] = json!(x + x.signum());
            }
        },
        "material_approved" => {
            // An explicit test material choice, not industrial approval. The
            // frozen 2mm bands and cavity gaps exceed the new 0.75mm thickness.
            let thickness = if mechanism == "sleeve" { 1.0 } else { 0.75 };
            for material in job["materials"].as_array_mut().unwrap() {
                material["thickness_mm"] = json!(thickness);
                material["basis"] = json!("template_declared_example");
                material["title"] = json!("noncu explicit test material choice, no industrial approval");
                if mechanism == "sleeve" { material["id"] = json!("REAPPROVED_CARD"); }
            }
            for panel in job["panels"].as_array_mut().unwrap() {
                panel["thickness_mm"] = json!(thickness);
                // The supplementary sleeve case changes approved material
                // identity at the same declared thickness; it does not infer a
                // different crease/lap model from new material physics.
                if mechanism == "sleeve" { panel["material_ref"] = json!("REAPPROVED_CARD"); }
            }
        },
        "part_relations_or_repeat_order" => {
            let mut ids = BTreeMap::new();
            for field in ["parts", "panels", "fold_bands", "connections", "steps"] {
                for item in job[field].as_array().unwrap() {
                    let id = item["id"].as_str().unwrap();
                    ids.insert(id.to_owned(), format!("REPEAT_{id}"));
                }
            }
            for item in job["conditions"]["allowed_contacts"].as_array().unwrap() {
                let id = item["id"].as_str().unwrap();
                ids.insert(id.to_owned(), format!("REPEAT_{id}"));
            }
            let mut second = job.clone();
            rename(&mut second, &ids);
            for part in second["parts"].as_array_mut().unwrap() {
                let x = part["placement"]["translation"]["x"].as_f64().unwrap();
                part["placement"]["translation"]["x"] = json!(x + 400.0);
            }
            // Rigid translations declare an absolute start pose as well as a
            // displacement. Copying only placement correctly fails continuity.
            for step in second["steps"].as_array_mut().unwrap() {
                if step["motion"]["kind"] == "rigid_translation" {
                    let start = &mut step["motion"]["data"]["start_pose"]["translation"]["x"];
                    *start = json!(start.as_f64().unwrap() + 400.0);
                }
            }
            for field in ["parts", "panels", "fold_bands", "connections"] {
                job[field].as_array_mut().unwrap().extend(second[field].as_array().unwrap().iter().cloned());
            }
            job["conditions"]["allowed_contacts"].as_array_mut().unwrap().extend(second["conditions"]["allowed_contacts"].as_array().unwrap().iter().cloned());
            // Both complete assemblies form before either assembly opens.
            let original = job["steps"].as_array().unwrap();
            let repeated = second["steps"].as_array().unwrap();
            job["steps"] = Value::Array(original.iter().chain(repeated).filter(|s| s["purpose"] == "forming").chain(original.iter().chain(repeated).filter(|s| s["purpose"] != "forming")).cloned().collect());
            // A completed glued assembly remains present during the other
            // assembly's steps. Declare those new post-engagement step IDs for
            // the same local faces/polygon/depth; do not exempt whole panels.
            let steps = job["steps"].as_array().unwrap().clone();
            let connections = job["connections"].as_array().unwrap().clone();
            for allowance in job["conditions"]["allowed_contacts"].as_array_mut().unwrap() {
                let connection = connections.iter().find(|c|c["contact_allowance_refs"].as_array().unwrap().contains(&allowance["id"])).unwrap();
                let start = steps.iter().position(|s|s["id"]==connection["engage_after_step_id"]).unwrap();
                allowance["active_step_ids"] = json!(steps[start..].iter().map(|s|s["id"].clone()).collect::<Vec<_>>());
            }
        },
        _ => panic!("unknown explicit change {change}"),
    }
    serde_json::to_value(serde_json::from_value::<sf_domain::PackageStructureJob>(job).unwrap()).unwrap()
}

pub fn expected_area(mechanism: &str, change: &str) -> f64 {
    // L*W + 2*L*H + 2*W*H + 2*f*(L+W), f=2mm.
    let area = match mechanism {
        "container" => 6800.0,
        "outer_liner" => 6800.0 + 3210.0,
        "cover" => 3600.0 + 7568.0,
        "sleeve" => 2212.5,
        _ => unreachable!(),
    };
    area * if change == "part_relations_or_repeat_order" { 2.0 } else { 1.0 }
}

pub fn expected_parts(mechanism: &str, change: &str) -> usize {
    let base = if mechanism == "container" || mechanism == "sleeve" { 1 } else { 2 };
    base * if change == "part_relations_or_repeat_order" { 2 } else { 1 }
}
