"""Owned rustc-only verification; never calls Cargo or updates shared sources."""
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BUILD = ROOT / "scenarios/模板与交接/runtime/source-clock/consumer-build.stdout.txt"
producer = (ROOT / "packages/commands/src/host_factory_check.rs").read_text(encoding="utf-8")
start = producer.index("// factory-check-pure:start")
end = producer.index("// factory-check-pure:end")
# Exact current pure producer section, not a separately maintained checker.
# Exclude only ProjectHost plumbing, which must be validated by Root dispatch.
geometry = (ROOT / "packages/commands/src/host_factory_check/geometry.rs").as_posix()
(HERE / "checks.projection.rs").write_text(
    f'#[path = "{geometry}"] mod geometry;\n' + producer[start:end], encoding="utf-8")
LIBS = {}
for line in BUILD.read_text(encoding="utf-8").splitlines():
    try:
        row = json.loads(line)
    except ValueError:
        continue
    name = row.get("target", {}).get("name")
    if row.get("reason") != "compiler-artifact" or row.get("profile", {}).get("test"):
        continue
    if name not in {"sf_domain", "serde_json"}:
        continue
    # Use the manifest's exact hashed dependency artifact, never the mutable
    # unqualified target/debug/libsf_simulation.rlib alias or a newest-file glob.
    paths = [Path(p) for p in row["filenames"]]
    candidates = [p for p in paths if p.suffix == ".rmeta" and p.parent.name == "deps"]
    lib = candidates[0].with_suffix(".rlib")
    if not lib.is_file():
        raise FileNotFoundError(lib)
    LIBS[name] = lib
assert len(LIBS) == 2, LIBS
command = [str(Path.home() / ".cargo/bin/rustc.exe"), "--edition=2024", "--test",
           str(HERE / "pure_tests.rs"), "-L", f"dependency={ROOT / 'target/debug/deps'}",
           "-o", str(HERE / "pure-tests.exe")]
for name, path in LIBS.items():
    command.extend(["--extern", f"{name}={path}"])
(HERE / "library-set.json").write_text(json.dumps({"source_build_log": str(BUILD),
    "libraries": {n: {"path": str(p), "size": p.stat().st_size,
                       "mtime_ns": p.stat().st_mtime_ns} for n,p in LIBS.items()},
    "command": command, "Cargo_used": False}, ensure_ascii=False, indent=2), encoding="utf-8")
build = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
(HERE / "pure-build.stdout.txt").write_text(build.stdout, encoding="utf-8")
(HERE / "pure-build.stderr.txt").write_text(build.stderr, encoding="utf-8")
if build.returncode:
    print(build.stderr)
    sys.exit(build.returncode)
run = subprocess.run([str(HERE / "pure-tests.exe"), "--nocapture"], cwd=ROOT,
                     capture_output=True, text=True, encoding="utf-8")
(HERE / "pure-tests.stdout.txt").write_text(run.stdout, encoding="utf-8")
(HERE / "pure-tests.stderr.txt").write_text(run.stderr, encoding="utf-8")
print(run.stdout)
print(run.stderr)
sys.exit(run.returncode)
