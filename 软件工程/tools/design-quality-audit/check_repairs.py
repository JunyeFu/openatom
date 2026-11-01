"""Verify the DQ repair's current sources and actual consumer evidence; no product gate change."""
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent / 'output'


def read(ref):
    return json.loads((ROOT / ref).read_text(encoding='utf-8-sig'))


def pointer(data, path):
    for key in path.strip('/').split('/'):
        data = data[int(key)] if isinstance(data, list) else data[key]
    return data


def main():
    gate = read('docs/04_开发与验收/完整设计与执行准入.json')
    budget = read('docs/04_开发与验收/任务框架与工量里程碑.json')
    windows = read('docs/04_开发与验收/本机并行窗口计划.json')
    maturity = read('docs/04_开发与验收/软件开发流程与当前成熟度评估.json')
    nfr = read('docs/01_业务需求/非功能需求与测量验收.json')
    f = nfr['requirements'][5]
    assert f['id'] == 'NFR-06'
    assert gate['design_freeze']['currently_passed_design_reviews'] == 30
    assert not gate['design_freeze']['currently_pending_design_reviews']
    assert gate['execution_entry']['actual_entry'] is None
    assert windows['frontend_supervision'] == maturity['frontend_supervision'] == gate['frontend_supervision']
    assert windows['latest_completion_and_followup']['final_report_pages'] == gate['final_topic_pdf']['actual_output']['actual_pages'] == 25
    for ref, forbidden in [
        ('docs/04_开发与验收/本机并行窗口计划.md', '当前13项文档规格通过/47项待复审'),
        ('docs/00_项目基线/技术与经济可行性.md', 'S03需求文档收口后当前余15项'),
        ('docs/04_开发与验收/阶段实现计划.md', 'MS-01实际用户接收尚未登记')]:
        assert forbidden not in (ROOT / ref).read_text(encoding='utf-8-sig'), ref
    totals = budget['totals']
    feasible = (ROOT / 'docs/00_项目基线/技术与经济可行性.md').read_text(encoding='utf-8-sig')
    assert f"{totals['remaining_human_hours'][0]}—{totals['remaining_human_hours'][1]}小时" in feasible
    assert 'E-DESIGN与E-TOPIC-PDF剩余预算均为0' in feasible
    source = read('docs/04_开发与验收/特征细化/DQ02_保存修复交接.json')
    for operation in source['public_json_proposal']['operations']:
        assert pointer(nfr, operation['path']) == operation['value'], operation['path']
    assert len(nfr['requirements']) == 11
    assert sum(map(len, nfr['test_execution_contract']['required_check_groups'].values())) == 38
    assert f['actual_result'] is None and f['runtime_status'] == 'not_executed'
    assert f['failure_exit_criteria']['record_binding']['runtime_wire_fields_added'] == 0
    save = read('tools/design-quality-audit/window5/output/dq02-result.json')
    branches = save['branches']
    assert {b['branch_id'] for b in branches} == set(f['failure_exit_criteria']['branch_ids'])
    for b in branches:
        assert b['terminal_reference_output']['state'] == 'failed'
        assert b['judgments']['branch_acceptance_status'] == 'pass'
        assert b['judgments']['normal_save_succeeded'] is False
        if b['branch_id'] == 'NFR06-F04':
            assert b['judgments']['project_health_status'] == 'fail'
            assert b['retained_root_open']['status'] == 'pass'
        else:
            assert b['judgments']['project_health_status'] == 'pass'
            assert b['original_root_open']['status'] == 'pass'
    assert len(save['backup_exists_but_reopen_rejected']) == 3
    assert all(r['open_evidence']['status'] == 'fail' for r in save['backup_exists_but_reopen_rejected'])
    professional = read('tools/design-quality-audit/window3/dq05-result.json')
    assert professional['failed'] == 0 and professional['passed'] == professional['case_count']
    delivery = read('docs/04_开发与验收/特征细化/DQ_S04修复验收.json')
    assert delivery['source_stable'] is True
    assert set(delivery['finding_ids']) == {'DQ-03', 'DQ-04', 'DQ-05'}
    browser = read(delivery['actual_browser_regression']['DQ_ref'])
    assert browser['completed'] and not browser['page_errors']
    assert len(browser['results']) == delivery['actual_browser_regression']['DQ_cases']
    assert all(row['passed'] for row in browser['results'])
    assert browser['evidence_kind'] == 'actual_browser_design_prototype'
    assert browser['actual_product_runtime'] is None
    interaction = read('tools/design-quality-audit/window4/output/probe-result.json')
    assert not interaction['blockers'] and interaction['cases_passed'] == interaction['cases_total']
    actual_source = (ROOT / 'docs/02_交互设计/S04_可操作设计原型/workbench.js').read_text(encoding='utf-8-sig')
    source_receipt = {'kind': 'original_DQ_independent_source_snapshot'}
    if actual_source != (ROOT / 'tools/design-quality-audit/window4/output/consumed-workbench.js').read_text(encoding='utf-8-sig'):
        # Later accepted repairs supersede the original snapshot; require their actual evidence.
        followup_ref = gate['prototype_logic_review_ref']
        followup = read(followup_ref)
        assert followup['status'] == 'completed_repair_and_browser_review'
        assert followup['summary']['open_defects'] == 0
        if followup.get('accepted_followup_ref'):
            followup_ref = followup['accepted_followup_ref']
            parallel = read(followup_ref)
            assert parallel['status'] == 'completed_repair_and_independent_review'
            assert parallel['summary']['open_findings'] == 0
            current = read(parallel['verification']['baseline_17_ref'])
            regression_ref = parallel['verification']['prior_DQ_ref']
        else:
            current = read(followup['verification']['final_ref'])
            regression_ref = followup['verification']['previous_DQ_regression_ref']
        assert current['failed'] == 0 and current['source_stable']
        source_mtime = (ROOT / followup['scope']['source_ref']).stat().st_mtime * 1000
        assert abs(current['source_mtime_completed'] - source_mtime) < 1
        regression = read(regression_ref)
        assert regression['completed'] and not regression['page_errors']
        assert len(regression['results']) == 16 and all(r['passed'] for r in regression['results'])
        source_receipt = {'kind': 'accepted_subsequent_prototype_repair', 'ref': followup_ref,
                          'current_source_sequences': len(current['cases']), 'prior_DQ_browser_cases': 16}
    cross = read('tools/design-quality-audit/window6/cross-layer-result.json')
    assert not cross['failed_ids']
    cross_receipt = read('docs/04_开发与验收/特征细化/DQ_跨层独立复核.json')
    assert cross_receipt['verdict'] == 'received'
    assert cross_receipt['final_current_source_closure']['window3_and_window4_current_source_receipts_match']
    mainline = read('docs/04_开发与验收/主线同步检查结果.json')
    assert not mainline['errors']
    selected = read('docs/00_项目基线/首版任务选择.json')['selected']
    tasks = {t['id']: t for t in read('docs/04_开发与验收/tasks.json')['tasks']}
    assert all(tasks[t['id']]['status'] == 'planned' for t in selected)
    assert all(p['status'] == 'draft_not_dispatchable' for p in read('docs/04_开发与验收/编码任务包/任务包索引.json')['packages'])
    result = {
        'status': 'passed_current_sources_and_independent_repair_evidence',
        'DQ01': 'current summaries align with gate, report and remaining budget',
        'DQ02': {'branches': len(branches), 'required_groups': 38, 'false_backup_cases_rejected': 3,
                 'four_failure_branches_normal_save_succeeded': False},
        'DQ03_DQ04': {'actual_source_sequences': interaction['cases_total'], 'blockers': []},
        'DQ05': {'actual_source_sequences': professional['case_count'], 'failed': 0},
        'actual_browser_design_prototype': {'cases': len(browser['results']), 'failed': 0},
        'current_source_receipt': source_receipt,
        'independent_sequence_scope': 'Original DQ acceptance snapshots; later source uses the accepted followup receipt when present.',
        'cross_layer_failed_ids': cross['failed_ids'], 'mainline_errors': mainline['errors'],
        'original_design_reviews': 30, 'product_tasks_planned': len(selected),
        'actual_product_runtime': None, 'actual_Windows_1176_1177_recovery': False,
        'interpretation': 'Design repair closure and finite prototype/file evidence only; product entry and real-world gates remain.'}
    (OUT / 'repair-closure-result.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
