"""S06-01有限算法/旧独立预期检查；不是产品Engine，不改写输入或旧预期。"""
import copy
import heapq
import importlib.util
import json
from pathlib import Path

S = Path(__file__).resolve().parent.parent
A = S / 'docs/04_开发与验收'


def read(path):
    return json.loads((S / path).read_text(encoding='utf-8-sig'))


def import_proof(path, name):
    spec = importlib.util.spec_from_file_location(name, S / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def intervals(rows):
    merged = []
    for start, end in sorted(rows):
        assert start <= end
        if start == end:
            continue
        if merged and start <= merged[-1][1]:
            merged[-1][1] = max(merged[-1][1], end)
        else:
            merged.append([start, end])
    return merged


def intersection(left, right):
    return intervals([[max(a, c), min(b, d)] for a, b in intervals(left)
                      for c, d in intervals(right) if max(a, c) < min(b, d)])


def first_fit(calendar, at, work):
    for a, b in intervals(calendar):
        start = max(at, a)
        if start < b and start + work <= b:
            return start
    return None


def duration(rows):
    return sum(b - a for a, b in intervals(rows))


def route_is_acyclic(graph):
    active, complete = set(), set()
    def visit(node):
        if node in active:
            return False
        if node in complete:
            return True
        active.add(node)
        if not all(visit(child) for child in graph[node]):
            return False
        active.remove(node)
        complete.add(node)
        return True
    return all(visit(node) for node in sorted(graph))


def finite(case):
    kind = case['kind']
    if kind == 'dispatch':
        eligible = [row for row in case['jobs'] if row['ready'] <= case['at']]
        return [row['id'] for row in sorted(eligible, key=lambda row:
                (-row['priority'], row['due'] is None,
                 row['due'] if row['due'] is not None else 0, row['ready'], row['id']))]
    if kind == 'atomic_kit':
        stock, capacity = copy.deepcopy(case['stock']), case['capacity']
        ok = capacity > 0 and all(stock[k] >= qty for k, qty in case['need'].items())
        if ok:
            for k, qty in case['need'].items():
                stock[k] -= qty
            capacity -= 1
        return {'committed': ok, 'stock': stock, 'capacity': capacity}
    if kind == 'eligibility':
        eligible = [row['id'] for row in case['items']
                    if row['holder'] == 'store' and row['quality'] == 'released'
                    and row['reserve'] in (None, 'self')]
        free = [row['id'] for row in case['items']
                if row['id'] in eligible and row['reserve'] is None]
        return {'free': free, 'ordinary_for_self': eligible,
                'excluded_from_free_count': len(case['items']) - len(free)}
    if kind == 'calendar_phases':
        at, jobs, worker_work, active = 0, [], 0, 0
        for phase in case['phases']:
            calendar = intersection(case['machine'], case['worker']) if phase['worker'] else case['machine']
            start = first_fit(calendar, at, phase['duration'])
            assert start is not None
            at = start + phase['duration']
            jobs.append([phase['id'], start, at])
            active += phase['duration']
            if phase['worker']:
                worker_work += phase['duration']
        return {'phases': jobs, 'worker_active': worker_work, 'active': active,
                'waiting': at - active, 'completion': at}
    if kind == 'non_preemptive_no_fit':
        return first_fit(case['calendar'], case['at'], case['duration'])
    if kind == 'pause_resume':
        remaining, worked = case['duration'], []
        for a, b in intervals(case['calendar']):
            start = max(a, case['at'])
            amount = min(remaining, max(0, b - start))
            if amount:
                worked.append([start, start + amount])
                remaining -= amount
            if not remaining:
                break
        assert not remaining
        finish = worked[-1][1]
        return {'active_intervals': worked, 'completion': finish,
                'active': duration(worked), 'waiting': finish - case['at'] - duration(worked),
                'phase_resource_intervals': copy.deepcopy(worked),
                'job_resource_interval': [case['at'], finish]}
    if kind == 'output_once':
        slot = {'holder': 'job', 'pending': True, 'ready': case['completed_at']}
        transfers, accepted = 0, None
        for attempt in case['tries']:
            if slot['pending'] and attempt['room']:
                slot['holder'], slot['pending'] = 'destination', False
                transfers += 1
                accepted = attempt['at']
        return {'created': 1, 'transfers': transfers, 'accepted_at': accepted,
                'wait_interval': [slot['ready'], accepted], 'final_holder': slot['holder']}
    if kind == 'quality':
        result = []
        for sample in case['samples']:
            q = sample['quality']
            result.append('pending' if q is None else 'released' if q == 'PASS' else
                          'rework' if sample['completed_reworks'] < sample['max'] else 'isolate')
        return result
    if kind == 'pending_capacity':
        def move(occupied):
            succeeds = occupied < case['capacity']
            return {'holder': 'pending_store' if succeeds else 'inspection',
                    'resources_released': succeeds, 'quality': 'pending'}
        return {'full': move(case['occupied']),
                'after_declared_consumer_frees_slot': move(case['occupied'] - 1)}
    if kind == 'full_order_acceptance':
        at_first, at_second = [row['at'] for row in case['receipts']]
        assert len({row['item'] for row in case['receipts']}) == case['demand']
        received, accepted_full, accepted_partial = set(), set(), set()
        states = []
        for receipt in case['receipts']:
            received.add(receipt['item'])
            accepted_partial.add(receipt['item'])
            if len(received) >= case['demand']:
                accepted_full.update(received)
            states.append((len(accepted_full), len(accepted_partial)))
        return {'nonpartial': {'accepted_after_first': states[0][0],
                'accepted_after_second': states[1][0], 'physical_received': len(received),
                'completion': at_second, 'tardiness': max(0, at_second - case['due'])},
                'partial': {'accepted_after_first': states[0][1],
                'accepted_after_second': states[1][1], 'completion': at_second},
                'storage_count_after_acceptance': len(received)}
    if kind == 'transport_backpressure':
        cargo = {'id': 'C1', 'holder': 'transport'}
        departed_id = cargo['id']
        arrived = case['depart'] + case['travel']
        receive = max(arrived, case['receiver_ready']) + case['unload']
        cargo['holder'] = 'destination'
        available = receive + case['return']
        return {'arrived': arrived, 'received': receive, 'carrier_available': available,
                'wait': [arrived, receive], 'carrier_held': [case['depart'], available],
                'same_cargo_identity': departed_id == cargo['id']}
    if kind == 'window_and_union':
        return {'accepted': sum(row['quantity'] for row in case['acceptance_events'] if row['at'] <= case['end']),
                'occupied': duration(case['occupancy']), 'wait_union': duration(case['waits'])}
    if kind == 'snapshot_and_quiescence':
        frozen = copy.deepcopy(case['input'])
        next_input = copy.deepcopy(case['input'])
        next_input['work_ms'] = case['edit_next_work_ms']
        blockers = {'needed': case['input']['initial_qty'], 'accepted': 0}
        assert case['no_future_enabling_event']
        return {'frozen_work_ms': frozen['work_ms'], 'next_work_ms': next_input['work_ms'],
                'run_calculation_complete': case['no_future_enabling_event'],
                'target_complete': blockers['accepted'] >= blockers['needed'],
                'missing_target_quantity': blockers['needed'] - blockers['accepted'],
                'last_time': case['window_end']}
    if kind == 'zero_duration_route':
        acyclic = route_is_acyclic(case['acyclic'])
        pending, reached_sink = ['a'], 0
        assert acyclic
        while pending:
            node = pending.pop(0)
            children = case['acyclic'][node]
            if children:
                pending.extend(children)
            else:
                reached_sink += 1
        return {'acyclic_accepted': acyclic, 'unbounded_rejected': not route_is_acyclic(case['unbounded']),
                'finite_zero_work_output_count': reached_sink}
    if kind == 'first_article':
        return [approval == case['current'] for approval in case['approvals']]
    if kind == 'transport_quantity':
        result = []
        for sample in case['samples']:
            load = min(sample['goal'], sample['stock'], sample['capacity'])
            if not sample['partial'] and load < sample['capacity']:
                load = 0
            result.append(load)
        return result
    if kind == 'prepare_ahead':
        start = case['previous_batch_finish']
        end = start + case['changeover']
        guard, preparations = set(), []
        for family in ['B', 'A', 'B']:
            if end in guard:
                continue
            preparations.append(family)
            guard.add(end)
        return {'setup': [start, end], 'assembly_earliest': max(end, case['material_arrives']),
                'after_one_idle_setup_no_same_time_family_flip': preparations == ['B']}
    if kind == 'professional_binding_and_unknown_cost':
        return {'eligible': [row['outcome'] == 'Pass' and row['ack'] and row['matching']
                            for row in case['bindings']],
                'cost': None if case['price'] is None else case['price']}
    if kind == 'zero_stage_calendar_boundary':
        return [first_fit(calendar, case['at'], case['duration']) for calendar in case['calendars']]
    if kind == 'capacity_integral':
        all_intervals, occupied_units = [], 0
        for claim in case['claims']:
            same_claim = intervals(claim['intervals'])
            all_intervals.extend(same_claim)
            occupied_units += claim['units'] * duration(same_claim)
        available_units = case['capacity'] * duration(case['calendar'])
        return {'any_busy': duration(all_intervals), 'occupied_capacity_time': occupied_units,
                'available_capacity_time': available_units, 'utilization': occupied_units / available_units}
    if kind == 'single_step_closed_timestamp':
        queue = [(e['at'], e['phase'], e['id'], e['kind']) for e in case['events']]
        heapq.heapify(queue)
        at = queue[0][0]
        state = {'free': False, 'stock': 0, 'pending': 0, 'accepted': 0, 'jobs_left': 1}
        processed = []
        while queue and queue[0][0] == at:
            _, _, ident, event = heapq.heappop(queue)
            processed.append(event)
            if event == 'release':
                state['free'] = True
            elif event == 'arrival':
                state['stock'] += 1
            elif event == 'start':
                assert state['stock'] and state['free']
                state['stock'] -= 1
                state['free'] = False
                state['jobs_left'] -= 1
                heapq.heappush(queue, (at, 0, 'zero-done', 'zero_done'))
            elif event == 'zero_done':
                state['pending'] += 1
                heapq.heappush(queue, (at, 2, 'transfer', 'transfer'))
            elif event == 'transfer':
                state['pending'] -= 1
                state['accepted'] += 1
                state['free'] = True
            if state['jobs_left'] and state['free'] and state['stock']:
                # Exactly one start candidate for this finite input. Later phases
                # are not observed until this timestamp closes.
                if not any(e[3] == 'start' for e in queue):
                    heapq.heappush(queue, (at, 3, 'start', 'start'))
        return {'step_at': at, 'processed': processed, 'accepted': state['accepted'],
                'resource_free': state['free'], 'pending_output': state['pending'],
                'next_event_at': queue[0][0]}
    raise AssertionError(('unhandled_finite_case', case['id']))


def main():
    spec = read('docs/03_架构与契约/调度与状态转换算法.json')
    source = read(spec['finite_cases_ref'])
    assert [r['id'] for r in spec['rules']] == [f'ALG-{i:02}' for i in range(1, 15)]
    for rule in spec['rules']:
        assert rule['inputs'] and rule['algorithm'] and rule['outputs'] and rule['boundary']
    assert len(spec['interfaces']) == len(spec['applicable_views']) == 6
    assert spec['actual_product_runtime'] is None and not spec['actual_full_engine_implemented']
    rows = []
    for case in source['cases']:
        result = finite(case)
        assert result == case['expected'], (case['id'], result, case['expected'])
        rows.append({'id': case['id'], 'rules': case['rules'], 'result': result,
                     'status': 'passed_finite_design_reasoning_not_product_Run'})
    assert len(rows) == 21
    assert {rule for row in rows for rule in row['rules']} == {r['id'] for r in spec['rules']}
    # 执行原有有限手算函数，但不调用其写文件main，不反写独立预期。
    business = import_proof('scenarios/真实业务设计/复算业务案例.py', 'business_math')
    derived = [business.multi_order('A'), business.multi_order('B'), business.calendar_detail(),
               business.quality_case(), *business.coarse_detail()]
    old = read('scenarios/真实业务设计/expected-business.design.json')
    assert derived == old['cases']
    control = import_proof('scenarios/从零搭建控制盒厂房/推导时间轴.py', 'control_math')
    expected = read('scenarios/从零搭建控制盒厂房/expected-timeline.design.json')
    for case in expected['cases']:
        control.verify(case)
    assert [c['expected_completion_ms'] for c in expected['cases']] == [440000, 570000]
    delayed = expected['cases'][1]['transfers'][5]
    assert (delayed['arrival_ms'], delayed['accepted_ms'], delayed['resource_available_ms']) == (370000, 420000, 425000)
    factory = read('scenarios/两厂房装配包装/expected-events.design.json')
    first_receive = 60 + 10 + 30 + 10
    first_pack, cart_available = first_receive + 15, first_receive + 30
    second_pack = max(120, cart_available) + 10 + 30 + 10 + 15
    assert [first_pack, second_pack] == [125, 205]
    for variant in factory['variants']:
        finishes = [e['time'] for e in variant['expected_events'] if e['kind'] == 'pack_complete']
        assert finishes == ([125] if variant['id'] == 'A' else [125, 205])
        assert variant['actual_result'] is None
    result = {'document_kind': 'S06_01_static_and_finite_design_checks_not_product_engine',
              'checkpoint': 'S06-01', 'status': 'passed_static_specification_and_finite_reasoning',
              'algorithm_rules': 14, 'logical_handoffs': 6, 'applicable_views': 6,
              'finite_cases': rows, 'protected_old_expectation_checks': {
                  'six_business_variants': 'matched_original_functions_without_writing_source',
                  'two_control_box_variants': 'resource_holder_buffer_checks_440_570_425_preserved',
                  'two_hall_arithmetic': '125_205_110_140_preserved'},
              'actual_product_runtime': None, 'actual_product_cases_executed': 0,
              'actual_frontend_or_Figma_started': False, 'errors': []}
    (A / '调度算法检查结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: result[k] for k in ['status', 'algorithm_rules', 'logical_handoffs',
                      'actual_product_cases_executed', 'errors']}, ensure_ascii=False))


if __name__ == '__main__':
    main()
