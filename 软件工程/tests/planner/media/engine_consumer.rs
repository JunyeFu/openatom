use std::{fs,path::Path};
use serde_json::{Value,json};
use sf_simulation::{Engine,EngineInput};

fn make(v:&Value)->Engine {
    Engine::new(EngineInput {
        run_id:serde_json::from_value(json!("P302-CONTROLLED-RUN")).unwrap(),
        input_ref:serde_json::from_value(v["input_ref"].clone()).unwrap(),
        project:serde_json::from_value(v["project"].clone()).unwrap(),
        scenario:serde_json::from_value(v["scenario"].clone()).unwrap(),
        definitions:serde_json::from_value(v["definitions"].clone()).unwrap(),
        scope:serde_json::from_value(v["scenario"]["scope"].clone()).unwrap(),
    }).unwrap()
}
fn snapshot(e:&Engine)->Value {serde_json::to_value(e.snapshot()).unwrap()}
fn write(path:&Path,v:&Value){fs::write(path,serde_json::to_vec_pretty(v).unwrap()).unwrap();}
fn operation(v:&mut Value)->&mut Value {&mut v["definitions"].as_array_mut().unwrap().iter_mut().find(|d|d["id"]=="w5.op.phased").unwrap()["payload"]["data"]}
fn main(){
    let root=std::env::current_dir().unwrap();let output=root.join(option_env!("P302_TEST_OUTPUT").unwrap_or("scenarios/媒体表现/runtime"));
    let read=|path:&str|serde_json::from_slice::<Value>(&fs::read(root.join(path)).unwrap()).unwrap();
    let expected=read("tests/planner/media/expected.json");
    for variant in ["image","gif","model","model-simple","image-simple"] {
        let package=read(&format!("modules/media/p302-controlled/{variant}.module.json"));
        let _:sf_domain::ModulePackage=serde_json::from_value(package.clone()).unwrap();
        for asset in package["assets"].as_array().unwrap(){
            assert!(root.join("modules/media/p302-controlled").join(asset["path"].as_str().unwrap()).is_file());
            assert!(package["files"].as_array().unwrap().iter().any(|f|f["path"]==asset["path"]&&f["kind"]=="asset"));
            assert!(package["rights"].as_array().unwrap().iter().any(|r|r["asset_ref"]==asset["id"]&&r["license"]=="CC0-1.0"&&r["redistribution_confirmed"]==true));
        }
    }
    let base=read("packages/commands/tests/fixtures/production/p1-phase-calendar.json");
    let mut cases=vec![("calendar",base.clone()),("busy",read("packages/commands/tests/fixtures/production/p1-phase-busy.json"))];
    let mut output_wait=base.clone();output_wait["project"]["locations"][1]["capacity"]["value"]["coefficient"]=json!(0);cases.push(("output",output_wait));
    let mut gap=base.clone();gap["project"]["calendars"].as_array_mut().unwrap().push(json!({"id":"MACHINE-GAP","title":"off shift","source":"user_provided","intervals":[{"start_ms":0,"end_ms":20000},{"start_ms":40000,"end_ms":100000}]}));
    for resource in gap["project"]["resources"].as_array_mut().unwrap(){if resource["id"]=="CAP-CNC" {resource["calendar_id"]=json!("MACHINE-GAP");}if resource["id"]=="W-C" {resource["calendar_id"]=json!("ALL");}}
    operation(&mut gap)["phases"][1]["interruption_policy"]=json!("pause_resume");cases.push(("interrupt",gap));
    let mut missing=base.clone();missing["scenario"]["initial_conditions"]["items"]=json!([]);cases.push(("missing",missing));
    let mut cycle=base.clone();operation(&mut cycle)["duration"]=json!({"value":40000,"unit":"millisecond","basis":"user_provided","source_ref":null});operation(&mut cycle)["phases"]=json!([]);cases.push(("cycle",cycle));
    let mut zero=base.clone();for phase in operation(&mut zero)["phases"].as_array_mut().unwrap(){phase["duration"]["value"]=json!(0);}cases.push(("zero",zero));
    let mut results=vec![];let mut all_frames=vec![];
    for (name,v)in cases {
        write(&output.join(format!("{name}.input.json")),&v);
        let mut baseline:Option<Value>=None;
        for variant in ["image","gif","model"] {
            let package=read(&format!("modules/media/p302-controlled/{variant}.module.json"));
            let mut input=v.clone();
            for definition in input["definitions"].as_array_mut().unwrap(){if definition["payload"]["category"]=="devices" {definition["payload"]["data"]["presentation"]=package["definition"]["payload"]["data"].clone();}}
            let mut e=make(&input);let mut observations=vec![];let mut pause_frame=None;
            while !e.is_finished(){
                e.step();let current=snapshot(&e);
                let at=current["computed_until_ms"].clone();
                let mut objects:Vec<Value>=current["jobs"].as_array().unwrap().iter().map(|job|json!({"kind":"job","data":job})).collect();
                objects.extend(current["final_items"].as_array().unwrap().iter().map(|item|json!({"kind":"item","data":item})));
                objects.extend(input["project"]["resources"].as_array().unwrap().iter().map(|resource|json!({"kind":"resource","data":resource})));
                let page=json!({"run_id":current["id"],"at_ms":at,"objects":objects,"next_index":null});
                observations.push(page.clone());
                if name=="interrupt"&&at==20000&&!e.is_finished(){e.pause();pause_frame=Some((snapshot(&e),page));e.resume();}
            }
            let result=snapshot(&e);
            let job=result["jobs"].as_array().unwrap().iter().find(|j|j["operation_ref"]["id"]=="w5.op.phased").unwrap();
            assert_eq!(job["state"],expected["cases"][name]["state"],"{name}/{variant}");
            assert_eq!(job["finished_at_ms"],expected["cases"][name]["finished_at_ms"],"{name}/{variant}");
            if let Some(base_result)=&baseline {for field in expected["identical_fields"].as_array().unwrap(){let key=field.as_str().unwrap();assert_eq!(base_result[key],result[key],"{name}/{variant}/{key}");}}else{baseline=Some(result.clone());}
            write(&output.join(format!("{name}.{variant}.run.json")),&result);
            if variant=="model" {
                let op=v["definitions"].as_array().unwrap().iter().find(|d|d["id"]=="w5.op.phased").unwrap()["payload"]["data"].clone();
                for page in observations {all_frames.push(json!({"case":name,"label":format!("{name}@{}",page["at_ms"]),"frame":{"run":result,"snapshot":page,"job_id":job["id"],"operation":op}}));}
                if let Some((paused,page))=pause_frame{all_frames.push(json!({"case":name,"label":"interrupt-paused@20000","frame":{"run":paused,"snapshot":page,"job_id":job["id"],"operation":op}}));}
            }
            results.push(json!({"case":name,"variant":variant,"actual_engine":true,"state":job["state"],"finished_at_ms":job["finished_at_ms"],"events":result["events"].as_array().unwrap().len(),"identical_to_image":true}));
        }
    }
    write(&output.join("actual-frames.json"),&json!(all_frames));write(&output.join("engine-comparison.json"),&json!({"expected_frozen_before_execution":expected["frozen_before_execution"],"cases":results,"passed":true,"source":"sf_simulation::Engine; existing built rlib; no Host/listener/native operation"}));
    println!("5 canonical ModulePackages/files/rights checked; 21 actual Engine cases passed; PNG/GIF/GLB events/items/acceptance/resource/work/waits/jobs identical");
}
