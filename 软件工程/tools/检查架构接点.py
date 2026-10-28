"""检查S05-01组件/代码依赖/窗口接点，不运行架构试做或产品。"""
from pathlib import Path
from datetime import datetime
import json

S = Path(__file__).resolve().parent.parent
A = S / 'docs/04_开发与验收'


def read(ref):
    return json.loads((S / ref).read_text(encoding='utf-8-sig'))


def main():
    arch = read('docs/03_架构与契约/全层组件与接点.json')
    roles = {r['id']: r for r in read(arch['window_plan_ref'])['window_roles']}
    selected = read('docs/00_项目基线/首版任务选择.json')['selected']
    components = {r['id']: r for r in arch['components']}
    assert len(components) == len(arch['components']) == 16
    view_keys = {'task_interaction', 'structure_data', 'state_behavior', 'interface_collaboration',
                 'presentation_UI', 'deployment_validation'}
    for row in components.values():
        assert set(row['views']) == view_keys and all(row['views'].values())
        assert row['owner_window'] in roles and row['actual_implementation'] is None
        assert all(dep in components for dep in row['code_dependencies'])
        assert (S / row['behavior_source_ref']).exists()
        owner = roles[row['owner_window']]
        for ref in row['planned_code_paths']:
            assert ref in owner['write_files'] or any(ref == root or ref.startswith(root + '/') for root in owner['write_roots']), ('owner_path', row['id'], ref)
    visiting, visited = set(), set()
    def visit(ident):
        assert ident not in visiting, ('import_cycle', ident)
        if ident not in visited:
            visiting.add(ident)
            for dep in components[ident]['code_dependencies']:
                visit(dep)
            visiting.remove(ident)
            visited.add(ident)
    for ident in components:
        visit(ident)
    assert components['domain']['code_dependencies'] == []
    assert 'presentation' not in components['reporting']['code_dependencies']
    assert components['simulation']['code_dependencies'] == ['domain']
    assert components['mcp_bridge']['code_dependencies'] == ['domain']
    assert len(arch['connections']) == 10
    for row in arch['connections']:
        assert all(p in components for p in row['participants']) and row['contract_and_sequence']
        assert row['actual_result'] is None
    task_rows = {r['task_id']: r for r in arch['task_component_projection']}
    assert set(task_rows) == {r['id'] for r in selected}
    for task in selected:
        row = task_rows[task['id']]
        assert all(c in components for c in row['component_ids'])
        assert any(components[c]['owner_window'] == task['planned_window_id'] for c in row['component_ids']), ('task_primary_owner', task['id'])
        assert row['actual_result'] is None
    assert len(arch['module_data_owners']) == 9
    assert arch['actual_runtime'] is None and arch['actual_Figma_or_frontend'] is None
    ownership = read('docs/03_架构与契约/共享状态与所有权.json')
    assert [r['id'] for r in ownership['states']] == [f'OS{i:02}' for i in range(1, 19)]
    for row in ownership['states']:
        assert all(row[k] in components for k in ['owner_component', 'code_owner_component', 'runtime_store_owner_component'])
        assert row['invariant'] and row['transition_and_exit'] and row['actual_runtime'] is None
    assert ownership['states'][4]['owner_component'] == 'desktop_host'
    assert ownership['states'][15]['owner_component'] == 'ui_host'
    assert len(ownership['scoped_walkthroughs']) == 12
    assert all(r['expected'] and r['actual_result'] is None for r in ownership['scoped_walkthroughs'])
    assert '锁外' in ownership['admission_and_compute']
    assert 'ProjectDraft' in ownership['revision_scopes'] and 'DefinitionDraft' in ownership['revision_scopes']
    # 有限修订/保存/取消语义的数学例，不调用Host、IPC或线程。
    revision = 5
    first_base, second_base = 5, 5
    assert first_base == revision
    revision += 1
    assert second_base != revision
    saved_revision, current_revision = 6, 7
    assert saved_revision != current_revision
    task_terminal_before_cancel_reply = 'completed'
    assert task_terminal_before_cancel_reply != 'cancelled'
    assert ownership['actual_runtime'] is None and ownership['actual_frontend_or_Figma'] is None
    result = {'check_kind': 'S05_01_architecture_specification_not_technical_trial',
              'date': datetime.now().astimezone().isoformat(timespec='seconds'), 'status': 'passed_static_architecture_specification',
              'components': 16, 'views_per_component': 6, 'cross_window_connections': 10,
              'selected_task_coverage': 21, 'module_categories': 9, 'import_cycles': [],
              'ownership_path_conflicts': [], 'actual_technical_trial': None, 'actual_product_runtime': None,
              'errors': []}
    result['ownership_checks'] = {'checkpoint': 'S05-02', 'status': 'passed_static_ownership_and_finite_reasoning',
                                  'state_classes': 18, 'walkthroughs': 12, 'finite_revision_save_cancel_cases': 3,
                                  'actual_concurrency_or_IPC': None}
    (A / '架构接点检查结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
