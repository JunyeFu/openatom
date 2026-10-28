"""Unit-check the real SDK methods with explicit command/clock fixtures; no HTTP."""
import json
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'integrations/devices'))
import virtual_device as sdk
from journal import Journal
from protocol import ProtocolError


class PhaseReportTests(unittest.TestCase):
    def device(self, state='started', duration=1200):
        self.active = dict(state=state, command_id='actual-command-fixture', payload=json.dumps({'attempt': {'phase_id': 'actual-phase-fixture'}}))
        value = sdk.Device.__new__(sdk.Device)
        value.config = dict(processing_time_ms=duration)
        value.started_ns = 1_000_000_000
        value.journal = SimpleNamespace(active=lambda: self.active)
        return value

    def test_report_uses_own_monotonic_start_and_driver_duration(self):
        value=self.device()
        with patch.object(sdk.time,'monotonic_ns',return_value=1_500_000_000):
            self.assertEqual(value.phase_observation(),dict(phase_id='actual-phase-fixture',effective_work_ms=500,duration_ms=1200))

    def test_accepted_or_unknown_recovery_clock_has_no_report(self):
        value=self.device('accepted');self.assertIsNone(value.phase_observation())
        value=self.device();value.started_ns=None;self.assertIsNone(value.phase_observation())

    def test_delayed_poll_reports_the_driver_endpoint_without_advancing_business(self):
        value=self.device()
        with patch.object(sdk.time,'monotonic_ns',return_value=3_000_000_000):
            self.assertEqual(value.phase_observation()['effective_work_ms'],1200)
        self.assertEqual(self.active['state'],'started')

    def test_instant_cycle_waits_for_its_completion_receipt(self):
        self.assertIsNone(self.device(duration=0).phase_observation())

    def test_probe_packet_keeps_legacy_absence_and_binds_report_to_live_command(self):
        self.device()
        journal=Journal.__new__(Journal)
        journal.config=dict(mode='execution',device_id='device',device_boot_id='boot',dataset_id='dataset',identity=dict(session_id='session',execution_epoch_id='epoch'))
        journal.active=lambda:self.active
        descriptor=dict(probe_id='probe')
        self.assertNotIn('phase_progress',journal.telemetry(False,descriptor))
        progress=dict(phase_id='actual-phase-fixture',effective_work_ms=500,duration_ms=1200)
        packet=journal.telemetry(False,descriptor,progress)
        self.assertEqual(packet['active_command_id'],self.active['command_id']);self.assertEqual(packet['phase_progress'],progress)
        self.active['state']='accepted'
        with self.assertRaises(ProtocolError):journal.telemetry(False,descriptor,progress)


if __name__=='__main__':unittest.main()
