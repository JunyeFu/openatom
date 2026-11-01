use crate::contract::operation_name;
use rmcp::model::Tool;
use serde_json::{Value, json};
use sf_commands::{ApiRequest, CapabilityCatalog, OperationName};
use std::{collections::BTreeSet, io};

fn resolve<'a>(root: &'a Value, schema: &'a Value) -> io::Result<&'a Value> {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        root.pointer(
            reference
                .strip_prefix('#')
                .ok_or_else(|| io::Error::other("Schema reference is not local"))?,
        )
        .ok_or_else(|| io::Error::other("Canonical schema reference is missing"))
    } else {
        Ok(schema)
    }
}

fn collect_refs(value: &Value, names: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(name) = object
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|s| s.strip_prefix("#/$defs/"))
            {
                names.insert(name.to_owned());
            }
            for value in object.values() {
                collect_refs(value, names);
            }
        }
        Value::Array(array) => {
            for value in array {
                collect_refs(value, names);
            }
        }
        _ => {}
    }
}

pub fn tool_schema(operation: &OperationName) -> io::Result<Value> {
    let root =
        serde_json::to_value(rmcp::schemars::schema_for!(ApiRequest)).map_err(io::Error::other)?;
    let name = operation_name(operation);
    let mut selected = None;
    for variant in root["anyOf"]
        .as_array()
        .ok_or_else(|| io::Error::other("Canonical ApiRequest alternatives are missing"))?
    {
        let schema = resolve(&root, variant)?;
        let tag = resolve(&root, &schema["properties"]["operation"])?;
        if tag["enum"] == json!([name]) {
            selected = Some(schema.clone());
            break;
        }
    }
    let mut schema =
        selected.ok_or_else(|| io::Error::other("Operation has no canonical request schema"))?;
    let mut required = BTreeSet::new();
    collect_refs(&schema, &mut required);
    let mut definitions = serde_json::Map::new();
    while let Some(name) = required.pop_first() {
        if definitions.contains_key(&name) {
            continue;
        }
        let definition = root["$defs"]
            .get(&name)
            .ok_or_else(|| io::Error::other("Referenced canonical definition is missing"))?;
        collect_refs(definition, &mut required);
        definitions.insert(name, definition.clone());
    }
    schema["$defs"] = Value::Object(definitions);
    Ok(schema)
}

fn annotations(operation: &OperationName) -> Value {
    // MCP client hints describe effects; the shared Host still authorizes every call.
    let read_only = matches!(
        operation,
        OperationName::DescribeCapabilities
            | OperationName::GetContext
            | OperationName::GetObject
            | OperationName::GetRun
            | OperationName::CompareRuns
            | OperationName::GetTask
            | OperationName::QueryModuleLibrary
            | OperationName::CheckFactoryDraft
            | OperationName::GetExecutionSnapshot
            | OperationName::ReadExecutionView
    );
    let destructive = matches!(
        operation,
        OperationName::ApplyCommands
            | OperationName::AdoptArtifact
            | OperationName::CancelTask
            | OperationName::SaveProject
            | OperationName::ExportPackage
            | OperationName::ExportEvaluation
            | OperationName::ControlExecutionSession
            | OperationName::ExportExecutionReport
    );
    json!({"readOnlyHint":read_only,"destructiveHint":destructive,
           "openWorldHint":matches!(operation, OperationName::ReadCloudProduction | OperationName::StartExecutionSession | OperationName::ControlExecutionSession | OperationName::ExportExecutionReport | OperationName::ExportCloudData),"idempotentHint":read_only})
}

/// The ordinary Agent pipe is never a device Gateway, even if a catalog is stale.
pub fn agent_operation(operation: &OperationName) -> bool {
    !matches!(
        operation,
        OperationName::ApplyDeviceReceipt
            | OperationName::ApplyDeviceObservation
            | OperationName::GetExecutionCloudWork
            | OperationName::AcknowledgeExecutionCloudWork
    )
}

fn description(operation: &OperationName) -> &'static str {
    match operation {
        OperationName::StartExecutionSession => {
            "Explicitly prepare frozen current Project/Scenario revisions. Returns execution_preparation Task; observe its original request with get_task/H09. Prepared never starts production. Unknown admission must not be resubmitted."
        }
        OperationName::ReadCloudProduction => {
            "Read actual configuration/issues, catalog, fixed-filter data or execution pages. Returns cloud_read Task; preserve exact R/F/T, filters and all cursors. No credentials or device Gateway actions."
        }
        OperationName::GetExecutionSnapshot => {
            "Read selected execution identity at exact revision or actual latest. Preserve read origin, ownership, freshness and unsettled obligations; goal attainment alone is not terminal completion."
        }
        OperationName::ControlExecutionSession => {
            "Only an explicitly authorized user action using latest Host-owner access and original expected_execution_revision. On conflict refresh for review; never automatically retry. Run, pause and stop are distinct effects."
        }
        OperationName::ReadExecutionView => {
            "Read frozen inputs, exact R/F/T observations/history or recover one original request. not_found does not authorize resubmission; retain original identity and unresolved status."
        }
        OperationName::ExportExecutionReport => {
            "Export the fixed source identity/R/origin in PDF/HTML/CSV/JSON, using an actual current export_execution_report directory selection. Returns exporting Task; retain original request and do not retry unknown registration."
        }
        OperationName::ExportCloudData => {
            "Export the complete fixed-filter dataset (not just visible rows), with exact R or selected telemetry frontier/devices and export_cloud_data selection. Returns exporting Task; no arbitrary paths or fabricated tokens."
        }
        _ => {
            "Use exact canonical request, current scope/revision and actual Host authorization. Long actions return a TaskHandle; observe get_task for completion."
        }
    }
}

