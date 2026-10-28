from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B02-Q06';BASE=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));checks=[]
catalog={e['id']:e for e in read(HOME/'catalog.json')['equipment']};behavior=read(BASE/'b02q06/attempt-02/behavior-checks.json');names=['b02q06-ordinary-01','b02q06-ordinary-02'];proofs=[read(BASE/(n+'-proof.json'))for n in names]
for name,p in zip(names,proofs):
 assert p['passed']and p['exact_Run_after_UI_open']and p['viewing_preserves_Run']and read(BASE/(name+'-close.json'))['normal_close']
 for spec in behavior['checks']:
  key=spec['id'];s=catalog[key];winder=s['rule']=='assemble'
  for condition,value in spec['conditions'].items():
   rows=[r for r in p['samples']if r['device']==key and r['condition']==condition];assert len(rows)==(3 if value['wait']else 2);dev=key+'--'+condition;outputs=value['output_ids'];waste=value['waste_ids'];last={r['data']['id']:r['data']for r in rows[-1]['items']if r['kind']=='item'}
   for uid in value['source_ids']:assert last[uid]['lifecycle']=='transformed'
   for uid in outputs+waste:
    i=last[uid];assert i['holder']==dict(kind='location',id=dev+'-waste'if uid in waste else value['destination'])and i['quality']=='released'and i['lifecycle']=='available'and i['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))and not i['contents']
    assert set(i['composition_refs'])==set(value['source_ids'])and i['attributes']['presentation_state']['value']==('collected_paper_dust'if uid in waste else s['final'])
   for sample in rows[:-1]:
    j=sample['job'][0]['data'];current={r['data']['id']:r['data']for r in sample['items']if r['kind']=='item'}
    if condition in ['missing','missing-auxiliary','missing-component']:assert j['started_at_ms']is None
    elif condition in ['phase-wait','interrupted']:
     assert j['state']=='waiting_resource'
     for uid in value['source_ids']:assert current[uid]['holder']==dict(kind='job',id=j['id'])
    elif condition in ['output-wait','waste-wait']:
     assert j['state']=='waiting_output';held=waste if condition=='waste-wait'else outputs;assert set(j['pending_outputs'])==set(held)
     for uid in held:assert current[uid]['holder']==dict(kind='job',id=j['id'])
     for uid in outputs+waste:
      if uid not in held:assert current[uid]['holder']==dict(kind='location',id=dev+'-out'if uid in outputs else dev+'-waste')
   if condition=='interrupted':a,b=[r['job'][0]['data']for r in rows[:2]];assert a['effective_progress_ms']==b['effective_progress_ms']>0 and a['phase_ref']==b['phase_ref']=='auto'
   checks.append({'process':p['identity']['host_instance_id'],'device':key,'condition':condition,'passed':True,'holders_verified':True,'identity_quantity_origins_verified':True,'recovery_verified':True})
media=[];saved=BASE/'b02q06/native-saved-01/saved'
for folder in [HOME/'installable',HOME/'items']:
 for model in folder.rglob('*.glb'):
  d=read(model.parent/'module.json')['data']['definition'];target=saved/'bundled'/d['id'].encode().hex()/d['version'].encode().hex()/model.name;assert target.read_bytes()==model.read_bytes();media.append({'module_ref':{'id':d['id'],'version':d['version']},'asset':model.name,'exact_current_media_preserved':True})
(BASE/'b02q06/Native-persistence-checks.json').write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'processes':[p['identity']for p in proofs],'current_media_checks':media,'formal_NFR_timing':False},ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(f'Q06: {len(checks)} conditions across two ordinary processes; {len(media)} exact current assets preserved.')
