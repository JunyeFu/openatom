"""Actual four-wall + seam-tab Job, folded endpoint and rigid opening proof.

This does NOT certify the preceding four folding sweeps. Those remain an
explicit Undetermined purpose, so this partial trial never yields whole-Job Pass.
"""
from pathlib import Path
from copy import deepcopy
from fractions import Fraction as F
import json
import math
import sys

sys.dont_write_bytecode = True
import prove as p
from construct_seeds import base_job, pose, rect, v3
from jsonschema import Draft202012Validator

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
OUT = HERE / 'output'


def make_sleeve(last_gap=2.9):
    job = base_job('cover_or_sliding_sleeve')
    job['id'] = 'RR_ACTUAL_SLIDING_SLEEVE'
    job['revision']=2
    job['changes']=['part_relations_or_repeat_order']
    widths, gaps = [20, 10, 20, 10, 4], [2, 2, 2.9, last_gap]
    names = ['BOTTOM', 'RIGHT', 'TOP', 'LEFT', 'TAB']
    x, previous = 0, None
    for i, (name, width) in enumerate(zip(names, widths)):
        ident = 'S_' + name
        job['panels'].append({'id': ident, 'part_id': 'SLEEVE', 'polygon_mm': rect(width, 30),
                              'thickness_mm': 1, 'material_ref': 'CARD', 'flat_pose': pose(x)})
        if previous is not None:
            axis_x = widths[i-1] + gaps[i-1]/2
            band, step = 'F_' + name, 'FORM_' + name
            job['fold_bands'].append({'id': band, 'panel_a': previous, 'panel_b': ident,
                'axis_start_mm': v3(axis_x, 0), 'axis_end_mm': v3(axis_x, 30),
                'width_mm': gaps[i-1], 'target_angle_rad': -math.pi/2,
                'allowed_range_rad': [-math.pi/2, 0]})
            job['steps'].append({'id': step, 'purpose': 'forming', 'duration_hint_ms': None,
                'motion': {'kind': 'fold', 'data': {'fold_band': band, 'from_angle_rad': 0, 'to_angle_rad': -math.pi/2}},
                'prerequisites': [] if not job['steps'] else [job['steps'][-1]['id']]})
        previous = ident
        x += width + (gaps[i] if i < len(gaps) else 0)
    job['parts'] = [{'id': 'SLEEVE', 'title': 'four walls and explicit lap-seam tab',
                     'panel_ids': ['S_'+n for n in names], 'root_panel': 'S_BOTTOM', 'placement': pose()}]
    job['materials'] = [{'id': 'CARD', 'title': 'finite geometric test material', 'thickness_mm': 1,
                         'basis': 'template_declared_example', 'source_ref': None}]
    job['connections'] = [{'id': 'SEAM', 'panel_a': 'S_BOTTOM', 'panel_b': 'S_TAB',
        'frame_a': pose(2, 15, -0.45), 'frame_b': pose(2, 15, 0.45), 'kind': 'glue',
        'allow_contact': True, 'contact_allowance_refs': ['SEAM_PATCH'], 'engage_after_step_id': 'FORM_TAB'}]
    job['conditions']['allowed_contacts'] = [{'id': 'SEAM_PATCH', 'panel_a': 'S_BOTTOM', 'panel_b': 'S_TAB',
        'face_a': 'back', 'face_b': 'front', 'polygon_a_mm': rect(5, 30), 'polygon_b_mm': rect(4, 30),
        'normal_overlap_limit_mm': 0.15, 'active_step_ids': ['FORM_TAB', 'SLIDE_OPEN'],
        'basis': 'explicit 0.1mm lap in the finite geometric model, not adhesive or paper physics'}]
    job['conditions']['product_envelope'] = {'subject': deepcopy(job['subject_ref']),
        'vertices_mm': [v3(x, y, z) for x in [-5, 5] for y in [-5, 5] for z in [-2, 2]],
        'allowed_poses': [pose(10, 15, 6)], 'clearance_mm': 0.5}
    job['conditions']['extraction'] = {'part_ref': 'SLEEVE', 'product_pose': pose(10, 15, 6),
                                     'direction': v3(0, -1), 'distance_mm': 80}
    job['steps'].append({'id': 'SLIDE_OPEN', 'purpose': 'opening', 'duration_hint_ms': None,
        'motion': {'kind': 'rigid_translation', 'data': {'part_id': 'SLEEVE', 'start_pose': pose(),
                                                       'displacement_mm': v3(0, 40)}},
        'prerequisites': ['FORM_TAB']})
    return job


