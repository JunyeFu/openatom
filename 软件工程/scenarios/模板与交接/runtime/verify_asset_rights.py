"""Regenerate owned candidates; check the observed same-source rights repair only."""
from pathlib import Path
from copy import deepcopy
import json, subprocess, sys

ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).resolve().parent
NAMES=('machining-delivery','purchased-assembly','parallel-electromechanical','carton-factory')

def read(p): return json.loads(p.read_text('utf-8-sig'))
def strip_contribution_assets(v):
    v=deepcopy(v)
    def visit(x):
        if isinstance(x,dict):
            if isinstance(x.get('contribution'),dict): x['contribution']['assets']=[]
            for item in x.values(): visit(item)
        elif isinstance(x,list):
            for item in x: visit(item)
    visit(v); return v

def main():
    directories=[ROOT/'modules/templates'/name/'candidate' for name in NAMES]
    directories.append(ROOT/'modules/devices/template-support')
    before={p:read(p) for folder in directories for p in folder.rglob('*.json')}
    subprocess.run([sys.executable,'-X','utf8',str(ROOT/'modules/templates/build_runtime_candidates.py')],cwd=ROOT,check=True)
    changed=[]
    for p,old in before.items():
        new=read(p)
        assert strip_contribution_assets(old)==strip_contribution_assets(new),f'Unexpected semantic change: {p}'
        if old!=new: changed.append(p.relative_to(ROOT).as_posix())
    cases={}
    for name in NAMES:
        base=ROOT/'modules/templates'/name/'candidate'
        definitions={(d['id'],d['version']):d for d in read(base/'definitions.json')}
        assets=0; packages=0
        for file in base.rglob('module.json'):
            data=read(file)['data']; d=data['definition']; packages+=1
            assert d['contribution']['assets']==data['rights'],str(file)
            assert {a['id'] for a in data['assets']}=={a['asset_ref'] for a in data['rights']},str(file)
            if (d['id'],d['version']) in definitions:
                assert d==definitions[(d['id'],d['version'])],str(file)
            old=before[file]['data']
            assert data['rights']==old['rights'] and data['assets']==old['assets'],str(file)
            assets+=len(data['rights'])
        cases[name]={'packages_checked':packages,'existing_asset_rights':assets,'package_definition_aggregate_equal':True,'existing_rights_and_asset_lists_unchanged':True}
    result={'cases':cases,'changed_json_files':changed,'only_contribution_assets_changed':True,'new_license_or_redistribution_authority_added':False,'independent_Host_store_test':'separate actual commands test; never weakened here'}
    (OUT/'asset-rights-repair.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result,ensure_ascii=False))

if __name__=='__main__': main()
