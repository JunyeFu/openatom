"""Prepare a source-labeled two-station Native/Codex read-only acceptance case.

This prepares files only. It does not make a model call or claim runtime success.
"""
import argparse
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[4]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--bridge', type=Path, required=True)
parser.add_argument('--stage', type=Path, required=True)
args = parser.parse_args()
args.stage.mkdir(parents=True, exist_ok=False)
seed = args.stage / 'seed'
seed.mkdir()
fixture = json.loads((ROOT / 'packages/commands/tests/fixtures/production/local-assembly-pack.json').read_text(encoding='utf-8'))


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding='utf-8')


def document(role, kind, identity, file, revision=None, module=None):
    return {'role': role, 'object_ref': {'kind': kind, 'id': identity}, 'revision': revision,
            'module_ref': module, 'file_ref': {'path': file, 'kind': kind}, 'required': True}


write(seed / 'active.json', {'schema_version': '1.0', 'data': fixture['project']})
write(seed / 'scenario.json', {'schema_version': '1.0', 'data': fixture['scenario']})
project = fixture['project']
entries = [document('active_project', 'project', project['id'], 'active.json', project['revision']),
           document('scenario_input', 'scenario', fixture['scenario']['id'], 'scenario.json', fixture['scenario']['revision'])]
for index, definition in enumerate(fixture['definitions']):
    name = f'module-{index}.json'
    entries.append(document('module_definition', 'module', definition['id'], name, module={'id': definition['id'], 'version': definition['version']}))
    write(seed / name, {'schema_version': '1.0', 'data': {'definition': definition, 'assets': [], 'files': [], 'dependencies': [], 'rights': [], 'issues': []}})
write(seed / 'project.json', {'schema_version': '1.0', 'data': {'project_id': project['id'], 'project_revision': project['revision'],
    'title': project['title'], 'generation': 1, 'documents': entries, 'module_refs': project['module_refs'], 'assets': [], 'missing': [], 'restart_policy': 'restart_from_initial_input'}})
prompt = args.stage / 'prompt.txt'
prompt.write_text('''本任务仅验证已安装Codex客户端与真实超级工厂MCP桥的互通，不开展软件开发、不写任何本地源或全局配置。
标准工作流原文已由启动器直接附在本次输入，禁止调用shell或其它服务代读文件。
通过实际已连接的superfactory工具完成：
1. describe_capabilities，使用workspace scope、project_id=null、api_version=0.1、唯一request_id。
2. get_context，读取真实活动工程与会话。
3. 对返回的真实project_id使用get_object读取project draft。params完整格式为 {"object_id":{"kind":"project","id":"替换为真实project_id"},"source":{"kind":"draft"},"node_index":null}；不能写object_ref或省略source。若因字段格式被拒，应按此格式纠正读取，不猜工程内容。
4. query_module_library，行业paper_carton_manufacturing、产品族die_cut_fold_glued_paper_carton、category_filters=[templates]、site_envelope=null、source_scope=[bundled]。
输入是仓库现有local-assembly-pack自建示例，不是真实工厂。不可编辑、运行、补假数据或伪称全业务验收。不要用文件中的数据替代MCP返回值。
最后中文说明实际Host/工程/修订、工具清单是否包含check_factory_draft、实际模板结果或错误；逐项保留真实失败。禁止仅阅读源码后声称连接成功。''', encoding='utf-8')
write(args.stage / 'case.json', {'root': str(ROOT), 'seed': str(seed.resolve()), 'bridge': str(args.bridge.resolve()),
    'python': sys.executable, 'prompt': str(prompt.resolve()), 'output': str((args.stage / 'actual').resolve()),
    'authority': {'project_id': project['id'], 'operations': ['describe_capabilities', 'get_context', 'get_object', 'query_module_library'],
                  'definition_drafts': [], 'allow_view_control': False, 'edit': []},
    'render_kinds': ['line_canvas', 'multi_hall_canvas'], 'deadline_seconds': 900, 'client': 'codex'})
print(args.stage / 'case.json')
