"""Describe and freeze the actual retained Save, never construct a package by hand."""
import datetime, json, re
from pathlib import Path

here = Path(__file__).resolve().parent
root = here.parents[2]
delivery = here / 'native-input'
receipt = json.loads((delivery / 'current.json').read_text(encoding='utf-8-sig'))
project = Path(receipt['project_directory'].removeprefix('\\\\?\\')).resolve()
assert project.is_dir() and project.is_relative_to(root.resolve())
assert project.parent == delivery.resolve()
assert (project / 'project.json').is_file()
output = (here / 'native-input-targeted.stdout.txt').read_text(encoding='utf-8-sig')
assert re.findall(r'test result: ok\. (\d+) passed;', output) == ['5']
assert 'FAILED' not in output
assert receipt['fresh_Host_reads'] and receipt['physical_directory_move']
assert receipt['full_public_Run_unchanged'] and not receipt['test_TempDir_required_for_retained_package']
assert len(receipt['assets']) == 4 and all(a['same_as_original_bytes'] for a in receipt['assets'])
assert receipt['actual_Job_records'] > 0 and receipt['actual_port_records'] > 0
expected = json.loads((here / 'expected.json').read_text(encoding='utf-8-sig'))['P303']
for observation in receipt['counter_observations']:
    assert [observation['completed_output']['value']['coefficient'], observation['accepted_output']['value']['coefficient']] == expected[str(observation['at_ms'])]
job_states = {row['at_ms']: row['job'] for row in receipt['Job_observations']}
assert job_states[75000]['state'] == 'waiting_output' and job_states[75000]['finished_at_ms'] is None
assert job_states[85000]['state'] == 'completed' and job_states[85000]['finished_at_ms'] == 85000
manifest = json.loads((project / 'project.json').read_text(encoding='utf-8-sig'))['data']
assert len(manifest['documents']) > 0
for document in manifest['documents']:
    assert (project / document['file_ref']['path']).is_file()