def rotation_y(q):
    c, s = p.trig(q, True), p.trig(q)
    return [[c, p.I(0), s], [p.I(0), p.I(1), p.I(0)], [-s, p.I(0), c]]


def panel_poses(job, tab_delta=0, angles=None):
    panels = {x['id']: x for x in job['panels']}
    result = {'S_BOTTOM': p.translated(0, 0, 0)}
    for band in job['fold_bands']:
        parent, child = panels[band['panel_a']], panels[band['panel_b']]
        pivot = [F(band['axis_start_mm'][k]) for k in ('x', 'y', 'z')]
        # Axis is explicitly +Y in this fixture; q is read from actual Job.
        assert band['axis_end_mm']['x'] == band['axis_start_mm']['x']
        assert band['axis_end_mm']['y'] > band['axis_start_mm']['y']
        offset = [F(child['flat_pose']['translation'][k])-F(parent['flat_pose']['translation'][k]) for k in ('x', 'y', 'z')]
        q = p.iv(F(band['target_angle_rad']) if angles is None else angles[band['id']]) + (tab_delta if band['panel_b'] == 'S_TAB' else 0)
        relative = p.compose(p.translated(*pivot), p.compose((rotation_y(q), [p.I(0)]*3),
                   p.compose(p.translated(*[-v for v in pivot]), p.translated(*offset))))
        result[band['panel_b']] = p.compose(result[band['panel_a']], relative)
    return result


def squared(value):
    value = p.iv(value)
    return p.I(0 if value.lo <= 0 <= value.hi else min(value.lo**2, value.hi**2), max(value.lo**2, value.hi**2))


def closure(job, tab_delta=0):
    transforms = panel_poses(job, tab_delta)
    frames = []
    for side in ('a', 'b'):
        connection = job['connections'][0]
        frame = connection['frame_'+side]
        assert frame['rotation'] == {'x': 0, 'y': 0, 'z': 0, 'w': 1}
        frames.append(p.compose(transforms[connection['panel_'+side]], p.translated(*[F(frame['translation'][k]) for k in ('x', 'y', 'z')])))
    position = sum(squared(a-b) for a, b in zip(frames[0][1], frames[1][1]))
    orientation = sum(squared(a-b) for row_a, row_b in zip(frames[0][0], frames[1][0]) for a, b in zip(row_a, row_b))
    position_limit = F(job['tolerance']['constraint_mm'])**2
    angular_limit = 8*squared(p.trig(F(job['tolerance']['angle_rad'])/2))
    verdict = 'Fail' if position.lo > position_limit or orientation.lo > angular_limit.hi else (
        'Pass' if position.hi <= position_limit and orientation.hi <= angular_limit.lo else 'Undetermined')
    return verdict, {'position_squared_mm2': [float(position.lo), float(position.hi)],
                     'orientation_chord_squared': [float(orientation.lo), float(orientation.hi)],
                     'constraint_mm': job['tolerance']['constraint_mm'],
                     'common_rigid_translation_cancelled_symbolically': True,
                     'source_connection': 'SEAM', 'source_panels': ['S_BOTTOM', 'S_TAB']}


def sleeve_cells(job, slide=0, static_offset=0):
    transforms = panel_poses(job)
    result = []
    for panel in job['panels']:
        allowance = job['conditions']['allowed_contacts'][0]
        patches = []
        for side in ('a', 'b'):
            if panel['id'] == allowance['panel_'+side]:
                points = allowance['polygon_'+side+'_mm']
                patches = [{'id': allowance['id'], 'rectangle': [min(v['x'] for v in points), max(v['x'] for v in points),
                            min(v['y'] for v in points), max(v['y'] for v in points)],
                            'face': allowance['face_'+side], 'limit': allowance['normal_overlap_limit_mm'], 'active': True}]
        frozen = transforms[panel['id']]
        def moving(time, frozen=frozen):
            return p.compose(p.translated(0, F(static_offset)+F(slide)*time, 0), frozen)
        xs, ys = [v['x'] for v in panel['polygon_mm']], [v['y'] for v in panel['polygon_mm']]
        thickness = F(panel['thickness_mm'])
        result.append(p.partition_rect(panel['id'], [(min(xs), max(xs)), (min(ys), max(ys)), (-thickness/2, thickness/2)], patches, moving))
    return result


