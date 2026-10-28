use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sf_commands::*;
use std::{fs, path::PathBuf, process::Command};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("tools/type-contract-proof/api-fixtures")
                .join(format!("{name}.json")),
        )
        .unwrap(),
    )
    .unwrap()
}

fn roundtrip<T: DeserializeOwned + Serialize>(value: Value) {
    let parsed: T = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
}

fn create_request() -> Value {
    json!({
        "api_version":"0.1", "request_id":"create-1", "project_id":null,
        "scope":{"kind":"workspace"}, "operation":"apply_commands",
        "params":{"base_revision":null,"commands":[{"kind":"create_project","data":{
            "title":"最小工程", "time_reference":{"label":"相对时间","origin_iso8601":null,"timezone":null},
            "departure":"stay"
        }}]}
    })
}

#[test]
fn create_read_edit_requests_and_responses_roundtrip_at_the_real_crate_boundary() {
    let create = create_request();
    let parsed: ApiRequest = serde_json::from_value(create.clone()).unwrap();
    let ApiRequest::ApplyCommands(request) = parsed else {
        panic!("wrong operation")
    };
    assert!(request.project_id.is_none());
    assert!(matches!(request.scope, ApiScope::Workspace));
    assert!(request.params.base_revision.is_none());
    assert!(matches!(
        request.params.commands.as_slice(),
        [EditCommand::CreateProject(_)]
    ));
    roundtrip::<ApiRequest>(create);
    for operation in ["get_context", "get_object", "apply_commands"] {
        roundtrip::<ApiRequest>(fixture(&format!("{operation}.request")));
    }
    roundtrip::<Response<ProjectContext>>(fixture("get_context.response"));
    roundtrip::<Response<DomainObject>>(fixture("get_object.response"));
    roundtrip::<Response<EditCommit>>(fixture("apply_commands.response"));
}

#[test]
fn wire_boundary_rejects_malformed_requests_and_caller_supplied_authority() {
    let original = create_request();
    let mut cases = Vec::new();
    for field in [
        "api_version",
        "request_id",
        "project_id",
        "scope",
        "operation",
        "params",
    ] {
        let mut value = original.clone();
        value.as_object_mut().unwrap().remove(field);
        cases.push(value);
    }
    for (field, value) in [
        ("api_version", json!("9.0")),
        ("operation", json!("get_object")),
        ("scope", json!({"kind":"unknown"})),
        ("caller", json!({"origin":"interactive_ui"})),
        ("edit_grants", json!([])),
    ] {
        let mut mutated = original.clone();
        mutated[field] = value;
        cases.push(mutated);
    }
    for revision in [
        json!("r1"),
        json!(-1),
        json!(1.5),
        json!(9007199254740992u64),
    ] {
        let mut value = original.clone();
        value["params"]["base_revision"] = revision;
        cases.push(value);
    }
    let mut unknown = original.clone();
    unknown["params"]["commands"][0]["data"]["silent_default"] = json!(true);
    cases.push(unknown);
    for (index, value) in cases.into_iter().enumerate() {
        assert!(
            serde_json::from_value::<ApiRequest>(value).is_err(),
            "case {index}"
        );
    }
}

#[test]
fn revision_conflict_response_preserves_current_scope_and_both_revisions() {
    let mut value = fixture("apply_commands.response");
    value["outcome"] = json!({"status":"error","data":{
        "code":"RevisionConflict", "message":"expected revision 1; current revision 2",
        "object_refs":[], "field_errors":[], "expected_revision":1,"actual_revision":2
    }});
    value["context"]["project_revision"] = json!(2);
    value["context"]["scope_revision"] = json!(2);
    let parsed: Response<EditCommit> = serde_json::from_value(value.clone()).unwrap();
    let ApiOutcome::Error(error) = parsed.outcome else {
        panic!("not an error")
    };
    assert!(matches!(error.code, ErrorCode::RevisionConflict));
    assert_eq!(error.expected_revision.unwrap().0.value(), 1);
    assert_eq!(error.actual_revision.unwrap().0.value(), 2);
    roundtrip::<Response<EditCommit>>(value);
}

