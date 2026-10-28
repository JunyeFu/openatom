"""检查现有设计消费者并推导明确名义时间；没有产品或专业程序运行。"""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BASE=Path(__file__).resolve().parent
cases=[];validated=[]
def add(case_id,data):
    cases.append(dict(id=case_id,manual_expected=data,actual_result=None))

def template(folder):
    file=ROOT/'modules/templates'/folder/'project.design.json'
    p=json.loads(file.read_text(encoding='utf-8'));mods={}
    for ref in p['module_refs']:
        f=file.parent/ref['manifest'];m=json.loads(f.read_text(encoding='utf-8'))
        assert m['id']==ref['id'] and m['version']==ref['version'];mods[m['id']]=m
    stores={s['id']:s for s in p['stores']};types={t['id'] for t in p['item_types']};resources={r['id'] for r in p['resources']}
    for s in p['initial_stock']:assert s['item_type']==stores[s['store']]['item_type'] and s['item_type'] in types
    for d in p['devices']:
        m=mods[d['module']];assert set(d['parameters'])<={x['id'] for x in m['parameters']}
        assert set(d['port_bindings'])=={x['id'] for x in m['ports']}
        for port in m['ports']:
            item=port.get('item') or d['parameters'][port['item_parameter']]
            store=stores[d['port_bindings'][port['id']]]
            assert store['item_type']==item and store['unit']==port['unit'],(folder,d['id'],port['id'])
        if 'phases' in d:assert sum(x['duration_ms'] for x in d['phases'])==d['parameters']['cycle_seconds']*1000
        for resource in d['job_resources']:assert resource==d['id'] or resource in resources
        for resource in d.get('resource_bindings',{}).values():assert resource in resources
        assert {r['role'] for r in m['behavior'].get('resources',[])}<=set(d.get('resource_bindings',{}))
        for phase in d.get('phases',[]):
            for resource in phase['phase_resources']:assert resource in resources
        for v in d['layout_m']:assert 0<=v<=24
    assert p['actual_result'] is None and p['professional_modes']['silent_downgrade'] is False
    assert p['connections']
    validated.append(dict(template=folder,device_count=len(p['devices']),mode=p['professional_modes']['selected'],
                          nominal_inputs_checked=True,actual_result=None))
    return p

def schedule(case_id,p,board_prep=None):
    devices={d['id']:d for d in p['devices']};end={};jobs=[]
    def job(name,item,ready,duration,owners):
        start=max([ready]+[end.get(o,0) for o in owners]);finish=start+duration
        for o in owners:end[o]=finish
        jobs.append(dict(kind=name,item=item,start_ms=start,end_ms=finish,resources=owners))
        return finish
    stock={s['item_type']:s['quantity'] for s in p['initial_stock']}
    def duration(device_id):return devices[device_id]['parameters']['cycle_seconds']*1000
    cnc=[]
    if 'CNC01' in devices:
        for i in range(2):
            phase={x['id']:x['duration_ms'] for x in devices['CNC01']['phases']}
            a=job('load',i,0,phase['load'],['CNC01','W-C'])
            b=job('auto',i,a,phase['auto'],['CNC01'])
            cnc.append(job('unload',i,b,phase['unload'],['CNC01','W-C']))
    else:cnc=[0,0]
    boards=[0,0]
    if 'Board01' in devices:
        prep=p['preparation_tasks'][0]['duration_ms'] if board_prep is None else board_prep
        ready=job('setup',None,0,prep,['Board01','W-E']);boards=[]
        for i in range(2):
            phase={x['id']:x['duration_ms'] for x in devices['Board01']['phases']}
            a=job('board_load',i,ready,phase['load'],['Board01','W-E'])
            b=job('board_place',i,a,phase['placement'],['Board01'])
            c=job('board_unload',i,b,phase['unload'],['Board01','W-E'])
            boards.append(job('board_finish',i,c,duration('BoardFinish01'),['BoardFinish01']))
    accepted=[]
    for i in range(2):
        ready=max(cnc[i],boards[i])
        if 'Assembly01' in devices:ready=job('assembly',i,ready,duration('Assembly01'),['Assembly01','W-A'])
        checked=job('inspection',i,ready,duration('Inspection01'),['Inspection01','W-B'])
        accepted.append(job('packing',i,checked,duration('Packing01'),['Packing01','W-B']))
    if 'Assembly01' in devices:assert stock['fastener']==2*4 and stock['carton']==2
    if 'Board01' in devices:assert stock['bare_pcb']==2 and stock['component']==2*8
    for resource in {r for j in jobs for r in j['resources']}:
        seq=sorted([j for j in jobs if resource in j['resources']],key=lambda j:j['start_ms'])
        assert all(a['end_ms']<=b['start_ms'] for a,b in zip(seq,seq[1:]))
    assert max(accepted)<=p['time']['observation_end']
    add(case_id,dict(jobs=jobs,accepted_at_ms=accepted,accepted_count=2,acceptance_scope='declared_factory_store',
                    quality_source='preset_or_declared_not_actual',initial_stock=stock))

