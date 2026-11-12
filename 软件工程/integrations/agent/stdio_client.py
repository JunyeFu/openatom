"""Finite MCP stdio consumer of the existing, desktop-authorized Host bridge.

No Host, grants, device credentials, execution runtime or retry policy lives here.
The desktop must authorize this actual bridge PID before initialization completes.
"""
import json
import math
import os
from pathlib import Path
import queue
import subprocess
import threading
import uuid

from .workflow import execution_response, unwrap


class TransportError(RuntimeError):
    pass


class StdioClient:
    def __init__(self, bridge, endpoint, host_instance_id, *, timeout=60, stderr=None):
        if not math.isfinite(timeout) or timeout <= 0:
            raise ValueError('A positive request timeout is required')
        self.timeout = timeout
        self.next_id = 0
        self.closed = False
        self.messages = queue.Queue()
        self.process = subprocess.Popen(
            [str(Path(bridge).resolve(strict=True)), '--host-pipe', endpoint,
             '--host-instance-id', host_instance_id],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=stderr if stderr is not None else subprocess.DEVNULL,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()
        try:
            self.rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {},
                'clientInfo': {'name': 'superfactory-execution-consumer', 'version': '2.0.1'}})
            self._send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
        except BaseException:
            self.close()
            raise

    def _read(self):
        try:
            for line in self.process.stdout:
                self.messages.put(json.loads(line))
        except (ValueError, OSError) as error:
            self.messages.put(TransportError('Invalid MCP stdout: ' + str(error)))
        finally:
            self.messages.put(TransportError('MCP bridge disconnected; original submission remains unresolved'))

    def _send(self, message):
        if self.closed:
            raise TransportError('MCP connection is closed; requests are never replayed')
        try:
            self.process.stdin.write(json.dumps(message, ensure_ascii=False, separators=(',', ':')).encode('utf-8') + b'\n')
            self.process.stdin.flush()
        except OSError as error:
            self.close()
            raise TransportError('MCP write failed; original submission remains unresolved') from error

    def rpc(self, method, params):
        self.next_id += 1
        identity = self.next_id
        self._send({'jsonrpc': '2.0', 'id': identity, 'method': method, 'params': params})
        # This synchronous consumer has one outstanding request. Notifications do
        # not extend the absolute budget and cannot hide a lost response forever.
        import time
        deadline = time.monotonic() + self.timeout
        try:
            while True:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise queue.Empty
                message = self.messages.get(timeout=remaining)
                if isinstance(message, Exception):
                    raise message
                if not isinstance(message, dict) or message.get('jsonrpc') != '2.0':
                    raise TransportError('Invalid JSON-RPC response')
                if 'id' not in message:
                    if 'method' not in message:
                        raise TransportError('Invalid MCP notification')
                    continue
                if message['id'] != identity:
                    raise TransportError('MCP response ID differs from the original request')
                if 'error' in message:
                    raise TransportError('MCP request rejected: ' + json.dumps(message['error'], ensure_ascii=False))
                if 'result' not in message:
                    raise TransportError('MCP response has no result')
                return message['result']
        except queue.Empty as error:
            self.close()
            raise TransportError('MCP response timed out; original submission remains unresolved') from error
        except (TransportError, OSError):
            self.close()
            raise

    def send(self, request):
        return self.rpc('tools/call', {'name': 'superfactory_' + request['operation'],
                                      'arguments': request})

    def request(self, project_id, operation, params):
        request = {'api_version': '0.1', 'request_id': 'agent-' + str(uuid.uuid4()),
                   'project_id': project_id, 'scope': {'kind': 'project' if project_id else 'workspace'},
                   'operation': operation, 'params': params}
        response = execution_response(self.send(request))
        if response['request_id'] != request['request_id'] or response['api_version'] != request['api_version']:
            raise ValueError('Host Response identity differs from the original request')
        unwrap(response)
        return response

    def close(self):
        if self.closed:
            return
        self.closed = True
        try:
            self.process.stdin.close()
        except OSError:
            pass
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            self.process.wait(timeout=5)
        self.reader.join(timeout=1)
        self.process.stdout.close()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()
