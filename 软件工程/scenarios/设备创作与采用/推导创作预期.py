"""有限算术/声明预期，不是设备编辑器、运行引擎或素材授权验证器。"""
import json
from fractions import Fraction
from pathlib import Path

BASE=Path(__file__).resolve().parent
src=json.loads((BASE/'inputs.design.json').read_text(encoding='utf-8'))
cases=[]

def add(case_id,expected):
    cases.append(dict(id=case_id,manual_expected=expected,actual_result=None))

def fixed_case(duration,order):
    c=src['fixed_cycle']; by_id={x['id']:x for x in c['clips']}; pos=Fraction(0); clips=[]
    for clip_id in order:
        end=pos+Fraction(by_id[clip_id]['weight'])
        clips.append(dict(id=clip_id,start_ms=float(pos*duration),end_ms=float(end*duration)))
        pos=end
    assert pos==1
    count=min(c['inputs'][k]//v for k,v in c['recipe'].items())
    assert count*duration<=c['observation_ms'] and c['workers']==1
    return dict(clips=clips,effective_duration_ms=duration,completed=count,
                input_remaining={k:v-count*c['recipe'][k] for k,v in c['inputs'].items()},
                blocked_next_cycle_by='pcb',new_engine_result_created=False)

c=src['fixed_cycle']; order=[x['id'] for x in c['clips']]
add('fixed_60s',fixed_case(c['baseline_duration_ms'],order))
add('fixed_30s',fixed_case(c['changed_duration_ms'],order))
add('visual_reorder_60s',fixed_case(c['baseline_duration_ms'],c['reordered_ids']))
assert cases[0]['manual_expected']['completed']==cases[2]['manual_expected']['completed']

c=src['phase_wait']; now=0; intervals=[]; waits=[]; worker_intervals=[]
for phase in c['phases']:
    start=now
    if phase['worker']:
        for a,b in c['worker_busy_intervals_ms']:
            if start<b and start+phase['duration_ms']>a:
                start=b
        if start>now: waits.append(dict(start_ms=now,end_ms=start,reason='phase_resource',kept_pose='auto_end'))
    end=start+phase['duration_ms']
    intervals.append(dict(phase=phase['id'],start_ms=start,end_ms=end,end_pose=phase['end_pose']))
    if phase['worker']: worker_intervals.append([start,end])
    now=end
accepted=max(now,c['output_ready_at_ms'])
if accepted>now: waits.append(dict(start_ms=now,end_ms=accepted,reason='output_capacity',kept_pose='finished'))
samples=[]
for t in c['sample_times_ms']:
    if t>=accepted: state='idle'; pose='idle'; holder='output_store'; progress=None
    else:
        wait=next((w for w in waits if w['start_ms']<=t<w['end_ms']),None)
        if wait: state='waiting_'+wait['reason'];pose=wait['kept_pose'];progress=None
        else:
            work=next(j for j in intervals if j['start_ms']<=t<j['end_ms'])
            state='working_'+work['phase'];pose=work['phase']+'_progress'
            progress=(t-work['start_ms'])/(work['end_ms']-work['start_ms'])
        holder='machine'
    samples.append(dict(at_ms=t,state=state,pose=pose,holder=holder,
                        local_progress=progress,work_finished_count=int(t>=now),received_count=int(t>=accepted)))
assert now==75000 and accepted==85000
assert sum(j['end_ms']-j['start_ms'] for j in intervals)==60000
assert worker_intervals==[[0,10000],[70000,75000]]
assert [s['work_finished_count'] for s in samples]==[0,0,0,0,1,1,1]
assert [s['received_count'] for s in samples]==[0,0,0,0,0,0,1]
add('phase_wait_and_output_hold',dict(intervals=intervals,waits=waits,worker_intervals=worker_intervals,
    effective_ms=60000,work_finished_ms=now,received_ms=accepted,machine_held_ms=accepted,samples=samples))

a=src['adoption']; old={i:a['base_version'] for i in a['instances']}
visual=dict(old);visual[a['selected_instance']]=a['visual_version']
add('selected_visual_upgrade',dict(versions_after=visual,durations_ms={i:a['base_duration_ms'] for i in old},
    prior_result_scope='original_scheme',new_engine_result_created=False))
behavior=dict(old);behavior[a['selected_instance']]=a['behavior_version']
durations={i:a['base_duration_ms'] for i in old};durations[a['selected_instance']]=a['changed_duration_ms']
add('selected_behavior_upgrade',dict(versions_after=behavior,durations_ms=durations,
    changed_scheme_requires_new_execution=True,prior_result_scope='original_scheme'))
add('port_id_change',dict(affected_instance=a['selected_instance'],required_rebinding=[a['dependent_connection_id'],a['dependent_tag_id']],
    untouched_instance='B',automatic_rewire=False,old_scheme_changed=False))
add('public_handoff',dict(included_assets=[x['id'] for x in src['handoff_assets'] if x['redistribute']],
    omitted_assets=[x['id'] for x in src['handoff_assets'] if not x['redistribute']],
    replacement_slot='private-photo',behavior_preserved=True,permission_basis='self_built_inputs_not_actual_permission_evidence'))

assert len(cases)==8 and all(c['actual_result'] is None for c in cases)
(BASE/'expected-authoring.design.json').write_text(json.dumps(dict(document_kind='finite_manual_design_expectations',cases=cases),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('8 design variants consistent: durations, phase waiting, holders, explicit instance adoption and declared asset handoff. No editor/runtime result.')
