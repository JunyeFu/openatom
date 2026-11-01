"""Prepare actual external creation of the same nine fixed, typed DTOs."""
import copy,json,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];OUT=ROOT/'.work/delivery'
def read(path):return json.loads(path.read_text(encoding='utf-8'))
def leaves(value,path=''):
    if isinstance(value,dict):
        return [leaf for key,child in value.items() for leaf in leaves(child,path+'/'+key.replace('~','~0').replace('/','~1'))]
    if isinstance(value,list):
        return [leaf for index,child in enumerate(value) for leaf in leaves(child,path+'/'+str(index))]
    return [path]
stage=OUT/'cases/generate';stage.mkdir(exist_ok=False)
definitions=read(OUT/'cases/nine/drafts.json')
case=read(OUT/'cases/success/case.json')
paths=sorted({path for definition in definitions for path in leaves({key:value for key,value in definition.items() if key!='id'})})
authority=copy.deepcopy(case['profiles'][0])
authority['definition_drafts']=[]
authority['edit']=[{'scope':{'kind':'edit','data':{'kind':'project','data':'P1'}},'branches':['create_definition_draft'],
    'objects':[],'creates':[{'kind':'definition_draft','max_new_objects':9,
        'fields':[{'pointer':path,'element_id':None} for path in paths]}],
    'source_versions':[],'reusable_resources':[]}]
case.update(mode='generate',output=str(stage/'actual'),profiles=[authority],drafts=[])
for name,value in [('case.json',case),('drafts.json',definitions),('source.json',{
    'source':'../nine/source.json','expected':'actual external create_definition_draft for all nine fixed DTOs; no Project adoption; cumulative nine-object budget',
    'input_payloads_changed':False,'rights_confirmation_or_version_publication':False})]:
    (stage/name).write_text(json.dumps(value,ensure_ascii=False,indent=2),encoding='utf-8')
shutil.copyfile(OUT/'cases/success/input.json',stage/'input.json')
print('Prepared external nine-category creation with exact initial-leaf grants')
