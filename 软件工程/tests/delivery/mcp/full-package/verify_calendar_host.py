"""Run the original public creation consumers with an explicit closed producer map."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[4]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--libraries', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=False)
producer = json.loads(args.libraries.read_text(encoding='utf-8-sig'))
selected = {'sf_commands': Path(producer['commands']), 'sf_domain': Path(producer['sf_domain']),
            'serde_json': Path(producer['serde_json'])}
target = selected['sf_commands'].parent.parent
fingerprints = target / '.fingerprint'
frozen = args.output.resolve() / 'dependencies'
frozen.mkdir()
metadata = {}
libraries = {}


def freeze(name, path):
    suffix = path.stem.rsplit('-', 1)[1]
    matches = list(fingerprints.glob(f'*-{suffix}/lib-{name}.json'))
    assert len(matches) == 1, (name, matches)
    shutil.copy2(path, frozen / path.name)
    shutil.copy2(matches[0], frozen / (matches[0].parent.name + '.json'))
    libraries[name] = str(frozen / path.name)
    metadata[name] = json.loads(matches[0].read_text(encoding='utf-8'))


def dependency(name, digest):
    matches = [p for p in fingerprints.glob('*/lib-' + name)
               if p.read_text(encoding='utf-8').strip() == digest.to_bytes(8, 'little').hex()]
    assert len(matches) == 1, (name, matches)
    return target / 'deps' / f'lib{name}-{matches[0].parent.name.rsplit("-", 1)[1]}.rlib'


for name, path in selected.items():
    freeze(name, path)
pending = ['sf_commands', 'sf_domain']
while pending:
    owner = pending.pop()
    for _, name, _, digest in metadata[owner]['deps']:
        if name in libraries or not (name.startswith('sf_') or name == 'serde'):
            continue
        freeze(name, dependency(name, digest))
        if name.startswith('sf_'):
            pending.append(name)
shutil.copy2(args.libraries, args.output / 'producer-libraries.json')
command = ['C:/Users/fujunye/.cargo/bin/rustc.exe', '--edition=2024', '--test',
           str(ROOT / 'tests/delivery/mcp/full-package/calendar_creation.rs'),
           '-L', 'dependency=' + str(frozen), '-L', 'dependency=' + str(target / 'deps'),
           '-o', str(args.output.resolve() / 'calendar-tests.exe')]
for name in ['sf_commands', 'sf_domain', 'serde', 'serde_json']:
    command += ['--extern', name + '=' + libraries[name]]
results = []
for index, invocation in enumerate([command, [str(args.output.resolve() / 'calendar-tests.exe')]]):
    with (args.output / f'{index}.stdout.txt').open('wb') as stdout, (args.output / f'{index}.stderr.txt').open('wb') as stderr:
        run = subprocess.run(invocation, cwd=ROOT, env=dict(os.environ, CARGO_MANIFEST_DIR=str(ROOT / 'packages/commands')),
                             stdout=stdout, stderr=stderr)
    results.append({'command': invocation, 'exit_code': run.returncode})
    if run.returncode:
        break
(args.output / 'result.json').write_text(json.dumps({'Cargo_used': False, 'producer': str(args.libraries),
    'frozen_libraries': libraries, 'results': results, 'Native_or_Codex_acceptance': False}, indent=2), encoding='utf-8')
print('Actual public Host creation consumer:', [r['exit_code'] for r in results])
raise SystemExit(results[-1]['exit_code'])
