//! Public dispatch tests. These require Root's real Host registration; they do
//! not substitute a private checker call for public check_factory_draft.
#[path = "../../../scenarios/目标驱动建厂/runtime-factory-check/fixture_support.rs"]
mod data;
#[path = "fixtures/production/host_support.rs"]
mod host_support;
use data::{has, rectangle, request};
use host_support::*;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;

fn host(value: &Value) -> Harness {
    let tree = Sandbox::new();
    let directory = tree.directory("factory-check-input");
    seed(&directory, value);
    let ui = id("factory-check-ui");
    let mut h = Harness {
        host: ProjectHost::new(id("factory-check-host"), ui.clone()),
        ui,
        tree,
    };
    let handle = h.import(&directory);
    h.finish_one();
    let draft = h.imported(&handle);
    let _: EditCommit = ok(h.adopt(&handle, &draft, true));
    h
}
fn check(h: &mut Harness, params: Value) -> FactoryDraftCheckResult {
    ok(h.call("check_factory_draft", params))
}
#[test]
fn public_capability_discovery_then_check_preserves_exact_schema_and_project_scope() {
    let mut h = host(&data::fixture());
    let agent = id("factory-check-catalog-agent");
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            session_id: agent.clone(),
            granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            allowed_operations: vec![
                OperationName::DescribeCapabilities,
                OperationName::CheckFactoryDraft,
            ],
            allowed_definition_drafts: vec![],
            allow_view_control: false,
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    let before = h.snapshot();
    for caller in [h.ui.clone(), agent] {
        let catalog: CapabilityCatalog =
            ok(h.call_as(&caller, false, "describe_capabilities", json!({})));
        let operation = catalog
            .implemented_operations
            .iter()
            .find(|entry| matches!(entry.operation, OperationName::CheckFactoryDraft))
            .unwrap();
        assert_eq!(
            operation.input_schema_ref,
            "packages/commands/generated/schemas/check_factory_draft.request.schema.json"
        );
        assert_eq!(
            operation.output_schema_ref,
            "packages/commands/generated/schemas/check_factory_draft.response.schema.json"
        );
        assert_eq!(
            serde_json::to_value(&operation.allowed_scopes).unwrap(),
            json!([{"kind":"project"}])
        );
        assert!(operation.available_controls.is_empty());
        let result: FactoryDraftCheckResult =
            ok(h.call_as(&caller, true, "check_factory_draft", request()));
        assert!(
            result
                .checks
                .iter()
                .all(|check| matches!(check.outcome, CheckOutcome::Pass))
        );
        assert_eq!(h.snapshot(), before);
    }
}
#[test]
fn public_check_complete_fixture_is_readonly_and_retains_old_run_and_original_orders() {
    let value = data::fixture();
    let mut h = host(&value);
    let handle = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&handle);
    let old = serde_json::to_value(h.full(&run.id)).unwrap();
    let before = h.snapshot();
    let mut params = request();
    params["generation_goal"]["accepted_quantity_per_day"]["value"]["coefficient"] = json!(10000);
    let result = check(&mut h, params);
    assert!(
        result
            .checks
            .iter()
            .all(|c| matches!(c.outcome, CheckOutcome::Pass)),
        "{result:?}"
    );
    assert_eq!(result.project_revision.0.value(), 1);
    assert_eq!(h.snapshot(), before);
    assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), old);
    assert_eq!(
        h.object("scenario", "LOCAL")["orders"],
        value["scenario"]["orders"]
    );
    assert!(
        serde_json::to_value(&result)
            .unwrap()
            .get("goal_met")
            .is_none()
    );
}
#[test]
fn public_check_reports_named_missing_cycle_calendar_height_site_and_spacing() {
    let mut value = data::fixture();
    value["definitions"][0]["payload"]["data"]["duration"] = Value::Null;
    value["project"]["devices"][0]["height_m"] = Value::Null;
    let mut h = host(&value);
    let before = h.snapshot();
    let mut params = request();
    params["generation_goal"]["production_calendar_ref"] = Value::Null;
    params["generation_goal"]["site_envelope"] = json!({"length_m":null,"width_m":null,"height_m":null,"clearance_m":null,"regions":[],"usable_polygon_m":[]});
    let result = check(&mut h, params);
    for kind in [
        DraftCheckKind::MaterialProcessConnectivity,
        DraftCheckKind::SiteBounds,
        DraftCheckKind::Height,
        DraftCheckKind::ClearanceAndStaticLanes,
    ] {
        assert!(has(&result, kind, CheckOutcome::Undetermined), "{result:?}");
    }
    for code in [
        "duration_missing",
        "production_calendar",
        "device_height_missing",
        "site_missing",
        "clearance_missing",
    ] {
        assert!(
            result.issues.iter().any(|i| i.code.ends_with(code)
                && i.object_ref.is_some()
                && i.field_path.is_some()),
            "{result:?}"
        );
    }
    assert_eq!(h.snapshot(), before);
}
#[test]
fn public_check_detects_polygon_crossing_height_lane_and_visual_scale_bypass() {
    let mut value = data::fixture();
    for d in value["project"]["devices"].as_array_mut().unwrap() {
        d["pose"]["translation"] = json!({"x":0,"y":0,"z":0});
    }
    value["project"]["devices"][0]["footprint_m"] = rectangle(1., 2., 4., 1.);
    value["project"]["devices"][1]["footprint_m"] = rectangle(2., 1., 1., 4.);
    value["project"]["devices"][1]["pose"]["scale"] = json!({"x":0.001,"y":0.001,"z":0.001});
    value["project"]["devices"][0]["height_m"] = json!(5);
    let mut params = request();
    params["generation_goal"]["site_envelope"]["regions"] = json!([{"id":"lane","kind":"static_lane","polygon_m":rectangle(0.,2.2,6.,0.2),"clearance_m":0}]);
    let mut h = host(&value);
    let result = check(&mut h, params);
    for kind in [
        DraftCheckKind::FootprintOverlap,
        DraftCheckKind::Height,
        DraftCheckKind::ClearanceAndStaticLanes,
    ] {
        assert!(has(&result, kind, CheckOutcome::Fail), "{result:?}");
    }
}
#[test]
fn public_check_scopes_objects_and_keeps_unselected_obstacles() {
    let mut value = data::fixture();
    value["project"]["devices"][1]["pose"] = value["project"]["devices"][0]["pose"].clone();
    let mut h = host(&value);
    let mut params = request();
    params["check_scope"]["objects"] = json!([{"kind":"device","id":"ASM-01"}]);
    let result = check(&mut h, params.clone());
    assert_eq!(
        serde_json::to_value(&result.scope).unwrap(),
        params["check_scope"]["objects"]
    );
    assert!(has(
        &result,
        DraftCheckKind::FootprintOverlap,
        CheckOutcome::Fail
    ));
    for key in ["check_references", "check_business", "check_static_layout"] {
        params["check_scope"][key] = json!(false);
    }
    let result = check(&mut h, params);
    assert_eq!(result.checks.len(), 6);
    assert!(
        result
            .checks
            .iter()
            .all(|c| matches!(c.outcome, CheckOutcome::Undetermined))
    );
    let mut params = request();
    params["check_scope"]["objects"] = json!([]);
    assert!(
        check(&mut h, params)
            .checks
            .iter()
            .all(|c| matches!(c.outcome, CheckOutcome::Undetermined))
    );
}
#[test]
fn public_check_rejects_stale_revision_missing_target_and_wrong_scope_atomically() {
    let mut h = host(&data::fixture());
    let before = h.snapshot();
    let mut params = request();
    params["revision"] = json!(0);
    let e = error(h.call("check_factory_draft", params));
    assert!(matches!(e.code, ErrorCode::RevisionConflict));
    assert_eq!(e.expected_revision.unwrap().0.value(), 0);
    assert_eq!(e.actual_revision.unwrap().0.value(), 1);
    let mut params = request();
    params["check_scope"]["objects"] = json!([{"kind":"device","id":"absent"}]);
    assert!(matches!(
        error(h.call("check_factory_draft", params)).code,
        ErrorCode::MissingReference
    ));
    let ui = h.ui.clone();
    assert!(matches!(
        error(h.call_as(&ui, false, "check_factory_draft", request())).code,
        ErrorCode::UnsupportedCapability
    ));
    assert_eq!(h.snapshot(), before);
}
#[test]
fn public_check_requires_real_session_operation_project_grant_and_revocation() {
    let mut h = host(&data::fixture());
    let agent = id("factory-check-agent");
    let context = HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        session_id: agent.clone(),
        granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
        allowed_operations: vec![OperationName::CheckFactoryDraft],
        allowed_definition_drafts: vec![],
        allow_view_control: false,
        selected_file_tokens: vec![],
        edit_grants: vec![],
    };
    h.host.register_external_session(context).unwrap();
    let before = h.snapshot();
    let _: FactoryDraftCheckResult = ok(h.call_as(&agent, true, "check_factory_draft", request()));
    let denied = id("no-check-operation");
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            session_id: denied.clone(),
            granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            allowed_operations: vec![OperationName::GetContext],
            allowed_definition_drafts: vec![],
            allow_view_control: false,
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    assert!(matches!(
        error(h.call_as(&denied, true, "check_factory_draft", request())).code,
        ErrorCode::PermissionDenied
    ));
    let req:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"wrong-project","project_id":"P2","scope":{"kind":"project"},"operation":"check_factory_draft","params":request()})).unwrap();
    assert!(matches!(
        error(h.host.dispatch(&agent, req)).code,
        ErrorCode::PermissionDenied
    ));
    h.host.revoke_external_session(&agent).unwrap();
    assert!(matches!(
        error(h.call_as(&agent, true, "check_factory_draft", request())).code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(h.snapshot(), before);
}
#[test]
fn public_check_routes_material_port_units_receiving_store_and_explicit_resource_calendars() {
    for change in ["port", "unit", "store", "calendar"] {
        let mut value = data::fixture();
        match change {
            "port" => {
                value["project"]["connections"][0]["data"]["destination"]["data"]["port_id"] =
                    json!("assembled")
            }
            "unit" => value["project"]["connections"][0]["data"]["unit"] = json!("kg"),
            "store" => {
                value["project"]["connections"][0]["data"]["destination_store"] = json!("pcbs-A")
            }
            _ => value["project"]["resources"][0]["calendar_id"] = json!("missing-calendar"),
        }
        let mut h = host(&value);
        let result = check(&mut h, request());
        let outcome = if change == "calendar" {
            CheckOutcome::Undetermined
        } else {
            CheckOutcome::Fail
        };
        assert!(
            has(&result, DraftCheckKind::ReferencesAndUnits, outcome),
            "{change}: {result:?}"
        );
    }
}
#[test]
fn public_fc24_bounds_pass_does_not_invent_height_or_spacing() {
    let mut value = data::fixture();
    value["project"]["halls"][0]["width_m"] = json!(5);
    value["project"]["halls"][0]["height_m"] = Value::Null;
    for (i, d) in value["project"]["devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        d["pose"]["translation"] = json!({"x":i*3,"y":0,"z":0});
        d["footprint_m"] = rectangle(0., 0., 2., 1.);
        d["height_m"] = Value::Null;
    }
    let mut params = request();
    params["generation_goal"]["site_envelope"] = json!({"length_m":5,"width_m":10,"height_m":null,"clearance_m":null,"regions":[],"usable_polygon_m":[]});
    let mut h = host(&value);
    let result = check(&mut h, params);
    assert!(has(&result, DraftCheckKind::SiteBounds, CheckOutcome::Pass));
    assert!(!has(
        &result,
        DraftCheckKind::SiteBounds,
        CheckOutcome::Fail
    ));
    assert!(has(
        &result,
        DraftCheckKind::Height,
        CheckOutcome::Undetermined
    ));
    assert!(has(
        &result,
        DraftCheckKind::ClearanceAndStaticLanes,
        CheckOutcome::Undetermined
    ));
}

#[test]
fn additional_product_scope_is_checked_and_static_aggregate_remains_explicitly_undetermined() {
    let mut value = data::fixture();
    let mut product = value["project"]["products"][0].clone();
    product["id"] = json!("P304-ASSEMBLY");
    product["item_type_ref"] = json!({"id":"assembly","revision":"r1"});
    value["project"]["products"]
        .as_array_mut()
        .unwrap()
        .push(product);
    let mut params = request();
    params["generation_goal"]["additional_acceptance_scopes"] = json!([{
        "product_ref":{"id":"P304-ASSEMBLY","revision":"r1"},"unit":"piece",
        "destination":"assemblies-A","stage":"factory_released"
    }]);
    let mut h = host(&value);
    let before = h.snapshot();
    let result = check(&mut h, params);
    assert!(
        !has(
            &result,
            DraftCheckKind::ReferencesAndUnits,
            CheckOutcome::Fail
        ),
        "{result:?}"
    );
    assert!(result.issues.iter().any(|i| i.code
        == "factory_check.aggregate_throughput_unverified"
        && i.field_path.as_deref() == Some("/generation_goal/additional_acceptance_scopes")));
    assert!(has(
        &result,
        DraftCheckKind::MaterialProcessConnectivity,
        CheckOutcome::Undetermined
    ));
    for kind in [
        DraftCheckKind::SiteBounds,
        DraftCheckKind::FootprintOverlap,
        DraftCheckKind::Height,
        DraftCheckKind::ClearanceAndStaticLanes,
    ] {
        assert!(has(&result, kind, CheckOutcome::Pass), "{result:?}");
    }
    assert_eq!(h.snapshot(), before);
}

#[test]
fn every_additional_scope_checks_exact_product_unit_destination_and_stage() {
    for (field, code, leaf) in [
        ("revision", "goal_product", "product_ref"),
        ("unit", "goal_unit", "unit"),
        ("destination", "goal_destination", "destination"),
        ("stage", "goal_acceptance_stage", "stage"),
    ] {
        let mut params = request();
        let mut additional = params["generation_goal"]["acceptance_scope"].clone();
        match field {
            "revision" => additional["product_ref"]["revision"] = json!("unselected-r2"),
            "unit" => additional["unit"] = json!("kg"),
            "destination" => additional["destination"] = json!("missing-receiver"),
            _ => additional["stage"] = json!("received"),
        }
        params["generation_goal"]["additional_acceptance_scopes"] = json!([additional]);
        let mut h = host(&data::fixture());
        let before = h.snapshot();
        let result = check(&mut h, params);
        let path = format!("/generation_goal/additional_acceptance_scopes/0/{leaf}");
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.code == format!("factory_check.{code}")
                    && i.field_path.as_deref() == Some(path.as_str())),
            "{field}: {result:?}"
        );
        assert!(has(
            &result,
            DraftCheckKind::ReferencesAndUnits,
            CheckOutcome::Fail
        ));
        assert!(has(&result, DraftCheckKind::SiteBounds, CheckOutcome::Pass));
        assert_eq!(h.snapshot(), before);
    }
}

