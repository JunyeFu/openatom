"""Add independent consumers of the already passed own delivery and nine fixed DTOs."""
import copy,json,shutil,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];OUT=ROOT/'.work/delivery'
def read(path):return json.loads(path.read_text(encoding='utf-8'))
def write(path,value):
    path.parent.mkdir(parents=True,exist_ok=True);path.write_text(json.dumps(value,ensure_ascii=False,indent=2),encoding='utf-8')
base=read(OUT/'cases/success/case.json')
producer=OUT/'cases/success/actual-r3'
assert read(OUT/'cases/success/r3.process.json')['exit_code']==0
assert read(producer/'client-result.json')['failure'] is None
stage=OUT/'cases/handoff';stage.mkdir(exist_ok=False)
shutil.copyfile(OUT/'cases/success/input.json',stage/'input.json')
shutil.copyfile(producer/'original-run.json',stage/'expected-run.json')
case=copy.deepcopy(base)
readonly=copy.deepcopy(base['profiles'][0]);readonly['edit']=[]
case.update(seed=str(producer/'saved'),output=str(stage/'actual'),mode='handoff',profiles=[base['profiles'][0],readonly],drafts=[])
write(stage/'case.json',case)
write(stage/'source.json',{'producer_saved_project':str(producer/'saved'),'producer_actual_Run':str(producer/'original-run.json'),
    'producer_validation':str(producer/'client-result.json'),'expected':'new process identical old Run, editable rerun, actual ZIP export/import proposal and explicit adoption; no human acceptance'})
stage=OUT/'cases/nine';stage.mkdir(exist_ok=False)
shutil.copyfile(OUT/'cases/success/input.json',stage/'input.json')
definitions=[];sources=[];grants=[]
for category in ['items','devices','resources','tasks','operations','transport','templates','professional','media']:
    source=ROOT/'tools/type-contract-proof/fixtures'/f'module-{category}.json'
    definition=read(source)['data']['definition']
    assert definition['payload']['category']==category
    definition['id']='noncu.draft.'+category
    definitions.append(definition);sources.append(str(source))
    grants.append({'scope':{'kind':'edit','data':{'kind':'definition_draft','data':{'id':definition['id'],'revision':1}}},
        'branches':['put_definition'],'objects':[{'target':{'kind':'definition_draft','id':definition['id']},
            'fields':[{'pointer':'/title','element_id':None}],'can_remove':False}],
        'creates':[],'source_versions':[],'reusable_resources':[]})
authority=copy.deepcopy(base['profiles'][0]);authority['edit']=grants;authority['definition_drafts']=[d['id'] for d in definitions]
case=copy.deepcopy(base);case.update(mode='nine',output=str(stage/'actual'),profiles=[authority],drafts=definitions)
write(stage/'case.json',case);write(stage/'drafts.json',definitions)
write(stage/'source.json',{'frozen_sources':sources,'only_input_transformation':'new private draft IDs; payload/version/contribution unchanged',
    'expected':'each actual draft title commits R1->R2 over stdio; all nine typed payloads unchanged; no Project adoption or Run mutation',
    'rights_confirmation_or_version_publication':False})
stage=OUT/'cases/cancel';stage.mkdir(exist_ok=False)
base=read(OUT/'cases/lifecycle/case.json')
shutil.copyfile(OUT/'cases/lifecycle/input.json',stage/'input.json')
readonly=copy.deepcopy(base['profiles'][0])
base.update(mode='cancel',output=str(stage/'actual'),profiles=[readonly,readonly],drafts=[])
write(stage/'case.json',base)
write(stage/'source.json',{'source_case':'lifecycle fixed input and its 1.2MB transport title',
    'expected':'MCP request cancellation never becomes business cancel; known previously accepted Engine TaskId survives and is not replayed'})
print('Prepared saved-project handoff and actual nine-category draft consumers')
