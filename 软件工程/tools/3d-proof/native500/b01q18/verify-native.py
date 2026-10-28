from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q18';BASE=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));checks=[]
catalog={e['id']:e for e in read(HOME/'catalog.json')['equipment']};behavior=read(BASE/'b01q18/attempt-06/behavior-checks.json');names=['b01q18-ordinary-01','b01q18-ordinary-02'];proofs=[read(BASE/(n+'-proof.json'))for n in names]
for name,p in zip(names,proofs):
 assert p['passed']and p['exact_Run_after_UI_open']and p['viewing_preserves_Run']and read(BASE/(name+'-close.json'))['normal_close']
 for spec in behavior['checks']:
  key=spec['id'];s=catalog[key]
  for condition,value in spec['conditions'].items():
   rows=[r for r in p['samples']if r['device']==key and r['condition']==condition];assert len(rows)==(3 if value['wait']else 2);dev=key+'--'+condition;raw_id='paper-'+dev;items=[r['data']for r in rows[-1]['items']if r['kind']=='item'];raw=next(i for i in items if i['id']==raw_id);assert raw['holder']==dict(kind='location',id=value['destination'])and raw['lifecycle']=='available'and raw['quality']==('rejected'if condition=='rejected'else'released');assert raw['attributes']['presentation_state']['value']==s['final']and raw['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0));assert raw['contents']==['unit-'+str(i)+'-'+dev for i in range(6)]
   for sample in rows:
    current=[r['data']for r in sample['items']if r['kind']=='item'];parent=next((i for i in current if i['id']==raw_id),None)
    if parent:
     assert parent['contents']==raw['contents']
     for uid in parent['contents']:
      child=next(i for i in current if i['id']==uid);assert child['holder']==dict(kind='container',id=raw_id)and child['quantity']==raw['quantity']and child['quality']=='released'and child['lifecycle']=='available'
   if s['auxiliary']:assert next(i for i in items if i['id']=='auxiliary-'+dev)['lifecycle']=='transformed'
   for sample in rows[:-1]:
    j=sample['job'][0]['data'];current=[r['data']for r in sample['items']if r['kind']=='item'];parent=next((i for i in current if i['id']==raw_id),None)
    if condition in ['missing','missing-auxiliary']:assert j['started_at_ms']is None
    elif value['wait']:
     assert parent['holder']==dict(kind='job',id=j['id'])and j['state']in['waiting_resource','waiting_output']
     if condition=='pending-recovery':assert parent['quality']=='pending'and {'machine-'+dev,'operator-'+dev}<=set(j['job_resources'])
     if condition=='reject-wait':assert parent['quality']=='rejected'
     if condition=='output-wait'and s['rule']=='preserve':assert j['effective_progress_ms']==0 and parent['attributes']['presentation_state']['value']==s['initial']
   if condition=='interrupted':
    a,b=[r['job'][0]['data']for r in rows[:2]];assert a['effective_progress_ms']==b['effective_progress_ms']>0 and a['phase_ref']==b['phase_ref']=='auto'
   checks.append({'process':p['identity']['host_instance_id'],'device':key,'condition':condition,'passed':True,'holders_verified':True,'same_package_and_six_contents':True,'recovery_verified':True})
media=[];saved=BASE/'b01q18/native-saved-r1/saved'
for folder in [HOME/'installable',HOME/'items']:
 for model in folder.rglob('*.glb'):
  d=read(model.parent/'module.json')['data']['definition'];target=saved/'bundled'/d['id'].encode().hex()/d['version'].encode().hex()/model.name;assert target.read_bytes()==model.read_bytes();media.append({'module_ref':{'id':d['id'],'version':d['version']},'asset':model.name,'exact_current_media_preserved':True})
(BASE/'b01q18/Native-persistence-checks.json').write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'processes':[p['identity']for p in proofs],'current_media_checks':media,'formal_NFR_timing':False},ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(f'Q18 two ordinary processes: {len(checks)} conditions and {len(media)} exact assets, package and six contents verified.')
