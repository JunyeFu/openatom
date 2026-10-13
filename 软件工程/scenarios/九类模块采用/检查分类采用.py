"""实际声明消费者的一致性检查，以及明确的有限采用推理；非运行加载器。"""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BASE=Path(__file__).resolve().parent
registry=json.loads((ROOT/'packages/module-kit/category-contracts.design.json').read_text(encoding='utf-8'))
records={};cases=[]
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
assert {x['category'] for x in catalog['modules']}==set(records)
for x in catalog['modules']:
    m=json.loads((ROOT/'modules'/x['manifest']).read_text(encoding='utf-8'))
    assert m['definition_state']=='design' and m['category']==x['category']

def add(name,expected):cases.append(dict(id=name,manual_expected=expected,actual_result=None))
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
    stock_created=0,energy_or_cost_not_inferred=True))

prof=records['professional']['professional_contract'];assert prof['available_modes']==[]
add('professional_provider_not_implemented',dict(intended_modes=prof['intended_modes'],callable_modes=[],
    result_created=False,material_created=0,scope='original_complete_machining_design_preserved'))
media=records['media']['presentation'];assert media['production_effect']=='none'
add('media_replacement',dict(anchor_role_bindings={'input':'shell','output':'assembled'},
    visual_ports_to_bind=['pcb','fastener'],behavior_changed=False,quantity_changed=False,
    missing_visual_anchor_is_not_missing_material_port=True))
policy=json.loads((ROOT/'modules/templates/parallel-electromechanical/project.design.json').read_text(encoding='utf-8'))['import_policy']
assert 'stock_and_demand_zero' in policy['into_existing_project']
add('template_import',dict(definitions_and_layout=True,current_stock_copied=False,current_demand_copied=False))
transport=records['transport']['behavior'];assert transport['identity_policy']=='preserve_cargo_ids'
add('transport_rule_import',dict(new_vehicle_created=False,new_cargo_created=False,vehicle_binding_required=True,
    receive_on_unloading_completion=True,free_on_declared_return_or_position=True))

assert len(cases)==10 and all(c['actual_result'] is None for c in cases)
(BASE/'expected-category-adoption.design.json').write_text(json.dumps(dict(document_kind='finite_category_design_expectations_not_runtime',
    categories_checked=list(records),declaration_entries=len(catalog['modules']),cases=cases),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('9 category payloads and 20 declarations checked; 10 finite adoption expectations. No runtime registration, GUI import or professional execution.')
