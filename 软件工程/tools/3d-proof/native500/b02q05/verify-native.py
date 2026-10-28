from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B02-Q05';BASE=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));checks=[]
catalog={e['id']:e for e in read(HOME/'catalog.json')['equipment']};behavior=read(BASE/'b02q05/attempt-03/behavior-checks.json');names=['b02q05-ordinary-05','b02q05-ordinary-06'];proofs=[read(BASE/(n+'-proof.json'))for n in names]
for name,p in zip(names,proofs):
 assert p['passed']and p['exact_Run_after_UI_open']and p['viewing_preserves_Run']and read(BASE/(name+'-close.json'))['normal_close']
 for spec in behavior['checks']:
  key=spec['id'];s=catalog[key];inspect=s['rule']=='inspect';peeler=key=='label-peeler'
  for condition,value in spec['conditions'].items():
   rows=[r for r in p['samples']if r['device']==key and r['condition']==condition];assert len(rows)==(3 if value['wait']else 2);dev=key+'--'+condition;raw=value['input_id'];outputs=value['output_ids'];backing=value['backing_ids'];last={r['data']['id']:r['data']for r in rows[-1]['items']if r['kind']=='item'}
   assert last[raw]['lifecycle']==('available'if inspect else'transformed')
   for uid in outputs+backing:
    i=last[uid];assert i['holder']==dict(kind='location',id=dev+'-backing'if uid in backing else value['destination'])and i['quality']==('rejected'if condition=='rejected'else'released')and i['lifecycle']=='available'and i['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))and not i['contents']
    assert set(i['composition_refs'])==(set()if inspect else set(value['source_ids']))
    assert i['attributes']['presentation_state']['value']==('spent_liner_matrix'if uid in backing else s['final'])
   if s['auxiliary']:assert last['auxiliary-'+dev]['lifecycle']=='transformed'
   for sample in rows[:-1]:
    j=sample['job'][0]['data'];current={r['data']['id']:r['data']for r in sample['items']if r['kind']=='item'}
    if condition in ['missing','missing-auxiliary']:assert j['started_at_ms']is None
    elif condition in ['phase-wait','interrupted']:
     assert j['state']=='waiting_resource'and current[raw]['holder']==dict(kind='job',id=j['id'])
     if s['auxiliary']:assert current['auxiliary-'+dev]['holder']==dict(kind='job',id=j['id'])
    elif condition in ['output-wait','backing-wait']:
     assert j['state']=='waiting_output'
     held=backing if condition=='backing-wait'else outputs
     assert set(j['pending_outputs'])==set(held)
     for uid in held:assert current[uid]['holder']==dict(kind='job',id=j['id'])
     if peeler:
      for uid in outputs if condition=='backing-wait'else backing:assert current[uid]['holder']==dict(kind='location',id=dev+'-out'if uid in outputs else dev+'-backing')
    elif condition=='pending-recovery':assert j['state']=='waiting_output'and current[raw]['holder']==dict(kind='job',id=j['id'])and current[raw]['quality']=='pending'and 'operator-'+dev in j['job_resources']
    elif condition=='reject-wait':assert j['state']=='waiting_output'and current[raw]['holder']==dict(kind='job',id=j['id'])and current[raw]['quality']=='rejected'
   if condition=='interrupted':a,b=[r['job'][0]['data']for r in rows[:2]];assert a['effective_progress_ms']==b['effective_progress_ms']>0 and a['phase_ref']==b['phase_ref']=='auto'
   checks.append({'process':p['identity']['host_instance_id'],'device':key,'condition':condition,'passed':True,'holders_verified':True,'identity_quantity_origins_verified':True,'recovery_verified':True})
media=[];saved=BASE/'b02q05/native-saved-03/saved'
for folder in [HOME/'installable',HOME/'items']:
 for model in folder.rglob('*.glb'):
  d=read(model.parent/'module.json')['data']['definition'];target=saved/'bundled'/d['id'].encode().hex()/d['version'].encode().hex()/model.name;assert target.read_bytes()==model.read_bytes();media.append({'module_ref':{'id':d['id'],'version':d['version']},'asset':model.name,'exact_current_media_preserved':True})
(BASE/'b02q05/Native-persistence-checks.json').write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'processes':[p['identity']for p in proofs],'current_media_checks':media,'formal_NFR_timing':False},ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(f'Q05: {len(checks)} conditions across two ordinary processes; {len(media)} exact current assets preserved.')