#[test]
fn schemas_preserve_the_sixteen_command_branches_and_required_project_header() {
    let schema = serde_json::to_value(schemars::schema_for!(EditCommand)).unwrap();
    let mut kinds: Vec<&str> = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|branch| branch["properties"]["kind"]["const"].as_str().unwrap())
        .collect();
    kinds.sort();
    let mut expected = [
        "create_project",
        "put_object",
        "remove_object",
        "set_project_settings",
        "create_definition_draft",
        "put_definition",
        "put_template_composition",
        "set_declared_parameter",
        "adopt_module",
        "instantiate_template",
        "put_packaging_job",
        "record_artifact_acknowledgment",
        "adopt_imported_draft",
        "commit_definition_version",
        "undo",
        "redo",
    ];
    expected.sort();
    assert_eq!(kinds, expected);
    let request =
        serde_json::to_value(schemars::schema_for!(Request<ApplyCommandsTag, ApplyCommandsParams>))
            .unwrap();
    assert!(
        request["required"]
            .as_array()
            .unwrap()
            .contains(&json!("project_id"))
    );
    assert_eq!(request["additionalProperties"], false);
    let rendered = request.to_string();
    for host_type in [
        "HostCallContext",
        "HostEditGrant",
        "CallerOrigin",
        "ResolvedCallScope",
    ] {
        assert!(!rendered.contains(host_type), "{host_type} leaked");
    }
}

#[test]
fn actual_rust_exports_compile_in_a_typescript_command_consumer() {
    let result = Command::new(env!("CARGO_BIN_EXE_export-contracts"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let consumer = root().join(".work/p0-01-project-command/command-consumer.ts");
    fs::create_dir_all(consumer.parent().unwrap()).unwrap();
    fs::write(&consumer, r#"
import type { ApiRequest } from '../../packages/commands/generated/typescript/ApiRequest';
import type { EditCommand } from '../../packages/commands/generated/typescript/EditCommand';
import type { ProjectState } from '../../packages/commands/generated/typescript/ProjectState';
import type { Revision } from '../../packages/commands/generated/typescript/Revision';
import type { EditScope } from '../../packages/commands/generated/typescript/EditScope';
const create: ApiRequest = {
  api_version:'0.1',request_id:'create-1',project_id:null,scope:{kind:'workspace'},operation:'apply_commands',
  params:{base_revision:null,commands:[{kind:'create_project',data:{
    title:'最小工程',time_reference:{label:'相对时间',origin_iso8601:null,timezone:null},departure:'stay'
  }}]}
};
const read: ApiRequest = {
  api_version:'0.1',request_id:'read-1',project_id:'P1',scope:{kind:'project'},operation:'get_object',
  params:{object_id:{kind:'project',id:'P1'},source:{kind:'draft'},node_index:null}
};
const edit: EditCommand = {kind:'set_project_settings',data:{title:'修改',time_reference:{label:'相对时间'},generation_context:null,sources:[]}};
const counter: Revision = 2;
const definition: EditScope = {kind:'definition_draft',data:{id:'draft-1',revision:1}};
// @ts-expect-error Required nullable header is not an omitted field.
const missingProject: ApiRequest = {api_version:'0.1',request_id:'r',scope:{kind:'workspace'},operation:'get_context',params:{selection:[]}};
// @ts-expect-error Revisions are numeric counters.
const badRevision: Revision = 'r1';
// @ts-expect-error Full ProjectState cannot be replaced by a project summary.
const partial: ProjectState = {id:'P1',revision:1,title:'partial'};
// @ts-expect-error Host authority is not a wire command.
const authority: EditCommand = {kind:'set_project_settings',data:{title:'x',time_reference:{label:'t'},generation_context:null,sources:[],edit_grants:[]}};
// @ts-expect-error Host context must never be exported to the wire consumer.
import type { HostCallContext } from '../../packages/commands/generated/typescript/ApiRequest';
"#).unwrap();
    let tsc = Command::new("node")
        .arg(root().join("node_modules/typescript/bin/tsc"))
        .args([
            "--strict",
            "--noEmit",
            "--target",
            "ES2022",
            "--module",
            "ESNext",
            "--moduleResolution",
            "Bundler",
        ])
        .arg(&consumer)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(
        tsc.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&tsc.stdout),
        String::from_utf8_lossy(&tsc.stderr)
    );
    for host in [
        "HostCallContext",
        "HostEditGrant",
        "ResolvedCallScope",
        "HostFieldGrant",
    ] {
        assert!(
            !root()
                .join(format!("packages/commands/generated/typescript/{host}.ts"))
                .exists()
        );
    }
}
