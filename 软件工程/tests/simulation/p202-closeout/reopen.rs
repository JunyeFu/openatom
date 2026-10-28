mod common;
use common::{host::*,*};
use serde_json::json;
use sf_domain::RunResult;
use std::path::PathBuf;
fn main(){
 let dir=PathBuf::from(std::env::args().nth(1).unwrap());let packet=read(&dir.join("packet.json"));let run:RunResult=serde_json::from_value(read(&dir.join("run.json"))).unwrap();
 let mut h=Harness::from_seed(Sandbox::new(),&dir.join("project"));
 assert_eq!(json!(h.full(&run.id)),json!(run));assert_eq!(history(&mut h,&run),read(&dir.join("history.json")));
 assert_eq!(frozen(&mut h,&run,"project","P1"),packet["project"]);
 assert_eq!(frozen(&mut h,&run,"scenario",packet["input"]["scenario"]["id"].as_str().unwrap()),packet["scenario"]);
 check_variants(packet["case"].as_str().unwrap(),&json!(run));
 write(&dir.join("child-receipt.json"),&json!({"passed":true,"pid":std::process::id(),"run_id":run.id,"history_exact":true,"frozen_inputs_exact":true}));
}
