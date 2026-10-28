"""One-time repair of the observed disk producer defect; preserve the existing payload and asset bytes."""
from pathlib import Path
import json
root=Path(__file__).resolve().parents[3];base=root/'modules/media/p302-line-grade';out=root/'scenarios/媒体表现/runtime-line-grade/standard-package-fix'
manifests=sorted((base/'installable').glob('*/module.json'));assert len(manifests)==32
before={str(p.relative_to(base)):json.loads(p.read_text(encoding='utf-8')) for p in manifests}
assert all('schema_version' not in p for p in before.values())
files={p.parent/f['path']:(p.parent/f['path']).read_bytes() for p in manifests for f in before[str(p.relative_to(base))]['files']}
old=b'JSON.stringify(pkg,null,2)';new=b'JSON.stringify({schema_version:"1.0",data:pkg},null,2)'
generator=base/'generate.mjs';original=generator.read_bytes();assert original.count(old)==1
generator.write_bytes(original.replace(old,new))
sources=[]
for p,b in files.items():
    if p.name=='generate.mjs':
        assert b==original
        p.write_bytes(b.replace(old,new));sources.append(str(p.relative_to(base)))
for p in manifests:
    payload=before[str(p.relative_to(base))]
    p.write_text(json.dumps({'schema_version':'1.0','data':payload},ensure_ascii=False,indent=2),encoding='utf-8')
    assert json.loads(p.read_text(encoding='utf-8'))['data']==payload
for p,b in files.items():assert p.read_bytes()==(b.replace(old,new) if p.name=='generate.mjs' else b),str(p)
(out/'payloads-before.json').write_text(json.dumps(before,ensure_ascii=False,indent=2),encoding='utf-8')
(out/'preservation-receipt.json').write_text(json.dumps({'manifests':32,'schema_version':'1.0','all_data_payloads_equal':True,'all_non_emitter_declared_files_byte_identical':True,'emitter_only_source_edits':sources,'geometry_and_parameters_unchanged':True,'standalone_generators_updated':len(sources),'note':'Editable source modelling code/parameters unchanged; its sole necessary change is the final JSON disk-envelope expression.'},ensure_ascii=False,indent=2),encoding='utf-8')
print(f'32 payload-preserving Versioned1.0 entries; {len(sources)} standalone emitters updated; all other declared bytes unchanged')
