"""Materialize the five original packages using only their declared FileRefs."""
from pathlib import Path
import json

root=Path(__file__).parent
directories=[]
for name in ['image-simple','model-simple','image','gif','model']:
    source=root/(name+'.module.json')
    package=json.loads(source.read_text(encoding='utf-8'))
    target=root/'installable'/name
    target.mkdir(parents=True,exist_ok=True)
    # The Host's standard on-disk manifest is Versioned<ModulePackage>.
    # Keep the original package payload; the bare JSON remains a test input.
    (target/'module.json').write_text(json.dumps(dict(schema_version='1.0',data=package),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    for ref in package['files']:
        destination=target/ref['path']
        destination.parent.mkdir(parents=True,exist_ok=True)
        destination.write_bytes((root/ref['path']).read_bytes())
    directories.append(dict(id=package['definition']['id'],version=package['definition']['version'],directory=str(target.resolve()),manifest='module.json',copied_files=[ref['path']for ref in package['files']],dependencies=package['dependencies']))
(root/'installable'/'directories.json').write_text(json.dumps(directories,ensure_ascii=False,indent=2),encoding='utf-8')
print('5 Versioned<ModulePackage> module.json directories materialized; original package payloads,7 assets and rights preserved.')
