"""Finite observer boundaries. Fixture responses are not real-service evidence."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('fv2_short_chain', Path(__file__).with_name('fv2_short_chain.py'))
entry = importlib.util.module_from_spec(spec)
spec.loader.exec_module(entry)


class ObserverTests(unittest.TestCase):
    def test_authority_wait_reads_until_explicit_action_and_device_grants_without_controls(self):
        def state(devices):
            return {'lifecycle_state': 'prepared', 'actual': {'delivery': {'read_origin': 'host'},
                'access': {'ownership': 'current_host_owner', 'allowed_session_controls': ['run', 'stop_dispatch'],
                           'device_acceptance_control_ids': devices}}}
        flow = Mock(); flow.exit_status.side_effect = [state([]), state(['selected-device'])]
        observed = []
        with patch.object(entry.time, 'monotonic', side_effect=[0, .1, .2]), patch.object(entry.time, 'sleep'):
            result = entry.await_execution_authority(flow, {'session_id': 'original'}, 1, 'selected-device', observed.append)
        self.assertEqual(result, observed[-1]); self.assertEqual(len(observed), 2)
        flow.control.assert_not_called()

    def test_authority_timeout_and_foreign_host_never_start_or_reprepare(self):
        flow = Mock(); flow.exit_status.return_value = {'lifecycle_state': 'prepared', 'actual': {
            'delivery': {'read_origin': 'host'}, 'access': {'ownership': 'current_host_owner',
                'allowed_session_controls': [], 'device_acceptance_control_ids': []}}}
        with patch.object(entry.time, 'monotonic', side_effect=[0, 1]):
            with self.assertRaisesRegex(RuntimeError, 'no run submitted'):
                entry.await_execution_authority(flow, {'session_id': 'original'}, 1, None, lambda value: None)
        flow.exit_status.return_value['actual']['access']['ownership'] = 'other_host'
        with self.assertRaisesRegex(ValueError, 'archived sources stay read-only'):
            entry.await_execution_authority(flow, {'session_id': 'original'}, 1, None, lambda value: None)
        flow.control.assert_not_called(); flow.prepare.assert_not_called()

    def test_new_host_catalog_enables_control_without_resetting_pending(self):
        flow = entry.ExecutionWorkflow(Mock(), {'project_id': 'project'}, {'implemented_operations': []})
        original = {'request': {'request_id': 'original'}, 'handle': None, 'status': 'registration_unknown'}
        flow.pending = original
        flow.refresh_catalog({'implemented_operations': [
            {'operation': 'control_execution_session', 'allowed_scopes': [{'kind': 'project'}]},
            {'operation': 'apply_device_receipt', 'allowed_scopes': [{'kind': 'project'}]}]})
        self.assertIn('control_execution_session', flow.operations)
        self.assertNotIn('apply_device_receipt', flow.operations)
        self.assertIs(flow.pending, original)
        flow.send.assert_not_called()

    def test_discovery_uses_workspace_and_intersects_actual_tools(self):
        client = Mock()
        client.request.return_value = {'outcome': {'status': 'ok', 'data': {'implemented_operations': [
            {'operation': 'control_execution_session'}, {'operation': 'get_execution_snapshot'}]}}}
        client.rpc.return_value = {'tools': [{'name': 'superfactory_get_execution_snapshot'}]}
        catalog, _ = entry.discover(client)
        self.assertEqual(catalog['implemented_operations'], [{'operation': 'get_execution_snapshot'}])
        client.request.assert_called_once_with(None, 'describe_capabilities', {})

    def test_stop_budget_cannot_promote_unsettled_output_to_success(self):
        flow = Mock()
        actual = {'terminal_and_settled': False, 'lifecycle_state': 'stop_requested',
                  'unsettled_obligations': {'commands': [], 'pending_outputs': ['original-output'], 'reservations': []}}
        flow.exit_status.return_value = actual
        observations = []
        with patch.object(entry.time, 'monotonic', side_effect=[0, 2]):
            with self.assertRaisesRegex(RuntimeError, 'remain'):
                entry.observe(flow, {'session_id': 'original'}, 1, observations.append, until_settled=True)
        self.assertEqual(observations, [actual])
        flow.control.assert_not_called()

    def test_business_stop_waits_for_original_publication_ack(self):
        flow = Mock()
        pending = {'terminal_and_settled': True, 'publication_synchronized': False}
        synced = {'terminal_and_settled': True, 'publication_synchronized': True}
        flow.exit_status.side_effect = [pending, synced]
        observations = []
        with patch.object(entry.time, 'monotonic', side_effect=[0, .1]), patch.object(entry.time, 'sleep'):
            result = entry.observe(flow, {'session_id': 'original'}, 1, observations.append, until_settled=True)
        self.assertIs(result, synced)
        self.assertEqual(observations, [pending, synced])
        flow.control.assert_not_called()

    def test_observer_exit_never_sends_stop_or_cancel(self):
        flow = Mock()
        flow.exit_status.return_value = {'terminal_and_settled': False, 'lifecycle_state': 'running'}
        with patch.object(entry.time, 'monotonic', side_effect=[0, 2]):
            result = entry.observe(flow, {'session_id': 'original'}, 1, lambda value: None)
        self.assertEqual(result['lifecycle_state'], 'running')
        flow.control.assert_not_called()

    def test_control_conflict_is_one_submission_and_requires_new_decision(self):
        flow = Mock()
        flow.control.return_value = {'status': 'revision_conflict'}
        action = {'kind': 'stop_dispatch'}
        with self.assertRaisesRegex(RuntimeError, 'not retried'):
            entry.control(flow, {'session_id': 'original'}, action)
        flow.control.assert_called_once_with(action, confirmed=True)

    def test_cancelled_task_is_not_success(self):
        flow = Mock()
        with self.assertRaisesRegex(RuntimeError, 'cancelled'):
            entry.finish(flow, {'status': 'cancelled'}, 5)
        flow.advance.assert_not_called()

    def test_raw_path_cannot_replace_native_save_token(self):
        with tempfile.TemporaryDirectory() as temporary:
            token = Path(temporary) / 'selection.json'
            entry.write(token, {'path': 'C:/fake-destination', 'purpose': 'save_project'})
            client = Mock()
            with self.assertRaisesRegex(ValueError, 'token is accepted'):
                entry.save(client, 'original-project', token, 1)
            client.request.assert_not_called()

    def test_save_uses_exact_handle_when_optional_request_id_is_absent(self):
        with tempfile.TemporaryDirectory() as temporary:
            token = Path(temporary) / 'selection.json'
            entry.write(token, {'selection_id': 'fixture-current-session-token', 'purpose': 'save_project'})
            handle = {'id': 'original-save', 'scope': {'kind': 'workspace'}, 'kind': 'exporting', 'input_revision': 4}
            state = {'originating_operation': 'save_project', 'task': dict(handle, state='completed'),
                     'result': {'kind': 'save', 'data': {'still_dirty': False}}}
            client = Mock()
            client.request.side_effect = [
                {'context': {'project_revision': 4}},
                {'request_id': 'admitted-save', 'outcome': {'status': 'ok', 'data': handle}},
                {'outcome': {'status': 'ok', 'data': state}}]
            self.assertEqual(entry.save(client, 'project', token, 1), state)
            self.assertEqual([call.args[1] for call in client.request.call_args_list], ['get_context', 'save_project', 'get_task'])


if __name__ == '__main__':
    unittest.main()
