"""Run the existing authenticated Codex CLI against one explicitly selected desktop.

No global client config, API key, provider installation, Host or Engine is created.
The actual main window still authorizes the independently verified bridge peer.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


def inspect_mcp_events(path, exit_code):
    events = []
    for line in path.read_text(encoding='utf-8').splitlines():
        event = json.loads(line)
        item = event.get('item', {})
        if item.get('type') == 'mcp_tool_call':
            events.append({'type': event['type'], 'server': item.get('server'), 'tool': item.get('tool'),
                           'status': item.get('status'), 'error': item.get('error')})
    completed = [event for event in events if event['type'] == 'item.completed' and event['server'] == 'superfactory']
    observed = bool(completed)
    failed = any(event['status'] != 'completed' for event in completed)
    return {'exit_code': exit_code, 'launcher_exit_code': exit_code or (3 if failed else 0 if observed else 2),
            'observed_MCP_events': events, 'superfactory_MCP_call_observed': observed,
            'superfactory_MCP_call_failed': failed}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bridge', type=Path, required=True)
    parser.add_argument('--host-pipe', required=True)
    parser.add_argument('--host-instance-id', required=True)
    parser.add_argument('--prompt-file', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if not args.bridge.is_file() or not args.prompt_file.is_file():
        parser.error('The actual bridge executable and UTF-8 prompt file must exist')
    executable = shutil.which('codex')
    if executable is None:
        parser.error('The existing Codex CLI is not on PATH')
    environment = os.environ.copy()
    # This acceptance entry uses existing ChatGPT authorization, never paid API-key billing.
    environment.pop('OPENAI_API_KEY', None)
    environment.pop('CODEX_API_KEY', None)
    flags = {'creationflags': subprocess.CREATE_NO_WINDOW} if sys.platform == 'win32' else {}
    version = subprocess.run([executable, '--version'], env=environment, capture_output=True, text=True, encoding='utf-8', check=True, **flags)
    login = subprocess.run([executable, 'login', 'status'], env=environment, capture_output=True, text=True, encoding='utf-8', **flags)
    if login.returncode != 0 or 'Logged in using ChatGPT' not in login.stdout + login.stderr:
        parser.error('Existing ChatGPT login is required; no login/configuration was changed')
    args.output.mkdir(parents=True, exist_ok=False)
    command = [executable, 'exec', '--ignore-user-config', '--ephemeral', '--sandbox', 'read-only',
               '--skip-git-repo-check', '--json', '-C', str(Path(__file__).resolve().parents[3]),
               '-c', 'project_root_markers=["Cargo.toml"]',
               '-c', 'mcp_servers.superfactory.command=' + json.dumps(str(args.bridge.resolve())),
               '-c', 'mcp_servers.superfactory.args=' + json.dumps(['--host-pipe', args.host_pipe, '--host-instance-id', args.host_instance_id]),
               '-c', 'mcp_servers.superfactory.startup_timeout_sec=120',
               '-c', 'mcp_servers.superfactory.required=true',
               '--output-last-message', str(args.output.resolve() / 'last-message.txt'), '-']
    (args.output / 'invocation.json').write_text(json.dumps({'command': command, 'global_config_changed': False,
        'client_version': version.stdout.strip(), 'client_source': executable,
        'authentication': 'existing ChatGPT login; API-key environment removed in child only',
        'prompt_file': str(args.prompt_file.resolve()), 'desktop_authorization': 'required in actual main window'}, ensure_ascii=False, indent=2), encoding='utf-8')
    print('Codex is connecting. The Host must authorize the actual verified peer and intended scope before business calls.', flush=True)
    workflow = Path(__file__).resolve().parents[1] / 'skills/superfactory-workflow/SKILL.md'
    prompt = ('本任务的标准工作流原文由本地启动器直接提供如下，无须调用shell读文件。'
              '业务操作只使用已配置的superfactory MCP工具。工具可能延迟加载：允许先使用客户端提供的工具搜索/元数据目录，仅查找superfactory；'
              '若提供functions.exec与ALL_TOOLS，可仅检索该服务的元数据并调用相应tools方法，禁止shell、文件和其他业务服务。'
              '发现后再执行当前任务；若该服务实际初始化失败，报告失败，不改配置、不换服务替代。\n\n'
              '<workflow-source>\n' + workflow.read_text(encoding='utf-8') + '\n</workflow-source>\n\n'
              '<current-task>\n' + args.prompt_file.read_text(encoding='utf-8') + '\n</current-task>')
    (args.output / 'effective-prompt.txt').write_text(prompt, encoding='utf-8')
    with (args.output / 'events.jsonl').open('wb') as stdout, (args.output / 'stderr.txt').open('wb') as stderr:
        result = subprocess.run(command, input=prompt.encode('utf-8'), env=environment, stdout=stdout, stderr=stderr, **flags)
    receipt = {**inspect_mcp_events(args.output / 'events.jsonl', result.returncode),
               'client_version': version.stdout.strip(), 'client_source': executable,
               'real_client_execution': True, 'business_acceptance': 'inspect actual MCP requests/results and Host readback; CLI exit alone is insufficient',
               'global_config_changed': False, 'new_API_key_or_purchase': False}
    (args.output / 'client-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2), encoding='utf-8')
    print(json.dumps(receipt, ensure_ascii=False))
    return receipt['launcher_exit_code']


if __name__ == '__main__':
    raise SystemExit(main())
