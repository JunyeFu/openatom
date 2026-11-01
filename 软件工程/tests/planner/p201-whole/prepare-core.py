"""Prepare an own JSONL Core runner; compilation waits for P001's exact rlib cut."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
source = (ROOT / 'tests/planner/node-inspector/core_consumer.rs').read_text(encoding='utf-8')
source = source.replace('P303-ui', 'P201-ui').replace('P303-core', 'P201-core').replace('P303-', 'P201-')
source = source.replace('scenarios/媒体表现/runtime-node-inspector/core-session-', '.work/p201-whole-20261017/current-core-session-')
marker = ' fs::create_dir_all(directory).unwrap();let project=&input["project"];let scenario=&input["scenario"];'
generic = '''
 if let Some(packages)=input["packages"].as_array(){
  fs::create_dir_all(directory).unwrap();let project=&input["project"];let scenario=&input["scenario"];
  let mut entries=vec![doc("active_project","project",project["id"].as_str().unwrap(),"active.json",project["revision"].clone()),doc("scenario_input","scenario",scenario["id"].as_str().unwrap(),"scenario.json",scenario["revision"].clone())];
  write(&directory.join("active.json"),&json!({"schema_version":"1.0","data":project}));
  write(&directory.join("scenario.json"),&json!({"schema_version":"1.0","data":scenario}));
  for(i,package)in packages.iter().enumerate(){
   let definition=&package["definition"];let path=format!("module-{i}.json");
   let mut entry=doc("module_definition","module",definition["id"].as_str().unwrap(),&path,Value::Null);
   entry["module_ref"]=json!({"id":definition["id"],"version":definition["version"]});entries.push(entry);
   write(&directory.join(path),&json!({"schema_version":"1.0","data":package}));
  }
  write(&directory.join("project.json"),&json!({"schema_version":"1.0","data":{"project_id":project["id"],"project_revision":project["revision"],"title":project["title"],"generation":1,"documents":entries,"module_refs":project["module_refs"],"assets":[],"missing":[],"restart_policy":"restart_from_initial_input"}}));
  return;
 }
'''
assert source.count(marker) == 1
source = source.replace(marker, generic + marker)
source = source.replace('"selected_assets":["p303.align","p303.lower","p303.hold","p303.return"]', '"selected_assets":if input["packages"].is_array(){json!([])}else{json!(["p303.align","p303.lower","p303.hold","p303.return"])}')
source = source.replace('core.formal=formal.clone();', 'core.formal=if input["packages"].is_array(){Value::Null}else{formal.clone()};')
source = source.replace('"move_fresh"=>{', '''"select_open"=>{let selected=self.host.select_directory(&serde_json::from_value(json!("P201-ui")).unwrap(),serde_json::from_value(json!("import_package")).unwrap(),PathBuf::from(query["directory"].as_str().unwrap())).unwrap();json!(selected)},
  "move_fresh"=>{''')
old = 'let reply=self.raw(query["request"].clone());if !before.is_null()'
new = '''let reply=self.raw(query["request"].clone());
   if query["request"]["scope"]["kind"]=="workspace"&&query["request"]["operation"]=="apply_commands"&&reply["outcome"]["status"]=="ok"{
    let context=Self::data(self.request(json!({"kind":"workspace"}),"get_context",json!({"selection":[]})));
    if let Some(actual)=context["project"]["id"].as_str(){if actual!=self.project{self.project=actual.to_owned();self.formal=Value::Null;return reply;}}
   }
   if !before.is_null()'''
assert source.count(old) == 1
source = source.replace(old, new)
source = source.replace('"item-layout-producer frozen + CapturePreview frozen"', '"P201 actual shared Host; exact linked Commands frontier recorded beside executable"')
(ROOT / 'tests/planner/p201-whole/core_consumer.rs').write_text(source, encoding='utf-8')
print('Prepared own generic Core source; no rustc/Cargo invocation.')
