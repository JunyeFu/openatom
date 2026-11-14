"""Real subprocess/framing checks with a protocol fixture, not Host acceptance."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from integrations.agent.stdio_client import StdioClient, TransportError
from integrations.agent.workflow import ExecutionWorkflow

PEER = '''import json,sys,time
mode,log=sys.argv[1:]
for line in sys.stdin:
    request=json.loads(line)
    with open(log,'a',encoding='utf-8') as f:f.write(json.dumps(request)+'\\n')
    if 'id' not in request:continue
    if request['method']=='tools/call':
        if mode=='disconnect':break
        if mode=='timeout':
            time.sleep(.3)
            continue
        if mode=='malformed':
            print('bad stdout',flush=True)
            continue
        if mode=='notification':print(json.dumps({'jsonrpc':'2.0','method':'notifications/tools/list_changed'}),flush=True)
    identity=request['id']+(1 if mode=='wrong_id' and request['method']=='tools/call' else 0)
    result={'echo':request.get('params')}
    print(json.dumps({'jsonrpc':'2.0','id':identity,'result':result}),flush=True)
'''


class StdioTests(unittest.TestCase):
    def client(self, root, mode='ok', timeout=2):
        peer = root / 'peer.py'
        peer.write_text(PEER, encoding='utf-8')
        log = root / 'requests.jsonl'
        original = subprocess.Popen
        def launch(command, **kwargs):
            self.assertEqual(command[1:], ['--host-pipe', 'actual-pipe', '--host-instance-id', 'actual-host'])
            return original([sys.executable, '-X', 'utf8', '-u', str(peer), mode, str(log)], **kwargs)
        with patch('integrations.agent.stdio_client.subprocess.Popen', side_effect=launch):
            client = StdioClient(sys.executable, 'actual-pipe', 'actual-host', timeout=2)
            client.timeout = timeout
        return client, log

    def test_complete_canonical_request_and_notifications(self):
        with tempfile.TemporaryDirectory() as temporary:
            client, log = self.client(Path(temporary), 'notification')
            with client:
                request = {'operation': 'read_execution_view', 'request_id': 'original', 'params': {'unicode': '运输'}}
                result = client.send(request)
                self.assertEqual(result['echo']['arguments'], request)
                self.assertEqual(result['echo']['name'], 'superfactory_read_execution_view')
            messages = [json.loads(line) for line in log.read_text(encoding='utf-8').splitlines()]
            self.assertEqual([m['method'] for m in messages], ['initialize', 'notifications/initialized', 'tools/call'])
            self.assertIsNotNone(client.process.poll())

    def test_transport_failure_never_replays_original_admission(self):
        for mode in ('disconnect', 'timeout', 'wrong_id', 'malformed'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary:
                client, log = self.client(Path(temporary), mode, timeout=.15)
                flow = ExecutionWorkflow(client.send, {'project_id': 'project'},
                    {'implemented_operations': [{'operation': 'read_cloud_production', 'allowed_scopes': [{'kind': 'project'}]}]})
                with client:
                    with self.assertRaises(TransportError):
                        flow.cloud({'kind': 'dataset_catalog'})
                    self.assertEqual(flow.pending['status'], 'registration_unknown')
                    original = flow.pending['request']
                    with self.assertRaisesRegex(ValueError, 'no duplicate submission'):
                        flow.cloud({'kind': 'dataset_catalog'})
                messages = [json.loads(line) for line in log.read_text(encoding='utf-8').splitlines()]
                calls = [m for m in messages if m['method'] == 'tools/call']
                self.assertEqual(len(calls), 1)
                self.assertEqual(calls[0]['params']['arguments'], original)
                self.assertIsNotNone(client.process.poll())

    def test_closed_connection_rejects_new_calls(self):
        with tempfile.TemporaryDirectory() as temporary:
            client, _ = self.client(Path(temporary))
            client.close()
            client.close()
            with self.assertRaisesRegex(TransportError, 'closed'):
                client.rpc('tools/list', {})


if __name__ == '__main__':
    unittest.main()
