from pathlib import Path
import json,sys,math
ROOT=Path(__file__).resolve().parents[4];base=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf8'));checks=[]
passes=sys.argv[1:3] or ['b01q07r1-ordinary-01','b01q07r1-ordinary-02']
for name in passes:
 p=read(base/(name+'-proof.json'));assert p['passed'] and p['exact_Run_after_UI_open'] and p['viewing_preserves_Run'] and len(p['samples'])==20
 for condition in ['normal','missing','phase-wait','interrupted','output-wait','scrap-wait','full-scrap-wait']:
  rows=[s for s in p['samples'] if s['device']=='platen-die-cutter' and s['condition']==condition];assert len(rows)==(2 if condition=='normal' else 3);dev='platen-die-cutter--'+condition;job=rows[-1]['job'][0]['data'];assert job['state']=='completed'
  items=[r['data']for r in rows[-1]['items']if r['kind']=='item'];raw=next(i for i in items if i['id']=='paper-'+dev);product=[i for i in items if i['holder']==dict(kind='location',id=dev+'-out')];scraps=[i for i in items if i['holder']==dict(kind='location',id=dev+'-scrap')];assert len(product)==1 and len(scraps)==4 and raw['lifecycle']=='transformed'
  outputs=product+scraps;assert len({i['id']for i in outputs})==5 and abs(sum(i['attributes']['area_m2']['value']for i in outputs)-raw['attributes']['area_m2']['value'])<1e-12
  for i in outputs:assert i['composition_refs']==[raw['id']]and i['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))and i['attributes']['thickness_m']==raw['attributes']['thickness_m']
  assert product[0]['attributes']['presentation_state']['value']=='cross_cut_blank' and all(i['attributes']['presentation_state']['value']=='corner_waste'for i in scraps)
  for sample in rows[:-1]:
   j=sample['job'][0]['data'];held=[r['data'] for r in sample['items'] if r['kind']=='item' and r['data']['holder']==dict(kind='job',id=j['id']) and r['data']['lifecycle']in['available','in_process']]
   if condition=='missing':assert j['started_at_ms']is None and not held
   elif condition in ['phase-wait','interrupted']:assert j['state']=='waiting_resource' and [i['id']for i in held]==[raw['id']]
   elif condition in ['output-wait','scrap-wait','full-scrap-wait']:
    expected=4 if condition=='full-scrap-wait'and sample['at_ms']<30000 else 1
    assert j['state']=='waiting_output'and len(held)==expected and {i['id']for i in held}==set(j['pending_outputs'])and all(i['composition_refs']==[raw['id']]for i in held)
    delivered=[r['data']for r in sample['items']if r['kind']=='item'and r['data']['holder']['kind']=='location'and r['data']['composition_refs']==[raw['id']]];assert len(delivered)==5-expected
  if condition=='interrupted':a,b=[s['job'][0]['data']for s in rows[:2]];assert a['phase_ref']==b['phase_ref']=='auto' and a['effective_progress_ms']==b['effective_progress_ms']>0
  checks.append({'process':p['identity']['host_instance_id'],'condition':condition,'actual_outputs':5,'source_quantity_area_thickness_and_holders':True})
a,b=[read(base/(n+'-proof.json'))for n in passes];assert a['identity']['host_instance_id']!=b['identity']['host_instance_id']and len(checks)==14
for n in passes:assert read(base/(n+'-close.json'))['normal_close']
assert (base/'b01q07r1/native-saved-r1/saved/project.json').is_file()
(base/'b01q07r1/Native-persistence-r1-checks.json').write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'samples_per_process':20,'processes':[a['identity'],b['identity']],'actual_media_save':True,'formal_NFR_timing':False},indent=2)+'\n',encoding='utf8')
print('B01-Q07-R1: two ordinary Native processes; blank and four real corner outputs, source/area/quantity/thickness, holders, waits, pause and save/reopen PASS.')
