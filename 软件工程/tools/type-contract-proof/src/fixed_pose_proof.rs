//! H64-01 actual canonical DTO parsing/roundtrip; no geometry or product Host.
use super::{api::*, contract::*};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{fs, path::Path};

fn parse<T: DeserializeOwned + Serialize>(value: &Value) -> (bool, Option<Value>) {
    match serde_json::from_value::<T>(value.clone()) {
        Ok(typed) => {
            let encoded = serde_json::to_value(typed).unwrap();
            assert_eq!(&encoded, value, "fixed-pose fixture changed in roundtrip");
            (true, Some(encoded))
        }
        Err(_) => (false, None),
    }
}

pub fn run(root: &Path) {
    let input: Value = serde_json::from_slice(
        &fs::read(root.join("fixed-pose-cases.design.json")).unwrap(),
    ).unwrap();
    let cases = input["cases"].as_array().unwrap();
    let mut results = Vec::new();
    for case in cases {
        let expected = case["expected_shape_accepted"].as_bool().unwrap();
        let job = parse::<Versioned<PackageStructureJob>>(&case["job"]);
        let compute = parse::<Request<ComputePackagingTag, ComputePackagingParams>>(
            &case["compute_request"],
        );
        let edit = parse::<Request<ApplyCommandsTag, ApplyCommandsParams>>(
            &case["edit_request"],
        );
        assert_eq!((job.0, compute.0, edit.0), (expected, expected, expected));
        results.push(serde_json::json!({
            "id": case["id"], "expected_shape_accepted": expected,
            "job_accepted": job.0, "compute_accepted": compute.0,
            "put_job_accepted": edit.0,
            "job_roundtrip": job.1, "compute_roundtrip": compute.1,
            "edit_roundtrip": edit.1
        }));
    }
    let result = serde_json::json!({
        "status": "passed_actual_canonical_fixed_pose_parse_and_roundtrip",
        "cases": results, "shape_checks": cases.len() * 3,
        "cross_reference_or_geometry_checked_by_native_parse": false,
        "actual_product_runtime": null,
        "existing_independent_geometry_expectations_changed": false
    });
    fs::write(root.join("output/fixed-pose-native-result.json"),
        serde_json::to_vec_pretty(&result).unwrap()).unwrap();
}
