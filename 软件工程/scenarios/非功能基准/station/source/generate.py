"""Rebuild the B60 1.0.5 mounting repair without changing its Engine rules.

The preserved 1.0.1 documents are inputs, never overwritten. The native-geometry
intermediate is separate from every old exact candidate and the final 1.0.5 GLB.
Both remain candidates until the original MQ and Native gates pass.
"""
from copy import deepcopy
from pathlib import Path
import argparse
import json


HERE = Path(__file__).resolve().parent
VERSION = '1.0.5'


def presentation(value):
    if isinstance(value, dict):
        if 'nodes' in value and 'coordinate_frame' in value:
            return value
        for child in value.values():
            found = presentation(child)
            if found is not None:
                return found
    elif isinstance(value, list):
        for child in value:
            found = presentation(child)
            if found is not None:
                return found
    return None


def pose(x=0, y=0, z=0, sz=1):
    return dict(translation=dict(x=x, y=y, z=z),
                rotation=dict(x=0, y=0, z=0, w=1),
                scale=dict(x=1, y=1, z=sz), unit='meter',
                coordinate_frame='right_handed_z_up')


def build():
    detailed = json.loads((HERE / 'station-1.0.1.seed.json').read_text('utf-8-sig'))
    simple = json.loads((HERE / 'simple-1.0.1.seed.json').read_text('utf-8-sig'))
    p = presentation(detailed)
    nodes = p['nodes']
    named = {node['name']: node for node in nodes}
    metal = deepcopy(named['welded_base_bottom']['material'])
    white = deepcopy(named['rear_upper_left_panel']['material'])
    dark = deepcopy(named['cable_trunk']['material'])
    blue = deepcopy(named['cycle_ram_carriage']['material'])
    glass = deepcopy(named['clear_safety_door_-0.43']['material'])

    def add(name, size, position, material=metal, parent=None, kind='box'):
        index = len(nodes)
        data = dict(zip(('x', 'y', 'z'), size)) if kind == 'box' else dict(radius=size[0], height=size[1])
        nodes.append(dict(id=index, name=name, parent_index=parent,
                          geometry=dict(kind=kind, data=data),
                          material=deepcopy(material), transform=pose(*position)))
        return index

    # Keep the two-second cycle, original physical frame, item support, ports,
    # and camera envelope. Reach the existing workpiece at Y=0 with an actual
    # cantilever; the guided carriage remains on the rear guide axes at Y=.5.
    carriage = named['cycle_ram_carriage']['id']
    for name in ('ram_tool_shaft', 'ram_tool_head'):
        named[name]['transform']['translation']['y'] = -.5
    add('ram_cantilever', (.62, .74, .14), (0, -.25, 0), blue, carriage)
    for x in (-.36, .36):
        bearing = named[f'guide_bearing_{x}']
        bearing['parent_index'] = carriage
        bearing['transform'] = pose(x, 0, .01)
    for z in (1.5, 2.4):
        add(f'guide_end_crossmember_{z}', (1.72, .16, .12), (0, .5, z), blue)
        for x in (-.36, .36):
            add(f'guide_end_socket_{x}_{z}', (.11, .15, .10), (x, .5, z))

    # The fixed actuator housing joins the crossmember. Its extending rod is
    # driven by the same stage-local keys as the carriage, not by wall time.
    add('ram_actuator_housing', (.18, .19, .24), (0, .5, 2.31), white)
    add('ram_actuator_mount', (.35, .24, .06), (0, .5, 2.43), metal)
    rod = add('ram_actuator_rod', (.035, .07), (0, .5, 2.155), metal, kind='cylinder')
    keys = []
    for key in p['actions'][0]['keys']:
        lower = key['pose']['translation']['z'] + .07
        length = 2.19 - lower
        keys.append(dict(at_progress=key['at_progress'],
                         pose=pose(0, .5, (2.19 + lower) / 2, length / .07)))
    p['actions'].append(dict(id='actual_actuator_extension', node_index=rod,
                             transition='linear', keys=keys))

    # Retain the original fixture top1.215m and item offset.075m: the original
    # .1m-high workpiece rests on the fixture and meets the ram at full stroke.
    for x in (-.105, .105):
        add(f'fixture_x_locator_{x}', (.03, .18, .06), (x, 0, 1.245), dark)
    for y in (-.105, .105):
        add(f'fixture_y_locator_{y}', (.18, .03, .06), (0, y, 1.245), dark)

    # D04: fix the fixture to a crossmember tied into both base rails. The
    # narrow pedestal fits between roller_4/5, retaining the 1.215m support.
    add('fixture_fixed_crossmember', (1.64, .045, .06), (0, 0, 1.07))
    add('fixture_fixed_pedestal', (.26, .045, .08), (0, 0, 1.13))
    for x in (-.125, .125):
        add(f'fixture_base_fixing_stud_{x}', (.003, .05), (x, 0, 1.19), metal, kind='cylinder')
        add(f'fixture_base_fixing_head_{x}', (.008, .008), (x, 0, 1.217), dark, kind='cylinder')

    # Real supports around the existing transparent guards, panel and handles.
    for x in (-.86, .86):
        add(f'front_guard_post_{x}', (.08, .08, 1.4), (x, -.86, 1.85), metal)
        # D01: mounting feet bridge the existing 20/50mm lower gaps and join
        # each upper post to a structural base rail, with unchanged post tops.
        add(f'front_guard_base_mount_{x}', (.16, .18, .08), (x, -.82, 1.13))
        add(f'gantry_base_mount_{x}', (.18, .22, .08), (x, .52, 1.13))
        add(f'side_lower_cover_{x}', (.06, 1.62, .9), (x, 0, .68), white)
        add(f'side_upper_guard_{x}', (.025, 1.55, 1.04), (x, 0, 1.96), glass)
        for z in (1.44, 2.5):
            add(f'side_guard_rail_{x}_{z}', (.08, 1.72, .06), (x, 0, z), metal)
    for z in (1.44, 2.5):
        add(f'front_guard_crossmember_{z}', (1.8, .08, .06), (0, -.86, z), metal)
    add('front_guard_middle_mullion', (.025, .05, 1.06), (0, -.85, 1.97), metal)
    for x in (-.1032, .1032):
        for z in (1.77, 1.93):
            add(f'door_handle_foot_{x}_{z}', (.035, .045, .04), (x, -.873, z), metal)
    # Rear louvers now terminate in a frame connected to both rear panels.
    for x in (-.395, .395):
        add(f'rear_louver_frame_{x}', (.09, .10, 1.2), (x, .92, 1.88), white)
    for z in (1.3, 2.46):
        add(f'rear_louver_crossmember_{z}', (.88, .10, .08), (0, .92, z), white)
    add('control_console_bracket', (.26, .12, .10), (-.73, -.89, 1.55), metal)
    add('motor_mount_pedestal', (.11, .25, .36), (.86, .1, .41), metal)
    add('drive_right_angle_gearbox', (.22, .23, .20), (.84, .1, 1.03), metal)
    for y in (-.71, .71):
        named[f'photo_sensor_window_{y}']['transform']['translation']['x'] = -.615
        add(f'sensor_mount_{y}', (.10, .10, .05), (-.66, y, 1.32), metal)
        # D03: retain the lower end at .880m; extend through the mount into
        # the sensor underside at 1.320m, with a 5mm seated cable end.
        add(f'sensor_cable_{y}', (.018, .018, .445), (-.69, y, 1.1025), dark)
    # D02: the tower stem is behind the crossbeam; its foot needs a real
    # cantilevered mounting plate, not just the collar above the stem.
    add('stack_light_mount_bracket', (.12, .22, .045), (.72, .64, 2.59))
    add('lamp_stem_collar', (.03, .02), (.72, .68, 2.755), dark, kind='cylinder')
    for z in (2.8275, 2.9025):
        add(f'lamp_separator_{z}', (.045, .01), (.72, .68, z), dark, kind='cylinder')

    # Fasteners connect the declared panels/frame rather than decorate space.
    for x in (-.86, .86):
        for y in (-.86, .52):
            for z in (1.47, 2.47):
                add(f'frame_fastener_{x}_{y}_{z}', (.032, .035), (x, y, z), metal, kind='cylinder')
    for x in (-.36, .36):
        for z in (-.045, .065):
            add(f'carriage_bearing_fastener_{x}_{z}', (.018, .025), (x, -.075, z), metal, carriage, kind='cylinder')

    # The lightweight LOD keeps its existing recognition, ports and workpiece
    # anchor. It is a new exact version too, never a replacement detailed mesh.
    for document in (detailed, simple):
        document['data']['definition']['version'] = VERSION
    detailed['data']['dependencies'][0]['version'] = VERSION
    p['lod']['simplified']['data']['version'] = VERSION
    return {'station.module-document.json': detailed, 'simple.module-document.json': simple}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=HERE.parent / 'candidate-1.0.5-native')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    for name, document in build().items():
        (args.output / name).write_text(json.dumps(document, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print('Rebuilt 1.0.5 native-geometry intermediate; final GLB and MQ/Native acceptance still required.')
