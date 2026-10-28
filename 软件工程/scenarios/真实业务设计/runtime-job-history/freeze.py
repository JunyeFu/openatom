"""Freeze only W01's actual Job-history continuation in the original P1-03 record."""
import datetime, json, re
from pathlib import Path

here = Path(__file__).resolve().parent
root = here.parents[2]
stdout = (here / 'engine-complete.stdout.txt').read_text(encoding='utf-8-sig')
stderr = (here / 'engine-complete.stderr.txt').read_text(encoding='utf-8-sig')
counts = [int(x) for x in re.findall(r'test result: ok\. (\d+) passed;', stdout)]
assert counts == [0, 16, 10, 27, 9, 4, 9, 18, 11, 27, 8, 12, 13, 4]
assert 'FAILED' not in stdout and 'error:' not in stderr
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
source_clock = root / 'scenarios/模板与交接/runtime/source-clock'
actual = []
for name in ['root_original', 'root_sdk_mapped', 'w5_original_source', 'w5_original_target', 'existing_seed', 'internal-pass', 'external-pass', 'internal-pending']:
    got = read(here / (name + '.actual.json'))
    old = read(source_clock / (name + '.actual.json'))
    assert got['unchanged_entire_Run'] and got['run'] == old
    assert got['job_history']
    actual.append({'case': name, 'whole_original_Run_equal': True, 'acceptance_ms': [e['at_ms'] for e in old['acceptance_events']], 'claims': len(old['resource_claims'])})
p303 = read(here / 'p303-actual-history.json')
assert p303['run'] == read(root / 'scenarios/图片时间轴/runtime/phase.bootstrap.actual.json')['formal_run']
assert [j['state'] for j in p303['observations']] == ['waiting_resource', 'waiting_output']
assert list(here.glob('actual-history-move-*/moved/history.json'))
now = datetime.datetime.now(datetime.timezone(datetime.timedelta(hours=8))).isoformat(timespec='seconds')
receipt = {
    'status': 'owned_actual_Engine_Job_history_complete_self_reviewed_frozen_pending_Core_Host',
    'owner': 'W01 / 实现窗口2', 'frozen': True, 'frozen_at': now,
    'scope_packet': 'docs/04_开发与验收/整包派发/P1-03_实际作业时点与保存历史完整续作.md',
    'handoff_ref': 'scenarios/真实业务设计/runtime-job-history/handoff.md',
    'owned_sources': ['packages/simulation/src/engine.rs', 'packages/simulation/src/job_history.rs', 'packages/simulation/src/lib.rs', 'packages/simulation/tests/job_history_units.rs'],
    'typed_signatures': ['Engine::job_history(&self) -> Vec<(SimTimeMs, JobState)>', 'Engine::jobs_at(&self, at: SimTimeMs) -> Option<Vec<JobState>>', 'sf_simulation::jobs_at_from_history(records: &[(SimTimeMs, JobState)], work: &[PhaseWorkInterval], window_start: SimTimeMs, computed_until: Option<SimTimeMs>, at: SimTimeMs) -> Option<Vec<JobState>>'],
    'existing_JobStatus': 'Working (not a new Running enum)',
    'semantics': 'Sparse actual same-instant closure records; actual same-Run effective work only; no future/nominal work, no final-Job fallback or second Engine/clock.',
    'verification': {'unique_full_current_Engine_tests': sum(counts), 'old_unchanged': 158, 'new': 10, 'failed': 0, 'counts_in_execution_order': counts, 'raw_stdout': 'engine-complete.stdout.txt', 'raw_stderr': 'engine-complete.stderr.txt', 'rustfmt_check': 'passed', 'original_actual_eight_Runs': actual, 'p303_actual_entire_Host_Run_equal': True, 'real_file_save_move_drop_Engine_reopen': True, 'actual_Host_or_Native_this_continuation': False},
    'new_test_corrections_without_production_weakening': ['Actual phase resource list includes the declared machine; only W-C must be absent in auto/wait.', 'After calendar interruption and same-instant dispatch canonical JobStatus is WaitingResource; progress stays frozen.', 'Actual P303 Host bootstrap supplies captured IDs, while preparation input_ref still names PHASE; raw input unchanged.', 'Original legacy fixture.assemble is 60000ms; 45000ms belongs to the separate phase-calendar example.'],
    'self_review': {'actual_boundaries_and_job_ids': True, 'read_results_cloned_no_mutations': True, 'resources_and_inventory_still_single_Engine': True, 'only_same_phase_actual_work_interpolated': True, 'waiting_interruption_zero_work_and_frontier_checked': True, 'original_all_owner_independent_tests_and_expectations_unchanged': True, 'all_three_original_steps_and_Q_and_release_preserved': True, 'new_public_API_DTO_or_dependency': False, 'Host_Native_other_window_source_Figma_or_listener_touched': False, 'Cargo_current_command_finished': True, 'whole_task_done': False},
    'remaining_owner_Core': ['Persist optional Job records at both existing Engine producers and keep old-history absence explicit.', 'Actual authorized Host ObjectsAt Job/Item/Resource integration and save/move/reopen checks.', 'Root/W5 actual Native projectPhase/time-axis/IX/NFR/user/release acceptance.']
}
(here / 'self-review.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
path = root / 'docs/04_开发与验收/整包派发/P1-03_整包执行.json'
progress = read(path)
assert len(progress['source_task_snapshot']['execution_plan']['steps']) == 3
assert progress['source_task_snapshot']['confirmed_question_ids'] == ['Q16', 'Q17', 'Q22', 'Q23', 'Q32', 'Q33']
assert progress['whole_task_done'] is False
progress['actual_job_history_continuation'] = receipt
path.write_text(json.dumps(progress, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'frozen_at': now, 'actual_Engine_tests': sum(counts), 'old_retained': 158, 'new': 10, 'original_complete_Runs_equal': 9, 'whole_task_done': False}, ensure_ascii=False))
