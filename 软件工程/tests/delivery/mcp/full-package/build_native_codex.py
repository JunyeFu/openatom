"""Link one owned acceptance app to the explicitly selected frozen Native dependency set.

Uses existing rustc artifacts, never Cargo or a second Commands/Engine implementation.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[4]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--desktop-fingerprint', type=Path, required=True)
parser.add_argument('--build-output', type=Path, required=True)
parser.add_argument('--tauri-config', type=Path, required=True)
parser.add_argument('--dependency-output', type=Path, action='append', default=[])
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=False)
native = ROOT / 'apps/planner/src-tauri/src'
# A Rust #[path] import changes default child-module lookup. Redirect only
# those declarations; keep the complete production function bodies unchanged.
project_source = (native / 'project_api.rs').read_text(encoding='utf-8')
for child in ['manufacturing_export', 'native_capture_probe', 'port_output', 'public_render_smoke']:
    declaration = f'pub(super) mod {child};'
    assert project_source.count(declaration) == 1, declaration
    project_source = project_source.replace(declaration,
        '#[path = ' + json.dumps((native / 'project_api' / (child + '.rs')).as_posix(), ensure_ascii=False) + ']\n' + declaration)
project_projection = args.output.resolve() / 'project_api.rs'
project_projection.write_text(project_source, encoding='utf-8')
harness = (ROOT / 'tests/delivery/mcp/full-package/native_codex.rs').read_text(encoding='utf-8')
for module in ['agent_pipe', 'project_api', 'native_render', 'native_render_capture']:
    source = project_projection if module == 'project_api' else native / (module + '.rs')
    harness = harness.replace('"../../../../apps/planner/src-tauri/src/' + module + '.rs"', json.dumps(source.as_posix(), ensure_ascii=False))
main = args.output.resolve() / 'native_codex.rs'
main.write_text(harness, encoding='utf-8')
target = args.desktop_fingerprint.resolve().parents[2]
fingerprints = target / '.fingerprint'
fingerprint = json.loads(args.desktop_fingerprint.read_text(encoding='utf-8'))
deps = {entry[1]: entry[3].to_bytes(8, 'little').hex() for entry in fingerprint['deps']}
externs = ['sf_commands', 'sf_domain', 'serde', 'serde_json', 'tauri', 'tauri_plugin_dialog', 'tokio', 'windows_sys', 'webview2_com', 'windows']
command = ['C:/Users/fujunye/.cargo/bin/rustc.exe', '--edition=2024', '--crate-name', 'p304_native_codex',
           str(main), '-L', 'dependency=' + str(args.output.resolve() / 'dependencies'), '-L', 'dependency=' + str(target / 'deps'),
           '-o', str(args.output / 'p304-native-codex.exe')]
libraries = {}
frozen = args.output.resolve() / 'dependencies'
frozen.mkdir()
def freeze_library(name, expected):
    matches = [path for path in fingerprints.glob('*/lib-' + name) if path.read_text(encoding='utf-8').strip() == expected]
    assert len(matches) == 1, (name, matches)
    suffix = matches[0].parent.name.rsplit('-', 1)[1]
    path = target / 'deps' / f'lib{name}-{suffix}.rlib'
    assert path.is_file(), path
    copied = frozen / path.name
    shutil.copy2(path, copied)
    metadata = matches[0].with_name('lib-' + name + '.json')
    shutil.copy2(metadata, frozen / (matches[0].parent.name + '.json'))
    libraries[name] = {'producer': str(path), 'frozen': str(copied)}
    return copied, json.loads(metadata.read_text(encoding='utf-8'))
local_dependencies = {}
for name in externs:
    path, metadata = freeze_library(name, deps[name])
    command += ['--extern', name + '=' + str(path)]
    if name == 'sf_commands':
        local_dependencies = {entry[1]: entry[3].to_bytes(8, 'little').hex() for entry in metadata['deps'] if entry[1].startswith('sf_')}
for name, expected in local_dependencies.items():
    if name not in libraries:
        freeze_library(name, expected)
shutil.copy2(args.desktop_fingerprint, args.output / 'desktop-fingerprint.json')
shutil.copy2(args.build_output, args.output / 'native-build-output.txt')
shutil.copy2(args.tauri_config, args.output / 'tauri-context.json')
for feature in json.loads(fingerprint['features']):
    command += ['--cfg', f'feature="{feature}"']
for line in '\n'.join(path.read_text(encoding='utf-8') for path in [args.build_output, *args.dependency_output]).splitlines():
    if line.startswith(('cargo:rustc-link-arg=', 'cargo:rustc-link-arg-bins=')):
        command += ['-C', 'link-arg=' + line.split('=', 1)[1]]
    elif line.startswith('cargo:rustc-cfg='):
        command += ['--cfg', line.split('=', 1)[1]]
    elif line.startswith('cargo:rustc-link-search='):
        command += ['-L', line.split('=', 1)[1]]
environment = dict(os.environ, CARGO_MANIFEST_DIR=str(ROOT / 'apps/planner/src-tauri'), CARGO_PKG_NAME='sf-desktop-host',
    CARGO_PKG_VERSION='0.1.0', CARGO_PKG_DESCRIPTION='Actual Native Codex acceptance', CARGO_PKG_AUTHORS='',
    OUT_DIR=str(args.build_output.parent / 'out'), TAURI_CONFIG=args.tauri_config.read_text(encoding='utf-8-sig'))
with (args.output / 'build.stdout.txt').open('wb') as stdout, (args.output / 'build.stderr.txt').open('wb') as stderr:
    result = subprocess.run(command, cwd=ROOT, env=environment, stdout=stdout, stderr=stderr)
(args.output / 'build.json').write_text(json.dumps({'command': command, 'exit_code': result.returncode,
    'desktop_fingerprint': str(args.desktop_fingerprint), 'build_output': str(args.build_output), 'dependency_outputs': [str(path) for path in args.dependency_output], 'tauri_config': str(args.tauri_config), 'libraries': libraries,
    'Cargo_used': False, 'model_call_executed': False}, ensure_ascii=False, indent=2), encoding='utf-8')
print('Owned Native link exit', result.returncode)
raise SystemExit(result.returncode)
