"""Build against actual locked Cargo artifacts; compile only the owned native consumer."""
import json
import os
import subprocess
import sys
import time
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / '.work/whole-packages/window5-quality-comparison-mcp'
NATIVE = OUT / ('native-' + str(time.time_ns()))
NATIVE.mkdir(parents=True, exist_ok=True)
for name in ['revoke.json', 'revoked.json', 'selection.json', 'client-result.json']:
    (NATIVE/name).unlink(missing_ok=True)
seed = NATIVE/'seed'
seed.mkdir(exist_ok=True)
value = json.loads((ROOT/'packages/commands/tests/fixtures/quality/quality-original.json').read_text(encoding='utf8'))
value['project']['title'] = 'L' * 1200000
def write(path, value): path.write_text(json.dumps(value, ensure_ascii=False, separators=(',', ':')), encoding='utf8')
def document(role, kind, object_kind, object_id, path, revision):
    return {'role': role, 'object_ref': {'kind': object_kind, 'id': object_id}, 'revision': revision, 'module_ref': None, 'file_ref': {'path': path, 'kind': kind}, 'required': True}
write(seed/'active.json', {'schema_version': '1.0', 'data': value['project']})
write(seed/'scenario.json', {'schema_version': '1.0', 'data': value['scenario']})
entries = [document('active_project', 'project', 'project', 'P1', 'active.json', 1), document('scenario_input', 'scenario', 'scenario', 'QUALITY-ORIGINAL', 'scenario.json', 1)]
for i, definition in enumerate(value['definitions']):
    path = f'module-{i}.json'
    entry = document('module_definition', 'module', 'module', definition['id'], path, None)
    entry['module_ref'] = {'id': definition['id'], 'version': definition['version']}
    entries.append(entry)
    write(seed/path, {'schema_version': '1.0', 'data': {'definition': definition, 'assets': [], 'files': [], 'dependencies': [], 'rights': [], 'issues': []}})
write(seed/'project.json', {'schema_version': '1.0', 'data': {'project_id': 'P1', 'project_revision': 1, 'title': value['project']['title'], 'generation': 1, 'documents': entries, 'module_refs': value['project']['module_refs'], 'assets': [], 'missing': [], 'restart_policy': 'restart_from_initial_input'}})
with (OUT/'native-build.stdout.jsonl').open('wb') as a, (OUT/'native-build.stderr.txt').open('wb') as b:
    p = subprocess.run(['cargo', 'build', '-p', 'sf-desktop-host', '-p', 'sf-mcp-bridge', '--locked', '--message-format=json'], cwd=ROOT, stdout=a, stderr=b)
assert p.returncode == 0, 'actual locked producer build failed; see native-build.stderr.txt'
artifacts = {}
linked_paths = set()
build_out = None
for line in (OUT/'native-build.stdout.jsonl').read_text(encoding='utf8').splitlines():
    event = json.loads(line)
    if event['reason'] == 'compiler-artifact':
        for filename in event['filenames']:
            if filename.endswith('.rlib'): artifacts[event['target']['name']] = filename
    if event['reason'] == 'build-script-executed':
        linked_paths.update(event['linked_paths'])
        if 'sf-desktop-host' in event['package_id']: build_out = event['out_dir']
externs = ['sf_commands', 'sf_domain', 'serde', 'serde_json', 'tauri', 'tauri_plugin_dialog', 'tokio', 'windows_sys']
# Cargo may emit host/build-script and target copies of the same crate. Use
# the actual desktop artifact's exact direct dependencies, rather than the last
# similarly named artifact (which can be a different serde feature context).
fingerprints = ROOT/'target/debug/.fingerprint'
desktop = max(fingerprints.glob('sf-desktop-host-*/lib-sf_desktop_host.json'), key=lambda p:p.stat().st_mtime)
dependencies = {entry[1]:entry[3].to_bytes(8, 'little').hex() for entry in json.loads(desktop.read_text())['deps']}
for name in externs:
    candidates = [p for p in fingerprints.glob('*/lib-'+name) if p.read_text() == dependencies[name]]
    assert len(candidates) == 1, (name, candidates)
    suffix = candidates[0].parent.name.rsplit('-', 1)[1]
    artifacts[name] = str(ROOT/'target/debug/deps'/f'lib{name}-{suffix}.rlib')
    assert Path(artifacts[name]).is_file(), artifacts[name]
args = ['rustc', '--edition=2024', '--crate-name', 'w5_native_mcp_consumer', str(ROOT/'tests/delivery/mcp_acceptance/native_consumer.rs'), '-o', str(NATIVE/'native-consumer.exe'), '-L', 'dependency='+str(ROOT/'target/debug/deps')]
for name in externs: args += ['--extern', name+'='+artifacts[name]]
for path in sorted(linked_paths): args += ['-L', path]
for line in (Path(build_out).parent/'output').read_text(encoding='utf8').splitlines():
    if line.startswith('cargo:rustc-link-arg=') or line.startswith('cargo:rustc-link-arg-bins='):
        args += ['-C', 'link-arg='+line.split('=', 1)[1]]
    elif line.startswith('cargo:rustc-cfg='):
        args += ['--cfg', line.split('=', 1)[1]]
bridge = NATIVE/'actual-bridge.exe'
shutil.copy2(ROOT/'target/debug/superfactory-mcp-bridge.exe', bridge)
env = dict(os.environ, CARGO_MANIFEST_DIR=str(ROOT/'apps/planner/src-tauri'), CARGO_PKG_NAME='sf-desktop-host', CARGO_PKG_VERSION='0.1.0', CARGO_PKG_DESCRIPTION='Independent actual producer consumer', CARGO_PKG_AUTHORS='', OUT_DIR=build_out, W5_NATIVE_SEED=str(seed), W5_TEST_PYTHON=sys.executable, W5_NATIVE_OUTPUT=str(NATIVE), W5_NATIVE_BRIDGE=str(bridge))
with (OUT/'native-consumer-build.stdout.txt').open('wb') as a, (OUT/'native-consumer-build.stderr.txt').open('wb') as b:
    p = subprocess.run(args, cwd=ROOT, env=env, stdout=a, stderr=b)
assert p.returncode == 0, 'independent native consumer compilation failed; see native-consumer-build.stderr.txt'
with (OUT/'native-consumer.stdout.txt').open('wb') as a, (OUT/'native-consumer.stderr.txt').open('wb') as b:
    p = subprocess.run([str(NATIVE/'native-consumer.exe')], cwd=ROOT, env=env, stdout=a, stderr=b, creationflags=subprocess.CREATE_NO_WINDOW, timeout=60)
print('actual native consumer exit', p.returncode)
write(OUT/'native-latest.json', {'output': str(NATIVE), 'exit_code': p.returncode})
if (NATIVE/'client-result.json').exists():
    result = json.loads((NATIVE/'client-result.json').read_text(encoding='utf8'))
    for group in result['actual_groups']: print(json.dumps(group, ensure_ascii=False))
print((OUT/'native-consumer.stderr.txt').read_text(encoding='utf8', errors='replace')[-1400:])
sys.exit(p.returncode)
