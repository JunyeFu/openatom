"""Offline H1 archive reproduction; Python standard library, Windows x64."""
import argparse
import copy
import ctypes
from ctypes import wintypes
import filecmp
import gzip
import json
import os
from pathlib import Path, PurePosixPath
import platform
import subprocess
import sys
import time
import zipfile


def read(path):
    with path.open(encoding="utf-8-sig") as stream:
        return json.load(stream)


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as stream:
        json.dump(value, stream, ensure_ascii=False, separators=(",", ":"))


class MemoryCounters(ctypes.Structure):
    _fields_ = [("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD)] + [
        (name, ctypes.c_size_t) for name in (
            "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage",
            "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage", "QuotaNonPagedPoolUsage",
            "PagefileUsage", "PeakPagefileUsage", "PrivateUsage")]


def child(exe, args, output, name):
    start = time.perf_counter()
    peak_private = peak_working = peak_commit = 0
    get_memory = ctypes.WinDLL("psapi", use_last_error=True).GetProcessMemoryInfo
    get_memory.argtypes = [wintypes.HANDLE, ctypes.POINTER(MemoryCounters), wintypes.DWORD]
    get_memory.restype = wintypes.BOOL
    with (output / (name + ".jsonl")).open("wb") as stdout, (output / (name + ".stderr.txt")).open("wb") as stderr:
        process = subprocess.Popen([str(exe), *map(str, args)], stdout=stdout, stderr=stderr,
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        print(f"{name}: started PID {process.pid}; progress log: {output / (name + '.jsonl')}", flush=True)
        notice = start
        try:
            while True:
                counters = MemoryCounters()
                counters.cb = ctypes.sizeof(counters)
                if get_memory(int(process._handle), ctypes.byref(counters), counters.cb):
                    peak_private = max(peak_private, counters.PrivateUsage)
                    peak_working = max(peak_working, counters.PeakWorkingSetSize)
                    peak_commit = max(peak_commit, counters.PeakPagefileUsage)
                if process.poll() is not None:
                    break
                now = time.perf_counter()
                if now - notice >= 30:
                    print(f"{name}: running {now-start:.0f}s, sampled private peak {peak_private/1024**3:.2f} GiB", flush=True)
                    notice = now
                time.sleep(0.05)
        except KeyboardInterrupt:
            process.terminate()
            process.wait()
            raise
    result = {"pid": process.pid, "exit_code": process.returncode,
              "seconds": time.perf_counter() - start,
              "peak_working_set_bytes": peak_working,
              "peak_commit_bytes": peak_commit,
              "sampled_peak_private_bytes": peak_private,
              "private_sampling_interval_ms": 50,
              "phases": [json.loads(line) for line in (output / (name + ".jsonl")).read_text(encoding="utf-8").splitlines()]}
    write(output / (name + "-metrics.json"), result)
    if process.returncode:
        raise RuntimeError(f"{name} failed; see {output / (name + '.stderr.txt')}")
    print(f"{name}: passed ({result['seconds']:.1f}s)", flush=True)
    return result


def assemble(inputs_zip, prepared_path, output):
    start = time.perf_counter()
    fixture = output / "fixture"
    inputs = output / "inputs"
    fixture.mkdir()
    inputs.mkdir()
    with zipfile.ZipFile(inputs_zip) as archive:
        for member in archive.infolist():
            parts = PurePosixPath(member.filename).parts
            if member.is_dir() or not parts or parts[0] != "H1":
                continue
            if ".." in parts or PurePosixPath(member.filename).is_absolute() or any(":" in p or "\\" in p for p in parts):
                raise ValueError("Unsafe input ZIP path")
            target = fixture.joinpath(*parts[2:]) if parts[1] == "source-project" else inputs.joinpath(*parts[1:])
            target.parent.mkdir(parents=True, exist_ok=True)
            with archive.open(member) as source, target.open("xb") as dest:
                import shutil
                shutil.copyfileobj(source, dest)
    frozen = read(inputs / "actual-frozen-inputs.json")
    latest = read(inputs / "actual-PG-snapshot.json")
    prepared = read(prepared_path)["snapshot"]
    with (inputs / "actual-PG-history.jsonl").open(encoding="utf-8") as stream:
        records = [json.loads(line)["object"] for line in stream if line.strip()]
    assert len(records) == latest["observation_frontier"] == 63584
    assert latest["execution_revision"] == 15904
    assert frozen["identity"] == latest["identity"] == prepared["identity"]
    assert prepared["execution_revision"] == 93 and prepared["coverage"]["clock_started"] is False
    assert all(r["record_sequence"] == i + 1 for i, r in enumerate(records))
    initial = [r for r in records if r["execution_revision"] == 0]
    assert len(initial) == 626 and all(r["at_host_ms"] == 0 for r in initial)
    # Derived R0 fixture, never presented as a retained original snapshot.
    # Metadata is from the actual time-zero R93 prepared frame; object states
    # come exclusively from the original R0 records. Production reader validates
    # every retained frame against the complete original observation prefix.
    baseline = copy.deepcopy(prepared)
    baseline["execution_revision"] = 0
    baseline["observation_frontier"] = len(initial)
    mapping = {"item": ("business_facts", "items"), "transport": ("business_facts", "transport"),
               "job": ("bound_jobs",), "device": ("devices",),
               "logical_work": ("logical_work",), "attempt": ("attempts",)}
    for kind, path in mapping.items():
        target = baseline
        for key in path[:-1]:
            target = target[key]
        target[path[-1]] = [r["object"]["data"] for r in initial if r["object"]["kind"] == kind]
    manifest = read(fixture / "project.json")
    documents = manifest["data"]["documents"]
    owner = {"kind": "project", "id": frozen["identity"]["project_id"]}
    directory = "documents/fv0-h1-reconstructed"
    bindings = []
    used = set()

    def entry(reference):
        return {"role": "supporting_file", "module_ref": None, "object_ref": owner,
                "revision": None, "file_ref": reference, "required": True}

    def bind(original, value=None, asset_path=None):
        existing = next((e for e in documents if e["file_ref"] == {"path": asset_path, "kind": "asset"} and e["required"]), None) if asset_path else None
        if existing and asset_path not in used:
            stored = existing["file_ref"]
        else:
            stored = {"path": f"{directory}/frozen-{len(bindings):04d}.bin", "kind": "asset"}
            dest = fixture / stored["path"]
            if asset_path:
                import shutil
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(fixture / asset_path, dest)
            else:
                write(dest, {"schema_version": "1.0", "data": value})
            documents.append(entry(stored))
        used.add(stored["path"])
        bindings.append({"original_file": original, "stored_file": stored,
                         "byte_length": (fixture / stored["path"]).stat().st_size})

    bind(frozen["project_input_ref"]["frozen_project_file"], frozen["project"])
    bind(frozen["project_input_ref"]["frozen_scenario_file"], frozen["scenario"])
    for i, module in enumerate(frozen["modules"]):
        bind({"path": f"documents/execution-00000001/modules/{i:04d}.json", "kind": "module"}, module)
    for asset in frozen["asset_bindings"]:
        bind(asset["archive_file"], asset_path=asset["asset"]["path"])
    history = {"document_kind": "execution_observation_history", "contract_version": "sf.execution/2.0-draft11",
               "identity": frozen["identity"], "observation_frontier": len(records), "records": records}
    record = {"frozen_inputs": frozen, "snapshots": [baseline, prepared, latest],
              "history": history, "published_revisions": [15904], "files": bindings}
    source_ref = {"path": directory + "/source.json.gz", "kind": "execution_archive"}
    with gzip.open(fixture / source_ref["path"], "wt", encoding="utf-8", compresslevel=1) as stream:
        json.dump({"schema_version": "1.0", "data": record}, stream, ensure_ascii=False, separators=(",", ":"))
    documents.append(entry(source_ref))
    write(fixture / "project.json", manifest)
    result = {"seconds": time.perf_counter() - start, "kind": "reconstructed_read_only_fixture",
              "original_history_records": len(records), "original_latest_revision": 15904,
              "retained_snapshot_revisions": [93, 15904], "derived_baseline_revision": 0,
              "derived_baseline_records": len(initial), "all_original_snapshot_revisions_retained": False,
              "original_failed_save_recovered": False, "modules": len(frozen["modules"]),
              "asset_bindings": len(frozen["asset_bindings"])}
    write(output / "fixture-description.json", result)
    return fixture, inputs / "actual-PG-snapshot.json", fixture / source_ref["path"], result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", required=True, type=Path, help="Downloaded SuperFactory-FV0-Inputs-20261101.zip")
    parser.add_argument("--output", required=True, type=Path, help="New, nonexistent output directory (about 4 GB required)")
    args = parser.parse_args()
    if os.name != "nt":
        parser.error("This binary and memory measurement require Windows x64")
    home = Path(__file__).resolve().parent
    exe = home / "fv0_h1.exe"
    prepared = home / "actual-prepared-R93.json"
    if not all(p.is_file() for p in (args.inputs, exe, prepared)):
        parser.error("Inputs ZIP, fv0_h1.exe and actual-prepared-R93.json are required; extract the complete toolkit ZIP")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {"status": "running", "platform": platform.platform(), "python": platform.python_version(),
              "processor": platform.processor(), "build_profile": "debug", "native_ui_measured": False,
              "live_cloud_or_physical_devices_used": False, "b_machine_acceptance": "must be run by B"}
    write(output / "report.json", report)
    try:
        print("Preparing H1 fixture from downloaded records...", flush=True)
        fixture, expected, source, description = assemble(args.inputs.resolve(), prepared, output)
        report["fixture"] = description
        report["codec"] = child(exe, ["codec", source, output], output, "codec")
        report["save"] = child(exe, ["save", fixture, expected, output / "saved-project"], output, "save")
        report["reopen"] = child(exe, ["reopen", output / "saved-project", expected], output, "reopen")
        assert report["save"]["pid"] != report["reopen"]["pid"]
        immutable = {e["file_ref"]["path"] for e in read(fixture / "project.json")["data"]["documents"]
                     if e["required"] and e["file_ref"]["kind"] in ("asset", "module", "run_result", "execution_archive")}
        for relative in immutable:
            if not filecmp.cmp(fixture / relative, output / "saved-project" / relative, shallow=False):
                raise RuntimeError(f"Saved immutable file differs: {relative}")
        report["immutable_files_byte_equal"] = len(immutable)
        report["status"] = "passed"
    except (Exception, KeyboardInterrupt) as error:
        report["status"] = "failed"
        report["error"] = str(error) or "Interrupted by user; partial output retained, not a successful save"
        write(output / "report.json", report)
        raise
    write(output / "report.json", report)
    print(f"PASS: {output / 'report.json'}", flush=True)


if __name__ == "__main__":
    main()
