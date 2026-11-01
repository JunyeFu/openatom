"""Component checks only: no GUI, shared port, cloud account, or Native runtime."""
import copy
from contextlib import closing
from io import BytesIO
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
from urllib.error import HTTPError, URLError

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))
from integrations.devices.journal import Journal, JournalError
from integrations.devices.protocol import VERSION, ProtocolError, bootstrap, encoded
from integrations.devices.virtual_device import Device, HttpClient, NoRedirects, RetryableError

OUTPUT = Path(os.environ.get("SF_DEVICE_TEST_OUTPUT", str(ROOT / ".work/v2-implementation-20261026/devices/test-output")))


def config(path, mode="execution"):
    return {"contract_version": VERSION, "mode": mode, "base_url": "https://device-component.invalid",
            "token": "private-stdin-test-token", "journal_path": str(path.resolve()),
            "device_id": "device-a", "device_boot_id": "boot-a", "dataset_id": mode + "-dataset",
            "identity": {"session_id": "session-a", "execution_epoch_id": "epoch-a",
                         "project_id": "project-a", "host_context_session_id": "host-a"} if mode == "execution" else None,
            "processing_time_ms": 35, "accept_commands": mode == "execution"}


def sample_command(c, command_id="command-a"):
    return {"contract_version": VERSION, "identity": copy.deepcopy(c["identity"]),
            "attempt": {"logical_job_id": "job-a", "phase_id": "phase-a", "attempt_ordinal": 0,
                        "command_id": command_id, "selected_binding_id": "binding-a", "phase_token": "phase-token-a"},
            "device_id": c["device_id"], "action": "process_unit", "target_device_boot_id": c["device_boot_id"],
            "parameters": {"phase_id": "phase-a", "nominal_work_ms": 0,
                           "quantity": {"value": {"coefficient": 1, "decimal_places": 0}, "unit": "piece"}}}


def sample_probe(c, suffix="1"):
    return {"probe_id": "probe-" + suffix, "device_id": c["device_id"], "device_boot_id": c["device_boot_id"],
            "issued_execution_revision": 1, "issued_at_host_ms": 1000, "expires_at_host_ms": 4000}


def wrapped(kind, data):
    return {"contract_version": VERSION, "result": {"kind": kind, "data": data}}


class ComponentCloud:
    """An explicit in-process test client, never substituted into production IPC."""
    def __init__(self, c):
        self.config = c
        self.commands = []
        self.known = {}
        self.probe = None
        self.polls = {}
        self.events = {}
        self.calls = []
        self.cancel = False
        self.lose_poll_once = False
        self.lose_event_once = False
        self.lose_lookup = False
        self.device = None

    def request(self, method, path, payload):
        j = self.device.journal
        if self.config["mode"] == "execution":
            assert not j.db.in_transaction, "network must be outside transaction"
        self.calls.append((method, path, payload))
        if path.endswith("/boots/register"):
            assert j.db.execute("SELECT identity FROM boot_state").fetchone()
            c = self.config
            return wrapped("device_boot", {"device_id": c["device_id"], "device_boot_id": c["device_boot_id"],
                           "session_id": c["identity"]["session_id"], "execution_epoch_id": c["identity"]["execution_epoch_id"],
                           "state": "active", "retired_at_published_revision": None})
        if path.endswith("/poll"):
            request = json.loads(payload)
            stored = j.db.execute("SELECT request FROM poll_requests WHERE poll_id=?", (request["poll_request_id"],)).fetchone()
            assert stored and stored[0] == payload, "poll must exist durably before send"
            key = request["poll_request_id"]
            if key not in self.polls:
                cmd = self.commands.pop(0) if request["accept_commands"] and self.commands else None
                if cmd:
                    self.known[cmd["attempt"]["command_id"]] = cmd
                self.polls[key] = {"poll_request_id": key, "state": "command" if cmd else "empty",
                                   "command": copy.deepcopy(cmd), "observation_probe": copy.deepcopy(self.probe)}
            if self.lose_poll_once:
                self.lose_poll_once = False
                raise RetryableError("test response lost after storage")
            return wrapped("device_poll", copy.deepcopy(self.polls[key]))
        if path.endswith("/events"):
            event = json.loads(payload)
            seq = event.get("observation", event)["device_sequence"]
            if self.config["mode"] == "telemetry_only":
                with closing(sqlite3.connect(Path(self.config["journal_path"]).as_uri() + "?mode=ro", uri=True)) as db:
                    row = db.execute("SELECT payload FROM event_outbox WHERE sequence=?", (seq,)).fetchone()
            else:
                row = j.db.execute("SELECT payload FROM event_outbox WHERE sequence=?", (seq,)).fetchone()
            assert row and row[0] == payload, "event must exist durably before send"
            if seq in self.events:
                assert self.events[seq][0] == payload, "event retry must be byte-identical"
            else:
                ack = {"receipt_id": "receipt-" + str(seq), "cloud_received_at": "2026-10-26T00:00:00.000000Z",
                       "storage_status": "stored", "host_admission_state": "pending" if self.config["mode"] == "execution" else "not_applicable"}
                self.events[seq] = (payload, ack)
            if self.lose_event_once:
                self.lose_event_once = False
                raise RetryableError("test response lost after storage")
            return wrapped("event_stored", self.events[seq][1])
        if method == "GET" and path.startswith("/v2/commands/"):
            if self.lose_lookup:
                raise RetryableError("test lookup unknown")
            cmd = self.known[path.rsplit("/", 1)[1]]
            return wrapped("command", {"command": copy.deepcopy(cmd),
                           "cloud_delivery": {"state": "claimed", "cancel_requested": self.cancel,
                                              "observed_at": "2026-10-26T00:00:00.000000Z"},
                           "host_confirmation": {"published_execution_revision": None, "latest_admission": None}})
        raise AssertionError("unexpected component endpoint")


