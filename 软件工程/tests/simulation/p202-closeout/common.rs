#![allow(dead_code)]
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
pub mod host;
#[path = "../../../packages/commands/tests/fixtures/production/p1_engine_support.rs"]
pub mod engine_support;
use host::*;
use serde_json::{json, Value};
use sf_domain::*;
use sf_commands::RunView;
use std::{collections::BTreeSet,path::PathBuf};
pub fn crate_dir() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")) }
pub fn delivery_dir() -> PathBuf { crate_dir().join(".work/delivery") }
pub fn source(case: &str) -> Value {
    let mut v=read(&crate_dir().join(format!("fixtures/{case}.json")));
    let selected=v["project"]["views"]["selected_hall"].clone();
    if !v["project"]["halls"].as_array().unwrap().iter().any(|h| h["id"]==selected) {
        v["project"]["views"]["selected_hall"]=v["project"]["halls"][0]["id"].clone();
    }
    v
}
pub fn business(mut v:Value)->Value { for k in ["id","input_ref"] {v.as_object_mut().unwrap().remove(k);} v }
pub fn moments(run:&RunResult)->Vec<u64> {
    let mut times=BTreeSet::new(); times.insert(0);
    let v=json!(run); let end=v["window"]["end_ms"].as_u64().unwrap(); times.insert(end);
    for e in &run.events {let t=json!(e)["at_ms"].as_u64().unwrap(); times.insert(t); if t>0 {times.insert(t-1);} }
    if end>=86400000 {times.insert(86400000);times.insert(86399999);}
    times.into_iter().collect()
}
pub fn history(h:&mut Harness,run:&RunResult)->Value {
    let selected:Vec<_>=run.final_items.iter().map(|i|json!({"kind":"item","id":i.id})).collect();
    let mut rows=vec![];
    for at in moments(run) {
        let mut cursor:Option<SafeUint>=None; let mut objects=vec![];
        loop {
            let view:RunView=ok(h.call("get_run",json!({"run_id":run.id,"query":{"kind":"objects_at","data":{"at_ms":at,"objects":selected,"page":{"after_index":cursor,"limit":64}}}})));
            let RunView::Objects(p)=view else {panic!("objects")}; objects.extend(p.objects);cursor=p.next_index;if cursor.is_none(){break;}
        }
        let values:Vec<Value>=objects.into_iter().map(|o|json!(o)["data"].clone()).collect();
        let ids:BTreeSet<_>=values.iter().map(|i|i["id"].as_str().unwrap()).collect(); assert_eq!(ids.len(),values.len());
        rows.push(json!({"at_ms":at,"items":values}));
    }
    let mut cursor:Option<SafeUint>=None;let mut events=vec![];
    loop {
        let view:RunView=ok(h.call("get_run",json!({"run_id":run.id,"query":{"kind":"events","data":{"interval":null,"page":{"after_index":cursor,"limit":64}}}})));
        let RunView::Events(p)=view else {panic!("events")}; events.extend(p.events);cursor=p.next_index;if cursor.is_none(){break;}
    }
    assert_eq!(json!(events),json!(run.events));
    json!({"samples":rows,"events":events})
}
pub fn frozen(h:&mut Harness,run:&RunResult,kind:&str,id:&str)->Value {
    let v:sf_commands::DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":kind,"id":id},"source":{"kind":"run_snapshot","data":{"run_id":run.id,"at_ms":0}},"node_index":null})));
    json!(v)["data"].clone()
}
pub fn check_variants(case:&str,v:&Value) {
    use engine_support::{intervals,held_ms};
    match case {
        "same-carrier"=>assert_eq!(intervals(v,"T1"),vec![(0,80000),(80000,160000)]),
        "concurrent-unload"=>{
            let t=v["transport"].as_array().unwrap();assert_eq!(t.len(),2);
            assert_eq!(t.iter().filter(|t|t["received_ms"]==50000).count(),1);
            assert_eq!(t.iter().filter(|t|t["received_ms"].is_null()).count(),1);
            assert_eq!(held_ms(v,"T1"),320000);
        },
        "split-batch"=>{
            assert_eq!(v["transport"].as_array().unwrap().len(),2);
            let physical:Vec<_>=v["final_items"].as_array().unwrap().iter().filter(|i|i["lifecycle"]!="transformed").collect();
            assert_eq!(physical.iter().map(|i|i["quantity"]["value"]["coefficient"].as_u64().unwrap()).sum::<u64>(),3);
            assert!(physical.iter().all(|i|i["holder"]==json!({"kind":"location","id":"cargo-destination"})));
            assert!(v["allocations"].as_array().unwrap().is_empty());
        },
        "closed-calendar"=>{assert!(v["transport"].as_array().unwrap().is_empty());assert_eq!(held_ms(v,"T1"),0);},
        _=>{}
    }
}
