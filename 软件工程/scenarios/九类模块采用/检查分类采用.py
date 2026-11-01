"""实际声明消费者的一致性检查，以及明确的有限采用推理；非运行加载器。"""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BASE=Path(__file__).resolve().parent
registry=json.loads((ROOT/'packages/module-kit/category-contracts.design.json').read_text(encoding='utf-8'))
records={};cases=[]
categories={c['id']:c for c in registry['categories']}
for c in registry['categories']:
    path=ROOT/'modules'/c['example_manifest'];m=json.loads(path.read_text(encoding='utf-8'))
    assert set(registry['common_fields'])<=set(m) and m['category']==c['id']
    assert all(f in m for f in c['payload_fields']) and m['definition_state']=='design'
    if c['id']=='devices':assert 'behavior' in m  # 当前例明确内联，外引用是采用时另一选择。
    if c['id']=='templates':assert (path.parent/m['project']).exists()
    if c['id']=='media':assert (path.parent/m['presentation']['path']).exists()
    records[c['id']]=m
assert len(records)==9
catalog=json.loads((ROOT/'modules/catalog.json').read_text(encoding='utf-8'))
assert {c['id'] for c in catalog['categories']}==set(categories)
assert {x['category'] for x in catalog['modules']}==set(records)
for x in catalog['modules']:
    path=ROOT/'modules'/x['manifest'];m=json.loads(path.read_text(encoding='utf-8'))
    assert m['definition_state']=='design' and m['category']==x['category']
    assert m['id']==x['id'] and set(registry['common_fields'])<=set(m)
    assert all(f in m for f in categories[m['category']]['payload_fields'])
    if m['category']=='templates':
        assert (path.parent/m['project']).exists()
        if 'generation_profile' in m:assert (path.parent/m['generation_profile']).exists()

def add(name,expected,scope='first_release_design_expectation'):
    cases.append(dict(id=name,manual_expected=expected,release_scope=scope,actual_result=None))
item=records['items']['item_definition']
assert 'quantity' not in item and item['quantity_is_instance_state']
add('item_definition_import',dict(type_unit=item['unit'],current_items_created=0,current_stock_created=0))
resource=records['resources']['resource_definition']
assert resource['capacity']['unit']=='person' and resource['current_occupation_is_instance_state']
add('resource_definition_import',dict(capacity_per_explicit_instance=1,current_workers_created=0,occupation_copied=False))

rule=records['operations']['operation_definition'];device=records['devices']
ports={p['id']:p for p in device['ports']};bindings={r:r for r in ['shell','pcb','fastener']}
for line in rule['inputs']:
    port=ports[bindings[line['role']]]
    assert port['item']==line['role'] and port['unit']==line['unit']
