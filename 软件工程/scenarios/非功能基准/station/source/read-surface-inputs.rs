use serde_json::{Value,json};
use std::{path::PathBuf,fs};

fn main(){
    let root=PathBuf::from(std::env::args().nth(1).expect("input root"));
    let mut cases=Vec::new();
    for variant in ["balanced","dense"]{
        let path=root.join(variant);
        let archive=sf_commands::read_project_archive(&path).expect("actual current directory reader");
        let project=serde_json::to_value(&archive.project).unwrap();
        assert_eq!(project["revision"],4);
        assert_eq!(project["devices"].as_array().unwrap().len(),60);
        let scenario=serde_json::to_value(archive.scenarios.values().next().unwrap()).unwrap();
        assert_eq!(scenario["initial_conditions"]["items"].as_array().unwrap().len(),3000);
        assert!(archive.manifest.documents.iter().all(|entry|serde_json::to_value(entry).unwrap()["role"]!="run_result"));
        let module:Value=serde_json::from_slice(archive.files.get("module-0.json").unwrap()).unwrap();
        assert_eq!(module["data"]["definition"]["version"],"1.0.3");
        let source=&module["data"]["definition"]["payload"]["data"]["presentation"]["source"]["data"];
        let bytes=archive.files.get(source["path"].as_str().unwrap()).unwrap();
        assert_eq!(&bytes[..4],b"glTF");
        assert_eq!(bytes.len(),1_156_868);
        cases.push(json!({"variant":variant,"project_id":project["id"],"revision":4,"devices":60,"initial_items":3000,"actual_archive_reader":"sf_commands::read_project_archive","module_version":"1.0.3","asset_bytes":bytes.len(),"new_Run_created":false}));
    }
    fs::write(root.join("archive-readback.json"),serde_json::to_vec_pretty(&json!({"result":"passed","cases":cases,"Native_acceptance":false,"formal_MQ_acceptance":false,"benchmark_ready":false})).unwrap()).unwrap();
    println!("{}",json!({"result":"passed","cases":2,"Native_acceptance":false}));
}