class DeviceTests(unittest.TestCase):
    def setUp(self):
        OUTPUT.mkdir(parents=True, exist_ok=True)
        self.directory = Path(tempfile.mkdtemp(prefix=self._testMethodName + "-", dir=OUTPUT))
        self.devices = []

    def tearDown(self):
        for d in self.devices:
            d.close()

    def device(self, mode="execution", name="journal.sqlite"):
        c = config(self.directory / name, mode)
        cloud = ComponentCloud(c)
        d = Device(c, cloud)
        cloud.device = d
        self.devices.append(d)
        if mode == "execution":
            d.register()
        return d, cloud

    def accept(self, d, cloud):
        cloud.commands.append(sample_command(d.config))
        d.poll_due_ns = 0
        d.poll()

    def kinds(self, d):
        return [json.loads(r[0])["observation"]["receipt"]["kind"]
                for r in d.journal.db.execute("SELECT payload FROM event_outbox ORDER BY sequence")
                if "observation" in json.loads(r[0])]

    def test_persist_before_http_and_exact_event_retry_fifo(self):
        d, cloud = self.device()
        self.accept(d, cloud)
        d.progress()
        cloud.lose_event_once = True
        d.flush_one()
        self.assertEqual(d.journal.first_unsent()["sequence"], 1)
        d.retry_due.clear()
        d.flush_one()
        d.flush_one()
        calls = [json.loads(x[2])["observation"]["device_sequence"] for x in cloud.calls if x[1].endswith("/events")]
        self.assertEqual(calls, [1, 1, 2])
        raw_calls = [x[2] for x in cloud.calls if x[1].endswith("/events")]
        self.assertEqual(raw_calls[0], raw_calls[1])
        self.assertEqual(self.kinds(d), ["accepted", "started"])

    def test_unknown_empty_poll_reuses_request_and_original_probe(self):
        d, cloud = self.device()
        cloud.probe = sample_probe(d.config)
        cloud.lose_poll_once = True
        d.poll()
        first = d.journal.pending_poll()["request"]
        d.accepting = False
        cloud.probe = sample_probe(d.config, "2")
        d.retry_due.clear()
        d.poll()
        self.assertIsNone(d.journal.pending_poll())
        poll_calls = [x[2] for x in cloud.calls if x[1].endswith("/poll")]
        self.assertEqual(poll_calls, [first, first])
        row = d.journal.db.execute("SELECT payload FROM event_outbox").fetchone()
        self.assertEqual(json.loads(row[0])["observation_probe_id"], "probe-1")

    def test_execution_probe_send_clock_brackets_real_attempts_and_excludes_cooldown(self):
        d, cloud = self.device()
        cloud.probe = sample_probe(d.config)
        d.poll()
        entries = []
        original = cloud.request
        def observed(*args):
            entries.append(time.perf_counter_ns())
            return original(*args)
        cloud.lose_event_once = True
        with patch.object(cloud, "request", side_effect=observed):
            d.flush_one()
            d.flush_one()
            self.assertEqual(len(entries), 1)
            d.retry_due.clear()
            d.flush_one()
        attempts = d.journal.db.execute("SELECT attempt,started_qpc_ns,finished_qpc_ns,outcome FROM execution_send_attempts ORDER BY attempt").fetchall()
        self.assertEqual([(a[0], a[3]) for a in attempts], [(1, "transport_unknown"), (2, "response_received")])
        for entry, (_, started, finished, _) in zip(entries, attempts):
            self.assertLessEqual(started, entry)
            self.assertLessEqual(entry, finished)
        calls = [payload for _, path, payload in cloud.calls if path.endswith("/events")]
        self.assertEqual(len(calls), 2)
        self.assertEqual(calls[0], calls[1])

    def test_unknown_command_poll_does_not_claim_another_command(self):
        d, cloud = self.device()
        cloud.commands = [sample_command(d.config), sample_command(d.config, "command-b")]
        cloud.lose_poll_once = True
        d.poll()
        self.assertIsNone(d.journal.active())
        d.retry_due.clear()
        d.poll()
        self.assertEqual(d.journal.active()["command_id"], "command-a")
        self.assertEqual(len(cloud.commands), 1)
        self.assertEqual(self.kinds(d), ["accepted"])

    def test_poll_request_response_and_command_conflicts_are_immutable(self):
        d, cloud = self.device()
        self.accept(d, cloud)
        row = d.journal.db.execute("SELECT * FROM poll_requests").fetchone()
        req, res = json.loads(row["request"]), json.loads(row["response"])
        changed_req = dict(req, accept_commands=False)
        with self.assertRaises(ProtocolError):
            d.journal.prepare_poll(changed_req)
        changed_response = dict(res, observation_probe=sample_probe(d.config))
        with self.assertRaises(ProtocolError):
            d.journal.accept_poll(req, changed_response, True)
        new_req = dict(req, poll_request_id="another-poll")
        d.journal.prepare_poll(new_req)
        bad_cmd = copy.deepcopy(res["command"])
        bad_cmd["parameters"]["nominal_work_ms"] = 999
        with self.assertRaises(ProtocolError):
            d.journal.accept_poll(new_req, dict(res, poll_request_id="another-poll", command=bad_cmd), True)
        self.assertEqual(d.journal.command("command-a")["payload"], encoded(res["command"]))
        self.assertEqual(self.kinds(d), ["accepted"])

    def test_started_outbox_transaction_rolls_back_together(self):
        d, cloud = self.device()
        self.accept(d, cloud)
        before = d.journal.db.execute("SELECT next_sequence FROM boot_state").fetchone()[0]
        with patch.object(d.journal, "emit", side_effect=sqlite3.IntegrityError("test disk boundary")):
            with self.assertRaises(sqlite3.IntegrityError):
                d.progress()
        self.assertEqual(d.journal.active()["state"], "accepted")
        self.assertIsNone(d.started_ns)
        self.assertEqual(d.journal.db.execute("SELECT next_sequence FROM boot_state").fetchone()[0], before)
        self.assertEqual(self.kinds(d), ["accepted"])

    def test_cancel_before_started_has_no_action_or_completed(self):
        d, cloud = self.device()
        self.accept(d, cloud)
        cloud.cancel = True
        d.progress()
        self.assertIsNone(d.journal.active())
        self.assertIsNone(d.started_ns)
        self.assertEqual(self.kinds(d), ["accepted", "cancelled_not_started"])
        self.assertIsNone(d.journal.command("command-a")["completed_duration_ms"])

    def test_cancel_after_started_rejected_once_and_original_work_completes(self):
        d, cloud = self.device()
        d.config["processing_time_ms"] = 90
        self.accept(d, cloud)
        d.progress()
        original_start = d.started_ns
        cloud.cancel = True
        d.cancel_due_ns = 0
        d.progress()
        d.cancel_due_ns = 0
        d.progress()
        self.assertEqual(d.started_ns, original_start)
        self.assertEqual(self.kinds(d), ["accepted", "started", "cancel_rejected_started"])
        time.sleep(0.095)
        d.progress()
        self.assertEqual(self.kinds(d)[-1], "completed")
        self.assertGreaterEqual(d.journal.command("command-a")["completed_duration_ms"], 90)

    def test_device_maintenance_observation_is_independent_from_host_acceptance_and_recovers(self):
        d, cloud = self.device()
        d.maintenance = True
        cloud.probe = sample_probe(d.config, "maintenance")
        d.poll_due_ns = 0
        d.poll()
        event = json.loads(d.journal.db.execute("SELECT payload FROM event_outbox ORDER BY sequence DESC LIMIT 1").fetchone()[0])
        self.assertFalse(event["accept_new_work"])
        self.assertTrue(d.accepting)
        self.assertTrue(d.status()["maintenance"])
        d.maintenance = False
        cloud.probe = sample_probe(d.config, "recovered")
        d.poll_due_ns = 0
        d.poll()
        event = json.loads(d.journal.db.execute("SELECT payload FROM event_outbox ORDER BY sequence DESC LIMIT 1").fetchone()[0])
        self.assertTrue(event["accept_new_work"])
        self.assertTrue(d.accepting)

    def test_maintenance_completes_original_started_work_and_defers_next_command_until_recovery(self):
        d, cloud = self.device()
        self.accept(d, cloud)
        d.progress()
        original_start = d.started_ns
        d.maintenance = True
        self.assertTrue(d.accepting)
        self.assertEqual(d.journal.active()["command_id"], "command-a")
        self.assertEqual(d.started_ns, original_start)
        deadline = time.monotonic_ns() + 250_000_000
        while d.journal.active() is not None and time.monotonic_ns() < deadline:
            d.progress()
            time.sleep(0.001)
        self.assertEqual(d.journal.command("command-a")["state"], "completed")
        cloud.commands.append(sample_command(d.config, "command-b"))
        d.poll_due_ns = 0
        d.poll()
        self.assertIsNone(d.journal.command("command-b"))
        d.maintenance = False
        d.poll_due_ns = 0
        d.poll()
        self.assertEqual(d.journal.command("command-b")["state"], "accepted")
        self.assertEqual(self.kinds(d).count("completed"), 1)

    def test_real_monotonic_duration_not_nominal_or_network_ack(self):
        d, cloud = self.device()
        self.accept(d, cloud)
        before = time.monotonic_ns()
        d.progress()
        self.assertGreaterEqual(d.started_ns, before)
        d.progress()
        self.assertEqual(d.journal.active()["state"], "started")
        cloud.lose_lookup = True
        # Wait on the device's clock; Windows GetTickCount64 can tick at 15.625ms.
        deadline = d.started_ns + d.config["processing_time_ms"] * 1_000_000
        while time.monotonic_ns() < deadline:
            time.sleep(0.005)
        d.progress()
        self.assertIsNone(d.journal.active())
        row = d.journal.command("command-a")
        self.assertGreaterEqual(row["completed_duration_ms"], 35)
        self.assertEqual(self.kinds(d), ["accepted", "started", "completed"])
        # An unknown completion ACK keeps bytes pending; it never repeats the action.
        while d.journal.first_unsent()["sequence"] < 3:
            d.flush_one()
        cloud.lose_event_once = True
        d.flush_one()
        d.progress()
        d.retry_due.clear()
        d.flush_one()
        self.assertEqual(self.kinds(d), ["accepted", "started", "completed"])

    def test_prepare_equipment_and_pause_resume_keep_one_physical_action(self):
        d, cloud = self.device()
        cmd = sample_command(d.config)
        cmd["action"] = "prepare_equipment"
        cmd["parameters"]["quantity"]["unit"] = "cycle"
        cloud.commands.append(cmd)
        d.poll_due_ns = 0
        d.poll()
        d.progress()
        origin = d.started_ns
        with patch("integrations.devices.virtual_device.time.monotonic_ns", return_value=origin + 10_000_000):
            d.set_paused(True)
        with patch("integrations.devices.virtual_device.time.monotonic_ns", return_value=origin + 110_000_000):
            d.progress()
            self.assertEqual(d.journal.active()["state"], "paused")
            self.assertFalse(d.shutdown())
            d.set_paused(False)
        with patch("integrations.devices.virtual_device.time.monotonic_ns", return_value=origin + 136_000_000):
            d.progress()
        self.assertEqual(d.journal.command("command-a")["completed_duration_ms"], 36)
        self.assertEqual(self.kinds(d), ["accepted", "started", "paused", "resumed", "completed"])

    def test_factory_command_duration_controls_report_and_actual_completion(self):
        c = config(self.directory / "factory.sqlite")
        c["processing_time_source"] = "command"
        cloud = ComponentCloud(c)
        d = Device(c, cloud)
        cloud.device = d
        self.devices.append(d)
        d.register()
        cmd = sample_command(c)
        cmd["parameters"]["nominal_work_ms"] = 80
        cloud.commands.append(cmd)
        d.poll()
        d.progress()
        origin = d.started_ns
        with patch("integrations.devices.virtual_device.time.monotonic_ns", return_value=origin + 40_000_000):
            d.progress()
            self.assertEqual(d.journal.active()["state"], "started")
            self.assertEqual(d.phase_observation()["duration_ms"], 80)
            self.assertEqual(d.phase_observation()["effective_work_ms"], 40)
        with patch("integrations.devices.virtual_device.time.monotonic_ns", return_value=origin + 81_000_000):
            d.progress()
        self.assertEqual(d.journal.command("command-a")["completed_duration_ms"], 81)
        self.assertEqual(self.kinds(d), ["accepted", "started", "completed"])
        invalid = config(self.directory / "invalid.sqlite", "telemetry_only")
        invalid["processing_time_source"] = "command"
        with self.assertRaises(ProtocolError):
            bootstrap(invalid)

    def test_factory_demo_speed_keeps_real_elapsed_receipts_and_visible_phase_minimum(self):
        for nominal, duration in [(90000, 3000), (900, 1000)]:
            c = config(self.directory / f"factory-speed-{nominal}.sqlite")
            c.update(processing_time_source="command", processing_speed=30)
            cloud = ComponentCloud(c)
            d = Device(c, cloud)
            cloud.device = d
            self.devices.append(d)
            d.register()
            cmd = sample_command(c)
            cmd["parameters"]["nominal_work_ms"] = nominal
            cloud.commands.append(cmd)
            d.poll()
            d.progress()
            origin = d.started_ns
            with patch("integrations.devices.virtual_device.time.monotonic_ns", return_value=origin + (duration - 1) * 1_000_000):
                d.progress()
                self.assertEqual(d.journal.active()["state"], "started")
                self.assertEqual(d.phase_observation()["duration_ms"], duration)
            with patch("integrations.devices.virtual_device.time.monotonic_ns", return_value=origin + (duration + 1) * 1_000_000):
                d.progress()
            self.assertEqual(d.journal.command("command-a")["completed_duration_ms"], duration + 1)
            self.assertEqual(self.kinds(d), ["accepted", "started", "completed"])
        c["processing_speed"] = 0
        with self.assertRaises(ProtocolError):
            bootstrap(c)

    def test_unknown_cancel_lookup_does_not_start(self):
        d, cloud = self.device()
        self.accept(d, cloud)
        cloud.lose_lookup = True
        d.progress()
        self.assertEqual(d.journal.active()["state"], "accepted")
        self.assertIsNone(d.started_ns)
        self.assertFalse(d.shutdown())

    def test_each_new_probe_samples_current_state_and_duplicate_does_not_resample(self):
        d, cloud = self.device()
        d.accepting = False
        cloud.probe = sample_probe(d.config)
        d.poll()
        first = d.journal.db.execute("SELECT * FROM event_outbox").fetchone()
        d.accepting = True
        d.poll_due_ns = 0
        d.poll()
        self.assertEqual(d.journal.db.execute("SELECT COUNT(*) FROM event_outbox").fetchone()[0], 1)
        self.assertEqual(d.journal.db.execute("SELECT payload FROM event_outbox").fetchone()[0], first["payload"])
        cloud.probe = sample_probe(d.config, "2")
        self.accept(d, cloud)
        rows = d.journal.db.execute("SELECT payload FROM event_outbox WHERE probe_id IS NOT NULL ORDER BY sequence").fetchall()
        self.assertEqual(len(rows), 2)
        old, new = [json.loads(r[0]) for r in rows]
        self.assertEqual((old["device_state"], old["accept_new_work"]), ("idle", False))
        self.assertEqual((new["device_state"], new["active_command_id"]), ("busy", "command-a"))
        self.assertGreater(new["device_sequence"], old["device_sequence"])

    def test_probe_identity_cannot_be_rewritten(self):
        d, cloud = self.device()
        cloud.probe = sample_probe(d.config)
        d.poll()
        cloud.probe["expires_at_host_ms"] += 1000
        d.poll_due_ns = 0
        with self.assertRaises(ProtocolError):
            d.poll()
        self.assertEqual(d.journal.db.execute("SELECT COUNT(*) FROM event_outbox").fetchone()[0], 1)

    def test_busy_and_disabled_continue_probe_poll_with_accept_commands_false(self):
        d, cloud = self.device()
        self.accept(d, cloud)
        cloud.probe = sample_probe(d.config)
        d.poll_due_ns = 0
        d.poll()
        request = json.loads([x[2] for x in cloud.calls if x[1].endswith("/poll")][-1])
        self.assertFalse(request["accept_commands"])
        cloud.commands.append(sample_command(d.config, "command-b"))
        self.assertEqual(len(cloud.commands), 1)
        self.assertEqual(d.journal.db.execute("SELECT COUNT(*) FROM event_outbox WHERE probe_id IS NOT NULL").fetchone()[0], 1)

    def test_safe_shutdown_requires_no_active_unknown_poll_or_outbox(self):
        d, cloud = self.device()
        self.accept(d, cloud)
        self.assertFalse(d.shutdown())
        cloud.cancel = True
        d.progress()
        self.assertFalse(d.shutdown())
        d.flush_one()
        d.flush_one()
        self.assertTrue(d.shutdown())
        self.assertEqual(d.journal.db.execute("SELECT clean_shutdown FROM boot_state").fetchone()[0], 1)
        e, c = self.device(name="unknown-poll.sqlite")
        c.lose_poll_once = True
        e.poll()
        self.assertFalse(e.shutdown())
        e.retry_due.clear()
        e.poll()
        self.assertTrue(e.shutdown())

    def test_boot_identity_and_single_writer_in_another_process(self):
        d, cloud = self.device()
        code = "import json,sys;from integrations.devices.journal import Journal,JournalError\ntry: Journal(json.loads(sys.stdin.readline()))\nexcept JournalError: print('writer_rejected');sys.exit(7)"
        result = subprocess.run([sys.executable, "-c", code], input=json.dumps(d.config), text=True,
                                capture_output=True, cwd=ROOT, timeout=10)
        self.assertEqual(result.returncode, 7)
        self.assertEqual(result.stdout.strip(), "writer_rejected")
        d.journal.close()
        self.devices.remove(d)
        changed = dict(d.config, device_boot_id="another-boot")
        with self.assertRaisesRegex(JournalError, "identity mismatch"):
            Journal(changed)
        with self.assertRaisesRegex(JournalError, "automatic restart is forbidden"):
            Journal(d.config)

    def test_crash_retains_started_journal_and_cannot_resume(self):
        c = config(self.directory / "crashed.sqlite")
        result = subprocess.run([sys.executable, str(Path(__file__).with_name("child_component.py")), "crash"],
                                input=json.dumps(c) + "\n", text=True, capture_output=True, cwd=ROOT, timeout=10)
        self.assertEqual(result.returncode, 17)
        with closing(sqlite3.connect(c["journal_path"])) as db:
            self.assertEqual(db.execute("SELECT state FROM commands").fetchone()[0], "started")
            self.assertEqual(db.execute("SELECT COUNT(*) FROM event_outbox").fetchone()[0], 2)
            self.assertEqual(db.execute("SELECT clean_shutdown FROM boot_state").fetchone()[0], 0)
        time.sleep(0.04)
        with self.assertRaises(JournalError):
            Device(c, ComponentCloud(c))
        with closing(sqlite3.connect(c["journal_path"])) as db:
            self.assertEqual(db.execute("SELECT state FROM commands").fetchone()[0], "started")

    def test_actual_telemetry_child_samples_two_real_one_second_slots(self):
        c = config(self.directory / "telemetry.sqlite", "telemetry_only")
        started = time.monotonic()
        result = subprocess.run([sys.executable, str(Path(__file__).with_name("child_component.py")), "telemetry"],
                                input=json.dumps(c) + "\n", text=True, capture_output=True, cwd=ROOT, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertGreaterEqual(time.monotonic() - started, 2.05)
        stats = json.loads(result.stdout)
        self.assertEqual(stats["sampled_slots"], 2)
        self.assertEqual(stats["missed_slots"], 0)
        with closing(sqlite3.connect(c["journal_path"])) as db:
            rows = db.execute("SELECT slot,sampled_monotonic_ns FROM telemetry_samples ORDER BY slot").fetchall()
            self.assertEqual([r[0] for r in rows], [1, 2])
            self.assertGreaterEqual(rows[1][1] - rows[0][1], 950_000_000)
            origin = db.execute("SELECT origin_monotonic_ns FROM telemetry_clock").fetchone()[0]
            attempts = db.execute("SELECT sequence,attempt,started_monotonic_ns,finished_monotonic_ns,outcome FROM telemetry_attempts ORDER BY sequence,attempt").fetchall()
            self.assertEqual(len(attempts), 2)
            for (slot, sampled), (sequence, attempt, sent, finished, outcome) in zip(rows, attempts):
                self.assertEqual((sequence, attempt, outcome), (slot, 1, "response_received"))
                self.assertGreaterEqual(sampled - origin, slot * 1_000_000_000)
                self.assertLess(sampled - origin, (slot + 1) * 1_000_000_000)
                self.assertGreaterEqual(sent, sampled)
                self.assertLess(sent - origin, (slot + 1) * 1_000_000_000)
                self.assertGreaterEqual(finished, sent)
            events = [json.loads(r[0]) for r in db.execute("SELECT payload FROM event_outbox")]
            for event in events:
                self.assertEqual(event["source_purpose"], "telemetry_only")
                self.assertIsNone(event["session_id"])
                self.assertIsNone(event["execution_epoch_id"])
                self.assertIsNone(event["active_command_id"])
                self.assertFalse(event["accept_new_work"])
                self.assertNotIn("observation_probe_id", event)
            self.assertEqual(db.execute("SELECT COUNT(*) FROM poll_requests").fetchone()[0], 0)

    def test_telemetry_shutdown_stops_sampling_and_drains_original_unknown_event(self):
        d, cloud = self.device("telemetry_only", "telemetry-drain.sqlite")
        d.sampling_origin_ns = time.monotonic_ns() - 1_050_000_000
        d.sample_telemetry()
        original = d.journal.first_unsent()["payload"]
        cloud.lose_event_once = True
        d.flush_one()
        deadline = time.monotonic() + 1
        while not d.journal.db.execute("SELECT 1 FROM telemetry_attempts").fetchone() and time.monotonic() < deadline:
            d.flush_one()
            time.sleep(0.005)
        self.assertEqual(d.status()["pending_events"], 1)
        self.assertEqual(d.journal.db.execute("SELECT outcome FROM telemetry_attempts").fetchall()[0][0], "transport_unknown")
        d.flush_one()  # A retry cooldown is not an actual network attempt.
        self.assertEqual(d.journal.db.execute("SELECT COUNT(*) FROM telemetry_attempts").fetchone()[0], 1)
        self.assertFalse(d.shutdown())
        d.sampling_origin_ns -= 1_000_000_000  # Component elapsed-slot input, not a timing claim.
        d.telemetry_retry_due[1] = 0
        deadline = time.monotonic() + 1
        while d.status()["pending_events"] and time.monotonic() < deadline:
            d.step()
            time.sleep(0.005)
        self.assertEqual(d.status()["pending_events"], 0)
        self.assertEqual(d.status()["sampled_slots"], 1)
        self.assertEqual(d.journal.db.execute("SELECT COUNT(*) FROM telemetry_samples").fetchone()[0], 1)
        calls = [payload for _, path, payload in cloud.calls if path.endswith("/events")]
        self.assertEqual(calls, [original, original])
        self.assertEqual(len(cloud.events), 1)
        self.assertEqual([row[0] for row in d.journal.db.execute("SELECT outcome FROM telemetry_attempts ORDER BY attempt")], ["transport_unknown", "response_received"])
        self.assertTrue(d.shutdown())

    def test_slow_unknown_telemetry_does_not_block_sampling_or_first_send(self):
        c = config(self.directory / "slow-telemetry.sqlite", "telemetry_only")

        class SlowCloud(ComponentCloud):
            def __init__(self, config):
                super().__init__(config)
                self.entries = []

            def request(self, method, path, payload):
                seq = json.loads(payload)["device_sequence"]
                self.entries.append((seq, time.monotonic_ns()))
                result = super().request(method, path, payload)
                if seq == 1 and sum(s == 1 for s, _ in self.entries) == 1:
                    time.sleep(2.1)
                    raise RetryableError("test slow response lost after storage")
                return result

        cloud = SlowCloud(c)
        d = Device(c, cloud)
        cloud.device = d
        self.devices.append(d)
        deadline = d.sampling_origin_ns + 3_100_000_000
        while time.monotonic_ns() < deadline:
            d.step()
            time.sleep(0.005)
        d.shutdown()
        deadline = time.monotonic() + 3
        while not d.journal.can_shutdown() and time.monotonic() < deadline:
            d.step()
            time.sleep(0.005)
        self.assertTrue(d.shutdown())
        samples = d.journal.db.execute("SELECT slot,sequence,sampled_monotonic_ns FROM telemetry_samples ORDER BY slot").fetchall()
        self.assertEqual([(r[0], r[1]) for r in samples], [(1, 1), (2, 2), (3, 3)])
        self.assertEqual(d.missed_slots, 0)
        for slot, seq, sampled in samples:
            first_send = next(ns for s, ns in cloud.entries if s == seq)
            self.assertGreaterEqual(sampled - d.sampling_origin_ns, slot * 1_000_000_000)
            self.assertGreaterEqual(first_send, sampled)
            self.assertLess(first_send - d.sampling_origin_ns, (slot + 1) * 1_000_000_000)
        calls = [payload for _, path, payload in cloud.calls if path.endswith("/events") and json.loads(payload)["device_sequence"] == 1]
        self.assertEqual(len(calls), 2)
        self.assertEqual(calls[0], calls[1])
        self.assertEqual(len(cloud.events), 3)
        outcomes = [r[0] for r in d.journal.db.execute("SELECT outcome FROM telemetry_attempts WHERE sequence=1 ORDER BY attempt")]
        self.assertEqual(outcomes, ["transport_unknown", "response_received"])

    def test_delayed_telemetry_skips_missed_slots_instead_of_backfill(self):
        d, cloud = self.device("telemetry_only")
        # Deliberate elapsed-slot fixture, not a claimed real-time load test.
        d.sampling_origin_ns = time.monotonic_ns() - 4_100_000_000
        d.sample_telemetry()
        self.assertEqual(d.missed_slots, 3)
        self.assertEqual(d.journal.db.execute("SELECT COUNT(*) FROM event_outbox").fetchone()[0], 1)
        self.assertEqual(d.journal.db.execute("SELECT slot FROM telemetry_samples").fetchone()[0], 4)
        d.sample_telemetry()
        self.assertEqual(d.journal.db.execute("SELECT COUNT(*) FROM event_outbox").fetchone()[0], 1)

    def test_real_cli_private_stdin_idle_shutdown_and_no_secret_persistence(self):
        c = config(self.directory / "cli.sqlite", "telemetry_only")
        raw = json.dumps(c) + '\n{"action":"status"}\n{"action":"shutdown"}\n'
        p = subprocess.run([sys.executable, "-u", str(ROOT / "integrations/devices/virtual_device.py")],
                           input=raw, text=True, capture_output=True, cwd=ROOT, timeout=10)
        self.assertEqual(p.returncode, 0, p.stderr)
        lines = [json.loads(x) for x in p.stdout.splitlines()]
        self.assertEqual(lines[-1]["event"], "shutdown")
        self.assertNotIn(c["token"], p.stdout + p.stderr)
        self.assertNotIn(c["base_url"], p.stdout + p.stderr)
        raw_db = Path(c["journal_path"]).read_bytes()
        self.assertNotIn(c["token"].encode(), raw_db)
        self.assertNotIn(c["base_url"].encode(), raw_db)
        with closing(sqlite3.connect(c["journal_path"])) as db:
            self.assertEqual(db.execute("SELECT clean_shutdown FROM boot_state").fetchone()[0], 1)

    def test_bad_bootstrap_and_redirect_do_not_leak_secrets(self):
        c = config(self.directory / "unused.sqlite")
        for url in ["http://localhost", "https://a.invalid/?token=x", "https://u:p@a.invalid", "https://a.invalid/path"]:
            with self.assertRaises(ProtocolError):
                bootstrap(dict(c, base_url=url))
        with self.assertRaises(ProtocolError):
            NoRedirects().redirect_request(None, None, 302, "test", None, "https://other.invalid")
        bad = dict(c, base_url="http://private-stdin-test-token.invalid")
        p = subprocess.run([sys.executable, str(ROOT / "integrations/devices/virtual_device.py")],
                           input=json.dumps(bad) + "\n", text=True, capture_output=True, cwd=ROOT, timeout=10)
        self.assertEqual(p.returncode, 2)
        self.assertNotIn(c["token"], p.stdout + p.stderr)
        self.assertNotIn("http:", p.stdout + p.stderr)

    def test_wrong_boot_command_is_rejected_before_acceptance(self):
        d, cloud = self.device()
        bad = sample_command(d.config)
        bad["target_device_boot_id"] = "other-boot"
        cloud.commands.append(bad)
        with self.assertRaises(ProtocolError):
            d.poll()
        self.assertIsNone(d.journal.active())
        self.assertEqual(d.journal.db.execute("SELECT COUNT(*) FROM event_outbox").fetchone()[0], 0)

    def test_accepted_and_terminal_outbox_failures_keep_previous_state(self):
        d, cloud = self.device()
        cloud.commands.append(sample_command(d.config))
        with patch.object(d.journal, "emit", side_effect=sqlite3.IntegrityError("test atomic write")):
            with self.assertRaises(sqlite3.IntegrityError):
                d.poll()
        self.assertIsNone(d.journal.active())
        self.assertIsNotNone(d.journal.pending_poll())
        self.assertEqual(self.kinds(d), [])
        d.poll()  # Exact original poll response can be committed now.
        d.progress()
        deadline = d.started_ns + d.config["processing_time_ms"] * 1_000_000
        while time.monotonic_ns() < deadline:
            time.sleep(0.005)
        with patch.object(d.journal, "emit", side_effect=sqlite3.IntegrityError("test atomic write")):
            with self.assertRaises(sqlite3.IntegrityError):
                d.progress()
        self.assertEqual(d.journal.active()["state"], "started")
        self.assertEqual(self.kinds(d), ["accepted", "started"])
        d.progress()
        self.assertEqual(self.kinds(d), ["accepted", "started", "completed"])

    def test_http_client_preserves_exact_bytes_and_sanitizes_unknown_errors(self):
        c = config(self.directory / "http-unused.sqlite")
        client = HttpClient(c)
        expected = wrapped("device_poll", {"poll_request_id": "poll", "state": "empty", "command": None, "observation_probe": None})
        raw = encoded({"poll_request_id": "poll"})
        stream = BytesIO(encoded(expected))
        with patch.object(client.opener, "open", return_value=stream) as opened:
            self.assertEqual(client.request("POST", "/v2/devices/device-a/poll", raw), expected)
            req = opened.call_args.args[0]
            self.assertEqual(req.data, raw)
            self.assertEqual(req.get_header("Authorization"), "Bearer " + c["token"])
            self.assertEqual(req.get_method(), "POST")
        for err in [URLError(c["token"]), HTTPError(c["base_url"], 503, c["token"], {}, None)]:
            with patch.object(client.opener, "open", side_effect=err):
                with self.assertRaises(RetryableError) as caught:
                    client.request("POST", "/v2/devices/device-a/poll", raw)
                self.assertNotIn(c["token"], str(caught.exception))
        with patch.object(client.opener, "open", side_effect=HTTPError(c["base_url"], 403, c["token"], {}, None)):
            with self.assertRaises(ProtocolError):
                client.request("POST", "/v2/devices/device-a/poll", raw)

    def test_two_independent_execution_children_complete_their_own_real_wait(self):
        children = []
        for i, duration in enumerate([250, 400]):
            c = config(self.directory / (str(i) + ".sqlite"))
            c.update(device_id="device-" + str(i), device_boot_id="boot-" + str(i), processing_time_ms=duration)
            p = subprocess.Popen([sys.executable, str(Path(__file__).with_name("child_component.py")), "execution"],
                                 stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                 text=True, cwd=ROOT)
            p.stdin.write(json.dumps(c) + "\n")
            p.stdin.flush()
            children.append((p, c))
        for p, c in children:
            out, err = p.communicate(timeout=10)
            self.assertEqual(p.returncode, 0, err)
            data = json.loads(out)
            self.assertEqual(data["device_id"], c["device_id"])
            self.assertGreaterEqual(data["duration_ms"], c["processing_time_ms"])
            self.assertEqual(data["kinds"], ["accepted", "started", "completed"])
            with closing(sqlite3.connect(c["journal_path"])) as db:
                self.assertEqual(db.execute("SELECT clean_shutdown FROM boot_state").fetchone()[0], 1)
                self.assertEqual(db.execute("SELECT COUNT(*) FROM event_outbox WHERE acknowledgement IS NULL").fetchone()[0], 0)

    def test_emitted_wire_matches_frozen_schema_subset(self):
        d, cloud = self.device()
        cloud.probe = sample_probe(d.config)
        self.accept(d, cloud)
        d.progress()
        time.sleep(0.04)
        d.progress()
        validator = FrozenSchema()
        for row in d.journal.db.execute("SELECT payload FROM event_outbox"):
            payload = json.loads(row[0])
            validator.check(payload, "CloudReceiptRequest" if "observation" in payload else "ExecutionTelemetryObservation")
        for row in d.journal.db.execute("SELECT request FROM poll_requests"):
            validator.check(json.loads(row[0]), "DevicePollRequest")
        t, _ = self.device("telemetry_only", "schema-telemetry.sqlite")
        t.journal.sample_slot(1, time.monotonic_ns())
        validator.check(json.loads(t.journal.first_unsent()["payload"]), "TelemetryOnlyObservation")


class FrozenSchema:
    """Small test-only interpreter for the subset exercised by device messages."""
    def __init__(self):
        self.execution = json.loads((ROOT / "docs/06_v2.0设计与开发控制/接口契约.schema.design.json").read_text(encoding="utf-8-sig"))
        self.quantity = json.loads((ROOT / "packages/commands/generated/schemas/get_run.response.schema.json").read_text(encoding="utf-8-sig"))

    def check(self, value, definition):
        self.validate(value, self.execution["$defs"][definition], self.execution)

    def validate(self, value, schema, document):
        if "$ref" in schema:
            ref = schema["$ref"]
            if not ref.startswith("#"):
                document = self.quantity
            return self.validate(value, document["$defs"][ref.rsplit("/", 1)[1]], document)
        if "oneOf" in schema or "anyOf" in schema:
            matches = 0
            for branch in schema.get("oneOf", schema.get("anyOf")):
                try:
                    self.validate(value, branch, document)
                except AssertionError:
                    continue
                matches += 1
            assert matches == 1 if "oneOf" in schema else matches > 0
        if "const" in schema:
            assert type(value) is type(schema["const"]) and value == schema["const"]
        if "enum" in schema:
            assert value in schema["enum"]
        kind = schema.get("type")
        if kind == "object":
            assert isinstance(value, dict)
            assert set(schema.get("required", [])) <= set(value)
            if schema.get("additionalProperties") is False:
                assert set(value) <= set(schema["properties"])
            for k, v in value.items():
                self.validate(v, schema["properties"][k], document)
        elif kind == "string":
            assert isinstance(value, str) and len(value) >= schema.get("minLength", 0)
            if "pattern" in schema:
                import re
                assert re.search(schema["pattern"], value)
        elif kind == "integer":
            assert type(value) is int and schema.get("minimum", value) <= value <= schema.get("maximum", value)
        elif kind == "boolean":
            assert type(value) is bool
        elif kind == "null":
            assert value is None


if __name__ == "__main__":
    unittest.main()
