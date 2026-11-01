"""Verify owned disk packages with the exact existing Core domain dependency."""
import json
import subprocess
from pathlib import Path

root = Path.cwd()
out = root / 'scenarios/图片时间轴/runtime'
baseline = json.loads((out / 'leave-disk-baseline.json').read_text(encoding='utf-8-sig'))
exact = []
for package in baseline['packages']:
    directory = root / package['directory']
    actual = json.loads((directory / 'module.json').read_text(encoding='utf-8-sig'))
    assert actual == package['document'], package['directory']
    assert actual['schema_version'] == '1.0'
    assert set(actual) == {'schema_version', 'data'}
    for name, content in package['declared_files'].items():
        assert (directory / name).read_text(encoding='utf-8') == content, name
    exact.append({'directory': package['directory'], 'envelope_payload_assets_license_equal_to_before_round': True})

build = json.loads((out / 'core-build.json').read_text(encoding='utf-8-sig'))
core = build['existing_core']
fingerprints = root / 'target/debug/.fingerprint'
core_name = core.removeprefix('lib').removesuffix('.rlib').replace('_', '-', 1)
fingerprint = json.loads((fingerprints / core_name / 'lib-sf_commands.json').read_text())
expected = next(dep[-1] for dep in fingerprint['deps'] if dep[1] == 'sf_domain').to_bytes(8, 'little').hex()
matches = [p for p in fingerprints.glob('sf-domain-*/lib-sf_domain') if p.read_text().strip() == expected]
assert len(matches) == 1, matches
domain = root / 'target/debug/deps' / ('lib' + matches[0].parent.name.replace('sf-domain', 'sf_domain') + '.rlib')
serde = root / 'target/debug/deps' / build['exact_serde_json']
command = ['C:/Users/fujunye/.cargo/bin/rustc.exe', '--edition', '2024', str(out / 'manifest_contract.rs'), '-L', 'dependency=target/debug/deps', '--extern', f'sf_domain={domain}', '--extern', f'serde_json={serde}', '-o', str(out / 'manifest_contract.exe')]
result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8')
(out / 'manifest-build.stdout.txt').write_text(result.stdout, encoding='utf-8')
(out / 'manifest-build.stderr.txt').write_text(result.stderr, encoding='utf-8')
assert result.returncode == 0, result.stderr
run = subprocess.run([str(out / 'manifest_contract.exe'), str(root)], capture_output=True, text=True, encoding='utf-8')
assert run.returncode == 0, run.stderr
report = {**json.loads(run.stdout), 'exact_baseline_comparisons': exact, 'existing_core': core, 'exact_domain': domain.name, 'exact_serde_json': serde.name, 'cargo_invoked': False, 'production_payload_or_assets_modified': False}
(out / 'manifest-contract-result.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps(report, ensure_ascii=False, indent=2))
