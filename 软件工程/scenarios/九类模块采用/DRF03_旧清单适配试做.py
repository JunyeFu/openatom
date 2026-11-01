"""读取真实九类旧清单，转换有定义的字段；只写隔离试做材料，不是产品导入器。"""
from copy import deepcopy
from decimal import Decimal
from fractions import Fraction
from pathlib import Path
import json
import sys

sys.dont_write_bytecode = True
BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[1]
OUT = BASE / 'adapter-output'
sys.path.insert(0, str(ROOT / 'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False)+'\n', encoding='utf-8')


def amount(value, unit):
    decimal = Decimal(str(value))
    tup = decimal.as_tuple()
    coefficient = int(''.join(str(x) for x in tup.digits)) * (-1 if tup.sign else 1)
    if tup.exponent > 0:
        coefficient *= 10**tup.exponent
    return {'value': {'coefficient': coefficient, 'decimal_places': max(0, -tup.exponent)}, 'unit': unit}


def duration(source, basis):
    value = Fraction(str(source['value']))
    if source['unit'] == 'second':
        value *= 1000
    else:
        assert source['unit'] == 'millisecond'
    if value.denominator != 1 or value < 0 or value > 2**53 - 1:
        raise ValueError('duration_not_representable_as_integer_ms')
    return {'value': int(value), 'unit': 'millisecond', 'basis': basis, 'source_ref': None}


def pose(position, unit, frame):
    return {'translation': dict(zip(['x', 'y', 'z'], position)), 'rotation': {'x': 0, 'y': 0, 'z': 0, 'w': 1},
            'scale': {'x': 1, 'y': 1, 'z': 1}, 'unit': unit, 'coordinate_frame': frame}


class Adapter:
    def __init__(self, source_path, profile):
        self.path, self.profile = source_path, profile
        self.source = read(source_path)
        self.files, self.assets, self.rights, self.issues = [], [], [], []
        self.archives = {}

    def issue(self, code, path, message, blocks):
        self.issues.append({'code': code, 'field_path': path, 'message': message, 'blocks': blocks, 'object_ref': None})

    def resource(self, row):
        return {'role': row['role'], 'quantity': amount(row['quantity'], self.profile['resource_units'][row['role']]), 'qualifications': []}

    def attribute(self, field):
        return {'name': field['name'], 'value_type': field['type'], 'unit': field.get('unit'),
                'shape': [field['shape']] if 'shape' in field else None, 'options': field.get('options', []), 'basis': self.profile['source_basis']}

    def rights_for(self, path, source=None, license_text=None):
        asset_id = 'source:' + path
        asset = {'id': asset_id, 'path': path, 'kind': 'json'}
        right = {'asset_ref': asset_id, 'source': source, 'license': license_text, 'redistribution_confirmed': False, 'missing_reason': 'legacy contribution rights not confirmed'}
        self.assets.append(asset)
        self.rights.append(right)
        self.files.append({'path': path, 'kind': 'asset'})
        return asset

    def presentation(self, source):
        if 'media' in source:
            locator = source['media']
        else:
            locator = source
        if locator.get('path') is None:
            self.issue('missing_media_path', '/definition/payload/data/presentation', 'No actual source media path; optional presentation remains null.', ['preview'])
            return None
        assert locator['kind'] == 'native_scene'
        local = self.path.parent / locator['path']
        assert local.resolve().is_relative_to(self.path.parent.resolve())
        raw = read(local)
        archive_path = 'sources/' + locator['path']
        self.archives[archive_path] = raw
        self.rights_for(archive_path, 'actual legacy native-scene JSON', None)
        frame, unit = raw['coordinate_system'], raw['unit']
        nodes = []
        index = {node['id']: i for i, node in enumerate(raw['nodes'])}
        assert len(index) == len(raw['nodes'])
        for i, node in enumerate(raw['nodes']):
            if node['primitive'] == 'box':
                geometry = {'kind': 'box', 'data': dict(zip(['x', 'y', 'z'], node['size']))}
            else:
                assert node['primitive'] == 'cylinder'
                geometry = {'kind': 'cylinder', 'data': {'radius': node['radius'], 'height': node['height']}}
            color = node['color'].removeprefix('#')
            nodes.append({'id': i, 'name': node['id'], 'parent_index': None, 'transform': pose(node['position'], unit, frame),
                          'geometry': geometry,
                          'material': {'color_rgba': [int(color[k:k+2], 16)/255 for k in [0, 2, 4]]+[1],
                                       'metallic': self.profile['native_render_choices']['metallic'], 'roughness': self.profile['native_render_choices']['roughness'], 'texture': None}})
        roles, ports = {}, {}
        for key, target, name_key in [('anchor_roles', roles, 'id'), ('port_anchors', ports, 'port')]:
            for anchor in raw.get(key, []):
                child_id = len(nodes)
                nodes.append({'id': child_id, 'name': 'anchor:' + anchor[name_key], 'parent_index': index[anchor['node']],
                              'transform': pose(anchor['local_position'], unit, frame),
                              'geometry': {'kind': 'plane', 'data': self.profile['native_render_choices']['anchor_helper_plane_size']},
                              'material': {'color_rgba': [0, 0, 0, 0], 'metallic': 0, 'roughness': 1, 'texture': None}})
                target[anchor[name_key]] = {'kind': 'scene_node', 'data': child_id}
        named = []
        for source_pose in raw['poses']:
            transforms = []
            for change in source_pose['offsets']:
                node_index = index[change['node']]
                base = raw['nodes'][node_index]['position']
                absolute = [a+b for a, b in zip(base, change['translation'])]
                transforms.append({'node_index': node_index, 'pose': pose(absolute, unit, frame)})
            named.append({'id': source_pose['id'], 'node_transforms': transforms})
        return {'kind': 'native_scene', 'source': {'kind': 'native_scene', 'data': raw['id']}, 'unit': unit, 'coordinate_frame': frame,
                'scale': self.profile['native_render_choices']['scale'], 'nodes': nodes, 'poses': named, 'actions': [], 'timeline': [], 'tags': [],
                'anchor_roles': roles, 'port_anchors': ports, 'lod': {'detailed': None, 'simplified': None, 'region': 'device_icons'}, 'production_effect': 'none'}

    def operation(self, row, ports=None):
        port_map = self.profile['operation_ports'] if ports is None else {p['id']: p['id'] for p in ports}
        typed_ports = {} if ports is None else {p['id']: p for p in ports}
        def material(line):
            if 'role' in line:
                role, port, unit = line['role'], port_map[line['role']], line['unit']
            else:
                port = line['port']
                role, unit = typed_ports[port]['item'], typed_ports[port]['unit']
            return {'role': role, 'port': port, 'quantity': amount(line['quantity'], unit), 'required_quality': []}
        ins, outs = [material(x) for x in row['inputs']], [material(x) for x in row['outputs']]
        identity = self.profile['identity_aliases'].get(row.get('identity_policy'), 'assembly_with_composition' if row['builtin'] == 'assemble' else None)
        assert identity is not None
        return {'builtin': row['builtin'], 'item_roles': list(dict.fromkeys(x['role'] for x in ins+outs)), 'inputs': ins, 'outputs': outs,
                'resource_roles': [self.resource(x) for x in row.get('resource_roles', row.get('resources', []))],
                'duration': duration(row['duration'], self.profile['source_basis']), 'phases': [], 'hold_on_output_wait': [], 'quality_and_routes': None,
                'identity_policy': identity, 'family': None}

    def ports(self, rows):
        return [{'id': p['id'], 'direction': {'in': 'input', 'out': 'output'}[p['direction']],
                 'payload': {'kind': 'material', 'data': {'item_role': p['item'], 'unit': p['unit'], 'quantity_rule': None}}} for p in rows]

    def convert(self):
        src, cat = self.source, self.source['category']
        parameters, attrs = [], []
        if cat == 'items':
            raw = src['item_definition']
            declarations = [self.attribute(x) for x in raw['attribute_schema']]
            values = {}
            for key, val in raw['attributes'].items():
                decl = next(x for x in raw['attribute_schema'] if x['name'] == key)
                assert decl['type'] == 'vector3' and len(val) == 3
                values[key] = {'kind': 'vector3', 'value': dict(zip(['x', 'y', 'z'], val))}
            payload = {'unit': raw['unit'], 'tracking': raw['tracking'], 'spec_ref': self.profile['subject_refs'][raw['spec_ref']],
                       'attributes': values, 'attribute_schema': declarations, 'presentation_or_media_ref': None}
        elif cat == 'resources':
            raw = src['resource_definition']
            payload = {'kind': raw['kind'], 'role': raw['role'], 'capacity': amount(raw['capacity']['value'], raw['capacity']['unit']),
                       'qualifications': raw.get('qualifications', []), 'default_calendar_ms': [{'start_ms': a, 'end_ms': b} for a, b in raw['default_calendar_ms']], 'presentation': None}
        elif cat == 'tasks':
            raw = src['task_definition']
            payload = {'kind': {'preparation': 'manual_preparation'}[raw['kind']], 'target_role': raw['target_role'],
                       'prerequisites': [self.profile['readiness_refs'][key] for key in raw['prerequisites']], 'steps': [],
                       'resource_roles': [self.resource(x) for x in raw['resource_roles']], 'duration': duration(raw['duration'], self.profile['source_basis']),
                       'equipment_access': raw['equipment_access'], 'completion_effect': {'target_role': raw['target_role'], 'marker': raw['completion_effect']['readiness_flag'], 'produces_material': raw['produces_material']}}
        elif cat == 'operations':
            payload = self.operation(src['operation_definition'])
        elif cat == 'devices':
            payload = {'placement': src['placement'], 'ports': self.ports(src['ports']),
                       'behavior': {'inline': self.operation(src['behavior'], src['ports']), 'operation_ref': None, 'selected': 'inline'},
                       'configuration': {'run_mode': {'kind': 'enum', 'value': src['behavior']['repeat_mode']}},
                       'presentation': self.presentation(src['presentation']), 'operations': src['operations']}
            self.issue('legacy_run_mode_consumer_pending', '/definition/payload/data/configuration/run_mode', 'Stored exact legacy strategy as a declared enum; the product consumer must explicitly support it before execution.', ['execution'])
        elif cat == 'transport':
            raw = src['behavior']
            payload = {'ports': self.ports(src['ports']), 'builtin': raw['builtin'], 'batch_capacity': amount(raw['batch_capacity']['value'], raw['batch_capacity']['unit']),
                       'resources': [self.resource(x) for x in raw['resources']], 'times': {key: duration(value, self.profile['source_basis']) for key, value in raw['times'].items()},
                       'acceptance_conditions': [], 'identity_policy': raw['identity_policy'], 'presentation': self.presentation(src['presentation'])}
        elif cat == 'templates':
            project = read(self.path.parent / src['project'])
            self.archives['sources/project.design.json'] = project
            self.files.append({'path': 'sources/project.design.json', 'kind': 'asset'})
            payload = {'project': 'sources/project.design.json', 'generation_profile': None}
            self.issue('template_inner_file_not_canonical', '/definition/payload/data/project', 'Preserved real design project; it is not yet TemplateSubset. Height, footprints, stores/port endpoints, phases and explicit seed require precise materialization, never omission.', ['adoption', 'execution', 'complete_conclusion'])
        elif cat == 'professional':
            raw = src['professional_contract']
            def fields(names):
                return [{'name': key, 'value_type': 'string', 'unit': None, 'required': True, 'scope': self.profile['field_reference_scope']} for key in names]
            payload = {'input_fields': fields(raw['input_fields']), 'output_artifact_type': raw['output_artifact_type'], 'output_fields': fields(raw['output_fields']),
                       'intended_modes': raw['intended_modes'], 'unit': {'mm': 'millimeter'}[raw['unit']],
                       'coordinate_frame': self.profile['professional_frame_selection'], 'scope': raw['scope']}
            self.issue('professional_frame_and_field_types_unconfirmed', '/definition/payload/data', 'Trial uses explicit choice profile for machine frame and opaque field-reference types; actual provider remains not implemented and no capabilities are registered.', ['execution', 'complete_conclusion'])
        elif cat == 'media':
            payload = self.presentation(src['presentation'])
        else:
            raise AssertionError(cat)
        for parameter in src.get('parameters', []):
            parameters.append({'id': parameter['id'], 'label': parameter.get('label'), 'value_type': parameter['type'],
                               'unit': 'millisecond' if parameter['type'] == 'duration' else parameter.get('unit'),
                               'binding': self.profile['parameter_aliases'][parameter['binding']], 'minimum': None, 'maximum': None,
                               'integer': parameter.get('integer', False), 'options': parameter.get('options', []), 'basis': self.profile['source_basis']})
        raw_contribution = src['contribution']
        # Unresolved author/license placeholders stay visibly unresolved rather than becoming a licence grant.
        author = None if raw_contribution['author'] == '待填写' else raw_contribution['author']
        license_text = None if raw_contribution['license'] == '待声明' else raw_contribution['license']
        self.issue('legacy_rights_unresolved', '/definition/contribution', 'Original author/license placeholders preserved in archived manifest; no redistribution approval was created.', ['redistribution'])
        self.archives['sources/manifest.design.json'] = deepcopy(src)
        self.files.append({'path': 'sources/manifest.design.json', 'kind': 'asset'})
        definition = {'id': src['id'], 'version': src['version'], 'title': src['title'], 'definition_state': src['definition_state'],
                      'contribution': {'author': author, 'license': license_text, 'assets': self.rights}, 'parameters': parameters,
                      'attribute_schema': attrs, 'payload': {'category': cat, 'data': payload}}
        return {'schema_version': '1.0', 'data': {'definition': definition, 'files': self.files, 'dependencies': [], 'rights': self.rights, 'assets': self.assets, 'issues': self.issues}}


def main():
    profile = read(BASE / 'DRF03_适配试做选项.json')
    registry = read(ROOT / 'packages/module-kit/category-contracts.design.json')
    validator = Draft202012Validator(read(ROOT / 'tools/type-contract-proof/output/schemas/module.schema.json'))
    OUT.mkdir(exist_ok=True)
    results, fields = [], []
    for category in registry['categories']:
        adapter = Adapter(ROOT / 'modules' / category['example_manifest'], profile)
        converted = adapter.convert()
        validator.validate(converted)
        folder = OUT / category['id']
        folder.mkdir(exist_ok=True)
        for name, data in adapter.archives.items():
            path = folder / name
            path.parent.mkdir(parents=True, exist_ok=True)
            write(path, data)
        write(folder / 'module.json', converted)
        for field in category['field_contracts']:
            fields.append({'category': category['id'], 'logical_field': field['path'], 'legacy_manifest_ref': 'modules/' + category['example_manifest'],
                           'canonical_carriers': field['canonical_carriers'], 'unit_rule': field['unit_rule'],
                           'trial_status': 'inner_template_preserved_blocking' if category['id'] == 'templates' else 'converted_or_explicit_optional_or_trial_choice',
                           'candidate_ref': str((folder/'module.json').relative_to(ROOT)).replace('\\', '/'), 'validation_rule': field['validation_rule']})
        results.append({'category': category['id'], 'source_id': adapter.source['id'], 'candidate_id': converted['data']['definition']['id'],
                        'candidate_schema': 'passed', 'issues': adapter.issues, 'actual_import_or_execution': None})
    assert len(results) == 9 and len(fields) == 61
    write(BASE / 'DRF03_逐项适配映射.json', {'document_kind': 'actual_legacy_to_candidate_field_materials_not_product_loader',
          'source_schema_ref': 'tools/type-contract-proof/output/schemas/module.schema.json', 'profile_ref': 'scenarios/九类模块采用/DRF03_适配试做选项.json',
          'fields': fields, 'common_mapping': {'id/version/title/definition_state': 'data.definition unchanged', 'category': 'data.definition.payload.category',
          'contribution': 'typed author/license/asset rights; unresolved placeholders become null with blocking issue and raw archive',
          'parameters': 'type→value_type, explicit parameter_aliases, duration→integer ms typed container', 'attribute_schema': 'type→value_type, shape=3→[3], explicit source basis'},
          'original_manifests_modified': False})
    result = {'document_kind': 'actual_isolated_legacy_adapter_trial_not_product_import', 'status': 'nine_outer_candidates_schema_checked_inner_blockers_preserved',
              'cases': results, 'outer_candidate_schema_count': 9, 'payloads_with_defined_conversion': 8, 'template_inner_materialization': 'blocked_not_silently_empty',
              'field_mapping_count': len(fields), 'source_fixtures_copied_as_results': False, 'original_manifests_modified': False,
              'provider_capabilities_registered': False, 'actual_Host': None, 'actual_result': None, 'shared_files_written': False}
    write(BASE / 'DRF03_适配试做结果.json', result)
    print(json.dumps({key: result[key] for key in ['outer_candidate_schema_count', 'payloads_with_defined_conversion', 'template_inner_materialization', 'field_mapping_count', 'actual_Host']}, ensure_ascii=False))


if __name__ == '__main__':
    if '--inner-template' in sys.argv:
        from DRF03_C04_内层模板适配 import main as inner_main
        inner_main()
    else:
        main()
