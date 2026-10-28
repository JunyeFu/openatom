"""Verify two ordinary Native processes and saved assets for a paper transform/assembly batch."""
from pathlib import Path
import json,sys
from acceptance_profiles import native_conditions

ROOT=Path(__file__).resolve().parents[3]; BASE=ROOT/'.work/native500-quality-20261025'
batch,attempt,first_pass,second_pass,saved_dir=sys.argv[1:6]
HOME=ROOT/'modules/media/native-equipment-500/reviews'/batch
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
behavior=read(BASE/attempt/'behavior-checks.json'); checks=[]
config=read(HOME/'acceptance-input.json')
required=native_conditions(read(HOME/'catalog.json')['equipment'],config.get('native_wait_reuse'))
proofs=[read(BASE/(name+'-proof.json')) for name in [first_pass,second_pass]]
states={s['id']:s['states'][0] for s in read(HOME/'items/inventory.json')['metadata'] if s['states']}
for name,proof in zip([first_pass,second_pass],proofs):
    assert proof['passed'] and proof['exact_Run_after_UI_open'] and proof['viewing_preserves_Run']
    assert read(BASE/(name+'-close.json'))['normal_close']
    for spec in behavior['checks']:
        key=spec['id']
        for condition,value in spec['conditions'].items():
            if (key,condition) not in required: continue
            rows=[s for s in proof['samples'] if s['device']==key and s['condition']==condition]
            assert len(rows)==(3 if value['wait'] else 2)
            dev=key+'--'+condition; outputs=value['output_ids']; waste=value['waste_ids']
            last={s['data']['id']:s['data'] for s in rows[-1]['items'] if s['kind']=='item'}
            preserve=value.get('preserves_identity',False)
            for uid in value['source_ids']: assert last[uid]['lifecycle']==('available' if preserve else 'transformed')
            for uid in outputs+waste:
                item=last[uid]
                assert item['holder']==dict(kind='location',id=dev+'-waste' if uid in waste else value['destination'])
                assert item['quality']=='released' and item['lifecycle']=='available'
                assert item['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0)) and not item['contents']
                if preserve:
                    assert outputs==value['source_ids'] and not item['composition_refs']
                    assert item['attributes']==value['expected_attributes'] and item['item_type_ref']==value['expected_item_type']
                else: assert set(item['composition_refs'])==set(value['source_ids'])
                assert item['attributes']['presentation_state']['value']==states[key+('-input' if preserve else '-waste' if uid in waste else '-output')]
            for sample in rows[:-1]:
                job=sample['job'][0]['data']; current={s['data']['id']:s['data'] for s in sample['items'] if s['kind']=='item'}
                if condition.startswith('missing'):
                    assert job['started_at_ms'] is None
                elif condition in ['phase-wait','interrupted']:
                    assert job['state']=='waiting_resource'
                    for uid in value['source_ids']: assert current[uid]['holder']==dict(kind='job',id=job['id'])
                elif condition in ['output-wait','waste-wait']:
                    assert job['state']=='waiting_output'
                    held=waste if condition=='waste-wait' else outputs
                    assert set(job['pending_outputs'])==(set() if preserve else set(held))
                    for uid in held: assert current[uid]['holder']==dict(kind='job',id=job['id'])
                    for uid in outputs+waste:
                        if uid not in held: assert current[uid]['holder']==dict(kind='location',id=dev+('-out' if uid in outputs else '-waste'))
            if condition=='interrupted':
                a,b=[s['job'][0]['data'] for s in rows[:2]]
                assert a['effective_progress_ms']==b['effective_progress_ms']>0 and a['phase_ref']==b['phase_ref']=='auto'
            checks.append({'process':proof['identity']['host_instance_id'],'device':key,'condition':condition,'passed':True,'holders_verified':True,'identity_quantity_origins_verified':True,'recovery_verified':True})
media=[]; saved=ROOT/saved_dir
for folder in [HOME/'installable',HOME/'items']:
    for model in folder.rglob('*.glb'):
        definition=read(model.parent/'module.json')['data']['definition']
        target=saved/'bundled'/definition['id'].encode().hex()/definition['version'].encode().hex()/model.name
        assert target.read_bytes()==model.read_bytes()
        media.append({'module_ref':{'id':definition['id'],'version':definition['version']},'asset':model.name,'exact_current_media_preserved':True})
out=BASE/attempt.split('/')[0]/'Native-persistence-checks.json'
out.write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'processes':[p['identity'] for p in proofs],'current_media_checks':media,'formal_NFR_timing':False},ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print(f'{batch}: {len(checks)} conditions across two ordinary processes; {len(media)} exact current assets preserved.')
