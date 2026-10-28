"""Trusted software-in-loop child: one private stdin bootstrap, then controls.

Launch: python -u integrations/devices/virtual_device.py
The command line intentionally has no URL, token, or bootstrap-file option.
"""
import json
import queue
import ssl
import sqlite3
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import HTTPRedirectHandler, HTTPSHandler, Request, build_opener
from uuid import uuid4

try:
    from .journal import Journal, JournalError
    from .protocol import VERSION, ProtocolError, bootstrap, command, encoded, fields, poll_result, response
except ImportError:
    from journal import Journal, JournalError
    from protocol import VERSION, ProtocolError, bootstrap, command, encoded, fields, poll_result, response


class RetryableError(OSError):
    """Outcome unknown: retain the original bytes and retry in this process."""


class NoRedirects(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ProtocolError("service redirects are forbidden")


class HttpClient:
    def __init__(self, config):
        self.origin = config["base_url"].rstrip("/")
        self.token = config["token"]
        self.opener = build_opener(HTTPSHandler(context=ssl.create_default_context()), NoRedirects())

    def request(self, method, path, payload=None):
        request = Request(self.origin + path, data=payload, method=method,
                          headers={"Authorization": "Bearer " + self.token,
                                   "Content-Type": "application/json", "Accept": "application/json"})
        try:
            with self.opener.open(request, timeout=2.0) as result:
                body = result.read(1024 * 1024 + 1)
        except HTTPError as e:
            status = e.code
            e.close()
            if status >= 500 or status in (408, 429):
                raise RetryableError("service outcome unknown") from None
            raise ProtocolError("service rejected request") from None
        except (URLError, TimeoutError, ConnectionError):
            raise RetryableError("service outcome unknown") from None
        if len(body) > 1024 * 1024:
            raise ProtocolError("service response too large")
        try:
            return json.loads(body)
        except (ValueError, UnicodeError):
            raise ProtocolError("invalid service JSON") from None


class Device:
    POLL_NS = 1_000_000_000
    RETRY_NS = 250_000_000

    def __init__(self, config, client=None):
        self.config = bootstrap(config)
        self.journal = Journal(config)
        self.client = client if client is not None else HttpClient(config)
        self.accepting = config["accept_commands"]
        self.maintenance = False
        self.closing = False
        self.registered = config["mode"] == "telemetry_only"
        self.poll_due_ns = 0
        self.cancel_due_ns = 0
        self.retry_due = {}
        self.started_ns = None
        self.paused_ns = None
        self.telemetry_executor = None
        if config["mode"] == "telemetry_only":
            # Two first-send lanes and a separate retry lane. Only this loop writes SQLite.
            clients = [self.client] + [HttpClient(config) if client is None else client for _ in range(2)]
            self.telemetry_executor = ThreadPoolExecutor(max_workers=3)
            self.telemetry_senders = [{"client": c, "future": None, "sequence": None} for c in clients]
            self.telemetry_attempted = set()
            self.telemetry_retry_due = {}
        self.sampling_origin_ns = time.monotonic_ns()
        if config["mode"] == "telemetry_only":
            self.journal.telemetry_origin(self.sampling_origin_ns)
        self.last_slot = 0
        self.missed_slots = 0
        self.last_error = None

    @property
    def prefix(self):
        return "/v2/devices/" + quote(self.config["device_id"], safe="")

    @property
    def accepts_new_work(self):
        return self.accepting and not self.maintenance

    def request(self, key, method, path, payload, kind):
        """Return None only for a transport-unknown outcome; no transaction spans I/O."""
        if time.monotonic_ns() < self.retry_due.get(key, 0):
            return None
        if self.journal.db.in_transaction:
            raise JournalError("HTTP cannot run inside journal transaction")
        event = json.loads(payload) if key == "event" else None
        sequence = event["device_sequence"] if event is not None and "observation_probe_id" in event else None
        started = time.perf_counter_ns() if sequence is not None else None
        try:
            data = response(self.client.request(method, path, payload), kind)
        except RetryableError:
            if sequence is not None:
                self.journal.execution_send_attempt(sequence, started, time.perf_counter_ns(), "transport_unknown")
            self.retry_due[key] = time.monotonic_ns() + self.RETRY_NS
            self.last_error = "transport_unknown"
            return None
        if sequence is not None:
            self.journal.execution_send_attempt(sequence, started, time.perf_counter_ns(), "response_received")
        self.retry_due.pop(key, None)
        self.last_error = None
        return data

    def register(self):
        c = self.config
        # boot_state already durably holds every identity field before C19.
        payload = encoded({"contract_version": VERSION,
                           "session_id": c["identity"]["session_id"],
                           "execution_epoch_id": c["identity"]["execution_epoch_id"],
                           "device_boot_id": c["device_boot_id"]})
        data = self.request("register", "POST", self.prefix + "/boots/register", payload, "device_boot")
        if data is None:
            return
        fields(data, "device_id session_id execution_epoch_id device_boot_id state retired_at_published_revision")
        if (any(data[k] != c[k] for k in ["device_id", "device_boot_id"])
                or any(data[k] != c["identity"][k] for k in ["session_id", "execution_epoch_id"])
                or data["state"] != "active" or data["retired_at_published_revision"] is not None):
            raise ProtocolError("boot registration mismatch or retired boot")
        with self.journal.transaction():
            self.journal.db.execute("UPDATE boot_state SET registered=?", (encoded(data),))
        self.registered = True

    def flush_one(self):
        if self.telemetry_executor is not None:
            self.flush_telemetry()
            return
        row = self.journal.first_unsent()
        if row is None:
            return
        data = self.request("event", "POST", self.prefix + "/events", row["payload"], "event_stored")
        if data is None:
            return
        self.check_event_ack(data)
        self.journal.acknowledge(row["sequence"], data)

    def check_event_ack(self, data):
        fields(data, "receipt_id cloud_received_at storage_status host_admission_state")
        expected = "pending" if self.config["mode"] == "execution" else "not_applicable"
        if (not isinstance(data["receipt_id"], str) or not data["receipt_id"]
                or not isinstance(data["cloud_received_at"], str) or not data["cloud_received_at"]
                or data["storage_status"] != "stored" or data["host_admission_state"] != expected):
            raise ProtocolError("invalid event acknowledgement")

    @staticmethod
    def send_telemetry(client, path, payload):
        started = time.monotonic_ns()
        try:
            data = response(client.request("POST", path, payload), "event_stored")
        except RetryableError:
            data = None
        return started, time.monotonic_ns(), data

    def flush_telemetry(self):
        for sender in self.telemetry_senders:
            future = sender["future"]
            if future is None or not future.done():
                continue
            started, finished, data = future.result()
            sequence = sender["sequence"]
            self.journal.telemetry_attempt(sequence, started, finished,
                "transport_unknown" if data is None else "response_received")
            if data is None:
                self.telemetry_retry_due[sequence] = finished + self.RETRY_NS
                self.last_error = "transport_unknown"
            else:
                self.check_event_ack(data)
                self.journal.acknowledge_telemetry(sequence, data)
                self.telemetry_retry_due.pop(sequence, None)
                self.last_error = None
            sender["future"] = None
            sender["sequence"] = None
        in_flight = {s["sequence"] for s in self.telemetry_senders if s["future"] is not None}
        rows = [r for r in self.journal.db.execute("SELECT * FROM event_outbox WHERE acknowledgement IS NULL ORDER BY sequence")
                if r["sequence"] not in in_flight]
        new_rows = [r for r in rows if r["sequence"] not in self.telemetry_attempted]
        retry_rows = [r for r in rows if r["sequence"] in self.telemetry_attempted
                      and time.monotonic_ns() >= self.telemetry_retry_due[r["sequence"]]]
        for index, sender in enumerate(self.telemetry_senders):
            candidates = new_rows if index < 2 else retry_rows
            if sender["future"] is None and candidates:
                row = candidates.pop(0)
                sender["sequence"] = row["sequence"]
                self.telemetry_attempted.add(row["sequence"])
                sender["future"] = self.telemetry_executor.submit(self.send_telemetry,
                    sender["client"], self.prefix + "/events", row["payload"])

    def close(self):
        if self.telemetry_executor is not None:
            self.telemetry_executor.shutdown(wait=True)
        self.journal.close()

    def poll(self):
        now = time.monotonic_ns()
        row = self.journal.pending_poll()
        if row is None:
            if now < self.poll_due_ns or (self.closing and not self.journal.active()):
                return
            c = self.config
            request = {"contract_version": VERSION, "session_id": c["identity"]["session_id"],
                       "execution_epoch_id": c["identity"]["execution_epoch_id"],
                       "device_boot_id": c["device_boot_id"], "poll_request_id": str(uuid4()),
                       "accept_commands": self.accepts_new_work and not bool(self.journal.active())}
            payload = self.journal.prepare_poll(request)
        else:
            payload = row["request"]
            request = json.loads(payload)
        data = self.request("poll", "POST", self.prefix + "/poll", payload, "device_poll")
        if data is None:
            return
        poll_result(data, request, self.config)
        self.journal.accept_poll(request, data, self.accepts_new_work, self.phase_observation())
        self.poll_due_ns = time.monotonic_ns() + self.POLL_NS

    def phase_observation(self):
        active = self.journal.active()
        if active is None or active["state"] != "started" or self.started_ns is None:
            return None
        duration = self.work_duration_ms(active)
        if duration == 0:
            return None
        elapsed = min(duration, (time.monotonic_ns() - self.started_ns) // 1_000_000)
        command = json.loads(active["payload"])
        return {"phase_id": command["attempt"]["phase_id"], "effective_work_ms": elapsed, "duration_ms": duration}

    def work_duration_ms(self, active):
        if self.config.get("processing_time_source") == "command":
            nominal = json.loads(active["payload"])["parameters"]["nominal_work_ms"]
            if "processing_speed" in self.config:
                speed = self.config["processing_speed"]
                return max(1000, (nominal + speed - 1) // speed)
            return nominal
        return self.config["processing_time_ms"]

    def set_paused(self, paused):
        active = self.journal.active()
        if active is None or self.started_ns is None:
            raise ProtocolError("pause transition requires active phase")
        now = time.monotonic_ns()
        self.journal.set_paused(active["command_id"], paused)
        if paused:
            self.paused_ns = now
        else:
            self.started_ns += now - self.paused_ns
            self.paused_ns = None

    def progress(self):
        active = self.journal.active()
        if active is None:
            return
        now = time.monotonic_ns()
        if active["state"] == "started":
            if self.started_ns is None:
                raise JournalError("action has no live-process monotonic start")
            elapsed_ns = now - self.started_ns
            if elapsed_ns >= self.work_duration_ms(active) * 1_000_000:
                self.journal.complete(active["command_id"], elapsed_ns // 1_000_000)
                self.started_ns = None
                return
        if now < self.cancel_due_ns:
            return
        data = self.request("cancel", "GET", "/v2/commands/" + quote(active["command_id"], safe=""), None, "command")
        if data is None:
            return  # Accepted never starts based on an unknown C14 result.
        fields(data, "command cloud_delivery host_confirmation")
        command(data["command"], self.config)
        if encoded(data["command"]) != active["payload"]:
            raise ProtocolError("immutable command lookup conflict")
        fields(data["cloud_delivery"], "state cancel_requested observed_at")
        delivery = data["cloud_delivery"]
        if (delivery["state"] not in ("queued", "claimed", "withdrawn")
                or type(delivery["cancel_requested"]) is not bool):
            raise ProtocolError("invalid cancellation response")
        if delivery["state"] != "claimed":
            raise ProtocolError("owned command is not claimed")
        # A cloud delivery flag is not a cancellation receipt. This transaction is.
        self.journal.start_or_cancel(active["command_id"], delivery["cancel_requested"], now)
        if active["state"] == "accepted" and not delivery["cancel_requested"]:
            # Timing begins AFTER Started and its outbox commit have completed.
            self.started_ns = time.monotonic_ns()
        self.cancel_due_ns = time.monotonic_ns() + self.POLL_NS

    def sample_telemetry(self):
        if self.closing:
            return
        now = time.monotonic_ns()
        slot = (now - self.sampling_origin_ns) // 1_000_000_000
        if slot > self.last_slot:
            self.missed_slots += slot - self.last_slot - 1
            self.journal.sample_slot(slot, now)
            self.last_slot = slot  # No backfill: only the state actually sampled now.

    def step(self):
        if not self.registered:
            self.register()
            return
        if self.config["mode"] == "execution":
            self.progress()
            self.poll()  # Busy and disabled devices still poll for probes.
        else:
            self.sample_telemetry()
        self.flush_one()

    def status(self):
        j = self.journal
        active = j.active()
        return {"event": "status", "device_id": self.config["device_id"],
                "device_boot_id": self.config["device_boot_id"], "registered": self.registered,
                "state": active["state"] if active else "idle", "accept_commands": self.accepting,
                "maintenance": self.maintenance,
                "shutdown_ready": self.registered and j.can_shutdown(),
                "pending_poll": bool(j.pending_poll()),
                "pending_events": j.db.execute("SELECT COUNT(*) FROM event_outbox WHERE acknowledgement IS NULL").fetchone()[0],
                "sampled_slots": self.last_slot - self.missed_slots, "missed_slots": self.missed_slots,
                "last_error": self.last_error}

    def shutdown(self):
        self.closing = True
        self.accepting = False
        if not self.registered or not self.journal.can_shutdown():
            return False
        self.journal.mark_shutdown()
        return True


def write(value):
    print(json.dumps(value, ensure_ascii=False, separators=(",", ":")), flush=True)


def read_controls(messages):
    for line in sys.stdin:
        messages.put(line)
    messages.put(None)


def main():
    if len(sys.argv) != 1:
        write({"event": "error", "code": "bootstrap_must_use_private_stdin"})
        return 2
    device = None
    try:
        first = sys.stdin.readline(65537)
        if len(first) > 65536:
            raise ProtocolError("bootstrap too large")
        config = bootstrap(json.loads(first))
        device = Device(config)
        messages = queue.Queue()
        threading.Thread(target=read_controls, args=(messages,), daemon=True).start()
        write(device.status())
        eof = False
        while True:
            try:
                control_line = messages.get_nowait()
            except queue.Empty:
                control_line = ""
            if control_line is None:
                eof = True
                device.closing = True
                device.accepting = False
            elif control_line:
                control = json.loads(control_line)
                if control == {"action": "shutdown"}:
                    if device.shutdown():
                        write({"event": "shutdown", "state": "idle", "outbox_settled": True})
                        return 0
                    write({"event": "shutdown_blocked", "status": device.status()})
                elif control == {"action": "status"}:
                    write(device.status())
                elif (isinstance(control, dict) and set(control) == {"action", "paused"}
                      and control["action"] == "set_paused" and type(control["paused"]) is bool
                      and config["mode"] == "execution" and not device.closing):
                    device.set_paused(control["paused"])
                    write(device.status())
                elif (isinstance(control, dict) and set(control) == {"action", "accept_commands"}
                      and control["action"] == "set_accept_commands"
                      and type(control["accept_commands"]) is bool
                      and config["mode"] == "execution" and not device.closing):
                    device.accepting = control["accept_commands"]
                    write(device.status())
                elif (isinstance(control, dict) and set(control) == {"action", "maintenance"}
                      and control["action"] == "set_maintenance"
                      and type(control["maintenance"]) is bool
                      and config["mode"] == "execution" and not device.closing):
                    device.maintenance = control["maintenance"]
                    write(device.status())
                else:
                    raise ProtocolError("unsupported private control")
            if eof and device.shutdown():
                write({"event": "shutdown", "state": "idle", "outbox_settled": True})
                return 0
            device.step()
            time.sleep(0.01)
    except (ProtocolError, JournalError, ValueError, UnicodeError, sqlite3.Error, OSError):
        # Keep the journal untouched for reconciliation. Never echo bootstrap,
        # HTTP exceptions, paths, or a server response into stdout/stderr.
        write({"event": "error", "code": "device_requires_reconciliation"})
        return 2
    finally:
        if device is not None:
            device.close()


if __name__ == "__main__":
    raise SystemExit(main())
