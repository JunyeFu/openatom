"""Link the same existing Engine/serde artifacts as this private actual Core."""
import json
import subprocess
from pathlib import Path

root = Path.cwd()
out = root / 'scenarios/图片时间轴/runtime'
core_build = json.loads((out / 'core-build.json').read_text(encoding='utf-8-sig'))
core = Path(core_build['existing_core'])
fingerprint = root / 'target/debug/.fingerprint' / core.stem.removeprefix('lib').replace('_', '-', 1) / 'lib-sf_commands.json'
dependencies = json.loads(fingerprint.read_text())['deps']
expected = next(dep[-1] for dep in dependencies if dep[1] == 'sf_simulation').to_bytes(8, 'little').hex()
matches = [root / 'target/debug/deps' / ('lib' + path.parent.name.replace('sf-simulation-', 'sf_simulation-') + '.rlib')
           for path in (root / 'target/debug/.fingerprint').glob('sf-simulation-*/lib-sf_simulation')
           if path.read_text().strip() == expected]
assert len(matches) == 1, matches
command = ['C:/Users/fujunye/.cargo/bin/rustc.exe', '--edition=2024', str(out / 'capture_frames.rs'),
           '-L', 'dependency=target/debug/deps', '--extern', f'sf_simulation={matches[0]}',
           '--extern', f'serde_json={root / "target/debug/deps" / core_build["exact_serde_json"]}',
           '-o', str(out / 'capture_frames.exe')]
result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8')
(out / 'capture-build.stdout.txt').write_text(result.stdout, encoding='utf-8')
(out / 'capture-build.stderr.txt').write_text(result.stderr, encoding='utf-8')
(out / 'capture-build.json').write_text(json.dumps({'command': command, 'exit_code': result.returncode,
    'exact_commands_Engine': str(matches[0])}, indent=2), encoding='utf-8')
print(result.stdout, result.stderr)
raise SystemExit(result.returncode)
