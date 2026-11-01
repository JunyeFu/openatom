"""Recreate omitted L03 input from frozen source; redirect every output locally.

The omitted old registration JSON is reconstructed from its unchanged canonical
module manifests. No production result or business expectation is generated.
"""
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / '.work/delivery/inputs'
OUT.mkdir(parents=True, exist_ok=True)
entries = []
for directory in (ROOT / 'modules/media/p302-line-grade/installable').iterdir():
    if directory.name.endswith('-fit'):
        continue  # Four separate fit experiments were not the original 28-package catalog.
    manifest = directory / 'module.json'
    if manifest.is_file():
        definition = json.loads(manifest.read_text(encoding='utf-8'))['data']['definition']
        entries.append(dict(id=definition['id'], version=definition['version'], directory=directory.relative_to(ROOT).as_posix()))
entries.sort(key=lambda e: (not e['id'].endswith('.simple'), e['id']))
assert len(entries) == 28 and all(e['version'] == '0.1.0' for e in entries)
path = ROOT / 'scenarios/媒体表现/runtime-l03/build_input.py'
source = path.read_text(encoding='utf-8')
source = source.replace('plan=old.build()', '''original_read = old.read
def read_with_manifest_catalog(path):
    if path == 'scenarios/媒体表现/runtime-line-grade/root-registration.json':
        return {'register_order': deepcopy(RECONSTRUCTED_ENTRIES)}
    return original_read(path)
old.read = read_with_manifest_catalog
plan=old.build()''')
source = source.replace('(HERE/n).write_text', '(OUT/n).write_text').replace("(HERE/'contract-check.ts').write_text", "(OUT/'contract-check.ts').write_text")
source = source.replace("project=read(source)['data']", 'project=deepcopy(EMPTY_PROJECT)')
empty = json.loads((OUT / 'empty-project.json').read_text(encoding='utf-8'))
exec(compile(source, str(path), 'exec'), {'__file__': str(path), 'OUT': OUT, 'RECONSTRUCTED_ENTRIES': entries, 'EMPTY_PROJECT': empty})
(OUT / 'input-reconstruction.json').write_text(json.dumps(dict(source=path.relative_to(ROOT).as_posix(), omitted_registration_reconstructed_from=entries, business_logic_unchanged=True, results_generated=False), indent=2), encoding='utf-8')
print('Frozen L03 builder input reconstructed into .work/delivery/inputs only.')
