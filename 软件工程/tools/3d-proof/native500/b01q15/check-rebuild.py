from pathlib import Path
import json,shutil,subprocess,sys
root=Path(__file__).resolve().parents[4];home=root/'modules/media/native-equipment-500/reviews/B01-Q15';dest=root/(sys.argv[1]if len(sys.argv)>1 else'.work/native500-quality-20261025/b01q15/rebuild-01')
if dest.exists():raise SystemExit('Keep existing rebuild evidence; choose a fresh destination for a new source revision.')
rows=[]
for suffix in ['-simple','']:
 source=home/('installable/tray-former'+suffix);target=dest/('tray-former'+suffix);shutil.copytree(source,target)
 subprocess.run(['node',str(target/'source/export.mjs')],cwd=root,check=True)
 equal={name:(source/name).read_bytes()==(target/name).read_bytes() for name in ['tray-former.glb','module.json','nameplate.png']}
 assert all(equal.values()),equal
 rows.append(dict(lod='simple'if suffix else'detailed',own_package_source_rebuild=True,files_equal=equal))
for entry in json.loads((home/'items/inventory.json').read_text(encoding='utf8'))['metadata']:
 source=home/'items'/entry['id'];target=dest/'items'/entry['id'];shutil.copytree(source,target)
 subprocess.run(['node',str(target/'source/item-build.mjs')],cwd=root,check=True)
 equal={name:(source/name).read_bytes()==(target/name).read_bytes() for name in ['paper.glb','module.json','item-definition/module.json']}
 assert all(equal.values()),equal
 rows.append(dict(item=entry['id'],own_package_source_rebuild=True,files_equal=equal))
(home/'source-rebuild-checks.json').write_text(json.dumps(dict(passed=True,scope='candidate dual LOD rebuild from its own supplied source and embedded surface/nameplate assets; not Native admission',destination=dest.relative_to(root).as_posix(),checks=rows,device_admitted=False),ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print('Candidate dual LOD and four actual Item packages rebuild byte-equal; not device admission.')