pub fn tools(catalog: &CapabilityCatalog) -> io::Result<Vec<Tool>> {
    catalog.implemented_operations.iter().filter(|capability| {
        agent_operation(&capability.operation) && !capability.allowed_scopes.is_empty()
            && (!matches!(capability.operation, OperationName::ApplyCommands) || !catalog.available_command_kinds.is_empty())
    }).map(|capability| {
        let name = operation_name(&capability.operation);
        let mut schema = tool_schema(&capability.operation)?;
        schema["properties"]["scope"] = json!({"enum":capability.allowed_scopes});
        serde_json::from_value(json!({
            "name":format!("superfactory_{name}"),
            "description":description(&capability.operation),
            "inputSchema":schema,
            "annotations":annotations(&capability.operation),
            "_meta":{"input_schema_ref":capability.input_schema_ref,"output_schema_ref":capability.output_schema_ref}
        })).map_err(io::Error::other)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hints_distinguish_queries_from_effects_without_granting_permission() {
        for (names, read_only) in [
            (
                &[
                    "describe_capabilities",
                    "get_context",
                    "get_object",
                    "get_run",
                    "compare_runs",
                    "get_task",
                    "query_module_library",
                    "check_factory_draft",
                ][..],
                true,
            ),
            (
                &[
                    "apply_commands",
                    "render_view",
                    "run_scenario",
                    "compute_packaging",
                    "cancel_task",
                    "adopt_artifact",
                    "import_package",
                    "save_project",
                    "export_package",
                    "export_evaluation",
                ][..],
                false,
            ),
        ] {
            for name in names {
                let operation = serde_json::from_value(json!(name)).unwrap();
                let hints = annotations(&operation);
                assert_eq!(hints["readOnlyHint"], read_only, "{name}");
                assert_eq!(hints["idempotentHint"], read_only, "{name}");
                assert_eq!(hints["openWorldHint"], false, "{name}");
                assert!(!read_only || hints["destructiveHint"] == false, "{name}");
                // This is only descriptor metadata, not an ApiRequest extension.
                assert!(
                    tool_schema(&operation).unwrap()["properties"]
                        .get("grants")
                        .is_none()
                );
            }
        }
        for name in [
            "apply_commands",
            "adopt_artifact",
            "cancel_task",
            "save_project",
            "export_package",
            "export_evaluation",
        ] {
            assert_eq!(
                annotations(&serde_json::from_value(json!(name)).unwrap())["destructiveHint"],
                true,
                "{name}"
            );
        }
    }
    #[test]
    fn schemas_come_from_actual_wire_operations_and_resolve_locally() {
        for name in [
            "describe_capabilities",
            "get_context",
            "get_object",
            "apply_commands",
            "render_view",
            "run_scenario",
            "get_run",
            "compare_runs",
            "compute_packaging",
            "get_task",
            "cancel_task",
            "adopt_artifact",
            "import_package",
            "save_project",
            "export_package",
            "export_evaluation",
            "query_module_library",
            "check_factory_draft",
            "read_cloud_production",
        ] {
            let operation = serde_json::from_value(json!(name)).unwrap();
            let schema = tool_schema(&operation).unwrap();
            assert_eq!(schema["additionalProperties"], false);
            let mut refs = BTreeSet::new();
            collect_refs(&schema, &mut refs);
            for reference in refs {
                assert!(schema["$defs"].get(reference).is_some());
            }
            assert!(schema["properties"].get("caller").is_none());
            assert!(schema["properties"].get("grants").is_none());
        }
    }

    #[test]
    fn cloud_read_schema_is_project_only_and_describes_remote_read_effects() {
        let hints = annotations(&OperationName::ReadCloudProduction);
        assert_eq!(
            hints,
            // H08 reads business data but registers a Host Task. Match the
            // frozen V2 effect contract without broadening its read grant.
            json!({"readOnlyHint":false,"destructiveHint":false,
            "openWorldHint":true,"idempotentHint":false})
        );
        let schema = tool_schema(&OperationName::ReadCloudProduction).unwrap();
        assert_eq!(
            schema["properties"]["scope"]["properties"]["kind"]["const"],
            "project"
        );
        assert_eq!(schema["properties"]["scope"]["additionalProperties"], false);
        assert_eq!(schema["properties"]["scope"]["required"], json!(["kind"]));
    }
}
