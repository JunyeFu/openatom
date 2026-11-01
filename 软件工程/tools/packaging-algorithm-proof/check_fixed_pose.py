"""独立复核合入后的公共保持载体；有限矩形参考方法，不调用Host。"""
from copy import deepcopy
from fractions import Fraction as F
from pathlib import Path
import json
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
OUT = HERE / 'output'
sys.path.insert(0, str(ROOT / 'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator
from prove import Cell, Budget, certify_all, translated


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def write(name, data):
    (OUT / name).write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def carriers(versioned):
    compute = read(OUT / 'local-contact-api-request.json')
    compute['request_id'] = 'FIXED-POSE-INDEPENDENT-COMPUTE'
    compute['params']['packaging_job'] = deepcopy(versioned['data'])
    edit = read(ROOT / 'tools/type-contract-proof/api-fixtures/apply_commands.request.json')
    edit['request_id'] = 'FIXED-POSE-INDEPENDENT-EDIT'
    edit['params']['commands'] = [{'kind': 'put_packaging_job', 'data': deepcopy(versioned['data'])}]
    return {'job': versioned, 'compute_packaging': compute, 'apply_commands': edit}


def finite_hold(job, resolved_product=None):
    before = deepcopy(job)
    step = job['steps'][0]
    motion = step['motion']
    assert motion['kind'] == 'fixed_pose'
    target = motion['data']['part_id']
    parts = {part['id']: part for part in job['parts']}
    evidence = {'target_part_id': target, 'state_mutated': False, 'scope': 'identity rotations and rectangular panels only'}
    if target not in parts:
        return 'input_rejected', {**evidence, 'error_code': 'invalid_input', 'reason': 'unknown_part_id'}
    # These finite examples use exact identity poses. General pose matching belongs to the frozen numeric contract.
    if parts[target]['placement'] != motion['data']['pose']:
        return 'Fail', {**evidence, 'reason': 'current_pose_mismatch_no_teleport', 'classification': 'valid_input_professional_failure'}
    cells = []
    for panel in job['panels']:
        part = parts[panel['part_id']]
        poses = [panel['flat_pose'], part['placement']]
        for pose in poses:
            assert pose['rotation'] == {'x': 0, 'y': 0, 'z': 0, 'w': 1}
            assert pose['scale'] == {'x': 1, 'y': 1, 'z': 1}
            assert pose['unit'] == 'millimeter' and pose['coordinate_frame'] == 'right_handed_z_up'
        translation = [sum(F(pose['translation'][axis]) for pose in poses) for axis in ['x', 'y', 'z']]
        xs = [F(point['x']) for point in panel['polygon_mm']]
        ys = [F(point['y']) for point in panel['polygon_mm']]
        assert set(zip(xs, ys)) == {(min(xs), min(ys)), (min(xs), max(ys)), (max(xs), min(ys)), (max(xs), max(ys))}
        bounds = [(min(xs), max(xs)), (min(ys), max(ys)), (-F(panel['thickness_mm']) / 2, F(panel['thickness_mm']) / 2)]
        cells.append((panel, Cell(panel['id'], bounds, lambda time, xyz=translation: translated(*xyz))))
    assert not job['conditions']['allowed_contacts']
    held = [cell for panel, cell in cells if panel['part_id'] == target]
    peers = [cell for panel, cell in cells if panel['part_id'] != target]
    if resolved_product is not None:
        peers.append(resolved_product)
    budget = Budget(job['budget']['interval_query_limit'], job['budget']['subdivision_limit'])
    outcome, geometry = certify_all(held, peers, budget)
    evidence['geometry'] = geometry
    evidence['interval_queries'] = budget.queries
    if outcome != 'Pass':
        return outcome, evidence
    panel_map = {panel['id']: panel for panel in job['panels']}
    for connection in job['connections']:
        assert connection['engage_after_step_id'] is None  # Initial active bond in this finite negative example.
        frames = []
        for suffix in ['a', 'b']:
            panel = panel_map[connection['panel_' + suffix]]
            part = parts[panel['part_id']]
            frames.append([sum(F(pose['translation'][axis]) for pose in [connection['frame_' + suffix], panel['flat_pose'], part['placement']]) for axis in ['x', 'y', 'z']])
        gap = max(abs(a - b) for a, b in zip(*frames))
        if gap > F(job['tolerance']['constraint_mm']):
            return 'Fail', {**evidence, 'active_connection': connection['id'], 'closure_gap_mm': float(gap)}
    assert job == before
    evidence['non_target_poses_after'] = {key: part['placement'] for key, part in parts.items() if key != target}
    evidence['held_pose_after'] = parts[target]['placement']
    return 'Pass', evidence


def main():
    expected = read(HERE / 'fixed-pose-expectations.json')
    source = read(OUT / 'local-contact-job.json')
    base = deepcopy(source)
    job = base['data']
    job['id'] = 'FIXED_POSE_INDEPENDENT_JOB'
    job['connections'] = []
    job['conditions']['allowed_contacts'] = []
    job['steps'] = [{'id': 'HOLD_B', 'purpose': 'forming', 'duration_hint_ms': None, 'prerequisites': [],
                     'motion': {'kind': 'fixed_pose', 'data': {'part_id': 'PART_B', 'pose': deepcopy(job['parts'][1]['placement'])}}}]
    schemas = {
        'job': ROOT / 'tools/type-contract-proof/output/schemas/packaging-job.schema.json',
        'compute_packaging': ROOT / 'tools/type-contract-proof/output/api-schemas/compute_packaging.request.schema.json',
        'apply_commands': ROOT / 'tools/type-contract-proof/output/api-schemas/apply_commands.request.schema.json',
    }
    validators = {name: Draft202012Validator(read(path)) for name, path in schemas.items()}
    for name, instance in carriers(base).items():
        validators[name].validate(instance)  # Old shared Schema must fail here; never report premature integration.
    for name, instance in carriers(base).items():
        write('fixed-pose-' + name + '.json', instance)
    negatives = []
    for change in expected['schema_rejections']:
        invalid = deepcopy(base)
        data = invalid['data']['steps'][0]['motion']['data']
        if change == 'old_bare_pose':
            invalid['data']['steps'][0]['motion']['data'] = data['pose']
        elif change == 'missing_part_id':
            del data['part_id']
        elif change == 'missing_pose':
            del data['pose']
        else:
            raise AssertionError(change)
        rejected = {name: bool(list(validators[name].iter_errors(instance))) for name, instance in carriers(invalid).items()}
        assert all(rejected.values()), (change, rejected)
        negatives.append({'change': change, 'rejected_by': rejected})
    results = []
    for case in expected['cases']:
        example = deepcopy(base)
        current = example['data']
        data = current['steps'][0]['motion']['data']
        change = case['change']
        product = None
        if change == 'unknown_part':
            data['part_id'] = 'MISSING_PART'
        elif change == 'pose_mismatch':
            data['pose']['translation']['z'] = 9
        elif change == 'peer_collision':
            current['parts'][1]['placement']['translation']['z'] = 0
            data['pose'] = deepcopy(current['parts'][1]['placement'])
        elif change == 'product_collision':
            product = Cell('resolved_subject_finite_cube', [(-F(1, 2), F(1, 2))] * 3, lambda time: translated(25, 25, 2))
        elif change == 'active_bond_broken':
            current['connections'] = deepcopy(source['data']['connections'])
            current['connections'][0]['engage_after_step_id'] = None
            current['connections'][0]['allow_contact'] = False
            current['connections'][0]['contact_allowance_refs'] = []
        else:
            assert change == 'none'
        admission = carriers(example)
        for name, instance in admission.items():
            validators[name].validate(instance)
        before = deepcopy(current)
        actual, evidence = finite_hold(current, product)
        assert current == before
        results.append({'id': case['id'], 'expected': case['expected'], 'actual': actual,
                        'all_three_carrier_shapes_admitted': True, 'matches_independent_expectation': actual == case['expected'], 'evidence': evidence})
    result = {'evidence_level': expected['evidence_scope'], 'cases': results, 'case_count': len(results),
              'all_expected_matched': all(case['matches_independent_expectation'] for case in results), 'schema_negative_cases': negatives,
              'schema_negative_checks': len(negatives) * len(validators), 'carrier_positive_checks': (len(results) + 1) * len(validators),
              'existing_32_expectations_modified': False, 'actual_Host_invocation': False, 'actual_product_runtime': None,
              'shared_files_written': False, 'dependencies_installed': False}
    write('fixed-pose-result.json', result)
    print(json.dumps({key: result[key] for key in ['case_count', 'all_expected_matched', 'schema_negative_checks', 'carrier_positive_checks', 'actual_Host_invocation']}, ensure_ascii=False))
    if not result['all_expected_matched']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
