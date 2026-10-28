use schemars::schema_for;
use serde::{Serialize, de::DeserializeOwned};
use sf_domain::*;

#[test]
fn safe_uint_enforces_json_safe_integer_bounds() {
    for value in [0, 1, 9_007_199_254_740_991] {
        let number = SafeUint::try_from(value).unwrap();
        assert_eq!(number.value(), value);
        let json = value.to_string();
        assert_eq!(serde_json::to_string(&number).unwrap(), json);
        assert_eq!(
            serde_json::from_str::<SafeUint>(&json).unwrap().value(),
            value
        );
    }
    for value in [9_007_199_254_740_992, u64::MAX] {
        assert!(SafeUint::try_from(value).is_err());
    }
    for json in [
        "9007199254740992",
        "18446744073709551615",
        "-1",
        "1.5",
        "\"1\"",
        "null",
    ] {
        assert!(serde_json::from_str::<SafeUint>(json).is_err(), "{json}");
    }
}

#[test]
fn finite_number_preserves_precision_and_rejects_non_finite_values() {
    for value in [0.0, -1.25, 1.25, 0.12345678901234566, 1e300] {
        let number = FiniteNumber::try_from(value).unwrap();
        assert_eq!(number.value().to_bits(), value.to_bits());
        let json = serde_json::to_string(&number).unwrap();
        let restored = serde_json::from_str::<FiniteNumber>(&json).unwrap();
        assert_eq!(restored.value().to_bits(), value.to_bits(), "{json}");
    }
    assert_eq!(
        serde_json::to_string(&FiniteNumber::try_from(0.12345678901234566).unwrap()).unwrap(),
        "0.12345678901234566"
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(FiniteNumber::try_from(value).is_err());
    }
    for json in ["1e999", "NaN", "Infinity", "null", "\"1.25\""] {
        assert!(
            serde_json::from_str::<FiniteNumber>(json).is_err(),
            "{json}"
        );
    }
}

#[test]
fn revisions_and_times_keep_transparent_safe_integer_shapes() {
    assert_eq!(
        serde_json::to_string(&Revision(SafeUint::try_from(7).unwrap())).unwrap(),
        "7"
    );
    assert_eq!(
        serde_json::to_string(&SimTimeMs(SafeUint::try_from(1000).unwrap())).unwrap(),
        "1000"
    );
    assert_eq!(
        serde_json::to_string(&DurationMs(SafeUint::try_from(250).unwrap())).unwrap(),
        "250"
    );
    for json in ["-1", "1.5", "9007199254740992", "\"r1\"", "{\"value\":1}"] {
        assert!(serde_json::from_str::<Revision>(json).is_err());
        assert!(serde_json::from_str::<SimTimeMs>(json).is_err());
        assert!(serde_json::from_str::<DurationMs>(json).is_err());
    }
}

#[test]
fn package_relative_paths_reject_escape_and_noncanonical_separators() {
    for value in ["assets/model.glb", "模块/素材.png", "file.json"] {
        let path = RelativeFilePath::try_from(value.to_owned()).unwrap();
        assert_eq!(path.as_str(), value);
        assert_eq!(
            serde_json::from_str::<RelativeFilePath>(&serde_json::to_string(value).unwrap())
                .unwrap()
                .as_str(),
            value
        );
    }
    for value in [
        "",
        "/absolute",
        "../escape",
        "C:/escape",
        "a//b",
        "a/../b",
        "a/./b",
        "./b",
        "a/",
        "a\\b",
        "a\0b",
    ] {
        assert!(
            RelativeFilePath::try_from(value.to_owned()).is_err(),
            "{value:?}"
        );
        assert!(
            serde_json::from_str::<RelativeFilePath>(&serde_json::to_string(value).unwrap())
                .is_err(),
            "{value:?}"
        );
    }
}

