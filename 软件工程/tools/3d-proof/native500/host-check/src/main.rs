use sf_commands::ProjectHost;
use sf_domain::ObjectId;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::args_os().nth(1).expect("repository root"));
    let mut host = ProjectHost::new(ObjectId::try_from("native500-host".to_string()).unwrap(), ObjectId::try_from("native500-ui".to_string()).unwrap());
    for directory in std::fs::read_to_string(root.join("modules/media/native-equipment-500/register-directories.txt")).unwrap().lines() {
        host.register_bundled_module_directory(&root.join("modules/media/native-equipment-500").join(directory))
            .unwrap_or_else(|e| panic!("Native500 registration {directory} failed: {e:?}"));
    }
    let request = serde_json::from_value(serde_json::json!({
        "api_version":"0.1","request_id":"native500-query","project_id":null,
        "scope":{"kind":"workspace"},"operation":"query_module_library",
        "params":{"category_filters":[],"industry":null,"product_family":null,"site_envelope":null,"source_scope":["bundled"]}
    })).unwrap();
    let result = host.dispatch(&ObjectId::try_from("native500-ui".to_string()).unwrap(), request);
    assert_eq!(result["outcome"]["status"], "ok", "{result}");
    std::fs::write(root.join("tools/3d-proof/native500/host-library-result.json"),serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!("canonical Host bundle registration PASS (500 devices + 1000 LOD media)");
}
