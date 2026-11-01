//! Real public Host catalog conditions; matching never substitutes for a Run.
#[path = "fixtures/production/host_support.rs"] mod support;
use support::*;
use sf_commands::*;
use serde_json::{json,Value};
use std::path::{Path,PathBuf};

fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")}
fn fixtures()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/full_gf")}
fn fresh(name:&str)->Harness {let ui=id(&format!("{name}-ui"));Harness{host:ProjectHost::new(id(name),ui.clone()),ui,tree:Sandbox::new()}}
fn query(h:&mut Harness, industry:Value, family:Value, site:Value, sources:Value)->Value {
    let req=json!({"api_version":"0.1","request_id":"gf-catalog","project_id":null,"scope":{"kind":"workspace"},"operation":"query_module_library",
        "params":{"category_filters":["templates"],"industry":industry,"product_family":family,"site_envelope":site,"source_scope":sources}});
    let result:ModuleTemplateCandidates=ok(h.host.dispatch(&h.ui,serde_json::from_value(req).unwrap()));json!(result)
}
fn site()->Value {read(&fixtures().join("declared-model/generation-profile.json"))["data"]["site_applicability"].clone()}
fn matching(h:&mut Harness)->Value {query(h,json!("paper_carton_manufacturing"),json!("die_cut_fold_glued_paper_carton"),site(),json!(["bundled","project_packages"]))}
fn record(name:&str,value:Value){write(&root().join(".work/delivery-r2/catalog").join(name),&value)}

#[test]
fn exact_declared_model_matches_without_erasing_unknown_or_claiming_output() {
    let mut h=fresh("gf-exact");h.host.register_bundled_template_directory(&fixtures().join("declared-model")).unwrap();
    let result=matching(&mut h);let c=&result["candidates"][0];
    assert_eq!(c["module_ref"],json!({"id":"cu.complete-carton-example","version":"0.2.0"}));
    assert_eq!(c["applicable"],true,"{result}");
    assert!(c["missing"].as_array().unwrap().iter().any(|x|x["code"]=="template_example_not_measured"));
    assert!(c["template_profile"]["parameter_sources"].as_array().unwrap().iter().any(|x|x["basis"]=="unknown"));
    let context:ProjectContext=ok(h.host.dispatch(&h.ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"gf-empty","project_id":null,"scope":{"kind":"workspace"},"operation":"get_context","params":{"selection":[]}})).unwrap()));
    assert!(context.project.is_none());record("matching.json",result);
}

#[test]
fn explicit_industry_product_and_site_mismatches_return_no_matching_candidate() {
    let mut h=fresh("gf-mismatch");h.host.register_bundled_template_directory(&fixtures().join("declared-model")).unwrap();
    let mut small=site();small["length_m"]=json!(0.1);
    let cases=[("industry",query(&mut h,json!("other_industry"),json!("die_cut_fold_glued_paper_carton"),site(),json!(["bundled"]))),
        ("product",query(&mut h,json!("paper_carton_manufacturing"),json!("other_product"),site(),json!(["bundled"]))),
        ("site",query(&mut h,json!("paper_carton_manufacturing"),json!("die_cut_fold_glued_paper_carton"),small,json!(["bundled"])))];
    for (name,result) in cases {assert!(result["candidates"].as_array().unwrap().is_empty(),"{name}: {result}");assert!(!result["missing"].as_array().unwrap().is_empty());record(&format!("mismatch-{name}.json"),result);}
}

#[test]
fn missing_requested_or_declared_site_remains_unknown() {
    let mut h=fresh("gf-missing");h.host.register_bundled_template_directory(&fixtures().join("declared-model")).unwrap();
    let mut unknown=site();unknown["height_m"]=Value::Null;
    let cases=[("industry",query(&mut h,Value::Null,json!("die_cut_fold_glued_paper_carton"),site(),json!(["bundled"]))),
        ("product",query(&mut h,json!("paper_carton_manufacturing"),Value::Null,site(),json!(["bundled"]))),
        ("site_absent",query(&mut h,json!("paper_carton_manufacturing"),json!("die_cut_fold_glued_paper_carton"),Value::Null,json!(["bundled"]))),
        ("site",query(&mut h,json!("paper_carton_manufacturing"),json!("die_cut_fold_glued_paper_carton"),unknown,json!(["bundled"])))];
    for (name,result) in cases {assert_eq!(result["candidates"][0]["applicable"],false);assert!(result["candidates"][0]["missing"].as_array().unwrap().iter().any(|x|x["code"]=="module_library.unresolved_application_conditions"));record(&format!("missing-{name}.json"),result);}
    let mut original=fresh("gf-original");original.host.register_bundled_template_directory(&root().join("modules/templates/carton-factory/candidate")).unwrap();
    let result=matching(&mut original);assert_eq!(result["candidates"][0]["applicable"],false);assert_eq!(result["candidates"][0]["template_profile"]["site_applicability"]["height_m"],Value::Null);record("original-unknown.json",result);
}

