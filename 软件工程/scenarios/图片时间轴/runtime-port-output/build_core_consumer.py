"""Link the current real Core with its exact existing serde_json dependency."""
import json
import subprocess
from pathlib import Path

root = Path.cwd()
out = root / 'scenarios/图片时间轴/runtime-port-output'
core = max((root / 'target/debug/deps').glob('libsf_commands-*.rlib'), key=lambda p: p.stat().st_mtime_ns)
fingerprint = json.loads((root / 'target/debug/.fingerprint' / core.stem.removeprefix('lib').replace('_', '-', 1) / 'lib-sf_commands.json').read_text())
expected = next(dep[-1] for dep in fingerprint['deps'] if dep[1] == 'serde_json').to_bytes(8, 'little').hex()
json_libraries = []
for candidate in (root / 'target/debug/.fingerprint').glob('serde_json-*/lib-serde_json'):
    library = root / 'target/debug/deps' / f'lib{candidate.parent.name}.rlib'
    if candidate.read_text().strip() == expected and library.exists():
        json_libraries.append(library)
assert len(json_libraries) == 1, json_libraries
command = [str(Path('C:/Users/fujunye/.cargo/bin/rustc.exe')), '--edition', '2024', str(out / 'core_consumer.rs'), '-L', 'dependency=target/debug/deps', '--extern', f'sf_commands={core}', '--extern', f'serde_json={json_libraries[0]}', '-o', str(out / 'core_consumer.exe')]
result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8')
(out / 'core-build.stdout.txt').write_text(result.stdout, encoding='utf-8')
(out / 'core-build.stderr.txt').write_text(result.stderr, encoding='utf-8')
(out / 'core-build.json').write_text(json.dumps({'command': command, 'exit_code': result.returncode, 'existing_core': core.name, 'exact_serde_json': json_libraries[0].name}, ensure_ascii=False, indent=2), encoding='utf-8')
print(result.stdout, result.stderr)
raise SystemExit(result.returncode)
