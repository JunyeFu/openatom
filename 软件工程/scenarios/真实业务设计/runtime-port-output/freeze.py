"""Freeze the explicitly leased Engine/Host output producer and original P1-03 record."""
import datetime, json, re
from pathlib import Path

here = Path(__file__).resolve().parent
root = here.parents[2]
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
stdout = (here / 'complete-regression.stdout.txt').read_text(encoding='utf-8-sig')
stderr = (here / 'complete-regression.stderr.txt').read_text(encoding='utf-8-sig')
counts = [int(n) for n in re.findall(r'test result: ok\. (\d+) passed;', stdout)]
programs = re.findall(r'Running .*?\((?:target[\\/]debug[\\/]deps[\\/])([^)]+)\)', stderr)
assert len(counts) == len(programs) == 36 and sum(counts) == 452
assert 'FAILED' not in stdout and 'error:' not in stderr
groups, totals, current = [], {}, ''
for program, count in zip(programs, counts):
    name = program.rsplit('-', 1)[0]
    if name in ['sf_commands', 'sf_simulation']:
        current = name
    totals[current] = totals.get(current, 0) + count
    groups.append({'package': current, 'target': name, 'passed': count})
assert totals == {'sf_commands': 280, 'sf_simulation': 172}
assert next(r['passed'] for r in groups if r['target'] == 'port_output_host') == 5
assert next(r['passed'] for r in groups if r['target'] == 'port_output_history_units') == 4
assert next(r['passed'] for r in groups if r['target'] == 'whole_ix_host') == 5
assert 'no method named `read_run_port_output`' in (here / 'host-red.stderr.txt').read_text(encoding='utf-8-sig')
host = read(here / 'p303-host.actual.json')
engine = read(here / 'p303-engine.actual.json')
expected = read(here / 'expected.json')
for row in host['observations']:
    assert [row['completed_output']['value']['coefficient'], row['accepted_output']['value']['coefficient']] == expected['P303'][str(row['at_ms'])]
assert engine['original_entire_Run_equal']
old_root = root / 'scenarios/模板与交接/runtime/source-clock'
job_history_root = root / 'scenarios/真实业务设计/runtime-job-history'
old_runs = []
for name in ['root_original', 'root_sdk_mapped', 'w5_original_source', 'w5_original_target', 'existing_seed', 'internal-pass', 'external-pass', 'internal-pending']:
    current_run = read(job_history_root / (name + '.actual.json'))['run']
    assert current_run == read(old_root / (name + '.actual.json'))
    old_runs.append({'case': name, 'entire_Run_unchanged': True})
