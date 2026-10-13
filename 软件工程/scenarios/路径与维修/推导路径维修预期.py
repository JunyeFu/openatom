"""明确小情景的有限推导；不是产品路径/仿真/维护引擎。"""
import json,math
from pathlib import Path

BASE=Path(__file__).resolve().parent
src=json.loads((BASE/'inputs.design.json').read_text(encoding='utf-8'))
cases=[]
def add(case_id,expected):
    expected['acceptance_scope']='declared_transport_destination_or_operation_output_store_not_customer_order'
    cases.append(dict(id=case_id,manual_expected=expected,actual_result=None))

def length(points):
    return sum(math.dist(a,b) for a,b in zip(points,points[1:]))

def trip(case_id,points,fixed=False,empty_points=None):
    c=src['single_trip']; empty=round(length(empty_points)/c['speed_m_per_s']*1000) if empty_points else 0
    path_length=length(points)
    move=c['fixed_outbound_ms'] if fixed else round(path_length/c['speed_m_per_s']*1000)
    back=c['fixed_return_ms'] if fixed else move
    departed=empty+c['load_ms'];arrived=departed+move;accepted=arrived+c['unload_ms']
    add(case_id,dict(mode='fixed_time' if fixed else 'explicit_path',path_length_used_m=None if fixed else path_length,
        empty_reposition_ms=empty,load_interval_ms=[empty,departed],outbound_interval_ms=[departed,arrived],
        unload_interval_ms=[arrived,accepted],accepted_at_ms=accepted,return_interval_ms=[accepted,accepted+back],
        vehicle_free_at_ms=accepted+back,cargo_id='part-1',before_departure_holder='source_store',in_transit_holder='vehicle',after_acceptance_holder='destination_store'))

trip('fixed_time_after_move',src['single_trip']['moved_points'],fixed=True)
trip('path_baseline',src['single_trip']['base_points'])
trip('path_after_move',src['single_trip']['moved_points'])
trip('empty_reposition',src['single_trip']['base_points'],empty_points=src['single_trip']['empty_points'])

def opposing(case_id,blocked=False,outside_bay=True):
    c=src['shared_corridor']; travel=round(c['length_m']/c['speed_m_per_s']*1000)
    pending=[dict(vehicle=v,kind='loaded',ready_at_ms=c['load_ms']) for v in c['vehicles']]
    free=0; occupation=[]; accepted={}; returned={}; destination_waits=[]
    while pending:
        pending.sort(key=lambda j:(max(j['ready_at_ms'],free),j['ready_at_ms'],j['vehicle']))
        j=pending.pop(0);start=max(j['ready_at_ms'],free);arrival=start+travel
        zone_end=arrival
        if j['kind']=='loaded':
            ready=c['blocked_destination_A_free_at_ms'] if blocked and j['vehicle']=='A' else 0
            unload_start=max(arrival,ready);done=unload_start+c['unload_ms'];accepted[j['vehicle']]=done
            if unload_start>arrival:destination_waits.append(dict(vehicle=j['vehicle'],start_ms=arrival,end_ms=unload_start,holder='vehicle',outside_corridor=outside_bay))
            if not outside_bay:zone_end=done
            pending.append(dict(vehicle=j['vehicle'],kind='empty_return',ready_at_ms=done))
        else:returned[j['vehicle']]=arrival
        occupation.append(dict(vehicle=j['vehicle'],kind=j['kind'],start_ms=start,end_ms=zone_end,travel_arrival_ms=arrival))
        free=zone_end
    assert all(a['end_ms']<=b['start_ms'] for a,b in zip(occupation,occupation[1:]))
    add(case_id,dict(corridor_occupation=occupation,accepted_at_ms=accepted,vehicle_returned_at_ms=returned,
        destination_waits=destination_waits,outside_bay_by_vehicle={v:outside_bay for v in c['vehicles']},holder_until_acceptance='vehicle',road_capacity=1))

opposing('opposing_base')
opposing('blocked_with_outside_bay',blocked=True)
opposing('blocked_without_outside_bay',blocked=True,outside_bay=False)
by_id={x['id']:x['manual_expected'] for x in cases}
assert by_id['opposing_base']['accepted_at_ms']=={'A':13000,'B':21000}
assert by_id['opposing_base']['vehicle_returned_at_ms']=={'A':26000,'B':34000}
assert by_id['blocked_with_outside_bay']['accepted_at_ms']=={'A':23000,'B':21000}
assert by_id['blocked_without_outside_bay']['accepted_at_ms']=={'A':23000,'B':34000}
assert by_id['blocked_with_outside_bay']['vehicle_returned_at_ms']=={'B':29000,'A':37000}
assert by_id['blocked_without_outside_bay']['vehicle_returned_at_ms']=={'A':42000,'B':50000}

c=src['unreachable'];edges={tuple(x) for x in c['remaining_directed_edges']}
assert not all((a,b) in edges for a,b in zip(c['approved_route'],c['approved_route'][1:]))
add('unreachable_route',dict(route_status='needs_reconnection',accepted_at_ms=None,holder='source_store',straight_line_substituted=False))
c=src['cycle_wait'];dependencies={v:c['positions'][pos] for v,pos in c['requested_positions'].items()}
assert dependencies=={'A':'B','B':'A'} and not c['future_release_events']
add('cyclic_position_wait',dict(dependencies=dependencies,current_positions=c['positions'],future_release_events=[],
    diagnosis='current_arrangement_has_no_progress_event',positions_copied=False,cargo_retained=True))

