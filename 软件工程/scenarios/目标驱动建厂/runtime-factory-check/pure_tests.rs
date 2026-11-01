//! rustc-only consumer of the actual new producer core and unchanged public data.
#[path = "checks.projection.rs"]
mod projection;
use projection::checks;
mod fixture_support;
use CheckOutcome::*;

#[test]
fn declared_instance_parameter_uses_its_real_source_binding_and_keeps_library_immutable() {
    let mut v = fixture();
    let op = v["definitions"][0]["payload"]["data"].clone();
    v["definitions"][4]["payload"]["data"]["behavior"] =
        json!({"inline":op,"operation_ref":null,"selected":"inline"});
    v["definitions"][4]["parameters"] = json!([{"id":"cycle","value_type":"integer","binding":"/payload/data/behavior/inline/duration/value","label":null,"unit":"millisecond","minimum":1,"maximum":100000,"integer":true,"options":[],"basis":"module_declared"}]);
    v["project"]["devices"][0]["operation_binding"] = Value::Null;
    v["project"]["devices"][0]["configuration"] = json!({"cycle":{"kind":"integer","value":4000}});
    v["project"]["routes"][0]["steps"][0]["operation_ref"] =
        v["project"]["devices"][0]["module_ref"].clone();
    v["project"]["parameter_sources"] = json!([{"target":{"kind":"device","id":"ASM-01"},"field_path":"/configuration/cycle/value","value":{"kind":"integer","value":4000},"basis":"user_provided","source_ref":null}]);
    let before = v.clone();
    let out = inspect(&v, &request());
    assert!(
        out.checks.iter().all(|c| matches!(c.outcome, Pass)),
        "{out:?}"
    );
    assert_eq!(v, before);
    v["project"]["parameter_sources"][0]["basis"] = json!("unknown");
    let out = inspect(&v, &request());
    assert!(
        out.issues
            .iter()
            .any(|i| i.code.ends_with("duration_missing")),
        "{out:?}"
    );
    v["project"]["parameter_sources"][0]["basis"] = json!("user_provided");
    v["definitions"][4]["parameters"][0]["unit"] = json!("second");
    assert!(has(
        &inspect(&v, &request()),
        MaterialProcessConnectivity,
        Fail
    ));
}

