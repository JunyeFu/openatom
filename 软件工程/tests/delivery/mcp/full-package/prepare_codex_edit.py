"""Prepare a one-field real Codex edit, scoped to an isolated imported example."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--bridge', type=Path, required=True)
parser.add_argument('--stage', type=Path, required=True)
args = parser.parse_args()
subprocess.run([sys.executable, '-X', 'utf8', str(Path(__file__).with_name('prepare_codex_readonly.py')),
    '--bridge', str(args.bridge), '--stage', str(args.stage)], check=True)
file = args.stage / 'case.json'
case = json.loads(file.read_text(encoding='utf-8'))
case['authority']['operations'] += ['apply_commands']
case['authority']['edit'] = [{'scope': {'kind': 'edit', 'data': {'kind': 'project', 'data': 'P1'}},
    'branches': ['set_project_settings'], 'objects': [{'target': {'kind': 'project', 'id': 'P1'},
    'fields': [{'pointer': '/title', 'element_id': None}], 'can_remove': False}],
    'creates': [], 'source_versions': [], 'reusable_resources': []}]
file.write_text(json.dumps(case, ensure_ascii=False, indent=2), encoding='utf-8')
Path(case['prompt']).write_text('''这是用户授权的隔离验收工程，只验证当前已安装Codex的真实读取与单字段编辑。不读写本地文件，不调用shell，不改任何全局配置、审批策略、Host权限或实现源码。
通过superfactory工具：先get_context读真实project_id与修订，再get_object读取完整Project草稿。
get_object.params必须为 {"object_id":{"kind":"project","id":"替换为实际project_id"},"source":{"kind":"draft"},"node_index":null}。
只把Project.title改为“真实Codex单字段编辑验收”；其他字段原样保持。调用apply_commands，base_revision使用刚读真实修订；commands=[{"kind":"set_project_settings","data":{"title":"真实Codex单字段编辑验收","time_reference":来自实际Project,"generation_context":来自实际Project,"sources":来自实际Project}}]。
原样保留完整time_reference、generation_context、sources，不删除或补猜。禁止创建/删除任何其他对象、运行或导出。
成功后get_object读回，核实只有title变化、修订增加一次；若客户端审批拒绝则停止，不改变审批设置、不换工具规避。最后只报告真实提交/失败与实际前后修订。''', encoding='utf-8')
