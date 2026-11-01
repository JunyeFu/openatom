"""真实内层模板归一化试做；保留源语义与缺项，输出仅在本场景目录。"""
from copy import deepcopy
from pathlib import Path
import json
import sys

sys.dont_write_bytecode = True
BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[1]
OUT = BASE / 'template-output'
sys.path.insert(0, str(ROOT/'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator
from DRF03_旧清单适配试做 import Adapter, amount, duration, pose, read, write


def empty_bindings():
    return {'item_types': {}, 'resources': {}, 'targets': {}, 'ports': {}}


def issue(code, source_path, reason, kind='missing_input', blocks=None):
    return {'code': code, 'source_path': source_path, 'reason': reason, 'kind': kind,
            'blocks': ['execution', 'complete_conclusion'] if blocks is None else blocks}


def canonical_validator(name, defs):
    return Draft202012Validator({'$defs': defs, '$ref': '#/$defs/'+name})


def package(definition, files=None, assets=None, rights=None):
    return {'schema_version': '1.0', 'data': {'definition': definition, 'files': files or [],
            'dependencies': [], 'assets': assets or [], 'rights': rights or [], 'issues': []}}


def local_definition(ident, title, category, payload, parameters=None):
    return {'id': ident, 'version': '0.1.0', 'title': title, 'definition_state': 'design',
            'contribution': {'author': None, 'license': None, 'assets': []},
            'parameters': [] if parameters is None else parameters, 'attribute_schema': [],
            'payload': {'category': category, 'data': payload}}


def convert(source, source_path):
    profile = read(BASE/'DRF03_适配试做选项.json')
    profile['resource_units'].update({'packer': 'person', 'inspector': 'person'})
    module_sources = {}
    for ref in source['module_refs']:
        path = (source_path.parent / ref['manifest']).resolve()
        assert path.is_relative_to((ROOT/'modules').resolve())
        manifest = read(path)
        assert manifest['id'] == ref['id'] and manifest['version'] == ref['version']
        module_sources[ref['id']] = (path, manifest)
    workers = {row['id']: row for row in source['resources']}
    source_devices = {row['id']: row for row in source['devices']}
    source_locations = {row['id']: row for row in source['stores']}
    item_sources = {row['id']: row for row in source['item_types']}
    generated, archives, mappings, gaps = {}, {}, [], []
    scoped_refs, devices, phase_material, inactive = [], [], [], []
    prefix = 'local.' + source['id']
    line_for = {member: row['id'] for row in source['lines'] for member in row['members']}
    selected_policy = {
        'coordinate_convention': 'single hall-local XY diagram → right_handed_z_up plane Z=0; identity world origin is a coordinate convention, not measured site elevation',
        'counted_item_tracking': 'individual', 'imported_item_revision': 'r1',
        'phase_interruption_policy': 'pause_resume', 'preparation_equipment_access': 'exclusive',
        'basis': 'explicit round-3 design selection for previously absent policy; existing quantities/durations unchanged',
    }
    phase_kinds = {'load': 'loading', 'auto': 'processing', 'placement': 'processing', 'unload': 'unloading'}
    for index, row in enumerate(source['devices']):
        module_path, manifest = module_sources[row['module']]
        raw_behavior = manifest['behavior']
        rb = empty_bindings()
        ports = []
        for port in manifest['ports']:
            item = row['parameters'][port['item_parameter']] if 'item_parameter' in port else port['item']
            assert item in item_sources
            role = port['id']
            rb['item_types'][role] = item
            ports.append({'id': port['id'], 'direction': {'in': 'input', 'out': 'output'}[port['direction']],
                          'payload': {'kind': 'material', 'data': {'item_role': role, 'unit': port['unit'], 'quantity_rule': None}}})
        pm = {p['id']: p for p in ports}
        for role, resource_id in row.get('resource_bindings', {}).items():
            assert resource_id in workers
            rb['resources'][role] = resource_id
        for port_id, location_id in row['port_bindings'].items():
            assert location_id in source_locations
            rb['targets']['material_store.'+port_id] = {'kind': 'location', 'id': location_id}
        def resource(role, resource_id, count=1):
            assert resource_id in workers and workers[resource_id]['kind'] == 'worker'
            rb['resources'][role] = resource_id
            return {'role': role, 'quantity': amount(count, 'person'), 'qualifications': []}
        job_resources = []
        mapped_job_workers = set()
        for req in raw_behavior.get('resources', []):
            rid = row['resource_bindings'][req['role']]
            job_resources.append(resource(req['role'], rid, req['quantity']))
            mapped_job_workers.add(rid)
        for rid in row['job_resources']:
            if rid == row['id']:
                continue  # The execution-device capacity remains the common Engine's capacity.
            assert rid in workers
            if rid not in mapped_job_workers:
                job_resources.append(resource('legacy.worker.'+rid, rid))
        def material(raw):
            port = pm[raw['port']]
            item_id = rb['item_types'][raw['port']]
            required = row.get('input_quality', {}).get(item_id)
            return {'role': raw['port'], 'port': raw['port'], 'quantity': amount(raw['quantity'], port['payload']['data']['unit']),
                    'required_quality': [] if required is None else [required]}
        inputs = [material(x) for x in raw_behavior['inputs']]
        if raw_behavior['builtin'] == 'inspect_and_route':
            assert len(inputs) == 1
            outputs = [{'role': p['id'], 'port': p['id'], 'quantity': deepcopy(inputs[0]['quantity']), 'required_quality': []}
                       for p in ports if p['direction'] == 'output']
            # These are mutually exclusive routes, never three simultaneous outputs.
            rb['targets']['isolation'] = {'kind': 'location', 'id': row['port_bindings']['quarantine']}
            quality = {'source_kind': 'preset', 'result_ref': prefix+'.quality_inputs',
                       'max_completed_reworks': row['parameters']['max_reworks'], 'pass_port': 'passed', 'fail_port': 'rework',
                       'pending_policy': 'hold_station', 'pending_location_role': None, 'isolation_location_role': 'isolation', 'release_required': True}
        else:
            outputs = [material(x) for x in raw_behavior['outputs']]
            for line in outputs:
                line['required_quality'] = []  # Output preset declarations are handled by the explicit companion rule below.
            quality = None
        identity = {'transform': 'transform_with_origin', 'assemble': 'assembly_with_composition',
                    'inspect_and_route': 'preserve', 'pack': 'container_with_contents'}[raw_behavior['builtin']]
        phases = []
        if 'phases' in row:
            for raw_phase in row['phases']:
                requirements = [resource('legacy.worker.'+rid, rid) for rid in raw_phase['phase_resources']]
                phases.append({'id': raw_phase['id'], 'work_kind': phase_kinds[raw_phase['id']],
                               'duration': duration({'value': raw_phase['duration_ms'], 'unit': 'millisecond'}, 'module_declared'),
                               'phase_resources': requirements, 'interruption_policy': selected_policy['phase_interruption_policy']})
            total = sum(p['duration']['value'] for p in phases)
            assert total == row['parameters']['cycle_seconds'] * 1000
            cycle = None
            inactive.append({'device_id': row['id'], 'parameter': 'cycle_seconds', 'source_value': row['parameters']['cycle_seconds'],
                             'reason': 'Explicit phase plan is authoritative; do not expose an inactive inline-duration override as if it changed phases.'})
            phase_material.append({'device_id': row['id'], 'source_path': f'/devices/{index}/phases', 'phases': deepcopy(phases)})
        else:
            cycle = duration({'value': row['parameters']['cycle_seconds'], 'unit': 'second'}, 'module_declared')
        operation = {'builtin': raw_behavior['builtin'], 'item_roles': [p['id'] for p in ports], 'inputs': inputs, 'outputs': outputs,
                     'resource_roles': job_resources, 'duration': cycle, 'phases': phases, 'hold_on_output_wait': [],
                     'quality_and_routes': quality, 'identity_policy': identity, 'family': None}
        aid = prefix+'.device.'+row['id']
        local_ref = {'id': aid, 'version': '0.1.0'}
        adapter = Adapter(module_path, profile)
        presentation = adapter.presentation(manifest['presentation'])
        typed_config = {}
        parameters = []
        for param in manifest['parameters']:
            key = param['id']
            if key not in row['parameters'] or (key == 'cycle_seconds' and phases):
                continue
            value = row['parameters'][key]
            if param['type'] == 'item_type':
                inactive.append({'device_id': row['id'], 'parameter': key, 'source_value': value,
                                 'reason': 'Resolved explicitly into this specialized definition and item role bindings; no second misleading item-type configuration.'})
                continue
            if param['type'] == 'duration':
                typed_config[key] = {'kind': 'duration', 'value': duration({'value': value, 'unit': param['unit']}, 'module_declared')}
                binding = '/payload/data/behavior/inline/duration'
            else:
                value_type = 'integer' if param.get('integer') else param['type']
                typed_config[key] = {'kind': value_type, 'value': value}
                binding = '/payload/data/configuration/'+key+'/value'
                if key == 'max_reworks':
                    binding = '/payload/data/behavior/inline/quality_and_routes/max_completed_reworks'
            parameters.append({'id': key, 'label': param.get('label'), 'value_type': 'integer' if param.get('integer') else param['type'],
                               'unit': 'millisecond' if param['type'] == 'duration' else param.get('unit'),
                               'binding': binding, 'minimum': param.get('minimum'), 'maximum': param.get('maximum'),
                               'integer': param.get('integer', False), 'options': param.get('options', []), 'basis': 'module_declared'})
        payload = {'placement': 'fixed_during_simulation', 'ports': ports, 'behavior': {'inline': operation, 'operation_ref': None, 'selected': 'inline'},
                   'configuration': deepcopy(typed_config), 'presentation': presentation, 'operations': manifest['operations']}
        definition = local_definition(aid, row['id']+'（由原模板显式配置派生）', 'devices', payload, parameters)
        generated['definitions/'+row['id']+'/module.json'] = package(definition, adapter.files, adapter.assets, adapter.rights)
        for path, value in adapter.archives.items():
            archives['definitions/'+row['id']+'/'+path] = value
        # The per-definition files are written relative to its own directory below.
        scoped_refs.append(local_ref)
        mappings.append({'source_device_id': row['id'], 'source_module_ref': {'id': manifest['id'], 'version': manifest['version']},
                         'derived_module_ref': local_ref, 'path': 'definitions/'+row['id']+'/module.json',
                         'reason': 'Explicit template overrides/phases become a new local definition, never mutate shared 0.1.0.'})
        x, y = row['layout_m']
        devices.append({'id': row['id'], 'title': row['id'], 'hall_id': source['halls'][0]['id'], 'line_id': line_for[row['id']],
                        'module_ref': local_ref, 'pose': pose([x, y, 0], 'meter', 'right_handed_z_up'), 'footprint_m': [], 'height_m': None,
                        'configuration': typed_config, 'role_bindings': rb, 'presentation_override': None, 'operation_binding': None})
        gaps.append(issue('missing_device_geometry', f'/devices/{index}', 'Source has XY only; footprint/height/clearance remain unknown, never a site Pass.'))
    local_by_device = {d['id']: d for d in devices}
    quarantine_ids = {row['port_bindings']['quarantine'] for row in source['devices'] if 'quarantine' in row['port_bindings']}
    locations = []
    for raw in source['stores']:
        if raw['id'] in quarantine_ids:
            purpose, issuing = 'isolation', False
        elif raw['item_type'] == source['entry_exit']['output_type']:
            purpose, issuing = 'output', True
        elif raw['item_type'] in source['entry_exit']['input_types']:
            purpose, issuing = 'input', True
        else:
            purpose, issuing = 'buffer', True
        locations.append({'id': raw['id'], 'title': raw['id'], 'hall_id': raw['hall'],
                          'position': pose(raw['layout_m']+[0], 'meter', 'right_handed_z_up'), 'capacity': amount(raw['capacity'], raw['unit']),
                          'accepted_item_types': [raw['item_type']], 'purpose': purpose, 'ordinary_issue_enabled': issuing})
    def endpoint(text):
        if text in source_locations:
            return {'kind': 'location', 'data': text}, text
        matches = [ident for ident in source_devices if text.startswith(ident+'.')]
        assert len(matches) == 1, text
        ident = matches[0]
        port = text[len(ident)+1:]
        assert port in source_devices[ident]['port_bindings']
        return {'kind': 'port', 'data': {'module_ref': local_by_device[ident]['module_ref'], 'instance_id': ident, 'port_id': port}}, source_devices[ident]['port_bindings'][port]
    connections = []
    for i, raw in enumerate(source['connections']):
        source_end, source_store = endpoint(raw['from'])
        dest_end, dest_store = endpoint(raw['to'])
        assert source_store in source_locations and dest_store in source_locations
        connections.append({'kind': 'material', 'data': {'id': f'MC{i+1:02}', 'source': source_end, 'destination': dest_end,
                           'item_type': raw['item_type'], 'unit': item_sources[raw['item_type']]['unit'],
                           'quantity_rule': None, 'capacity_rule': None, 'destination_store': dest_store}})
    item_types = [{'id': raw['id'], 'revision': selected_policy['imported_item_revision'], 'title': raw['id'], 'unit': raw['unit'],
                   'tracking': selected_policy['counted_item_tracking'], 'spec_ref': None, 'attributes': {}, 'attribute_schema': []} for raw in source['item_types']]
    for i, raw in enumerate(source['item_types']):
        gaps.append(issue('unresolved_spec_reference', f'/item_types/{i}/spec_ref', 'Preserved source '+raw['spec_ref']+'; no actual specification object was supplied.'))
    halls = []
    for i, raw in enumerate(source['halls']):
        halls.append({'id': raw['id'], 'title': raw['id'], 'width_m': raw['width_m'], 'depth_m': raw['depth_m'], 'height_m': raw['height_m'],
                      'origin': pose([0, 0, 0], 'meter', 'right_handed_z_up'),
                      'input_locations': [x['id'] for x in locations if x['purpose'] == 'input'],
                      'output_locations': [x['id'] for x in locations if x['purpose'] == 'output']})
        if raw['height_m'] is None:
            gaps.append(issue('missing_hall_height', f'/halls/{i}/height_m', 'Original null preserved; draft only, no net-height/site Pass.'))
    lines = [{'id': raw['id'], 'title': raw['id'], 'hall_id': raw['hall'], 'transform': pose([0,0,0], 'meter', 'right_handed_z_up'),
              'members': raw['members'], 'boundary_ports': [], 'region_polygon_m': []} for raw in source['lines']]
    requirements = [{'kind': raw['kind'], 'role': 'legacy.worker.'+raw['id'], 'capacity': amount(raw['capacity'], 'person'), 'qualifications': [],
                     'default_calendar_ms': [{'start_ms': a, 'end_ms': b} for a, b in raw['calendar_ms']], 'presentation': None} for raw in source['resources']]
    preparation = []
    for i, raw in enumerate(source['preparation_tasks']):
        tid = prefix+'.preparation.'+raw['id']
        roles = [{'role': 'legacy.worker.'+rid, 'quantity': amount(1, 'person'), 'qualifications': []} for rid in raw['resources']]
        payload = {'kind': 'manual_preparation', 'target_role': 'target', 'prerequisites': [], 'steps': [], 'resource_roles': roles,
                   'duration': duration({'value': raw['duration_ms'], 'unit': 'millisecond'}, 'module_declared'),
                   'equipment_access': selected_policy['preparation_equipment_access'],
                   'completion_effect': {'target_role': 'target', 'marker': raw['id'], 'produces_material': False}}
        generated['definitions/preparation-'+raw['id']+'/module.json'] = package(local_definition(tid, raw['id'], 'tasks', payload))
        ref = {'id': tid, 'version': '0.1.0'}
        scoped_refs.append(ref)
        preparation.append({'source_path': f'/preparation_tasks/{i}', 'module_ref': ref, 'target_device_id': raw['target'],
                            'before_device_ids': [d['id'] for d in source['devices'] if d.get('ready_after') == raw['id']],
                            'resources': {role['role']: rid for role, rid in zip(roles, raw['resources'])},
                            'completion_marker': raw['id'], 'source_declared_job_ref': source_devices[raw['target']].get('artifact_ref'),
                            'material_delta': 0})
    for raw in source['devices']:
        if 'ready_after' in raw:
            assert any(p['completion_marker'] == raw['ready_after'] and raw['id'] in p['before_device_ids'] for p in preparation)
    claims = [{'claim_id': raw['id'], 'evidence_origin': 'imported_claim', 'original_payload': deepcopy(raw),
               'consumer_device_ids': [d['id'] for d in source['devices'] if d.get('artifact_ref') == raw['id']],
               'producer_result_created': False, 'human_acknowledgment_created': False} for raw in source['professional_artifacts']]
    for raw in source['devices']:
        if 'artifact_ref' in raw:
            assert any(c['claim_id'] == raw['artifact_ref'] for c in claims)
    quality = {'id': prefix+'.quality_inputs', 'source_kind': source['quality_source'], 'source_values': source['quality_inputs'],
               'output_quality_declarations': [{'device_id': d['id'], 'values': d['output_quality']} for d in source['devices'] if 'output_quality' in d],
               'input_quality_declarations': [{'device_id': d['id'], 'values': d['input_quality']} for d in source['devices'] if 'input_quality' in d],
               'actual_provider_or_field_quality': False}
    companion = {'document_kind': 'legacy_template_adoption_material', 'schema_version': '1.0', 'source_id': source['id'],
                 'selected_policy': selected_policy, 'source_definition_refs': source['module_refs'], 'derived_definition_map': mappings,
                 'phase_material': phase_material, 'inactive_parameters': inactive,
                 'source_controls': {key: deepcopy(source[key]) for key in ['time', 'assumptions', 'import_policy', 'entry_exit', 'professional_modes', 'adoption_preview']},
                 'device_execution_declarations': [{key: deepcopy(raw[key]) for key in ['id','kind','process_mode','scope','quality_source','artifact_ref','ready_after'] if key in raw} for raw in source['devices']],
                 'resource_requirement_source_ids': [raw['id'] for raw in source['resources']], 'source_resources': source['resources'],
                 'preparation_bindings': preparation, 'professional_claims': claims, 'quality_declarations': quality,
                 'deferred_seed': {'initial_stock': source['initial_stock'], 'demand': source['demand'], 'reason': 'No actual Product/BOM/Route revision was supplied; preserve the explicit seed draft, do not invent one.'},
                 'source_item_specs': {raw['id']: raw['spec_ref'] for raw in source['item_types']},
                 'source_geometry_rule': source['unresolved_geometry_rule'], 'unresolved_fields': gaps,
                 'consumer_contract_ref': 'docs/04_开发与验收/特征细化/DRF03_C04_伴随材料消费契约.json',
                 'actual_runtime_result': None}
    gaps.append(issue('missing_product_BOM_route', '/demand', 'Demand names an item type only; Product/BOM/Route input is absent. Preserve seed draft, no synthetic complete order.'))
    template = {'schema_version': '1.0', 'data': {'layout_scope': 'hall', 'halls': halls, 'locations': locations, 'lines': lines, 'devices': devices,
                'connections': connections, 'module_refs': scoped_refs, 'resource_requirements': requirements,
                'item_types': item_types, 'role_requirements': [], 'suggested_role_bindings': empty_bindings(), 'seed': None}}
    return template, companion, generated, archives


def check_references(template, companion, generated):
    data = template['data']
    hall_ids = {h['id'] for h in data['halls']}
    locs = {x['id']: x for x in data['locations']}
    devices = {d['id']: d for d in data['devices']}
    lines = {line['id']: line for line in data['lines']}
    types = {t['id']: t for t in data['item_types']}
    definitions = {(p['data']['definition']['id'],p['data']['definition']['version']):p['data']['definition'] for p in generated.values()}
    for d in devices.values():
        assert d['hall_id'] in hall_ids and d['line_id'] in lines
        assert d['id'] in lines[d['line_id']]['members']
        assert (d['module_ref']['id'],d['module_ref']['version']) in definitions
    for connection in data['connections']:
        c = connection['data']
        assert c['item_type'] in types and c['unit'] == types[c['item_type']]['unit']
        assert c['destination_store'] in locs
        for field in ['source','destination']:
            endpoint = c[field]
            if endpoint['kind'] == 'location':
                assert endpoint['data'] in locs
                if field == 'destination':
                    assert endpoint['data'] == c['destination_store']
            else:
                assert endpoint['kind'] == 'port'
                p = endpoint['data']
                device = devices[p['instance_id']]
                assert p['module_ref'] == device['module_ref']
                definition = definitions[(p['module_ref']['id'],p['module_ref']['version'])]
                port = next(row for row in definition['payload']['data']['ports'] if row['id'] == p['port_id'])
                assert port['direction'] == ('output' if field == 'source' else 'input')
                assert device['role_bindings']['item_types'][port['payload']['data']['item_role']] == c['item_type']
                store_binding = device['role_bindings']['targets']['material_store.'+p['port_id']]
                assert store_binding['kind'] == 'location' and store_binding['id'] in locs
                if field == 'destination':
                    assert store_binding['id'] == c['destination_store']
    for prep in companion['preparation_bindings']:
        assert prep['target_device_id'] in devices and all(d in devices for d in prep['before_device_ids'])
    return {'locations':len(locs),'connections':len(data['connections']),'devices':len(devices),'local_definitions':len(definitions),
            'internal_endpoint_and_definition_references':'passed','external_resource_choice_required':True,
            'product_BOM_route_and_geometry_complete':False}


def remap_layout(template, companion, namespace):
    """Explicit trial remapping; not a Host transaction or resource auto-selection."""
    t, c = deepcopy(template), deepcopy(companion)
    data = t['data']
    ids = [row['id'] for key in ['halls','locations','lines','devices'] for row in data[key]]
    ids += [row['data']['id'] for row in data['connections']]
    mapping = {ident: namespace+'.'+ident for ident in ids}
    assert len(mapping) == len(ids)
    for hall in data['halls']:
        hall['id'] = mapping[hall['id']]
        for key in ['input_locations','output_locations']:
            hall[key] = [mapping[x] for x in hall[key]]
    for loc in data['locations']:
        loc['id'],loc['hall_id'] = mapping[loc['id']],mapping[loc['hall_id']]
    for line in data['lines']:
        line['id'],line['hall_id'] = mapping[line['id']],mapping[line['hall_id']]
        line['members'] = [mapping[x] for x in line['members']]
    for device in data['devices']:
        for key in ['id','hall_id','line_id']:
            device[key] = mapping[device[key]]
        for target in device['role_bindings']['targets'].values():
            if target['kind'] == 'location':
                target['id'] = mapping[target['id']]
    for edge in data['connections']:
        value = edge['data']
        value['id'],value['destination_store'] = mapping[value['id']],mapping[value['destination_store']]
        for key in ['source','destination']:
            endpoint = value[key]
            if endpoint['kind'] == 'location':
                endpoint['data'] = mapping[endpoint['data']]
            else:
                endpoint['data']['instance_id'] = mapping[endpoint['data']['instance_id']]
    for prep in c['preparation_bindings']:
        prep['target_device_id'] = mapping[prep['target_device_id']]
        prep['before_device_ids'] = [mapping[x] for x in prep['before_device_ids']]
    for claim in c['professional_claims']:
        claim['consumer_device_ids'] = [mapping[x] for x in claim['consumer_device_ids']]
    for key in ['output_quality_declarations','input_quality_declarations']:
        for row in c['quality_declarations'][key]:
            row['device_id'] = mapping[row['device_id']]
    for key in ['phase_material','inactive_parameters']:
        for row in c[key]:
            row['device_id'] = mapping[row['device_id']]
    for row in c['device_execution_declarations']:
        row['id'] = mapping[row['id']]
    c['instance_namespace'] = namespace
    c['source_to_instance_ids'] = mapping
    return t, c


def source_field_ledger(source):
    """Every source leaf has a specific semantic destination or explicit draft status."""
    targets = {
        'halls': ('template.data.halls', 'typed_geometry_missing_height_preserved'),
        'stores': ('template.data.locations', 'typed_full_12_locations'),
        'lines': ('template.data.lines', 'typed_members_unknown_region'),
        'connections': ('template.data.connections', 'typed_tagged_endpoints_and_explicit_store_bindings'),
        'item_types': ('template.data.item_types + companion.source_item_specs', 'typed_types_real_spec_objects_missing'),
        'resources': ('template.data.resource_requirements + companion.source_resources', 'typed_requirements_explicit_worker_choice_missing'),
        'initial_stock': ('companion.deferred_seed.initial_stock', 'draft_real_Product_BOM_Route_missing'),
        'demand': ('companion.deferred_seed.demand', 'draft_real_Product_BOM_Route_missing'),
        'professional_artifacts': ('companion.professional_claims.original_payload', 'imported_claim_not_accepted'),
        'preparation_tasks': ('definitions/preparation-*/module.json + companion.preparation_bindings', 'typed_task_public_companion_integration_required'),
        'module_refs': ('companion.source_definition_refs + derived_definition_map', 'old_exact_refs_preserved_new_local_definitions_explicit'),
        'quality_inputs': ('companion.quality_declarations.source_values', 'preset_not_field_quality_public_companion_integration_required'),
        'quality_source': ('companion.quality_declarations.source_kind', 'preset_not_field_quality'),
        'unresolved_geometry_rule': ('companion.source_geometry_rule', 'explicit_no_site_Pass'),
    }
    controls = {'time','assumptions','import_policy','entry_exit','professional_modes','adoption_preview'}
    device_targets = {'id':'template.data.devices.id', 'module':'companion.derived_definition_map',
        'parameters':'local definition/configuration + companion.inactive_parameters',
        'port_bindings':'Device.role_bindings.targets.material_store.*', 'job_resources':'inline.resource_roles/phase_resources + execution device',
        'resource_bindings':'Device.role_bindings.resources', 'phases':'inline.phases + companion.phase_material',
        'layout_m':'Device.pose.translation XY in selected plane', 'ready_after':'companion.preparation_bindings',
        'artifact_ref':'companion.professional_claims.consumer_device_ids',
        'input_quality':'inline.inputs.required_quality + companion.quality_declarations',
        'output_quality':'companion.quality_declarations.output_quality_declarations',
        'kind':'companion.device_execution_declarations', 'process_mode':'companion.device_execution_declarations',
        'scope':'companion.device_execution_declarations', 'quality_source':'companion.device_execution_declarations'}
    metadata = {'document_kind','schema_revision','id','title','status','actual_result','layout_scope',
                'instantiation_contract_ref','canonical_file_conversion'}
    rows = []
    def walk(value, tokens):
        if isinstance(value, dict) and value:
            for key, child in value.items():
                walk(child, tokens+[key])
        elif isinstance(value, list) and value:
            for index, child in enumerate(value):
                walk(child, tokens+[str(index)])
        else:
            top = tokens[0]
            if top == 'devices':
                target = device_targets[tokens[2]]
                status = 'explicit_semantic_mapping_or_draft_rule'
            elif top in targets:
                target, status = targets[top]
            elif top in controls:
                target, status = 'companion.source_controls.'+top, 'preserved_draft_control'
            else:
                assert top in metadata, top
                target, status = 'source-project.design.json#/'+ '/'.join(tokens), 'original_metadata_not_new_runtime_evidence'
            rows.append({'source_path':'/'+ '/'.join(tokens), 'source_value':value, 'destination':target, 'status':status})
    walk(source, [])
    return rows


def main():
    defs = read(ROOT/'tools/type-contract-proof/output/schemas/all-types.schema.json')['$defs']
    if 'locations' not in defs['TemplateSubset']['properties'] or 'MaterialEndpoint' not in defs:
        raise SystemExit('Canonical C04 delta not generated yet; no fake conversion success.')
    source_path = ROOT/'modules/templates/parallel-electromechanical/project.design.json'
    source = read(source_path)
    template, companion, generated, archives = convert(source, source_path)
    Draft202012Validator(read(ROOT/'tools/type-contract-proof/output/schemas/template.schema.json')).validate(template)
    module_validator = Draft202012Validator(read(ROOT/'tools/type-contract-proof/output/schemas/module.schema.json'))
    for value in generated.values():
        module_validator.validate(value)
    references = check_references(template, companion, generated)
    source_manifest = read(source_path.parent/'manifest.json')
    definition = local_definition(source_manifest['id'], source_manifest['title'], 'templates',
                                  {'project': 'template.json', 'generation_profile': None})
    definition['version'] = '0.2.0'
    file_names = ['template.json', 'adoption-material.json', 'source-project.design.json']+list(generated)+list(archives)
    outer = package(definition, [{'path': name, 'kind': 'template' if name == 'template.json' else 'asset'} for name in file_names])
    outer['data']['dependencies'] = deepcopy(template['data']['module_refs'])
    outer['data']['issues'] = [{'code': 'template_real_inputs_missing', 'message': 'Geometry, specification objects, Product/BOM/Route and explicit worker choices are unresolved; draft only.',
                              'field_path': '/definition/payload/data/project', 'object_ref': None, 'blocks': ['execution', 'complete_conclusion']},
                             {'code': 'companion_consumer_requires_public_integration', 'message': 'Do not dispatch by reading template.json alone; consume adoption-material.json under the explicit owned contract.',
                              'field_path': '/files', 'object_ref': None, 'blocks': ['execution', 'complete_conclusion']},
                             {'code': 'legacy_rights_unresolved', 'message': 'Original author/license remain unconfirmed; no redistribution grant.',
                              'field_path': '/definition/contribution', 'object_ref': None, 'blocks': ['redistribution']}]
    module_validator.validate(outer)
    OUT.mkdir(exist_ok=True)
    write(OUT/'module.json',outer)
    write(OUT/'template.json',template)
    write(OUT/'adoption-material.json',companion)
    write(OUT/'source-project.design.json',source)
    write(BASE/'DRF03_C04_逐字段去向.json', {'source_ref':'modules/templates/parallel-electromechanical/project.design.json',
          'fields':source_field_ledger(source), 'unknown_or_dropped_fields':[], 'original_source_modified':False})
    for name,value in generated.items():
        # Each derived definition owns its own relative native-scene assets.
        path = OUT/name
        path.parent.mkdir(parents=True,exist_ok=True)
        write(path,value)
    for name,value in archives.items():
        path = OUT/name
        path.parent.mkdir(parents=True,exist_ok=True)
        write(path,value)
    for name in file_names:
        assert (OUT/name).is_file(), name
    for name, value in generated.items():
        for entry in value['data']['files']:
            assert (OUT/name).parent.joinpath(entry['path']).is_file(), (name, entry['path'])
    result = {'document_kind':'actual_C04_inner_template_schema_and_reference_trial_not_execution',
              'status':'canonical_structural_adaptation_passed_draft_missing_real_inputs',
              'source_ref':'modules/templates/parallel-electromechanical/project.design.json',
              'template_ref':'scenarios/九类模块采用/template-output/template.json',
              'companion_ref':'scenarios/九类模块采用/template-output/adoption-material.json',
              'canonical_template_schema':'passed','canonical_local_module_schemas':len(generated), 'canonical_outer_module_schema':'passed',
              'outer_candidate_ref':'scenarios/九类模块采用/template-output/module.json', 'listed_files_resolved':len(file_names), 'references':references,
              'source_hall_height':source['halls'][0]['height_m'],'adapted_hall_height':template['data']['halls'][0]['height_m'],
              'phase_plans_preserved':len(companion['phase_material']), 'preparation_bindings_preserved':len(companion['preparation_bindings']),
              'professional_claims_preserved':len(companion['professional_claims']), 'initial_stock_rows_preserved':len(companion['deferred_seed']['initial_stock']),
              'new_seed_or_human_ack_created':False,'site_Pass':False,'actual_Host_or_Engine':None,
              'old_22_29_7_expectations_changed':False,'original_template_modified':False}
    write(BASE/'DRF03_C04_内层适配结果.json',result)
    print(json.dumps({k:result[k] for k in ['status','canonical_template_schema','canonical_local_module_schemas','references','phase_plans_preserved','preparation_bindings_preserved','professional_claims_preserved','actual_Host_or_Engine']},ensure_ascii=False))


if __name__ == '__main__':
    main()