#[test]
fn region_changes_and_missing_profile_are_not_an_invented_geometry_fit() {
    let mut h=fresh("gf-regions");h.host.register_bundled_template_directory(&fixtures().join("declared-model")).unwrap();
    let mut changed=site();changed["regions"][0]["kind"]=json!("keep_out");
    let result=query(&mut h,json!("paper_carton_manufacturing"),json!("die_cut_fold_glued_paper_carton"),changed,json!(["bundled"]));
    assert_eq!(result["candidates"][0]["applicable"],false);record("changed-region-unknown.json",result);
    let mut missing=fresh("gf-profile-missing");missing.host.register_bundled_template_directory(&fixtures().join("missing-profile")).unwrap();
    let result=matching(&mut missing);assert_eq!(result["candidates"][0]["applicable"],false);assert_eq!(result["candidates"][0]["template_profile"],Value::Null);record("missing-profile.json",result);
}

#[test]
fn exact_version_conflict_is_not_hidden_by_project_source_precedence() {
    let mut h=fresh("gf-version-conflict");let task=h.import(&fixtures().join("conflict-current"));h.finish_one();let draft=h.imported(&task);
    let _:EditCommit=ok(h.adopt(&task,&draft,false));
    h.host.register_bundled_template_directory(&fixtures().join("conflict-bundle")).unwrap();
    for (name,sources) in [("bundled",json!(["bundled"])),("project",json!(["project_packages"])),("both",json!(["bundled","project_packages"]))] {
        let result=query(&mut h,Value::Null,Value::Null,Value::Null,sources);
        let candidate=result["candidates"].as_array().unwrap().iter().find(|c|c["module_ref"]["id"]=="cu.complete-carton-example").unwrap();
        assert_eq!(candidate["applicable"],false,"{name}: {result}");
        assert!(candidate["missing"].as_array().unwrap().iter().any(|x|x["code"]=="module_library.exact_version_conflict"));record(&format!("version-conflict-{name}.json"),result);
    }
    let req=json!({"api_version":"0.1","request_id":"gf-conflicted-preview","project_id":"full-gf:project:1","scope":{"kind":"project"},"operation":"get_object",
        "params":{"object_id":{"kind":"module","id":"cu.complete-carton-example"},"node_index":null,"source":{"kind":"template_preview","data":{"module_ref":{"id":"cu.complete-carton-example","version":"0.1.0"},"base_revision":2,"choices":null,"seed":null}}}});
    let result=h.host.dispatch(&h.ui,serde_json::from_value(req).unwrap());assert!(matches!(error(result.clone()).code,ErrorCode::InvalidInput));record("version-conflict-preview-rejected.json",result);
}

#[test]
fn exact_version_source_bytes_conflict_blocks_catalog_and_preview() {
    let mut h=fresh("gf-source-conflict");let task=h.import(&fixtures().join("conflict-current"));h.finish_one();let draft=h.imported(&task);let _:EditCommit=ok(h.adopt(&task,&draft,false));
    h.host.register_bundled_template_directory(&fixtures().join("conflict-bytes")).unwrap();
    let result=query(&mut h,Value::Null,Value::Null,Value::Null,json!(["bundled","project_packages"]));
    let candidate=result["candidates"].as_array().unwrap().iter().find(|c|c["module_ref"]["id"]=="cu.complete-carton-example").unwrap();
    assert_eq!(candidate["applicable"],false,"Exact package metadata alone cannot bless different profile bytes");
    assert!(candidate["missing"].as_array().unwrap().iter().any(|x|x["code"]=="module_library.exact_version_conflict"));record("version-source-bytes-conflict.json",result);
    let req=json!({"api_version":"0.1","request_id":"gf-source-conflicted-preview","project_id":"full-gf:project:1","scope":{"kind":"project"},"operation":"get_object",
        "params":{"object_id":{"kind":"module","id":"cu.complete-carton-example"},"node_index":null,"source":{"kind":"template_preview","data":{"module_ref":{"id":"cu.complete-carton-example","version":"0.1.0"},"base_revision":2,"choices":null,"seed":null}}}});
    let result=h.host.dispatch(&h.ui,serde_json::from_value(req).unwrap());assert!(matches!(error(result.clone()).code,ErrorCode::InvalidInput));record("version-source-bytes-preview-rejected.json",result);
}

#[test]
fn unchanged_exact_bundle_still_matches_after_real_archive_roundtrip() {
    let mut h=fresh("gf-same-version-roundtrip");
    let source=fixtures().join("same-version-project");
    let task=h.import(&source);h.finish_one();let draft=h.imported(&task);let _:EditCommit=ok(h.adopt(&task,&draft,false));
    h.host.register_bundled_template_directory(&fixtures().join("declared-model")).unwrap();
    let result=matching(&mut h);let candidate=result["candidates"].as_array().unwrap().iter().find(|c|c["module_ref"]["id"]=="cu.complete-carton-example").unwrap();
    assert_eq!(candidate["applicable"],true,"Unchanged archive source must not become a version conflict: {result}");
    assert!(!candidate["missing"].as_array().unwrap().iter().any(|x|x["code"]=="module_library.exact_version_conflict"));record("same-version-roundtrip-matching.json",result);
}
