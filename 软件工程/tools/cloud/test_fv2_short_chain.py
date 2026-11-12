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
    def test_stop_budget_cannot_promote_unsettled_output_to_success(self):
        flow = Mock()
        actual = {'terminal_and_settled': False, 'lifecycle_state': 'stop_requested',
                  'unsettled_obligations': {'commands': [], 'pending_outputs': ['original-output'], 'reservations': []}}
        flow.exit_status.return_value = actual
        observations = []
        with patch.object(entry.time, 'monotonic', side_effect=[0, 2]):
            with self.assertRaisesRegex(RuntimeError, 'obligations remain'):
                entry.observe(flow, {'session_id': 'original'}, 1, observations.append, until_settled=True)
        self.assertEqual(observations, [actual])
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


if __name__ == '__main__':
    unittest.main()
