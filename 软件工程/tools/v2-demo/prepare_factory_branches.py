"""Design the remaining exact-specification production branches before adoption.

Equipment geometry and declared conversions remain unchanged. Each branch gains
real receiving, transport, final-stock and order consumers in the common factory.
Different product specifications are never silently relabelled as compatible.
"""
import argparse
import copy
from collections import Counter
from pathlib import Path
import xml.etree.ElementTree as ET
from prepare_full_factory import DESIGN, ART, Drawing, read, write


def design(source):
    project = read(source / 'actual-frozen-project.json')
    scenario = read(source / 'actual-scenario.json')
    routes = {r['id']: r for r in project['routes']}
    devices = {d['id']: d for d in project['devices']}
    layout = read(DESIGN / 'layout.json')
    zones = {d: z for z in layout['zones'] for d in z['device_ids']}
    connected = set()
    rows = []
    services = {'air-table-loader', 'paper-jogger', 'pile-turner', 'suction-separator',
                'carton-counter-stacker', 'curved-conveyor', 'gantry-pick-place',
                'lift-conveyor', 'pile-lift', 'scara-robot', 'six-axis-robot'}
    for task in scenario['production_tasks']:
        route = routes[task['routing_ref']['id']]
        if len(route['steps']) > 1:
            connected.update(step['device_id'] for step in route['steps'])
            continue
        step = route['steps'][0]
        device = devices[step['device_id']]
        slug = device['id'].removeprefix('PF-')
        zone = zones[device['id']]
        rows.append({'device_id': device['id'], 'title': device['title'],
                     'zone_id': zone['id'], 'zone_title': zone['title'],
                     'task_id': task['id'], 'route_id': route['id'],
                     'role': 'material_handling_service' if slug in services else 'recycling' if slug == 'paper-waste-baler' else 'production_branch',
                     'input_type': step['item_bindings']['input'], 'output_type': step['item_bindings']['output'],
                     'flow': 'receiving -> actual equipment -> transport -> segregated dispatch -> order acceptance',
                     'specification': 'Original exact input/output types and material quantities; no inferred cross-device compatibility'})
    assert len(rows) == 61 and len(connected) == 31
    result = {'kind': 'factory_remaining_branch_design', 'target_template': '1.7.1',
              'source': str(source), 'hall_geometry_changed': False, 'equipment_positions_changed': False,
              'AI_views_reused': True, 'AI_reuse_reason': 'Same hall, 12 zones and all equipment positions; only front-band stock allocation and flow annotations change.',
              'existing_connected_devices': sorted(connected), 'branches': rows,
              'receiving_area_m': {'x_min': 3, 'x_max': 16, 'y_min': 1, 'y_max': 14},
              'dispatch_area_m': {'x_min': 60, 'x_max': 76, 'y_min': 1, 'y_max': 14},
              'recovery_area_m': {'x_min': 95, 'x_max': 101, 'y_min': 1, 'y_max': 14},
              'dispatch_clearance': {'location': 'PF-BRANCH-OUT-carton-counter-stacker',
                                     'slot_pitch_m': .013, 'rendered_carton_thickness_m': .0121,
                                     'reason': '1.7.0 Native observed 0.1 mm adjacent mesh overlap; presentation-only authored spacing change'},
              'acceptance': ['All 92 devices have actual work in one common Run',
                             'Every added branch waits for physical receiving transport and accepts only at dispatch',
                             'Original nested contents, quantities, resource exclusion and scrap are preserved',
                             'One missing-input and one blocked-receiving case retain WIP instead of inventing finished goods',
                             'Actual template adoption, run, save and fresh-process reopen; changed stocks and cargo visually checked'],
              'verified': False}
    write(DESIGN / 'remaining-branches-spec.json', result)
    path = ART / '纸包装整厂区域与设备分配.drawio'
    dr = Drawing(); dr.file = ET.parse(path).getroot(); dr.counter = 15000
    for page in list(dr.file):
        if page.get('id', '').startswith('remaining-branches-'): dr.file.remove(page)
    for index, zone in enumerate(layout['zones']):
        subset = [row for row in rows if row['zone_id'] == zone['id']]
        if not subset: continue
        dr.page(f'{11+index:02d} {zone["title"]} · 支线与配送', 1800, max(800, 270+len(subset)*100))
        dr.file[-1].set('id', 'remaining-branches-'+zone['id'])
        dr.box(zone['title']+'｜按原规格组织生产与配送', 50, 25, 1700, 60, font=28, style='strokeColor=none;fontStyle=1;')
        dr.box('沿用原设备位置；前区备料 x3–16 / 发运 x60–76 / 回收 x95–101，y1–14。\n不同规格分支独立接单，共享厂内车队；不能把搬运件数写成制造产量。', 50, 100, 1700, 90, font=20, style='strokeColor=none;align=left;')
        for i, row in enumerate(subset):
            y = 220+i*100
            a=dr.box('接收与备料', 50, y, 240, 75, '#edf3f8', 19)
            b=dr.box(row['title']+'\n'+row['device_id'], 400, y, 700, 75, zone['color'], 18)
            c=dr.box('分区发运 · 对应订单', 1260, y, 440, 75, '#dcebd9', 19)
            dr.edge(a,b,'原规格物料'); dr.edge(b,c,'实际产出 / 搬运完成')
        for cell in dr.root.findall('mxCell'):
            if cell.get('value'): cell.set('value',cell.get('value').replace('\n','<br>'))
            if cell.get('style'): cell.set('style',cell.get('style').replace('html=0','html=1'))
    dr.save(path)
    print('Designed 61 explicit branches; retained 31 connected devices and unchanged equipment layout.')


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',type=Path,required=True)
    design(parser.parse_args().source)
