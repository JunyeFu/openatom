from pathlib import Path
import json,sys,math
ROOT=Path(__file__).resolve().parents[4];base=ROOT/'.work/native500-quality-20261025';home=ROOT/'modules/media/native-equipment-500/reviews/B02-Q03-R1';read=lambda p:json.loads(p.read_text(encoding='utf-8'));checks=[]
passes=sys.argv[1:3] or ['q03r1-ordinary-03','q03r1-ordinary-04']
catalog={e['id']:e for e in read(home/'catalog.json')['equipment']}
for name in passes:
 p=read(base/(name+'-proof.json'));assert p['passed'] and p['exact_Run_after_UI_open'] and p['viewing_preserves_Run']
 for key,spec in catalog.items():
  op=read(home/'devices'/key/'module.json')['data']['definition']['payload']['data']['behavior']['inline'];extra=[o for o in op['outputs'] if o['role']!='output'];n=sum(o['quantity']['value']['coefficient'] for o in extra)
  conditions=['normal','missing','phase-wait','interrupted','output-wait']+(['scrap-wait'] if n else [])+(['missing-ink'] if spec['auxiliary'] else [])
  for condition in conditions:
   rows=[s for s in p['samples'] if s['device']==key and s['condition']==condition];assert len(rows)==(2 if condition=='normal' else 3)
   job=rows[-1]['job'][0]['data'];assert job['state']=='completed';dev=key+'--'+condition
   items=[r['data'] for r in rows[-1]['items'] if r['kind']=='item'];raw=next(i for i in items if i['id']=='paper-'+dev);out=[i for i in items if i['holder']==dict(kind='location',id=dev+'-out')];assert len(out)==1
   if n:
    scrap=[i for i in items if i['holder']==dict(kind='location',id=dev+'-scrap')];assert len(scrap)==n
    assert math.isclose(sum(i['attributes']['area_m2']['value'] for i in out+scrap),raw['attributes']['area_m2']['value'],abs_tol=1e-12)
    for i in out+scrap:assert set(i['composition_refs'])=={'paper-'+dev}|({'ink-'+dev} if spec['auxiliary'] else set())
    if spec['auxiliary']:assert out[0]['attributes']['applied_ink_volume_m3']['value']==.000002
   else:assert out[0]['id']==raw['id'] and raw['attributes']['presentation_state']['value']=='creased'
   for s in rows[:-1]:
    j=s['job'][0]['data'];held=[r['data'] for r in s['items'] if r['kind']=='item' and r['data']['holder']==dict(kind='job',id=j['id']) and r['data']['lifecycle'] in ['available','in_process']]
    if condition in ['missing','missing-ink']:assert j['started_at_ms'] is None and not held
    elif condition in ['phase-wait','interrupted','output-wait','scrap-wait']:
     assert j['state'] in ['waiting_resource','waiting_output'];assert len(held)==(1 if condition in ['scrap-wait','output-wait'] else 2 if spec['auxiliary'] else 1),(key,condition,len(held))
   if condition in ['scrap-wait','output-wait'] and n:
    for sample in rows[:-1]:
     sj=sample['job'][0]['data'];actual=[r['data'] for r in sample['items'] if r['kind']=='item'];produced=[i for i in actual if i['item_type_ref']['id'] in {key+'-output',*[key+'-'+e['role'] for e in extra]} and ('paper-'+dev) in i['composition_refs']];assert len(produced)==1+n
     assert set(sj['pending_outputs'])=={i['id'] for i in produced if i['holder']==dict(kind='job',id=sj['id'])}
   if condition=='interrupted':a,b=[s['job'][0]['data'] for s in rows[:2]];assert a['phase_ref']==b['phase_ref']=='auto' and a['effective_progress_ms']==b['effective_progress_ms']>0
   checks.append({'process':p['identity']['host_instance_id'],'device':key,'condition':condition,'identity_quantity_area_and_holders':True})
a,b=[read(base/(n+'-proof.json')) for n in passes];assert a['identity']['host_instance_id']!=b['identity']['host_instance_id'];assert len(checks)==14
saved=sys.argv[3] if len(sys.argv)>3 else 'q03r1/native-saved-r2';assert (base/saved/'saved/project.json').is_file()
for name in passes:assert read(base/(name+'-close.json'))['normal_close']
(base/(sys.argv[4] if len(sys.argv)>4 else 'q03r1/Native-persistence-r2-checks.json')).write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'samples_per_process':len(a['samples']),'processes':[a['identity'],b['identity']],'actual_media_save':True,'formal_NFR_timing':False},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Native persistence:two distinct ordinary processes,7 conditions each, exact Run, holders, output counts, paper area, ink and normal close PASS.')