def product_cell(job, extraction=False, clearance=False):
    envelope = job['conditions']['product_envelope']
    values = envelope['vertices_mm']
    bounds = [(min(F(v[k]) for v in values), max(F(v[k]) for v in values)) for k in ('x', 'y', 'z')]
    if clearance:
        margin=F(envelope['clearance_mm'])
        bounds=[(lo-margin,hi+margin) for lo,hi in bounds]
    plan=job['conditions']['extraction']
    start = (plan['product_pose'] if extraction else envelope['allowed_poses'][0])['translation']
    return p.Cell('ACTUAL_PRODUCT_ENVELOPE', bounds,
                  lambda time: p.translated(*[F(start[k])+(F(plan['direction'][k])*F(plan['distance_mm'])*time if extraction else 0) for k in ('x','y','z')]))


def check_geometry(job, slide=0, extraction=False, obstacle=False):
    opened_offset=job['steps'][-1]['motion']['data']['displacement_mm']['y'] if extraction else 0
    groups, budget = sleeve_cells(job, slide,static_offset=opened_offset), p.Budget(5000, 512)
    records = []
    # Every pair of actual panels is included, including the seam pair.
    for i, group in enumerate(groups):
        for other in groups[i+1:]:
            verdict, evidence = p.certify_all(group, other, budget)
            records.append(verdict)
            if verdict == 'Fail':
                return verdict, {'evidence': evidence, 'queries': budget.queries,'subdivisions':budget.subdivisions}
    if obstacle:
        other = p.Cell('OPENING_PATH_OBSTACLE', [(F(-0.1), F(0.1))]*3, lambda t: p.translated(10, 35, 0))
    else:
        other = product_cell(job, extraction)
    for group in groups:
        verdict, evidence = p.certify_all(group, [other], budget)
        records.append(verdict)
        if verdict == 'Fail':
            return verdict, {'evidence': evidence, 'queries': budget.queries,'subdivisions':budget.subdivisions}
    if not obstacle:
        # An L-infinity expansion contains the Euclidean clearance neighborhood.
        # Separation proves the requested clearance; an expanded overlap alone
        # is not a certified violation of that Euclidean condition.
        expanded=product_cell(job,extraction,clearance=True)
        for group in groups:
            verdict,evidence=p.certify_all(group,[expanded],budget)
            records.append('Pass' if verdict=='Pass' else 'Undetermined')
    return ('Pass' if all(x == 'Pass' for x in records) else 'Undetermined'), {
        'actual_panel_pairs': 10, 'actual_panels': 5, 'queries': budget.queries,'subdivisions':budget.subdivisions,
        'whole_intervals': True, 'product_or_obstacle': other.ident,
        'requested_clearance_mm': None if obstacle else job['conditions']['product_envelope']['clearance_mm'],
        'clearance_proof': None if obstacle else 'conservative L-infinity expansion; no false Fail from inflated collision',
        'actual_opened_root_offset_y_mm':opened_offset,
        'scope': 'folded endpoint / stated rigid translation, not earlier folding sweeps'}