assert read(job_history_root / 'p303-actual-history.json')['run'] == read(root / 'scenarios/图片时间轴/runtime/phase.bootstrap.actual.json')['formal_run']
now = datetime.datetime.now(datetime.timezone(datetime.timedelta(hours=8))).isoformat(timespec='seconds')
receipt = {
    'status': 'complete_owned_Engine_Host_port_output_producer_frozen_for_Root_Native_consumer',
    'owner': 'W01 / 实现窗口2', 'frozen': True, 'frozen_at': now,
    'packet_ref': 'docs/04_开发与验收/整包派发/P1-03_实际端口做完与移交来源完整续作.md',
    'handoff_ref': 'scenarios/真实业务设计/runtime-port-output/handoff.md',
    'owned_sources': ['packages/simulation/src/output_history.rs', 'packages/simulation/src/engine.rs', 'packages/simulation/src/lib.rs', 'packages/simulation/tests/port_output_history_units.rs', 'packages/commands/src/host_port_output.rs', 'packages/commands/src/project_host.rs', 'packages/commands/src/host_definition_cycle.rs', 'packages/commands/src/lib.rs', 'packages/commands/tests/port_output_host.rs'],
    'exact_typed_interface': {
        'Engine_history': 'Engine::port_output_history(&self) -> Vec<sf_simulation::PortOutputRecord>',
        'record_tuple': ['SimTimeMs actual occurrence', 'ObjectId Job', 'JobExecutionRef original execution', 'PortId actual spec ordinal', 'SubjectRevisionRef actual item type', 'Quantity frozen at occurrence', 'bool transferred(false=completed,true=transferred)', 'ObjectId actual physical item'],
        'Host_getter': 'ProjectHost::read_run_port_output(&self, session_id:&ObjectId, project_id:&ProjectId, scope:&ApiScope, scope_revision:&Revision, run_id:&RunId, at_ms:&SimTimeMs, port:&PortRef) -> Result<sf_commands::RunPortOutput, ApiError>',
        'DTO_fields': ['run_id:RunId', 'at_ms:SimTimeMs', 'port:PortRef', 'item_type_ref:Option<SubjectRevisionRef>', 'completed_output:Quantity', 'accepted_output:Quantity'],
        'accepted_output_meaning': 'actual Job-to-Location transfer, never customer/order acceptance or quality release'
    },
    'private_persistence': {'field': 'StoredRunHistory.port_outputs:Option<Vec<sf_simulation::PortOutputRecord>>', 'legacy_missing': 'None -> UnsupportedCapability', 'captured_empty': 'Some(empty) -> known zero for a legal frozen material output port', 'normal_Run': 'original save/move/freshHost history asset', 'DefinitionCycle': 'captured in original session history; no new formal device or forced Project persistence'},
    'actual_verification': {'total': 452, 'failed': 0, 'by_package': totals, 'groups': groups, 'new_tests_in_total': 9, 'original_current_tests_retained': 443, 'original_Engine_168_retained': True, 'new_Engine': 4, 'new_Host': 5, 'whole_ix_host': '5/5 original tests unchanged', 'original_eight_Runs': old_runs, 'original_P303_entire_Run_unchanged': True, 'actual_Host_save_move_fresh_reopen_and_legacy_absence': True, 'actual_scope_revision_UI_GetRun_frontier_exact_frozen_port': True, 'actual_mixed_item_types_rejected': True, 'actual_DefinitionCycle_Run_still_readable_labels_not_applicable': True, 'raw_stdout': 'complete-regression.stdout.txt', 'raw_stderr': 'complete-regression.stderr.txt', 'rustfmt_check': 'passed', 'Native_or_GUI_this_batch': False},
    'observed_red_and_repairs': [{'kind': 'missing producer', 'evidence': 'host-red.stderr.txt', 'repair': 'actual Engine facts, two Host captures, private typed getter and optional history'}, {'kind': 'new test archive setup', 'evidence': ['host-green-first.stderr.txt', 'host-targeted.stderr.txt'], 'repair': 'preserved original four assets, license, ModulePackage and rights; changed only new test seed, not product licensing or original inputs'}],
    'self_review': {'record_at_original_produce_and_actual_transfer': True, 'original_spec_ordinal_port_not_ID_parsing': True, 'quantity_type_and_item_frozen_at_occurrence': True, 'wait_retry_and_duplicate_branch_do_not_recount': True, 'preserve_quality_branch_rework_pack_bulk_multiport_tested': True, 'same_Run_units_and_types_exact_no_conversion_or_rounding': True, 'old_missing_unavailable_not_zero_or_replay': True, 'original_business_DTO_18_operations_API_domain_and_expectations_unchanged': True, 'no_new_dependency_config_lock_UI_listener_Figma_install_or_publish': True, 'all_three_original_steps_six_Q_SF_IX_NFR_release_preserved': True, 'Cargo_current_command_finished': True, 'whole_task_done': False},
    'remaining_Root': ['Native project_read_run_port_output binding using actual current UI scope/revision and exact PortRef', 'shared device.completed_output/device.accepted_output tag catalog and renderer source binding', 'actual Native/GUI/P303 whole consumer and original IX/NFR/target-machine/user/release acceptance']
}
(here / 'self-review.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
path = root / 'docs/04_开发与验收/整包派发/P1-03_整包执行.json'
progress = read(path)
assert len(progress['source_task_snapshot']['execution_plan']['steps']) == 3
assert progress['source_task_snapshot']['confirmed_question_ids'] == ['Q16', 'Q17', 'Q22', 'Q23', 'Q32', 'Q33']
assert progress['whole_task_done'] is False
progress['actual_port_output_continuation'] = receipt
path.write_text(json.dumps(progress, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'frozen_at': now, 'passed': 452, 'Engine': 172, 'Host_Commands': 280, 'new_included': 9, 'whole_task_done': False}, ensure_ascii=False))
