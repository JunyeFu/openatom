"""Read-only design probes; outputs are isolated audit evidence, never stage acceptance."""
from pathlib import Path
import json
import runpy

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent / 'output'


def read(ref):
    return json.loads((ROOT / ref).read_text(encoding='utf-8-sig'))


def dag(edges):
    visiting, visited = set(), set()

    def visit(key):
        if key in visiting:
            raise AssertionError(f'cycle at {key}')
        if key in visited:
            return
        visiting.add(key)
        for dep in edges[key]:
            assert dep in edges, (key, dep)
            visit(dep)
        visiting.remove(key)
        visited.add(key)

    for key in edges:
        visit(key)
    return len(visited)


def main():
    inv = read('tools/design-quality-audit/output/source-inventory.json')
    selection = read('docs/00_项目基线/首版任务选择.json')
    tasks = {t['id']: t for t in read('docs/04_开发与验收/tasks.json')['tasks']}
    selected = selection['selected']
    selected_ids = {t['id'] for t in selected}
    graphs = {key: dag({t['id']: t[key] for t in selected})
              for key in ['implementation_dependencies', 'release_dependencies']}
    release = {t['id']: t['release_dependencies'] for t in selected}
    predecessors = set()
    def add_predecessors(key):
        for dep in release[key]:
            if dep not in predecessors:
                predecessors.add(dep)
                add_predecessors(dep)
    add_predecessors('P5-01')
    assert predecessors == selected_ids - {'P5-01'}
    assert all(tasks[k]['status'] == 'planned' for k in selected_ids)
    assert len(selected_ids) == len(selected) == 21
    components = read('docs/03_架构与契约/全层组件与接点.json')
    comp = {c['id']: c for c in components['components']}
    dag({k: c['code_dependencies'] for k, c in comp.items()})
    assert comp['shared_ui']['code_dependencies'] == ['domain']
    for key in ['authoring_ui', 'packaging_ui', 'delivery_agent_ui']:
        assert 'factory_ui' not in comp[key]['code_dependencies']
    packages = read('docs/04_开发与验收/编码任务包/任务包索引.json')['packages']
    assert {p['task_id'] for p in packages} == selected_ids
    assert all(p['status'] == 'draft_not_dispatchable' and p['design_accepted'] for p in packages)
    assert all(len(p['design_view_refs']) == 6 for p in packages)
    trace = read('docs/04_开发与验收/首版需求验收追踪.json')
    case_ids = {c['id'] for c in trace['cases']}
    for req in trace['functional_requirements']:
        assert set(req['producer_tasks']) <= selected_ids
        assert set(req['case_ids']) <= case_ids
        assert req['evidence_owner_task'] in selected_ids
    operations = read('docs/03_架构与契约/公开操作与宿主调用契约.json')['operations']
    for op in operations:
        assert op['implementation_status'] == 'planned' and op['runtime_available'] is False
        for key in ['request_schema_ref', 'response_schema_ref']:
            assert (ROOT / op[key]).is_file()
    # A real existing finite recovery reference: deliberately do not invoke its main/file writer.
    reference = runpy.run_path(str(ROOT / 'packages/reporting/save_commit.reference.py'))
    restore_failure = reference['terminal'](7, 8, False, False)
    nfr = read('docs/01_业务需求/非功能需求与测量验收.json')
    nfr06 = next(r for r in nfr['requirements'] if r['id'] == 'NFR-06')
    assert restore_failure['old_root'] == 'not_claimed_intact'
    assert '失败保留旧有效工程' in nfr06['acceptance']['rule']
    gate = read('docs/04_开发与验收/完整设计与执行准入.json')
    window_md = (ROOT / 'docs/04_开发与验收/本机并行窗口计划.md').read_text(encoding='utf-8-sig')
    assert gate['design_freeze']['currently_passed_design_reviews'] == 30
    assert '当前13项文档规格通过/47项待复审' in window_md
    budget = read('docs/04_开发与验收/任务框架与工量里程碑.json')
    human = [sum(t['human_effort_hours'][i] for t in budget['tasks']) for i in (0, 1)]
    shared = [sum(t['human_effort_hours'][i] for t in budget['shared_work']) for i in (0, 1)]
    assert human == budget['totals']['selected_task_human_hours']
    assert shared == budget['totals']['shared_human_hours']
    assert [human[i] + shared[i] for i in (0, 1)] == budget['totals']['remaining_human_hours']
    aq = read('docs/04_开发与验收/架构质量抽查修复.json')
    assert aq['open_AQ_findings'] == []
    result = {
        'document_kind': 'read_only_design_audit_probes_not_product_test',
        'scope_entries': {'checkpoints': inv['checkpoint_count'], 'files': len(inv['files']),
                          'missing': inv['missing_refs']},
        'structural_checks': {'dependency_graphs_acyclic_nodes': graphs,
                              'components_acyclic': len(comp), 'shared_ui_independent': True,
                              'planned_product_tasks': len(selected), 'six_view_packages': len(packages),
                              'SF_cases_owners_resolve': len(trace['functional_requirements']),
                              'NFR_entries': len(trace['nonfunctional_requirements']),
                              'IX_entries': len(trace['integration_checks']),
                              'P5_release_predecessors': len(predecessors),
                              'NFR_required_check_groups': sum(map(len, nfr['test_execution_contract']['required_check_groups'].values())),
                              'operation_schema_pairs': len(operations),
                              'remaining_human_hours_arithmetic': budget['totals']['remaining_human_hours']},
        'confirmed_counterexamples': [
            {'key': 'current_summary_drift', 'gate_passed': 30, 'window_MD_passed': 13,
             'window_MD_next': 'S03-03', 'current_next': 'S07-01'},
            {'key': 'save_restore_failure_acceptance_branch_missing', 'reference_input': {
                'revision': 7, 'current_revision': 8, 'replace_ok': False, 'restore_ok': False},
             'actual_existing_finite_reference': restore_failure,
             'NFR06_unconditional_acceptance': nfr06['acceptance']['rule'],
             'interpretation': '旧根未恢复与旧有效备份可恢复不能混为数据丢失；NFR尚未明确此分支的打开方式、必要证据与判定。',
             'actual_Windows_fault_injection': False}],
        'prior_AQ_closed_ids': [f['id'] for f in aq['findings']],
        'semantic_verdict': 'see_30_checkpoint_manual_review_and_defect_list',
        'actual_product_runtime': None, 'source_mutations': []}
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / 'audit-probe-result.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'structural_checks': result['structural_checks'],
                      'confirmed_counterexamples': len(result['confirmed_counterexamples'])}, ensure_ascii=False))


if __name__ == '__main__':
    main()
