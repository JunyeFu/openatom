use sf_domain::{ModulePackage, SchemaVersion, Versioned};
use serde_json::{json, Value};
use std::{env, fs, path::Path};

fn main() {
    let root = env::args().nth(1).expect("product root");
    let directories = [
        "modules/devices/manual-assembly/presentation/p303-owned",
        "modules/devices/manual-assembly/presentation/p303-owned/phase",
    ];
    let mut checks = Vec::new();
    for (directory, version) in directories.into_iter().zip(["0.1.0", "0.2.0"]) {
        let bytes = fs::read(Path::new(&root).join(directory).join("module.json")).unwrap();
        let document: Value = serde_json::from_slice(&bytes).unwrap();
        let package: Versioned<ModulePackage> = serde_json::from_slice(&bytes).unwrap();
        assert!(matches!(package.schema_version, SchemaVersion::V1));
        assert_eq!(serde_json::to_value(&package.data.definition.id).unwrap(), json!("p303.owned.manual-assembly"));
        assert_eq!(serde_json::to_value(&package.data.definition.version).unwrap(), json!(version));
        assert_eq!(package.data.assets.len(), 4);
        assert_eq!(package.data.files.len(), 5);
        assert_eq!(package.data.rights.len(), 4);
        for file in &document["data"]["files"].as_array().unwrap()[..] {
            assert!(!fs::read(Path::new(&root).join(directory).join(file["path"].as_str().unwrap())).unwrap().is_empty());
        }
        let negative = [
            ("bare_ModulePackage", document["data"].clone()),
            ("missing_schema", json!({"data":document["data"]})),
            ("wrong_schema", json!({"schema_version":"2.0","data":document["data"]})),
            ("missing_data", json!({"schema_version":"1.0"})),
        ];
        let mut rejected = Vec::new();
        for (name, value) in negative {
            let error = serde_json::from_value::<Versioned<ModulePackage>>(value).unwrap_err();
            rejected.push(json!({"case":name,"status":"rejected","error":error.to_string()}));
        }
        checks.push(json!({"directory":directory,"version":version,"status":"passed","canonical_type":"sf_domain::Versioned<ModulePackage>","negative_cases":rejected}));
    }
    println!("{}", serde_json::to_string_pretty(&json!({"checks":checks})).unwrap());
}
