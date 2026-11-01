"""A separately named complete-input template; original installed bytes stay intact."""
import copy,json,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];OUT=ROOT/'.work/cu-delivery/fixtures/complete-template-v3'
source=ROOT/'modules/templates/carton-factory/candidate'
geometry=json.loads(Path(__file__).with_name('cu_geometry.json').read_text(encoding='utf-8'))
shutil.copytree(source,OUT)
def read(p):return json.loads(p.read_text(encoding='utf-8'))
def write(p,v):p.write_text(json.dumps(v,ensure_ascii=False,indent=2),encoding='utf-8')
module=read(OUT/'module.json');module['data']['definition']['id']='cu.complete-carton-example';module['data']['definition']['version']='0.1.0'
module['data']['definition']['title']='Explicit complete geometry example derived from original carton template';write(OUT/'module.json',module)
record={'id':geometry['source_id'],'title':geometry['title'],'locator':str(Path(__file__).with_name('cu_geometry.json')),
    'read_scope':geometry['source_scope'],'read_date':'2026-10-22','basis':geometry['basis']}
before={}
def rebind_self(v):
    if isinstance(v,dict):
        if v.get('id')=='community.template-carton-factory' and v.get('version')=='1.0.0':v.update(id='cu.complete-carton-example',version='0.1.0')
        for child in v.values():rebind_self(child)
    elif isinstance(v,list):
        for child in v:rebind_self(child)
for file in ['example-project.json','template.json','generation-profile.json']:
    v=read(OUT/file);before[file]=copy.deepcopy(v);data=v['data']
    if file=='template.json':data['adoption']['template_ref']={'id':'cu.complete-carton-example','version':'0.1.0'}
    for d in data.get('devices',[]):d['footprint_m']=geometry['footprint_m'];d['height_m']=geometry['height_m']
    for s in data.get('parameter_sources',[]):
        if s['target']['kind']=='device' and s['field_path'] in ['/footprint_m','/height_m']:
            s['basis']=geometry['basis'];s['source_ref']=geometry['source_id']
    if file=='example-project.json':data['sources'].append(record)
    rebind_self(data)
    write(OUT/file,v)
after=read(OUT/'example-project.json')['data'];original=before['example-project.json']['data']
for key in ['item_types','locations','resources','calendars','products','boms','routes','connections','setup_policies']:
    assert after[key]==original[key],key
for old,new in zip(original['devices'],after['devices']):
    assert {k:v for k,v in old.items() if k not in ['footprint_m','height_m']}=={k:v for k,v in new.items() if k not in ['footprint_m','height_m']}
write(OUT.parent/'complete-template-source-v3.json',{'original':str(source),'new_private_ref':{'id':'cu.complete-carton-example','version':'0.1.0'},
    'declared_geometry_input':geometry,'allowed_differences':'instance footprint/height and matching declared input-source records only; new private template identity',
    'cycle_BOM_quality_supply_and_original_modules_changed':False,'industrial_measurements_or_rendered_quality':False})
print('separate controlled template input prepared; original quantity, timing, behavior, resources and quality retained')
