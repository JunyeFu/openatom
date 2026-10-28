"""H64-01 schema/DTO consumers and finite stationary-rectangle reference cases."""
from copy import deepcopy
from fractions import Fraction
from pathlib import Path
import json
import sys

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / 'python-libs'))
from jsonschema import Draft202012Validator


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def number(value):
    return Fraction(str(value))


def translation(pose):
    # This finite reference intentionally covers identity poses only.
    assert pose['unit'] == 'millimeter'
    assert pose['coordinate_frame'] == 'right_handed_z_up'
    assert pose['rotation'] == {'x': 0, 'y': 0, 'z': 0, 'w': 1}
    assert pose['scale'] == {'x': 1, 'y': 1, 'z': 1}
    return tuple(number(pose['translation'][k]) for k in 'xyz')


def reference(job):
    """Exact arithmetic on the declared static rectangles; no product solver."""
    parts = {p['id']: p for p in job['parts']}
    held = job['steps'][0]['motion']['data']
    if held['part_id'] not in parts:
        return 'InvalidInputUnknownPart', None
    before = deepcopy(parts)
    target = parts[held['part_id']]
    requested = translation(held['pose'])
    current = translation(target['placement'])
    epsilon = number(job['tolerance']['distance_mm'])
    if sum((a-b)**2 for a, b in zip(requested, current)) > epsilon**2:
        assert parts == before
        return 'FailPoseMismatch', {'root_translation_error_squared_mm2': str(sum((a-b)**2 for a, b in zip(requested, current)))}
    boxes = {}
    panels = {p['id']: p for p in job['panels']}
    for panel in panels.values():
        world = tuple(a+b for a, b in zip(
            translation(parts[panel['part_id']]['placement']),
            translation(panel['flat_pose'])))
        xs = {number(v['x']) for v in panel['polygon_mm']}
        ys = {number(v['y']) for v in panel['polygon_mm']}
        assert len(xs) == len(ys) == 2 and len(panel['polygon_mm']) == 4
        assert {(number(v['x']), number(v['y'])) for v in panel['polygon_mm']} == {(x,y) for x in xs for y in ys}
        half = number(panel['thickness_mm']) / 2
        boxes[panel['id']] = ((min(xs)+world[0], max(xs)+world[0]),
                              (min(ys)+world[1], max(ys)+world[1]),
                              (world[2]-half, world[2]+half))
    assert job['conditions']['allowed_contacts'] == []
    pairs = list(boxes.items())
    for i, (aid, a) in enumerate(pairs):
        for bid, b in pairs[i+1:]:
            depths = [min(x[1], y[1])-max(x[0], y[0]) for x, y in zip(a, b)]
            if all(d > epsilon for d in depths):
                assert parts == before
                return 'FailStationaryCollision', {'panels': [aid, bid], 'overlap_depths_mm': [str(d) for d in depths], 'interval': [0, 1], 'basis': 'all poses constant over entire hold interval'}
    for connection in job['connections']:
        if connection['engage_after_step_id'] is not None:
            continue  # No deferred-activation cases in this finite reference.
        points = []
        for suffix in ['a', 'b']:
            panel = panels[connection['panel_'+suffix]]
            points.append(tuple(a+b+c for a,b,c in zip(
                translation(parts[panel['part_id']]['placement']),
                translation(panel['flat_pose']),
                translation(connection['frame_'+suffix]))))
        squared = sum((a-b)**2 for a,b in zip(*points))
        if squared > number(job['tolerance']['constraint_mm'])**2:
            assert parts == before
            return 'FailActiveConnectionClosure', {'connection_id': connection['id'], 'closure_error_squared_mm2': str(squared), 'interval': [0, 1]}
    assert parts == before
    return 'FixedHoldConditionsSatisfied', {'selected_part': held['part_id'], 'all_part_root_poses_unchanged': True, 'required_checks': ['all_pairs_collision', 'already_active_connection_closure'], 'not_a_complete_PackagingResult_Pass': True}


def main():
    design = read(ROOT/'fixed-pose-cases.design.json')
    native = read(ROOT/'output/fixed-pose-native-result.json')
    assert native['status'] == 'passed_actual_canonical_fixed_pose_parse_and_roundtrip'
    validators = [Draft202012Validator(read(ROOT/'output/schemas/packaging-job.schema.json')),
                  Draft202012Validator(read(ROOT/'output/api-schemas/compute_packaging.request.schema.json')),
                  Draft202012Validator(read(ROOT/'output/api-schemas/apply_commands.request.schema.json'))]
    rows = []
    for case, actual in zip(design['cases'], native['cases'], strict=True):
        assert case['id'] == actual['id']
        accepted = [not list(v.iter_errors(case[k])) for v, k in zip(validators, ['job','compute_request','edit_request'])]
        assert accepted == [case['expected_shape_accepted']]*3
        assert accepted == [actual[k] for k in ['job_accepted','compute_accepted','put_job_accepted']]
        outcome, witness = reference(actual['job_roundtrip']['data']) if accepted[0] else ('RejectedShape', None)
        assert outcome == case['expected_reference_outcome'], (case['id'], outcome)
        rows.append({'id':case['id'], 'shape_accepted':accepted[0], 'expected':case['expected_reference_outcome'], 'actual_finite_reference':outcome, 'witness':witness})
    result = {'status':'passed_actual_fixed_pose_schema_and_finite_reference_checks',
              'canonical_source_ref':'packages/domain/types.design.rs', 'shape_checks':len(rows)*3,
              'cases':rows, 'limits':design['limits'], 'actual_product_runtime':None,
              'full_continuous_packaging_geometry_proved':False, 'existing_independent_expectations_changed':False,
              'errors':[]}
    (ROOT/'output/fixed-pose-contract-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:result[k] for k in ['status','shape_checks','errors']},ensure_ascii=False))


if __name__ == '__main__':
    main()