#[test]
fn empty_static_regions_and_unknown_receiving_capacity_cannot_pass() {
    let mut r = request();
    r["generation_goal"]["site_envelope"]["regions"] = json!([]);
    let out = inspect(&fixture(), &r);
    assert!(has(&out, ClearanceAndStaticLanes, Undetermined));
    let mut v = fixture();
    v["project"]["locations"][0]["capacity"] = Value::Null;
    let out = inspect(&v, &request());
    assert!(
        out.issues
            .iter()
            .any(|i| i.code.ends_with("receiving_capacity_missing"))
    );
}
#[test]
fn exact_bom_ratio_and_shared_job_phase_pool_rule_follow_existing_engine_contract() {
    let mut v = fixture();
    v["project"]["boms"][0]["rows"][0]["quantity"]["value"]["coefficient"] = json!(2);
    assert!(
        inspect(&v, &request())
            .issues
            .iter()
            .any(|i| i.code.ends_with("bom_operation_ratio"))
    );
    let mut v = fixture();
    let req = v["definitions"][0]["payload"]["data"]["resource_roles"][0].clone();
    let duration = v["definitions"][0]["payload"]["data"]["duration"].clone();
    v["definitions"][0]["payload"]["data"]["duration"] = Value::Null;
    v["definitions"][0]["payload"]["data"]["phases"] = json!([{"id":"work","duration":duration,"phase_resources":[req],"interruption_policy":"non_preemptive","work_kind":"processing"}]);
    let out = inspect(&v, &request());
    assert!(
        !out.issues
            .iter()
            .any(|i| i.code.ends_with("resource_capacity")),
        "{out:?}"
    );
}
#[test]
fn noncanonical_coordinate_frame_is_named_not_silently_interpreted() {
    let mut v = fixture();
    v["project"]["devices"][0]["pose"]["coordinate_frame"] = json!("right_handed_y_up");
    let out = inspect(&v, &request());
    assert!(has(&out, SiteBounds, Undetermined));
}
#[test]
fn transfers_from_different_scenarios_cannot_form_a_fictitious_material_path() {
    let mut v = fixture();
    let mut loc = v["project"]["locations"][0].clone();
    loc["id"] = json!("other-assembly-store");
    loc["accepted_item_types"] = json!(["assembly"]);
    v["project"]["locations"].as_array_mut().unwrap().push(loc);
    v["project"]["connections"][3]["data"]["source"]["data"] = json!("other-assembly-store");
    v["project"]["connections"][3]["data"]["destination_store"] = json!("other-assembly-store");
    let (mut p, mut scenarios, d) = typed(&v);
    let mut second = scenarios.values().next().unwrap().clone();
    second.id = ScenarioId::try_from("SECOND".to_owned()).unwrap();
    p.scenario_refs.push(second.id.clone());
    let make = |id: &str, from: &str, to: &str| {
        serde_json::from_value::<SupplyRelation>(json!({"id":id,"source":from,"destination":to,"item_type_ref":{"id":"assembly","revision":"r1"},"target_quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"window":{"start_ms":0,"end_ms":240000},"transport_ref":{"id":"absent-transport","version":"0.1.0"},"resources":{},"departure_windows":[],"allow_partial_load":true})).unwrap()
    };
    scenarios
        .values_mut()
        .next()
        .unwrap()
        .supplies
        .push(make("leg1", "assemblies-A", "middle"));
    second
        .supplies
        .push(make("leg2", "middle", "other-assembly-store"));
    scenarios.insert(second.id.clone(), second);
    let r = request();
    let out = checks::inspect(
        &p,
        &scenarios,
        &d,
        &serde_json::from_value(r["generation_goal"].clone()).unwrap(),
        &serde_json::from_value::<Vec<ObjectRef>>(r["check_scope"]["objects"].clone()).unwrap(),
        true,
        true,
        true,
        &[],
        vec![],
    );
    assert!(
        out.issues
            .iter()
            .any(|i| i.code.ends_with("process_disconnected")),
        "{out:?}"
    );
}

#[test]
fn large_declared_demand_does_not_expand_jobs() {
    let mut v = fixture();
    v["scenario"]["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] =
        json!(1_000_000_000_000_u64);
    let before = v.clone();
    let out = inspect(&v, &request());
    assert!(
        out.checks.iter().all(|c| matches!(c.outcome, Pass)),
        "{out:?}"
    );
    assert_eq!(v, before);
}
#[test]
fn scoped_device_includes_calendar_of_its_actual_bound_resources() {
    let mut v = fixture();
    v["project"]["resources"][0]["calendar_id"] = json!("missing");
    let mut r = request();
    r["check_scope"]["objects"] = json!([{"kind":"device","id":"ASM-01"}]);
    let out = inspect(&v, &r);
    assert!(has(&out, MaterialProcessConnectivity, Undetermined));
    assert!(has(&out, ReferencesAndUnits, Undetermined));
}
#[test]
fn disconnected_process_and_acceptance_locations_are_not_cured_by_valid_units() {
    let mut v = fixture();
    let mut loc = v["project"]["locations"][0].clone();
    loc["id"] = json!("other-assembly-store");
    loc["accepted_item_types"] = json!(["assembly"]);
    v["project"]["locations"].as_array_mut().unwrap().push(loc);
    v["project"]["connections"][3]["data"]["source"]["data"] = json!("other-assembly-store");
    v["project"]["connections"][3]["data"]["destination_store"] = json!("other-assembly-store");
    assert!(
        inspect(&v, &request())
            .issues
            .iter()
            .any(|i| i.code.ends_with("process_disconnected"))
    );
    let mut v = fixture();
    let mut loc = v["project"]["locations"][0].clone();
    loc["id"] = json!("other-accepted");
    loc["accepted_item_types"] = json!(["package"]);
    v["project"]["locations"].as_array_mut().unwrap().push(loc);
    let mut r = request();
    r["generation_goal"]["acceptance_scope"]["destination"] = json!("other-accepted");
    assert!(
        inspect(&v, &r)
            .issues
            .iter()
            .any(|i| i.code.ends_with("acceptance_unreachable"))
    );
}
#[test]
fn usable_region_union_cannot_hide_an_interior_hole_and_adjacent_regions_join() {
    let mut v = fixture();
    v["project"]["devices"][0]["pose"]["translation"] = json!({"x":0,"y":0,"z":0});
    v["project"]["devices"][0]["footprint_m"] = rectangle(0., 0., 5., 5.);
    let mut r = request();
    r["check_scope"]["objects"] = json!([{"kind":"device","id":"ASM-01"}]);
    let regions = [
        rectangle(0., 0., 5., 1.),
        rectangle(0., 4., 5., 1.),
        rectangle(0., 0., 1., 5.),
        rectangle(4., 0., 1., 5.),
        rectangle(2., 0., 1., 5.),
    ];
    r["generation_goal"]["site_envelope"]["regions"]=json!(regions.iter().enumerate().map(|(i,p)|json!({"id":format!("use-{i}"),"kind":"usable","polygon_m":p,"clearance_m":null})).collect::<Vec<_>>());
    assert!(has(&inspect(&v, &r), SiteBounds, Fail));
    r["generation_goal"]["site_envelope"]["regions"] = json!([{"id":"left","kind":"usable","polygon_m":rectangle(0.,0.,2.5,5.),"clearance_m":null},{"id":"right","kind":"usable","polygon_m":rectangle(2.5,0.,2.5,5.),"clearance_m":null}]);
    assert!(!has(&inspect(&v, &r), SiteBounds, Fail));
}
#[test]
fn explicit_template_spacing_is_consumed_but_never_fills_missing_height() {
    let v = fixture();
    let mut r = request();
    r["generation_goal"]["site_envelope"]["clearance_m"] = Value::Null;
    r["generation_goal"]["site_envelope"]["height_m"] = Value::Null;
    let (p, s, d) = typed(&v);
    let profile:TemplateGenerationProfile=serde_json::from_value(json!({"industry":"fixture","product_family":"fixture","site_applicability":{"length_m":null,"width_m":null,"height_m":null,"clearance_m":3,"regions":[],"usable_polygon_m":[]},"process_graph":[],"nine_category_roles":[],"required_parameters":[],"parameter_sources":[],"layout_modes":[],"role_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}}})).unwrap();
    let out = checks::inspect(
        &p,
        &s,
        &d,
        &serde_json::from_value(r["generation_goal"].clone()).unwrap(),
        &serde_json::from_value::<Vec<ObjectRef>>(r["check_scope"]["objects"].clone()).unwrap(),
        true,
        true,
        true,
        &[profile],
        vec![],
    );
    assert!(has(&out, ClearanceAndStaticLanes, Fail));
    assert!(has(&out, Height, Undetermined));
}
#[test]
fn unknown_duration_source_is_not_a_real_cycle_time_and_role_quantity_is_checked() {
    let mut v = fixture();
    v["definitions"][0]["payload"]["data"]["duration"]["basis"] = json!("unknown");
    assert!(has(
        &inspect(&v, &request()),
        MaterialProcessConnectivity,
        Undetermined
    ));
    let mut v = fixture();
    v["definitions"][0]["payload"]["data"]["inputs"][0]["quantity"]["value"]["coefficient"] =
        json!(0);
    assert!(has(
        &inspect(&v, &request()),
        MaterialProcessConnectivity,
        Fail
    ));
}
use DraftCheckKind::*;
use fixture_support::*;
use serde_json::{Value, json};
use sf_domain::*;

fn inspect(v: &Value, r: &Value) -> FactoryDraftCheckResult {
    let (p, s, d) = typed(v);
    checks::inspect(
        &p,
        &s,
        &d,
        &serde_json::from_value(r["generation_goal"].clone()).unwrap(),
        &serde_json::from_value::<Vec<ObjectRef>>(r["check_scope"]["objects"].clone()).unwrap(),
        r["check_scope"]["check_references"].as_bool().unwrap(),
        r["check_scope"]["check_business"].as_bool().unwrap(),
        r["check_scope"]["check_static_layout"].as_bool().unwrap(),
        &[],
        vec![],
    )
}
#[test]
fn complete_actual_fixture_checks_all_six_without_running_or_modifying_inputs() {
    let v = fixture();
    let r = request();
    let before = v.clone();
    let out = inspect(&v, &r);
    assert!(
        out.checks.iter().all(|c| matches!(c.outcome, Pass)),
        "{:?}",
        out
    );
    assert_eq!(
        out.checks
            .iter()
            .map(|c| format!("{:?}", c.kind))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        6
    );
    assert_eq!(v, before);
    let path = std::path::Path::new(file!())
        .parent()
        .unwrap()
        .join("pure-complete.actual.json");
    std::fs::write(path, serde_json::to_vec_pretty(&out).unwrap()).unwrap();
}
#[test]
fn empty_scope_and_disabled_checks_never_pass() {
    for empty in [false, true] {
        let mut r = request();
        if empty {
            r["check_scope"]["objects"] = json!([]);
        } else {
            for f in ["check_references", "check_business", "check_static_layout"] {
                r["check_scope"][f] = json!(false);
            }
        }
        let out = inspect(&fixture(), &r);
        assert_eq!(out.checks.len(), 6);
        assert!(out.checks.iter().all(|c| matches!(c.outcome, Undetermined)));
    }
}
#[test]
fn missing_dimensions_height_clearance_calendar_and_time_are_named_unknown() {
    let mut v = fixture();
    let mut r = request();
    v["project"]["devices"][0]["footprint_m"] = json!([]);
    v["project"]["devices"][1]["height_m"] = Value::Null;
    v["definitions"][0]["payload"]["data"]["duration"] = Value::Null;
    r["generation_goal"]["site_envelope"]["regions"] = json!([]);
    r["generation_goal"]["site_envelope"]["length_m"] = Value::Null;
    r["generation_goal"]["site_envelope"]["width_m"] = Value::Null;
    r["generation_goal"]["site_envelope"]["height_m"] = Value::Null;
    r["generation_goal"]["site_envelope"]["clearance_m"] = Value::Null;
    r["generation_goal"]["production_calendar_ref"] = Value::Null;
    let out = inspect(&v, &r);
    for k in [
        MaterialProcessConnectivity,
        SiteBounds,
        FootprintOverlap,
        Height,
        ClearanceAndStaticLanes,
    ] {
        assert!(has(&out, k, Undetermined), "{out:?}");
    }
    for code in [
        "duration_missing",
        "production_calendar",
        "device_height_missing",
        "clearance_missing",
    ] {
        assert!(out.issues.iter().any(|i| i.code.ends_with(code)), "{out:?}");
    }
}
#[test]
fn no_scenario_and_empty_production_graph_are_not_connectivity_pass() {
    let mut v = fixture();
    v["scenario"]["production_tasks"] = json!([]);
    let out = inspect(&v, &request());
    assert!(has(&out, MaterialProcessConnectivity, Undetermined));
}
#[test]
fn real_unit_port_store_conflicts_fail() {
    for edit in ["unit", "port", "store"] {
        let mut v = fixture();
        match edit {
            "unit" => v["project"]["connections"][0]["data"]["unit"] = json!("kg"),
            "port" => {
                v["project"]["connections"][0]["data"]["destination"]["data"]["port_id"] =
                    json!("assembled")
            }
            _ => v["project"]["connections"][0]["data"]["destination_store"] = json!("pcbs-A"),
        };
        assert!(
            has(&inspect(&v, &request()), ReferencesAndUnits, Fail),
            "{edit}"
        );
    }
}
#[test]
fn exact_versions_not_latest_and_missing_resource_are_checked() {
    let mut v = fixture();
    v["project"]["devices"][0]["module_ref"]["version"] = json!("99.0.0");
    assert!(has(&inspect(&v, &request()), ReferencesAndUnits, Fail));
    let mut v = fixture();
    v["project"]["devices"][0]["role_bindings"]["resources"]["worker"] = json!("absent");
    assert!(has(&inspect(&v, &request()), ReferencesAndUnits, Fail));
}
#[test]
fn scope_keeps_unselected_obstacles_but_not_unrelated_business_objects() {
    let mut v = fixture();
    v["project"]["devices"][1]["pose"] = v["project"]["devices"][0]["pose"].clone();
    let mut r = request();
    r["check_scope"]["objects"] = json!([{"kind":"device","id":"ASM-01"}]);
    let out = inspect(&v, &r);
    assert!(has(&out, FootprintOverlap, Fail));
    assert!(
        out.checks
            .iter()
            .any(|c| matches!(c.kind, FootprintOverlap) && c.objects.len() == 2)
    );
    assert!(
        !out.checks
            .iter()
            .any(|c| matches!(c.kind, ReferencesAndUnits)
                && c.objects.iter().any(|o| o.id.as_str() == "worker-packing"))
    );
}
#[test]
fn hall_line_device_rigid_transform_is_composed_not_flattened() {
    let mut v = fixture();
    v["project"]["halls"][0]["origin"]["translation"]["x"] = json!(3);
    let mut pose = v["project"]["devices"][0]["pose"].clone();
    pose["translation"] = json!({"x":2,"y":3,"z":0});
    pose["rotation"] = json!({"x":0,"y":0,"z":std::f64::consts::FRAC_1_SQRT_2,"w":std::f64::consts::FRAC_1_SQRT_2});
    v["project"]["lines"] = json!([{"id":"L","title":"rotated line","hall_id":"H-A","transform":pose,"members":["ASM-01"],"boundary_ports":[],"region_polygon_m":[]}]);
    v["project"]["devices"][0]["line_id"] = json!("L");
    v["project"]["devices"][0]["pose"]["translation"] = json!({"x":1,"y":0,"z":0});
    let mut r = request();
    r["generation_goal"]["site_envelope"]["length_m"] = json!(30);
    r["check_scope"]["objects"] = json!([{"kind":"line","id":"L"}]);
    let out = inspect(&v, &r);
    assert!(has(&out, SiteBounds, Pass), "{out:?}");
    // final footprint x=4..5, y=4..5; a 4.5m envelope must reject it.
    r["generation_goal"]["site_envelope"]["length_m"] = json!(4.5);
    assert!(has(&inspect(&v, &r), SiteBounds, Fail));
}
#[test]
fn render_scale_cannot_hide_physical_overlap_or_site_bounds() {
    let mut v = fixture();
    v["project"]["devices"][1]["pose"]["translation"] = json!({"x":1.5,"y":1,"z":0});
    v["project"]["devices"][1]["pose"]["scale"] = json!({"x":0.001,"y":0.001,"z":0.001});
    assert!(has(&inspect(&v, &request()), FootprintOverlap, Fail));
    v["project"]["devices"][1]["pose"]["translation"]["x"] = json!(19.5);
    assert!(has(&inspect(&v, &request()), SiteBounds, Fail));
}
#[test]
fn crossing_polygons_overlap_without_any_vertex_inside() {
    let mut v = fixture();
    for d in v["project"]["devices"].as_array_mut().unwrap() {
        d["pose"]["translation"] = json!({"x":0,"y":0,"z":0});
    }
    v["project"]["devices"][0]["footprint_m"] = rectangle(1., 2., 4., 1.);
    v["project"]["devices"][1]["footprint_m"] = rectangle(2., 1., 1., 4.);
    assert!(has(&inspect(&v, &request()), FootprintOverlap, Fail));
}
#[test]
fn concave_boundary_rejects_edge_excursion_with_all_vertices_inside() {
    let mut v = fixture();
    v["project"]["devices"][0]["pose"]["translation"] = json!({"x":0,"y":0,"z":0});
    v["project"]["devices"][0]["footprint_m"] = rectangle(1., 3., 3., 1.);
    let mut r = request();
    r["generation_goal"]["site_envelope"]["usable_polygon_m"] = json!([{"x":0,"y":0},{"x":5,"y":0},{"x":5,"y":5},{"x":3,"y":5},{"x":3,"y":2},{"x":2,"y":2},{"x":2,"y":5},{"x":0,"y":5}]);
    assert!(has(&inspect(&v, &r), SiteBounds, Fail));
}
#[test]
fn touching_is_not_positive_area_overlap_and_declared_gap_is_exact() {
    let mut v = fixture();
    v["project"]["devices"][1]["pose"]["translation"]["x"] = json!(2);
    let out = inspect(&v, &request());
    assert!(!has(&out, FootprintOverlap, Fail));
    let mut r = request();
    r["generation_goal"]["site_envelope"]["clearance_m"] = json!(1);
    assert!(has(&inspect(&v, &r), ClearanceAndStaticLanes, Fail));
    v["project"]["devices"][1]["pose"]["translation"]["x"] = json!(3);
    assert!(!has(&inspect(&v, &r), ClearanceAndStaticLanes, Fail));
}
#[test]
fn height_and_keepout_lane_are_independent_checks() {
    let mut v = fixture();
    v["project"]["devices"][0]["height_m"] = json!(5);
    let mut r = request();
    r["generation_goal"]["site_envelope"]["regions"] = json!([{"id":"lane","kind":"static_lane","polygon_m":rectangle(1.2,0.,0.2,5.),"clearance_m":0}]);
    let out = inspect(&v, &r);
    assert!(has(&out, Height, Fail));
    assert!(has(&out, ClearanceAndStaticLanes, Fail));
    assert!(out.issues.iter().any(|i| i.message.contains("lane")));
}
#[test]
fn malformed_polygon_is_fail_tilt_is_named_undetermined() {
    let mut v = fixture();
    v["project"]["devices"][0]["footprint_m"] =
        json!([{"x":0,"y":0},{"x":2,"y":2},{"x":0,"y":2},{"x":2,"y":0}]);
    assert!(has(&inspect(&v, &request()), SiteBounds, Fail));
    let mut v = fixture();
    v["project"]["devices"][0]["pose"]["rotation"] = json!({"x":std::f64::consts::FRAC_1_SQRT_2,"y":0,"z":0,"w":std::f64::consts::FRAC_1_SQRT_2});
    assert!(has(&inspect(&v, &request()), SiteBounds, Undetermined));
}
#[test]
fn demand_is_not_run_capacity_or_inventory() {
    let v = fixture();
    let mut r = request();
    r["generation_goal"]["accepted_quantity_per_day"]["value"]["coefficient"] = json!(10000);
    let result = serde_json::to_value(inspect(&v, &r)).unwrap();
    assert!(result.get("goal_met").is_none());
    assert!(result.get("accepted_output").is_none());
    assert_eq!(v, fixture());
}
#[test]
fn fc03_metres_and_fc24_two_stations_fit_but_missing_height_clearance_stay_unknown() {
    let mut v = fixture();
    v["project"]["halls"][0]["width_m"] = json!(5);
    v["project"]["halls"][0]["depth_m"] = json!(16);
    v["project"]["halls"][0]["height_m"] = Value::Null;
    for (i, d) in v["project"]["devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        d["pose"]["translation"] = json!({"x":i*3,"y":0,"z":0});
        d["footprint_m"] = rectangle(0., 0., 2., 1.);
        d["height_m"] = Value::Null;
    }
    let mut r = request();
    r["generation_goal"]["site_envelope"] = json!({"length_m":5,"width_m":16,"height_m":null,"clearance_m":null,"regions":[],"usable_polygon_m":[]});
    let out = inspect(&v, &r);
    assert!(!has(&out, SiteBounds, Fail));
    assert!(has(&out, SiteBounds, Pass));
    assert!(has(&out, Height, Undetermined));
    assert!(has(&out, ClearanceAndStaticLanes, Undetermined));
}
