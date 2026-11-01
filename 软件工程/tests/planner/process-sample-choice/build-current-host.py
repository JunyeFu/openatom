"""Link the existing test transport against the just-built Native release Host.

Prerequisite: current Tauri release with custom-protocol,nfr-devtools. Cargo's
existing dependency fingerprints select exact libraries, not the newest rlib by
filename. The historical Oct16 consumer is preserved.
"""
from pathlib import Path
import datetime
import json
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / '.work/full-closeout-20261021/current-host-core'
OUT.mkdir(parents=True, exist_ok=True)
FINGERPRINT = ROOT / 'target/release/.fingerprint'
hosts = []
for path in FINGERPRINT.glob('sf-desktop-host-*/lib-sf_desktop_host.json'):
    data = json.loads(path.read_text())
    if sorted(json.loads(data['features'])) == ['custom-protocol', 'nfr-devtools']:
        hosts.append(path)
host_path = max(hosts, key=lambda path: path.stat().st_mtime)
host = json.loads(host_path.read_text())


def library(crate, wanted):
    package = crate.replace('_', '-') if crate.startswith('sf_') else crate
    for path in FINGERPRINT.glob(package + '-*/lib-' + crate + '.json'):
        value = int.from_bytes(bytes.fromhex(path.with_suffix('').read_text()), 'little')
        if value == wanted:
            return ROOT / 'target/release/deps' / ('lib' + crate + '-' + path.parent.name.rsplit('-', 1)[1] + '.rlib')
    raise ValueError(f'No exact Native dependency artifact for {crate}')


libraries = {row[1]: library(row[1], row[3]) for row in host['deps']
             if row[1] in ('sf_commands', 'sf_domain', 'serde_json')}
command = ['rustc', '--edition', '2024', str(ROOT / 'tests/planner/node-inspector/core_consumer.rs'),
           '-O', '-L', 'dependency=' + str(ROOT / 'target/release/deps')]
for name, path in libraries.items():
    command.extend(['--extern', name + '=' + str(path)])
command.extend(['-o', str(OUT / 'core_consumer.exe')])
result = subprocess.run(command, capture_output=True, text=True, encoding='utf8')
(OUT / 'build.stdout.txt').write_text(result.stdout, encoding='utf8')
(OUT / 'build.stderr.txt').write_text(result.stderr, encoding='utf8')
record = {'exit_code': result.returncode, 'finished_at': datetime.datetime.now().astimezone().isoformat(),
          'command': command, 'Native_frontier': str(host_path.relative_to(ROOT)), 'features': host['features'],
          'libraries': {name: str(path.relative_to(ROOT)) for name, path in libraries.items()},
          'historical_core_unchanged': True, 'Native_UI_used': False}
(OUT / 'build.json').write_text(json.dumps(record, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
print(json.dumps(record, ensure_ascii=False))
if result.returncode:
    print(result.stderr[-4000:])
raise SystemExit(result.returncode)