relative = project.relative_to(delivery).as_posix()
normal_path = project.as_posix()
now = datetime.datetime.now(datetime.timezone(datetime.timedelta(hours=8))).isoformat(timespec='seconds')
review = {
    'status': 'actual_saved_moved_freshHost_native_input_complete_frozen',
    'frozen': True, 'frozen_at': now,
    'packet_ref': 'docs/04_开发与验收/整包派发/P1-03_P3-03_真实桌面输入完整交付.md',
    'native_open_directory': normal_path,
    'project_relative_to_delivery': relative,
    'actual_receipt_ref': 'scenarios/真实业务设计/runtime-port-output/native-input/current.json',
    'walkthrough_ref': 'scenarios/真实业务设计/runtime-port-output/native-input/README.md',
    'test': {'command': 'cargo test -p sf-commands --test port_output_host', 'executions_this_batch': 1, 'passed': 5, 'failed': 0, 'stdout': 'native-input-targeted.stdout.txt', 'stderr': 'native-input-targeted.stderr.txt', 'rustfmt_check': 'passed', 'original_452_not_repeated': True, 'Cargo_finished_and_released': True},
    'actual_input': receipt,
    'independent_second_location_copy': 'Same successful test copied the retained directory into another location, opened a separate fresh Host, re-read75000=1/0 and checked full Run equality.',
    'self_review': {'target_resolved_inside_project_before_writing': True, 'actual_Run_Save_physical_move_not_handwritten_archive': True, 'persistent_directory_survives_test_TempDir_cleanup': True, 'current_manifest_all_documents_present': True, 'fresh_Host_quantity_and_Job_history': True, 'four_original_exact_asset_bytes_and_rights': True, 'original_order_goal_met_Run_unchanged': True, 'production_Engine_Host_API_Native_config_or_original_asset_modified': False, 'new_listener_UI_Figma_install_publish_or_push': False, 'actual_Native_open_or_GUI': False, 'human_acceptance': None, 'whole_release_passed': False},
    'remaining': ['Root opens this actual project directory in Native, selects the stored Run and same Job at75000 then85000.', 'Root verifies shared labels, image process, actual desktop and original IX/NFR/target/user/release independently.']
}
(here / 'native-input-self-review.json').write_text(json.dumps(review, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
port = json.dumps(receipt['port'], ensure_ascii=False, indent=2)
text = f'''# 75／85秒真实桌面输入

这是现有 P303 受控输入经真实同一 Host 执行、Save、物理搬移并以 fresh Host 打开得到的工程。不是手写归档；没有补写旧 Run 历史。原始回执见[current.json](current.json)，本轮一次目标验证5/5，Cargo已结束并释放。

**打开工程目录：** [{relative}]({relative}/project.json)。Native应选择其所在目录，不能选择本说明所在的父目录。

```text
{normal_path}
```

工程标题：`{receipt['project_title']}`；Project `{receipt['project_id']}`，当前R `{receipt['current_revision']}`。
Scenario：`{receipt['scenario_id']}`。
已保存Run：`{receipt['run_id']}`；计算窗口与前沿：0—120000ms。
实际Job：`{receipt['Job_id']}`；设备：CNC；单位：piece。

输出端口必须精确使用以下PortRef，不用规则模块ID、当前latest版本或显示名称替代：

```json
{port}
```

## 最小走查

1. 打开上述保留目录，选择**已有的Run**，不要为了显示历史重新运行。定位CNC实际Job，在75000ms打开过程/表现视图。
2. 75000ms时该Job为`waiting_output`、finished_at为空，仍持有待输出物品；不是最终Completed状态。该时点端口做完量1、移交量0。
3. 同一Run、同一Job可先读74999ms的0/0，再观察75000或80000ms的1/0，最后85000ms的1/1与Job Completed。不能用final jobs补画这些时点。
4. 四图是原始align/lower/hold/return SVG及原许可；它们是表现素材，不能产生库存或决定工时。真实phase/等待/数量取同Run的Job、work和port_outputs历史。
5. 可复制**整个工程目录**到另一位置后打开。测试已实际验证另一目录fresh Host仍读75000ms=1/0、完整Run一致；不依赖原TempDir或进程缓存。

|时点|工序做完量|已移交量|实际Job状态|
|---|---:|---:|---|
|74999ms|0|0|尚未形成该输出|
|75000ms|1|0|waiting_output|
|80000ms|1|0|waiting_output|
|85000ms|1|1|completed|

本案沿原P303同源输入：0—10秒load、10—55秒auto、55—70秒等资源、70—75秒unload、75—85秒等输出接收。不要拼入另一个5/30/5、45秒结束的phase-calendar例。订单与goal_met未改；这里goal_met={str(receipt['goal_met']).lower()}是原Run的真实字段，不是为演示补造的产能目标。

## 保存内容与边界

工程保留实际输入、冻结Run、{receipt['actual_Job_records']}条真实Job变更、{receipt['actual_port_records']}条实际端口输出/移交记录、精确模块版本和四张原图及权利声明。fresh Host通过原私有getter核对全部四个时点，通过get_run读取旧Job，通过RunModuleVersion读取四图并与原文件逐字节相等。所有manifest引用文件仍在工程内。

这些结果证明受控Host输入、历史保存和可搬移读取。Native尚未打开此包，实际共享标签、图片过程、GUI、真人操作及IX/NFR/release仍由后继窗口接收。若修改方案，另存副本；已存Run仍属于它自己的冻结输入。旧包没有port_outputs时必须显示unavailable，不能用本包数据为旧Run补零。
'''
(delivery / 'README.md').write_text(text, encoding='utf-8')
progress_path = root / 'docs/04_开发与验收/整包派发/P1-03_整包执行.json'
progress = json.loads(progress_path.read_text(encoding='utf-8-sig'))
assert progress['whole_task_done'] is False
progress['actual_native_input_delivery'] = review
progress_path.write_text(json.dumps(progress, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'frozen_at': now, 'native_open_directory': normal_path, 'Run': receipt['run_id'], 'Job': receipt['Job_id'], 'targeted_passed': 5, 'Cargo_released': True, 'Native_opened': False}, ensure_ascii=False))
