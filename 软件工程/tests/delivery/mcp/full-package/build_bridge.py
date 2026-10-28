"""Build the actual bridge sources with frozen existing artifacts, without Cargo."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[4]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--fingerprint', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=False)
target = args.fingerprint.resolve().parents[2]
metadata = json.loads(args.fingerprint.read_text(encoding='utf-8'))
frozen = args.output.resolve() / 'dependencies'
frozen.mkdir()
pending = {d[1]: d[3].to_bytes(8, 'little').hex() for d in metadata['deps']}
externs = {}
while pending:
    name, digest = pending.popitem()
    if name in externs:
        continue
    matches = [p for p in (target / '.fingerprint').glob('*/lib-' + name)
               if p.read_text(encoding='utf-8').strip() == digest]
    assert len(matches) == 1, (name, matches)
    path = target / 'deps' / ('lib' + name + '-' + matches[0].parent.name.rsplit('-', 1)[1] + '.rlib')
    shutil.copy2(path, frozen / path.name)
    externs[name] = str(frozen / path.name)
    detail = json.loads(matches[0].with_name('lib-' + name + '.json').read_text(encoding='utf-8'))
    pending.update({d[1]: d[3].to_bytes(8, 'little').hex() for d in detail['deps']
                    if d[1].startswith('sf_') and d[1] not in externs})
source = ROOT / 'apps/mcp-bridge/src'
base = ['C:/Users/fujunye/.cargo/bin/rustc.exe', '--edition=2024',
        '-L', 'dependency=' + str(frozen), '-L', 'dependency=' + str(target / 'deps'),
        '-L', 'native=C:/Users/fujunye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/windows_x86_64_msvc-0.53.1/lib']
for name, path in externs.items():
    base += ['--extern', name + '=' + path]
environment = dict(os.environ, CARGO_MANIFEST_DIR=str(ROOT / 'apps/mcp-bridge'), CARGO_PKG_VERSION='0.1.0')
library = args.output.resolve() / 'libsf_mcp_bridge.rlib'
test = args.output.resolve() / 'catalog-tests.exe'
executable = args.output.resolve() / 'superfactory-mcp-bridge.exe'
commands = [base + ['--crate-name', 'sf_mcp_bridge', '--test', str(source / 'lib.rs'), '-o', str(test)],
            [str(test), 'catalog::tests'],
            base + ['--crate-name', 'sf_mcp_bridge', '--crate-type', 'rlib', str(source / 'lib.rs'), '-o', str(library)],
            base + ['--crate-name', 'superfactory_mcp_bridge', '--extern', 'sf_mcp_bridge=' + str(library),
                    str(source / 'main.rs'), '-o', str(executable)]]
results = []
for index, command in enumerate(commands):
    with (args.output / f'{index}.stdout.txt').open('wb') as stdout, (args.output / f'{index}.stderr.txt').open('wb') as stderr:
        result = subprocess.run(command, cwd=ROOT, env=environment, stdout=stdout, stderr=stderr)
    results.append({'command': command, 'exit_code': result.returncode})
    if result.returncode:
        break
(args.output / 'build.json').write_text(json.dumps({'Cargo_used': False, 'results': results,
    'libraries': externs, 'source': str(source), 'original_fingerprint': str(args.fingerprint)}, indent=2), encoding='utf-8')
print('Actual bridge source checks/build:', [r['exit_code'] for r in results])
raise SystemExit(results[-1]['exit_code'])
