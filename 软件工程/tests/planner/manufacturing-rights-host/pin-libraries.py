from pathlib import Path
import json, shutil

root = Path(__file__).resolve().parents[3]
out = root / 'scenarios/专业桌面/runtime-manufacturing-ui-host'
out.mkdir(parents=True, exist_ok=True)
pins = out / 'pinned-libraries'
pins.mkdir(exist_ok=True)
fingerprint = root / 'target/debug/.fingerprint/sf-commands-4f8db19c7cfb9767/test-integration-test-manufacturing_export_host.json'
accepted = json.loads(fingerprint.read_text())
records = {}
for _, name, _, value in accepted['deps']:
    expected = value.to_bytes(8, 'little').hex()
    matches = [p for p in (root / 'target/debug/.fingerprint').glob('*/lib-' + name)
               if p.read_text().strip() == expected]
    if not matches:
        raise RuntimeError(f'Accepted producer dependency is no longer available: {name} / {expected}')
    candidates = []
    for stamp in matches:
        suffix = stamp.parent.name.rsplit('-', 1)[1]
        library = root / f'target/debug/deps/lib{name}-{suffix}.rlib'
        if library.is_file():
            candidates.append((stamp, library))
    if len(candidates) != 1:
        raise RuntimeError(f'Ambiguous accepted dependency: {name}: {candidates}')
    stamp, library = candidates[0]
    copied = pins / library.name
    shutil.copyfile(library, copied)
    if stamp.read_text().strip() != expected:
        raise RuntimeError(f'Dependency changed during capture: {name}')
    records[name] = {'source': str(library), 'copied': str(copied), 'fingerprint_stamp': str(stamp), 'accepted_stamp': expected}
(out / 'accepted-library-cut.json').write_text(json.dumps({
    'source': str(fingerprint), 'source_kind': 'exact accepted manufacturing_export_host-4f8db19c7cfb9767 dependency fingerprints; never latest-mtime selection',
    'libraries': records, 'current_source_is_not_this_compiled_cut': True,
}, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps({name: p['copied'] for name, p in records.items()}, ensure_ascii=False))