fn check_opaque_id<T>()
where
    T: TryFrom<String, Error = String> + Serialize + DeserializeOwned,
{
    // IDs are opaque data, including slash/dot characters, not file paths or UUIDs.
    for value in ["P1", "设备-甲", "opaque/../id", "r1"] {
        let id = T::try_from(value.to_owned()).unwrap();
        assert_eq!(serde_json::to_value(id).unwrap(), serde_json::json!(value));
        assert!(serde_json::from_str::<T>(&serde_json::to_string(value).unwrap()).is_ok());
    }
    for value in ["", "a\0b"] {
        assert!(T::try_from(value.to_owned()).is_err());
        assert!(serde_json::from_str::<T>(&serde_json::to_string(value).unwrap()).is_err());
    }
    assert!(serde_json::from_str::<T>("1").is_err());
}

#[test]
fn all_ids_preserve_opaque_strings_and_reject_empty_or_nul() {
    check_opaque_id::<ProjectId>();
    check_opaque_id::<ObjectId>();
    check_opaque_id::<ScenarioId>();
    check_opaque_id::<RunId>();
    check_opaque_id::<TaskId>();
    check_opaque_id::<ModuleId>();
    check_opaque_id::<AssetId>();
    check_opaque_id::<ArtifactId>();
    check_opaque_id::<RoleId>();
    check_opaque_id::<PortId>();
    check_opaque_id::<PhaseId>();
    check_opaque_id::<UnitId>();
    check_opaque_id::<SubjectRevisionId>();
}

#[test]
fn subject_revision_id_is_distinct_from_numeric_draft_revision() {
    let subject = serde_json::from_str::<SubjectRevisionId>("\"r1\"").unwrap();
    assert_eq!(subject.as_str(), "r1");
    assert!(serde_json::from_str::<SubjectRevisionId>("1").is_err());
    assert!(serde_json::from_str::<Revision>("\"r1\"").is_err());
    assert_eq!(serde_json::from_str::<Revision>("1").unwrap().0.value(), 1);
}

#[test]
fn version_ids_require_exact_semver_and_preserve_valid_identifiers() {
    for value in [
        "0.0.0",
        "1.2.3",
        "1.2.3-alpha.1",
        "1.2.3+01.build",
        "1.2.3-alpha.1+build.009",
        "1.2.3--",
    ] {
        let version = VersionId::try_from(value.to_owned()).unwrap();
        assert_eq!(
            serde_json::to_value(version).unwrap(),
            serde_json::json!(value)
        );
        assert!(serde_json::from_str::<VersionId>(&serde_json::to_string(value).unwrap()).is_ok());
    }
    for value in [
        "",
        "1",
        "1.2",
        "v1.2.3",
        "01.2.3",
        "1.02.3",
        "1.2.03",
        "1.2.3-01",
        "1.2.3-",
        "1.2.3+",
        "1.2.3+a+b",
        "1.2.3-a..b",
        "1.2.3+构建",
        " 1.2.3",
        "1.2.*",
    ] {
        assert!(VersionId::try_from(value.to_owned()).is_err(), "{value}");
        assert!(
            serde_json::from_str::<VersionId>(&serde_json::to_string(value).unwrap()).is_err(),
            "{value}"
        );
    }
}

#[test]
fn derived_json_schemas_keep_primitive_wire_constraints() {
    let integer = serde_json::to_value(schema_for!(SafeUint)).unwrap();
    assert_eq!(integer["type"], "integer");
    assert_eq!(integer["minimum"].as_f64(), Some(0.0));
    assert_eq!(integer["maximum"].as_f64(), Some(9_007_199_254_740_991.0));
    let id = serde_json::to_value(schema_for!(ProjectId)).unwrap();
    assert_eq!(id["type"], "string");
    assert_eq!(id["minLength"], 1);
    let path = serde_json::to_value(schema_for!(RelativeFilePath)).unwrap();
    assert_eq!(path["type"], "string");
    assert!(path["pattern"].as_str().unwrap().contains("\\u0000"));
    let version = serde_json::to_value(schema_for!(VersionId)).unwrap();
    assert_eq!(version["type"], "string");
    assert!(version["pattern"].as_str().unwrap().starts_with('^'));
}
