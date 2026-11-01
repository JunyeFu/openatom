import {readFile,writeFile,mkdir} from 'node:fs/promises';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-item-layout-consumer/',root);
await mkdir(out,{recursive:true});
let source=await readFile(new URL('tests/planner/factory-actual-items/core_consumer.rs',root),'utf8');
source=source.replaceAll('scenarios/媒体表现/runtime-factory-items','scenarios/媒体表现/runtime-item-layout-consumer');
source=source.replace('  "snapshot"=>self.snapshot(),',`  "snapshot"=>self.snapshot(),
  "select_save"=>{let base=std::env::current_exe().unwrap().parent().unwrap().join(format!("saved-{}",std::process::id()));fs::create_dir_all(&base).unwrap();let selected=self.host.select_directory(&serde_json::from_value(json!("P303-ui")).unwrap(),serde_json::from_value(json!("save_project")).unwrap(),base).unwrap();json!(selected)},
  "move_fresh"=>{let parent=std::env::current_exe().unwrap().parent().unwrap().to_path_buf();let from=parent.join(format!("saved-{}",std::process::id()));let to=parent.join(format!("moved-{}",std::process::id()));assert!(from.starts_with(&parent)&&to.starts_with(&parent));fs::rename(&from,&to).unwrap();self.host=ProjectHost::new(serde_json::from_value(json!(format!("layout-fresh-{}",std::process::id()))).unwrap(),serde_json::from_value(json!("P303-ui")).unwrap());self.formal=Value::Null;let selected=self.host.select_directory(&serde_json::from_value(json!("P303-ui")).unwrap(),serde_json::from_value(json!("import_package")).unwrap(),to.clone()).unwrap();json!({"selected":selected,"from":from,"to":to})},
  "register"=>{self.host.register_bundled_module_directory(&PathBuf::from(query["directory"].as_str().unwrap())).unwrap();json!({"ok":true})},
`);
source=source.slice(0,source.indexOf(' if input["item_media_fixture"]'))+`
 let snapshot=core.snapshot();let formal=core.run(json!({"kind":"project"}),snapshot["scenarios"][0].clone(),snapshot["project"]["revision"].clone(),json!({"kind":"project_scenario"}));core.formal=formal.clone();
 let context=Core::data(core.request(json!({"kind":"project"}),"get_context",json!({"selection":[]})));
 let ready=json!({"ready":true,"snapshot":snapshot,"formal_run":formal,"context":context,"capabilities":Core::data(core.request(json!({"kind":"workspace"}),"describe_capabilities",json!({}))),"producer_cut":"item-layout-producer frozen + CapturePreview frozen"});println!("{ready}");io::stdout().flush().unwrap();
 for line in io::stdin().lock().lines(){let query:Value=serde_json::from_str(&line.unwrap()).unwrap();println!("{}",core.reply(query));io::stdout().flush().unwrap();}
}
`;
await writeFile(new URL('core_consumer.rs',import.meta.url),source);
let core=await readFile(new URL('../media-adoption-source/core.mjs',import.meta.url),'utf8');core=core.replaceAll('runtime-media-adoption-consumer','runtime-item-layout-consumer').replace("fixture='scenarios/图片时间轴/runtime/phase.input.json'","fixture='scenarios/媒体表现/runtime-factory-items/root-input.json'");await writeFile(new URL('core.mjs',import.meta.url),core);