a=template('machining-delivery');b=template('purchased-assembly');c=template('parallel-electromechanical')
schedule('machining_delivery',a);schedule('purchased_assembly',b);schedule('parallel_manufacturing',c)
schedule('parallel_board_preparation_90s',c,90000)
assert [x['manual_expected']['accepted_at_ms'] for x in cases]==[[90000,150000],[70000,110000],[160000,200000],[220000,260000]]

add('insert_template_into_existing_project',dict(initial_stock_added=0,demand_added=0,
    cloned_object_ids=['copy1.CNC01','copy2.CNC01'],module_definition_identity_shared=True,
    basis='explicit_template_import_policy_not_actual_editor_test'))
add('missing_visual_asset',dict(choice='explicit_native_or_icon_replacement',behavior_changed=False,stock_changed=False,missing_asset_disclosed=True))
add('missing_behavior_definition',dict(affected_operation='not_executable',whole_factory_result='incomplete',unknown_operation_is_zero_time=False))
add('missing_requested_professional_result',dict(selected_mode='calculated_nominal',state='awaiting_actual_supported_provider',automatic_declared_downgrade=False))

snapshot=dict(time_ms=60000,origin='same_project_origin',
    items=[dict(id='item-job',holder='job1'),dict(id='item-transit',holder='vehicle1')],
    job=dict(id='job1',definition_version='0.1.0',effective_done_ms=20000,effective_total_ms=40000),
    transport=dict(item='item-transit',accept_at_ms=75000),
    future_arrival=dict(id='new-input',at_ms=90000),accepted_base=2,demand_total=4,
    actual_result=None)
remaining=snapshot['job']['effective_total_ms']-snapshot['job']['effective_done_ms']
assert remaining==20000 and snapshot['future_arrival']['at_ms']>snapshot['time_ms']
add('declared_snapshot_continuation',dict(snapshot=snapshot,job_finish_at_ms=snapshot['time_ms']+remaining,
    transport_accept_at_ms=75000,future_arrival_kept_at_ms=90000,past_events_replayed=False,
    accepted_increment=2,accepted_cumulative=4,remaining_demand_after=0,
    source_stock_recreated=False,active_job_definition_preserved='0.1.0',
    future_job_definition='0.2.0',future_job_duration_ms=30000,actual_resume_result=None))
add('non_resumable_professional_state',dict(can_read=True,can_resume=False,permitted_next='from_start_or_actual_supported_state',
    restarting_half_job_silently=False))
assert len(cases)==10 and all(x['actual_result'] is None for x in cases)
output=BASE/'expected-template-handoff.design.json'
previous=json.loads(output.read_text(encoding='utf-8-sig')) if output.exists() else {}
generated=dict(document_kind='finite_design_checks_not_product_acceptance',templates=validated,cases=cases)
for key in ['refinement_ref','drf05_handoff_cases','refinement_generation_note']:
    if key in previous:
        generated[key]=previous[key]
output.write_text(json.dumps(generated,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('3 template declarations match referenced ports/types/parameters; 10 finite time/handoff expectations. No product open/resume or professional execution.')
