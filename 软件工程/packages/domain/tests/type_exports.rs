use sf_domain::*;
use std::{fs, path::Path, process::Command};
use ts_rs::{Config, TS};

#[test]
fn exported_rust_value_types_compile_in_a_typescript_consumer() {
    let package = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = package.parent().unwrap().parent().unwrap();
    let config = Config::new();
    let declarations = [
        SafeUint::decl(&config),
        FiniteNumber::decl(&config),
        RelativeFilePath::decl(&config),
        Revision::decl(&config),
        SimTimeMs::decl(&config),
        DurationMs::decl(&config),
        ProjectId::decl(&config),
        ObjectId::decl(&config),
        ScenarioId::decl(&config),
        RunId::decl(&config),
        TaskId::decl(&config),
        ModuleId::decl(&config),
        AssetId::decl(&config),
        ArtifactId::decl(&config),
        RoleId::decl(&config),
        PortId::decl(&config),
        PhaseId::decl(&config),
        UnitId::decl(&config),
        SubjectRevisionId::decl(&config),
        VersionId::decl(&config),
    ];
    let mut output =
        "// Generated from sf-domain Rust value types. Do not edit by hand.\n".to_owned();
    for declaration in declarations {
        output.push_str(&format!("export {declaration}\n"));
    }
    let generated = package.join("generated/value-types.ts");
    fs::create_dir_all(generated.parent().unwrap()).unwrap();
    fs::write(&generated, output).unwrap();
    let consumer = root.join(".work/p0-01/type-consumer.ts");
    fs::create_dir_all(consumer.parent().unwrap()).unwrap();
    fs::write(&consumer, r#"
import type {
  SafeUint, FiniteNumber, RelativeFilePath, Revision, SimTimeMs, DurationMs,
  ProjectId, ObjectId, ScenarioId, RunId, TaskId, ModuleId, AssetId, ArtifactId,
  RoleId, PortId, PhaseId, UnitId, SubjectRevisionId, VersionId,
} from '../../packages/domain/generated/value-types';

const counts: [SafeUint, Revision, SimTimeMs, DurationMs, FiniteNumber] = [1, 2, 1000, 250, 1.25];
const paths: RelativeFilePath = 'assets/model.glb';
const ids: [ProjectId, ObjectId, ScenarioId, RunId, TaskId, ModuleId, AssetId, ArtifactId, RoleId, PortId, PhaseId, UnitId] =
  ['P1', 'O1', 'S1', 'R1', 'T1', 'M1', 'A1', 'AR1', 'role', 'port', 'phase', 'unit'];
const revision: SubjectRevisionId = 'r1';
const version: VersionId = '1.2.3';
// @ts-expect-error Numeric revision must not become a string or bigint.
const wrongRevision: Revision = 'r1';
// @ts-expect-error Opaque subject revision must not become a numeric counter.
const wrongSubject: SubjectRevisionId = 1;
// @ts-expect-error Safe integers retain number on the wire.
const wrongCount: SafeUint = 1n;
// @ts-expect-error Paths retain string on the wire.
const wrongPath: RelativeFilePath = 1;
"#).unwrap();
    let result = Command::new("node")
        .arg(root.join("node_modules/typescript/bin/tsc"))
        .args([
            "--strict",
            "--noEmit",
            "--target",
            "ES2022",
            "--module",
            "ESNext",
            "--moduleResolution",
            "Bundler",
        ])
        .arg(&consumer)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
