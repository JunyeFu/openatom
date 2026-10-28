"""S05-03载荷/参数/消费者与有限协议推演；不是运行模块加载器。"""
import json
import re
from pathlib import Path
from datetime import datetime, timedelta, timezone

S = Path(__file__).resolve().parent.parent
A = S / 'docs/04_开发与验收'


def read(ref):
    return json.loads((S / ref).read_text(encoding='utf-8-sig'))


def resolve_declared_parameter(manifest, binding):
    # 清单既有语法；只访问字段和索引，不解释脚本。
    assert re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*|\[\d+\])*', binding), binding
    tokens = re.findall(r'[A-Za-z_][A-Za-z0-9_]*|\d+', binding)
    assert tokens[0] in {'configuration', 'behavior', 'item_definition', 'resource_definition',
                         'task_definition', 'operation_definition', 'presentation', 'professional_contract'}
    value = manifest
    for token in tokens:
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def main():
    spec = read('packages/module-kit/category-contracts.design.json')
    catalog = read('modules/catalog.json')
    forms = read('docs/02_交互设计/首版表单与反馈.json')['forms']
    form_map = {f['category']: f['id'] for f in forms if f.get('category')}
    ids = [c['id'] for c in spec['categories']]
    assert ids == [c['id'] for c in catalog['categories']]
    assert len(ids) == len(set(ids)) == 9
    field_count = 0
    for category in spec['categories']:
        assert len(category['six_views']) == 6
        assert category['form_refs'] == ['F01', form_map[category['id']]]
        assert category['field_contracts']
        for field in category['field_contracts']:
            assert all(field[k] for k in ['path', 'type', 'required_when', 'meaning'])
            field_count += 1
    assert field_count == 61
    assert len(spec['adoption_protocol']) == 8 and len(spec['consumer_handoffs']) == 7
    assert spec['actual_result'] is None and spec['actual_protocol_execution'] is None
    assert spec['professional_adoption']['agent_acceptance_policy'] == 'existing_human_acceptance_reference_only'
    assert spec['external_calls']['definition_state_grants_runtime_capability'] is False
    assert spec['external_calls']['arbitrary_uploaded_scripts_executed'] is False
    checked_parameters = 0
    unresolved_draft_parameters = []
    for entry in catalog['modules']:
        manifest = read('modules/' + entry['manifest'])
        assert manifest['id'] == entry['id'] and manifest['category'] == entry['category']
        assert set(spec['common_fields']) <= set(manifest)
        for parameter in manifest.get('parameters', []):
            value = resolve_declared_parameter(manifest, parameter['binding'])
            if value is None:
                assert manifest['definition_state'] == 'design'
                assert parameter['required_for'] == 'execution_adoption'
                unresolved_draft_parameters.append({'module': manifest['id'], 'parameter': parameter['id']})
                checked_parameters += 1
                continue
            if parameter['type'] in {'number', 'duration', 'quantity'}:
                assert isinstance(value, (int, float)) and not isinstance(value, bool)
            elif parameter['type'] == 'enum':
                assert value in parameter['options']
            elif parameter['type'] == 'item_type':
                assert isinstance(value, str) and value
            if parameter.get('integer'):
                assert int(value) == value
            if 'minimum' in parameter:
                assert value >= parameter['minimum']
            checked_parameters += 1
    assert len(catalog['modules']) == 21
    seeds = read('packages/module-kit/demo-equipment/definitions.design.json')
    assert len(seeds['existing_definitions']) == 2 and len(seeds['candidate_definitions']) == 4
    for manifest in seeds['candidate_definitions']:
        assert manifest['definition_state'] == 'design'
        for parameter in manifest.get('parameters', []):
            resolve_declared_parameter(manifest, parameter['binding'])
            checked_parameters += 1

    examples = []
    # 独立名称相同不代表类型相同；物理量显式转换后才匹配。
    assert ('shell-A', 'piece', 'r1') != ('shell-B', 'piece', 'r1')
    assert 100 * 0.001 == 0.1
    examples.append({'id': 'CP01', 'claim': 'same_name_not_identity_explicit_mm_to_m', 'result': True})
    # 克隆局部对象只重映射实例ID和内部引用，原库存/人员不成为拷贝输入。
    id_map = {'D1': 'copy-D1', 'D2': 'copy-D2'}
    original_link = ('D1', 'out', 'D2', 'in')
    copied_link = (id_map[original_link[0]], original_link[1], id_map[original_link[2]], original_link[3])
    assert copied_link == ('copy-D1', 'out', 'copy-D2', 'in')
    examples.append({'id': 'CP02', 'result': list(copied_link), 'stock_seed_selected': False})
    # 已有期望的齐套规则只有一处行为，2壳/1PCB/4螺钉可执行一次，不是两次。
    stock = {'shell': 2, 'pcb': 1, 'fastener': 4}
    recipe = {'shell': 1, 'pcb': 1, 'fastener': 4}
    cycles = min(stock[k] // recipe[k] for k in recipe)
    assert cycles == 1
    examples.append({'id': 'CP03', 'selected_behavior': 'operation_ref', 'cycles': cycles, 'inline_executions': 0})
    # 同版本引用是精确键；草稿/实例和旧Run引用不因新版本到来被重写。
    prior = ('community.control-box-rule', '0.1.0')
    upgrade = ('community.control-box-rule', '0.2.0')
    assert prior != upgrade
    examples.append({'id': 'CP04', 'prior_run_ref': list(prior), 'new_draft_choice': list(upgrade)})
    # 规格值实际已有，schema新增单位/形状，不将规格导入转为现有库存。
    item = read('modules/items/shell-spec/manifest.json')['item_definition']
    declaration = item['attribute_schema'][0]
    assert declaration['unit'] == 'millimeter' and declaration['type'] == 'vector3'
    assert len(item['attributes'][declaration['name']]) == declaration['shape'] == 3
    assert 'quantity' not in item
    examples.append({'id': 'CP05', 'attribute': declaration['name'], 'created_inventory': 0})
    root = (S / 'modules/media/editable-bench').resolve()
    assert (root / 'assets/native-scene.json').resolve().is_relative_to(root)
    assert not (root / '../../../external.txt').resolve().is_relative_to(root)
    examples.append({'id': 'CP06', 'claim': 'relative_asset_boundary', 'result': True})
    provider = read('modules/professional/machining-producer/manifest.json')['professional_contract']
    assert provider['intended_modes'] and provider['available_modes'] == []
    examples.append({'id': 'CP07', 'intended_is_callable': False, 'actual_modes': []})
    assert spec['scope_boundary']['template_adoption']['restores_previous_runtime'] is False
    examples.append({'id': 'CP08', 'template_seed_is_new_scenario': True, 'restores_run': False})
    assert unresolved_draft_parameters == [
        {'module': 'community.parameterized-pack', 'parameter': 'product_item'},
        {'module': 'community.parameterized-inspect', 'parameter': 'product_item'}]
    examples.append({'id': 'CP09', 'unresolved_execution_parameter_count': 2,
                     'guessed_product_type': False, 'executable_without_binding': False})
    result = {'check_kind': 'S05_03_static_protocol_and_finite_reasoning_not_loader_execution',
              'date': datetime.now(timezone(timedelta(hours=8))).date().isoformat(),
              'status': 'passed_static_protocol_and_finite_reasoning', 'categories': 9,
              'manifest_entries': 21, 'candidate_definitions': 4, 'payload_field_contracts': field_count,
              'checked_parameter_leaf_bindings': checked_parameters, 'category_form_bindings': 9,
              'unresolved_draft_parameters': unresolved_draft_parameters,
              'adoption_steps': 8, 'consumers': 7, 'finite_reasoning_cases': examples,
              'actual_loader_or_engine_execution': None, 'actual_runtime': None, 'errors': []}
    (A / '九类协议检查结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: v for k, v in result.items() if k != 'finite_reasoning_cases'}, ensure_ascii=False))


if __name__ == '__main__':
    main()