#[test]
fn additional_acceptance_location_requires_its_own_declared_material_path() {
    let mut value = data::fixture();
    let mut destination = value["project"]["locations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "packages-B")
        .unwrap()
        .clone();
    destination["id"] = json!("P304-UNLINKED");
    value["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .push(destination);
    let mut params = request();
    let mut additional = params["generation_goal"]["acceptance_scope"].clone();
    additional["destination"] = json!("P304-UNLINKED");
    params["generation_goal"]["additional_acceptance_scopes"] = json!([additional]);
    let mut h = host(&value);
    let before = h.snapshot();
    let result = check(&mut h, params);
    assert!(
        result
            .issues
            .iter()
            .any(|i| i.code == "factory_check.acceptance_unreachable"
                && i.field_path.as_deref()
                    == Some("/generation_goal/additional_acceptance_scopes/0/destination")),
        "{result:?}"
    );
    assert!(has(
        &result,
        DraftCheckKind::MaterialProcessConnectivity,
        CheckOutcome::Fail
    ));
    assert!(has(&result, DraftCheckKind::Height, CheckOutcome::Pass));
    assert_eq!(h.snapshot(), before);
}

#[test]
fn additional_received_and_unloaded_scopes_use_the_actual_transport_declaration() {
    let mut value: Value =
        serde_json::from_str(include_str!("fixtures/production/p202-ix01-baseline.json")).unwrap();
    let supply = value["scenario"]["supplies"][0].clone();
    let mut product = value["project"]["products"][0].clone();
    product["id"] = json!("P304-TRANSIT-PRODUCT");
    product["item_type_ref"] = supply["item_type_ref"].clone();
    value["project"]["products"]
        .as_array_mut()
        .unwrap()
        .push(product);
    for stage in ["received", "unloaded_and_released"] {
        let mut params = request();
        params["generation_goal"]["additional_acceptance_scopes"] = json!([{
            "product_ref":{"id":"P304-TRANSIT-PRODUCT","revision":"r1"},
            "unit":supply["target_quantity"]["unit"],"destination":supply["destination"],"stage":stage
        }]);
        let mut h = host(&value);
        let before = h.snapshot();
        let result = check(&mut h, params);
        assert!(
            !result
                .issues
                .iter()
                .any(|i| i.code == "factory_check.goal_acceptance_stage"
                    && i.field_path.as_deref()
                        == Some("/generation_goal/additional_acceptance_scopes/0/stage")),
            "{stage}: {result:?}"
        );
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.code == "factory_check.aggregate_throughput_unverified")
        );
        assert_eq!(h.snapshot(), before);
    }
}