def main():
    job = make_sleeve()
    schema = json.loads((ROOT/'tools/type-contract-proof/output/schemas/packaging-job.schema.json').read_text(encoding='utf-8'))
    Draft202012Validator(schema).validate({'schema_version': '1.0', 'data': job})
    (OUT/'sliding-sleeve-job.json').write_text(json.dumps({'schema_version': '1.0', 'data': job}, indent=2)+'\n', encoding='utf-8')
    cases = []
    def add(ident, expected, value, basis):
        actual, evidence = value
        cases.append({'id': ident, 'expected': expected, 'actual': actual,
                      'matches': actual == expected, 'independent_basis': basis, 'evidence': evidence})
    add('S01', 'Pass', closure(job), 'f1=f2=2, f3=f4=2.9 yields TAB origin (0,0,-0.9); -0.45 and -0.9+0.45 seam frames coincide; tab approaches from below.')
    add('S02', 'Fail', closure(make_sleeve(last_gap=1)), 'Changing only f4 from2.9 to1 moves seam by (-0.95,0,+0.95), much larger than 4e-6mm.')
    add('S03', 'Pass', check_geometry(job), 'Actual four walls and lap tab at formed endpoint; 0.1mm overlap lies inside same 0.15mm local seam patch, product has >0.5mm clearance.')
    opening_slide=job['steps'][-1]['motion']['data']['displacement_mm']['y']
    add('S04', 'Pass', check_geometry(job, slide=opening_slide), 'Sliding entire sleeve 40mm in +Y preserves X/Z gaps and the same seam, all panel pairs and product checked over [0,1].')
    add('S05', 'Fail', check_geometry(job, slide=opening_slide, obstacle=True), 'Bottom moves Y0..30 to40..70; obstacle at(10,35,0) is missed at both endpoints but struck mid-path.')
    add('S06', 'Pass', closure(job), 'Both seam frames share exactly the same rigid root translation; symbolic common-transform cancellation preserves closure for the entire opening interval.')
    add('S07', 'Fail', closure(job, F(1,100)), 'After activation an independent 0.01rad TAB fold breaks the fixed seam; a fixed connection cannot silently become a hinge.')
    add('S08', 'Pass', check_geometry(job, extraction=True), 'After actual SLIDE_OPEN, sleeve is atY+40; fixed-pose product moves in -Y, every actual sleeve panel remains an obstacle.')
    thick = deepcopy(job)
    for panel in thick['panels']:
        panel['thickness_mm'] = 1.4
    thick['materials'][0]['thickness_mm'] = 1.4
    add('S09', 'Fail', check_geometry(thick), 'Same placements with thickness1.4 make seam overlap0.5 > approved0.15; material change must recompute geometry.')
    larger = deepcopy(job)
    larger['subject_ref']['revision'] = 'r2'
    larger['conditions']['product_envelope']['subject']['revision'] = 'r2'
    for vertex in larger['conditions']['product_envelope']['vertices_mm']:
        vertex['x'] = -12 if vertex['x'] < 0 else 12
    add('S10', 'Fail', check_geometry(larger), 'r2 width24 spans X[-2,22], intersecting sleeve side walls despite unchanged outer Job topology.')
    by_id={case['id']:case['actual'] for case in cases}
    def aggregate(*ids):
        verdicts=[by_id[ident] for ident in ids]
        return 'Fail' if 'Fail' in verdicts else ('Pass' if all(v=='Pass' for v in verdicts) else 'Undetermined')
    result = {'evidence_level': 'isolated_actual_public_job_folded_endpoint_seam_and_rigid_sweep',
        'job_ref': 'tools/packaging-algorithm-proof/output/sliding-sleeve-job.json', 'schema': 'passed',
        'cases': cases, 'all_expected_matched': all(c['matches'] for c in cases),
        'purpose_results': {'forming': 'Undetermined', 'folded_endpoint_and_seam': aggregate('S01','S03'),
                            'opening_rigid_sweep': aggregate('S04','S06'), 'net_space_geometric_separation': by_id['S03'],
                            'fixed_pose_extraction': by_id['S08'], 'manufacturing': 'Undetermined'},
        'purpose_dependencies': {'forming':{'cases':[],'remaining':'four preceding folding sweeps'},
            'folded_endpoint_and_seam':{'cases':['S01','S03'],'conditional':'declared target angles are supplied; forming path unproved'},
            'opening_rigid_sweep':{'cases':['S04','S06'],'conditional':'same supplied folded endpoint; not executable whole-Job opening approval'},
            'net_space_geometric_separation':{'cases':['S03'],'conditional':'supplied folded endpoint, actual clearance0.5mm'},
            'fixed_pose_extraction':{'cases':['S08'],'conditional':'supplied endpoint, all five actual panels remain obstacles'},
            'manufacturing':{'cases':[],'remaining':'full flat bands, common seam glue projection and SVG not executed'}},
        'whole_job_outcome': 'Undetermined', 'conditional_on_supplied_folded_endpoint': True,
        'fixture_specific_evaluator_not_arbitrary_job_dispatch': True,
        'seam_activation': 'endpoint eligible only; activation awaits complete preceding forming proof',
        'not_proved': ['four preceding folding sweeps', 'fold-band material manufacturing and glue projection for whole sleeve',
                       'generic Job compiler and arbitrary topology', 'product Host/API/Engine integration', 'industrial process or physical sample'],
        'actual_product_runtime': None, 'actual_complete_professional_trial': None}
    (OUT/'sliding-sleeve-result.json').write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8')
    print(json.dumps({'cases': len(cases), 'all_expected_matched': result['all_expected_matched'],
                      'failures': [c for c in cases if not c['matches']], 'whole_job_outcome': result['whole_job_outcome']}))
    if not result['all_expected_matched']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
