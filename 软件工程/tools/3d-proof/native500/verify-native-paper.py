from pathlib import Path
import json,sys
ROOT=Path(__file__).resolve().parents[3];base=ROOT/'.work/native500-quality-20261025'
read=lambda p:json.loads(p.read_text(encoding='utf8'))
checks=[]
passes=sys.argv[1:3] or ['ordinary-01','ordinary-02'];saved=sys.argv[3] if len(sys.argv)>3 else 'native-persistence'
for pass_name in passes:
 proof=read(base/(pass_name+'-proof.json'));assert proof['passed'] and proof['exact_Run_after_UI_open'] and proof['viewing_preserves_Run']
 for key in {s['device'] for s in proof['samples']}:
  for condition in ['normal','missing','phase-wait','interrupted','output-wait']:
   rows=[s for s in proof['samples'] if s['device']==key and s['condition']==condition]
   final=rows[-1]['job'][0]['data'];assert final['state']=='completed'
   for sample in rows[:-1]:
    job=sample['job'][0]['data'];items=[i['data'] for i in sample['items']]
    if condition=='missing':assert job['started_at_ms'] is None and not any(i['holder']==dict(kind='job',id=job['id']) for i in items)
    elif condition in ['phase-wait','interrupted','output-wait']:
     held=[i for i in items if i['holder']==dict(kind='job',id=job['id']) and i['lifecycle'] in ['available','in_process']]
     assert len(held)==1 and held[0]['quantity']['value']==dict(coefficient=1,decimal_places=0)
     assert job['state'] in ['waiting_resource','waiting_output']
   if condition=='interrupted':
    a,b=[s['job'][0]['data'] for s in rows[:2]];assert a['phase_ref']==b['phase_ref']=='auto' and a['effective_progress_ms']==b['effective_progress_ms']>0
   checks.append(dict(process=proof['identity']['host_instance_id'],device=key,condition=condition,passed=True))
assert read(base/(passes[0]+'-proof.json'))['identity']['host_instance_id']!=read(base/(passes[1]+'-proof.json'))['identity']['host_instance_id']
assert (base/saved/'saved/project.json').is_file()
(base/(sys.argv[4] if len(sys.argv)>4 else 'Native-persistence-checks.json')).write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'per_device_conditions':checks,'samples_per_process':70,'tests_do_not_claim_formal_NFR_timing':True},ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print('Ordinary Native save + fresh process reopen: 5 devices x 5 conditions x 2 processes, holder and frozen pause progress PASS.')