c=src['maintenance'];t=c['request_at_ms'];work=c['work_ms']
for variant in c['cases']:
    name=variant['id']; leases=[]; intervals=[]
    def segment(kind,start,end):
        if end>start:intervals.append(dict(kind=kind,start_ms=start,end_ms=end))
    if name=='known_non_preemptive_window':
        a,b=variant['known_unavailable_ms'];assert a<work
        segment('production',b,b+work)
        expected=dict(start_at_ms=b,first_accepted_at_ms=b+work,completed_items=1,wip_before_window=False,
            input_kits_remaining=c['initial_input_kits']-1,intervals=intervals,unavailable_interval_ms=[a,b],repair_parts_consumption='not_inferred_from_calendar')
    elif variant['stop_policy'] is None:
        segment('production',0,t)
        expected=dict(first_accepted_at_ms=None,completed_items=0,state='awaiting_disposition',holder='original_job',
            effective_work_done_ms=t,resume_ref='first',input_kits_remaining=c['initial_input_kits']-1,spares_consumed=0,intervals=intervals)
    else:
        policy=variant['stop_policy']
        if policy=='finish_current':
            segment('production',0,work);accepted=max(work,c['drain_receiver_ready_at_ms'])
            segment('output_wait',work,accepted);leases.append(dict(user='first',start_ms=0,end_ms=accepted));repair_ready=accepted
        else:
            segment('production',0,t);leases.append(dict(user='first',start_ms=0,end_ms=t));repair_ready=t
            if policy=='abort_disposition':
                repair_ready=t+c['abort_transfer_ms'];segment('isolate_transfer',t,repair_ready)
                leases.append(dict(user='disposition',start_ms=t,end_ms=repair_ready))
        repair_start=max(repair_ready,c['repair_worker_available_at_ms'],c['spare_arrives_at_ms'])
        repair_end=repair_start+c['repair_ms'];qualification_end=repair_end+c['qualification_ms']
        segment('waiting_for_maintenance_inputs',repair_ready,repair_start)
        segment('repair',repair_start,repair_end);segment('qualification',repair_end,qualification_end)
        leases.extend([dict(user='repair',start_ms=repair_start,end_ms=repair_end),dict(user='qualification',start_ms=repair_end,end_ms=qualification_end)])
        expected=dict(repair_start_ms=repair_start,repair_end_ms=repair_end,qualification_end_ms=qualification_end,
            repair_worker_work_ms=c['repair_ms'],spares_consumed=1,
            first_accepted_at_ms=None,completed_items=0,spare_wait_is_worker_work=False)
        if policy=='finish_current':
            second_end=qualification_end+work;segment('production_second',qualification_end,second_end)
            leases.append(dict(user='second',start_ms=qualification_end,end_ms=second_end))
            expected.update(first_accepted_at_ms=accepted,second_accepted_at_ms=second_end,completed_items=2,input_kits_remaining=c['initial_input_kits']-2)
        elif variant['qualification']=='FAIL':
            expected.update(state='awaiting_release',holder='original_job',resume_ref='first',effective_work_done_ms=t,input_kits_remaining=c['initial_input_kits']-1)
        elif policy=='pause_resume':
            end=qualification_end+(work-t);segment('production_resume',qualification_end,end)
            leases.append(dict(user='first',start_ms=qualification_end,end_ms=end))
            expected.update(first_accepted_at_ms=end,completed_items=1,effective_work_total_ms=work,input_kits_remaining=c['initial_input_kits']-1,
                original_item_identity_preserved=True)
        else:
            end=qualification_end+work;segment('production_second',qualification_end,end)
            leases.append(dict(user='second',start_ms=qualification_end,end_ms=end))
            expected.update(second_accepted_at_ms=end,completed_items=1,input_kits_remaining=c['initial_input_kits']-2,
                first_holder='isolate_store',first_quality='pending',original_inputs_returned=0,automatic_replacement_created=False)
        expected['intervals']=intervals
        expected['equipment_exclusive_use']=leases
        assert all(a['end_ms']<=b['start_ms'] for a,b in zip(leases,leases[1:]))
    add('maintenance_'+name,expected)

by_id={x['id']:x['manual_expected'] for x in cases}
assert by_id['maintenance_finish_current']['second_accepted_at_ms']==170000
assert by_id['maintenance_pause_resume']['first_accepted_at_ms']==110000
assert by_id['maintenance_abort_isolate']['second_accepted_at_ms']==140000
assert by_id['maintenance_qualification_fail']['first_accepted_at_ms'] is None
assert by_id['maintenance_missing_stop_policy']['first_accepted_at_ms'] is None
assert by_id['maintenance_known_non_preemptive_window']['first_accepted_at_ms']==140000
assert len(cases)==15 and all(x['actual_result'] is None for x in cases)
(BASE/'expected-logistics-maintenance.design.json').write_text(json.dumps(dict(document_kind='finite_manual_design_expectations_not_engine_result',observation_end_ms=src['observation_end_ms'],cases=cases),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('15 finite design variants consistent: path/empty travel, corridor exclusion, acceptance, waits, maintenance access, spares and resume/disposition. No runtime or field result.')