assert ports['assembled']['item']=='assembly'
stock={'shell':2,'pcb':1,'fastener':4}
count=min(stock[x['role']]//x['quantity'] for x in rule['inputs']);assert count==1
remaining={x['role']:stock[x['role']]-count*x['quantity'] for x in rule['inputs']}
add('operation_ref_adoption',dict(behavior_source='operation_ref',inline_behavior_used=False,
    item_role_to_port=bindings,output_role_to_port={'assembly':'assembled'},resource_role_to_instance={'assembler':'R1'},
    expected_output_count=count,input_remaining=remaining,composition_preserved=True,stock_commit_owner='factory_engine'))
add('double_behavior_source',dict(selected_source_required=True,execute_both_allowed=False,duplicate_output_allowed=False))

task=records['tasks']['task_definition'];assert task['produces_material'] is False
add('preparation_completion',dict(effect=task['completion_effect'],material_created=0,work_duration_seconds=30,
    target_and_qualified_setter_required=True,definition_import_does_not_execute_task=True))
power=json.loads((ROOT/'modules/resources/power-source/manifest.json').read_text(encoding='utf-8'))['resource_definition']
assert power['capacity']['unit']=='kW' and power['background']['value']==0
add('service_capacity',dict(demand_kW=6+6,capacity_kW=power['capacity']['value'],fits_declared_capacity=12<=14,
    stock_created=0,energy_or_cost_not_inferred=True),scope='deferred_design_expectation')

prof=records['professional']['professional_contract'];assert prof['available_modes']==[]
add('professional_provider_not_implemented',dict(intended_modes=prof['intended_modes'],callable_modes=[],
    result_created=False,material_created=0,scope='original_complete_machining_design_preserved'),scope='deferred_design_expectation')
media=records['media']['presentation'];assert media['production_effect']=='none'
add('media_replacement',dict(anchor_role_bindings={'input':'shell','output':'assembled'},
    visual_ports_to_bind=['pcb','fastener'],behavior_changed=False,quantity_changed=False,
    missing_visual_anchor_is_not_missing_material_port=True))
policy=json.loads((ROOT/'modules/templates/parallel-electromechanical/project.design.json').read_text(encoding='utf-8'))['import_policy']
assert all(token in policy['into_existing_project'] for token in [
    'preserve_existing_stock_and_demand', 'without_sample_stock_or_demand',
    'seed_requires_explicit_choice_in_new_scenario'])
add('template_import',dict(definitions_and_layout=True,current_stock_copied=False,current_demand_copied=False))
transport=records['transport']['behavior'];assert transport['identity_policy']=='preserve_cargo_ids'
add('transport_rule_import',dict(new_vehicle_created=False,new_cargo_created=False,vehicle_binding_required=True,
    receive_on_unloading_completion=True,free_on_declared_return_or_position=True))

boundary=registry['scope_boundary']
selection=json.loads((ROOT/boundary['selection_ref']).read_text(encoding='utf-8'))
selected={x['id'] for x in selection['selected']};deferred=set(selection['deferred'])
assert set(boundary['deferred_task_ids'])==deferred and len(deferred)==7
assert boundary['partial_task']['id']=='P0-03' and selection['partial_scope']==['P0-03']
for category in categories.values():
    assert category['first_release_scope'] and set(category['first_release_task_refs'])<=selected
    assert set(category['deferred_task_refs'])<=deferred
for example in boundary['retained_deferred_examples']:
    assert (ROOT/example['manifest_ref']).exists()
    if 'deferred_task_ref' in example:assert example['deferred_task_ref'] in deferred
add('first_release_and_deferred_scope',dict(categories_with_authoring_entry=list(categories),
    real_professional_owner=boundary['first_release_professional']['producer_owner'],
    deferred_tasks=boundary['deferred_task_ids'],P0_03_scope=boundary['partial_task']['included'],
    service_example_is_first_release_solver=False,machining_example_is_first_release_solver=False))

assert policy['new_project']=='use_explicit_demo_seed'
assert policy['object_ids']=='new_namespace_and_rewrite_references'
assert boundary['template_adoption']['restores_previous_runtime'] is False
add('template_new_project_explicit_seed',dict(explicit_seed_selection_required=True,
    seed_is_new_scenario_input=True,old_runtime_restored=False,new_namespace_and_reference_rewrite=True,
    existing_project_stock_and_demand_preserved=True))

professional=registry['professional_adoption']
contract=json.loads((ROOT/professional['binding_contract_ref']).read_text(encoding='utf-8'))
assert {'subject_revision','supported_scope','unit','coordinate_frame','producer_result_ref'}<=set(contract['artifact_fields'])
assert {'artifact_ref','accepted_subject','accepted_scope','decision_source'}<=set(contract['acceptance_fields'])
assert {'consumer_ref','operation_ref','artifact_ref','expected_subject_revision','required_payload','requested_scope','acceptance_ref'}<=set(contract['binding_fields'])
assert boundary['first_release_professional']['producer_owner']=='packages/packaging'
assert (ROOT/boundary['first_release_professional']['producer_ref']).exists()
assert set(boundary['first_release_professional']['factory_adoption_modes'])==set(contract['first_release_modes'])
assert set(boundary['first_release_professional']['deferred_modes'])==set(contract['deferred_modes'])
assert professional['agent_acceptance_policy']=='existing_human_acceptance_reference_only'
assert professional['material_commit_owner']=='simulation' and professional['actual_result'] is None
add('packaging_artifact_acceptance_binding_handoff',dict(handoff_order=professional['handoff_order'],
    binding_requirements=professional['binding_requirements'],agent_cannot_create_human_acceptance=True,
    computation_creates_stock=False,acceptance_creates_stock=False,binding_creates_stock=False))

gate=professional['packaging_calculated_nominal_check_gate']
assert set(gate)=={'Pass','Fail','Undetermined'}
assert gate['Pass']=='eligible_for_human_acceptance_and_matching_binding'
assert all(gate[x]=='save_check_artifact_no_qualified_packaging_binding' for x in ['Fail','Undetermined'])
assert boundary['first_release_professional']['declared_process_satisfies_real_packaging_computation'] is False
add('packaging_nonpass_or_declared_process',dict(required_checks=gate,
    declared_process_is_real_packaging_computation=False,nonpass_human_acceptance_does_not_change_check_result=True))

calls=registry['external_calls']
providers=json.loads((ROOT/calls['connector_contract_ref']).read_text(encoding='utf-8'))['providers']
assert calls['first_release_bridge_task_ref'] in selected and calls['deferred_connector_task_ref'] in deferred
assert calls['definition_state_grants_runtime_capability'] is False
assert calls['arbitrary_uploaded_scripts_executed'] is False
assert all(p['available_capabilities']==[] and p['implementation_status']=='deferred' for p in providers)
add('external_agent_and_professional_connectors',dict(first_release_bridge_task=calls['first_release_bridge_task_ref'],
    deferred_connector_task=calls['deferred_connector_task_ref'],available_connectors=[],
    connector_ids=[p['id'] for p in providers],same_host_commands_required=True,second_factory_state_created=False))

assert len(cases)==15 and all(c['actual_result'] is None for c in cases)
(BASE/'expected-category-adoption.design.json').write_text(json.dumps(dict(document_kind='finite_category_design_expectations_not_runtime',
    checkpoint='S03-03',contract_ref='packages/module-kit/category-contracts.design.json',
    categories_checked=list(records),declaration_entries=len(catalog['modules']),
    cases=cases),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'{len(records)} category payloads and {len(catalog["modules"])} declarations checked; '
      f'{len(cases)} finite adoption expectations. '
      'No runtime registration, GUI import or professional execution.')
