"""Check actual authorized creation of a required nonempty calendar, not a mock Host."""
import copy
import json
import concurrent_client as protocol

client = protocol.StdioClient()
failure = None
try:
    initial = protocol.project(client)
    calendar = copy.deepcopy(initial['calendars'][0])
    calendar['id'] = 'P304-CALENDAR'
    calendar['title'] = 'P304 explicit calendar copied from the declared example'
    reply = client.call('apply_commands', {'base_revision': initial['revision'], 'commands': [
        {'kind': 'put_object', 'data': {'kind': 'calendar', 'data': calendar}}]})
    protocol.data(reply)
    current = protocol.project(client)
    protocol.record('authorized_actual_nonempty_calendar_created', any(c['id'] == calendar['id'] for c in current['calendars']))
except Exception as error:
    failure = repr(error)
    raise
finally:
    (protocol.output / 'creation-probe-result.json').write_text(json.dumps({'checks': protocol.checks,
        'failure': failure, 'expected_success': True, 'actual_Native_Host': True,
        'actual_external_stdio': True, 'Codex_client': False}, ensure_ascii=False, indent=2), encoding='utf-8')
    client.close()
