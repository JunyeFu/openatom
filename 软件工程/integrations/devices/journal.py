"""Single-owner durable device journal. HTTP is always outside these transactions."""
from contextlib import contextmanager
from datetime import datetime, timezone
import json
from pathlib import Path
import sqlite3
import sys

try:
    from .protocol import VERSION, ProtocolError, encoded
except ImportError:  # Direct trusted script launch.
    from protocol import VERSION, ProtocolError, encoded


class JournalError(RuntimeError):
    pass


class Journal:
    def __init__(self, config):
        path = Path(config["journal_path"])
        if not path.is_absolute():
            raise JournalError("journal path must be absolute")
        path.parent.mkdir(parents=True, exist_ok=True)
        self.lock = path.with_suffix(path.suffix + ".writer.lock").open("a+b")
        try:
            self.lock.seek(0)
            if sys.platform == "win32":
                import msvcrt
                msvcrt.locking(self.lock.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(self.lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            self.lock.close()
            raise JournalError("journal already has a writer") from None
        self.config = config
        self.db = sqlite3.connect(path, isolation_level=None)
        self.db.row_factory = sqlite3.Row
        self.db.execute("PRAGMA synchronous=FULL")
        self.db.executescript("""
            CREATE TABLE IF NOT EXISTS boot_state (
              singleton INTEGER PRIMARY KEY CHECK(singleton=1), identity BLOB NOT NULL,
              next_sequence INTEGER NOT NULL, registered BLOB, clean_shutdown INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS poll_requests (
              poll_id TEXT PRIMARY KEY, request BLOB NOT NULL, response BLOB);
            CREATE TABLE IF NOT EXISTS commands (
              command_id TEXT PRIMARY KEY, payload BLOB NOT NULL, state TEXT NOT NULL,
              cancellation_rejected INTEGER NOT NULL DEFAULT 0, started_monotonic_ns INTEGER,
              completed_duration_ms INTEGER);
            CREATE UNIQUE INDEX IF NOT EXISTS one_active_command
              ON commands((1)) WHERE state IN ('accepted','started','paused');
            CREATE TABLE IF NOT EXISTS event_outbox (
              sequence INTEGER PRIMARY KEY, payload BLOB NOT NULL, probe_id TEXT UNIQUE,
              probe_descriptor BLOB, acknowledgement BLOB);
            CREATE TABLE IF NOT EXISTS telemetry_samples (
              slot INTEGER PRIMARY KEY, sequence INTEGER NOT NULL UNIQUE, sampled_monotonic_ns INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS telemetry_clock (
              singleton INTEGER PRIMARY KEY CHECK(singleton=1), origin_monotonic_ns INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS telemetry_attempts (
              sequence INTEGER NOT NULL, attempt INTEGER NOT NULL,
              started_monotonic_ns INTEGER NOT NULL, finished_monotonic_ns INTEGER NOT NULL,
              outcome TEXT NOT NULL, PRIMARY KEY(sequence,attempt));
            CREATE TABLE IF NOT EXISTS execution_send_attempts (
              sequence INTEGER NOT NULL, attempt INTEGER NOT NULL,
              started_qpc_ns INTEGER NOT NULL, finished_qpc_ns INTEGER NOT NULL,
              outcome TEXT NOT NULL, PRIMARY KEY(sequence,attempt));
        """)
        fixed = {k: config[k] for k in ["mode", "device_id", "device_boot_id", "dataset_id", "identity"]}
        existing = self.db.execute("SELECT identity FROM boot_state").fetchone()
        if existing:
            self.close()
            if existing["identity"] != encoded(fixed):
                raise JournalError("journal boot identity mismatch")
            raise JournalError("existing boot journal requires reconciliation; automatic restart is forbidden")
        with self.transaction():
            self.db.execute("INSERT INTO boot_state VALUES (1,?,1,NULL,0)", (encoded(fixed),))

    @contextmanager
    def transaction(self):
        self.db.execute("BEGIN IMMEDIATE")
        try:
            yield
            self.db.execute("COMMIT")
        except BaseException:
            self.db.execute("ROLLBACK")
            raise

    def close(self):
        self.db.close()
        self.lock.close()  # OS releases the exclusive lock, including on abnormal exit.

    def active(self):
        return self.db.execute("SELECT * FROM commands WHERE state IN ('accepted','started','paused')").fetchone()

    def command(self, command_id):
        return self.db.execute("SELECT * FROM commands WHERE command_id=?", (command_id,)).fetchone()

    def emit(self, payload, descriptor=None):
        """Caller holds the same transaction as the command/state change."""
        sequence = self.db.execute("SELECT next_sequence FROM boot_state").fetchone()[0]
        observation = payload.get("observation", payload)
        observation["device_sequence"] = sequence
        self.db.execute("INSERT INTO event_outbox(sequence,payload,probe_id,probe_descriptor) VALUES (?,?,?,?)",
                        (sequence, encoded(payload), descriptor["probe_id"] if descriptor else None,
                         encoded(descriptor) if descriptor else None))
        self.db.execute("UPDATE boot_state SET next_sequence=?", (sequence + 1,))
        return sequence

    def receipt(self, command, kind, duration_ms=None):
        c = self.config
        observation = {
            "session_id": c["identity"]["session_id"],
            "execution_epoch_id": c["identity"]["execution_epoch_id"],
            "command_id": command["attempt"]["command_id"], "device_id": c["device_id"],
            "phase_id": command["attempt"]["phase_id"], "phase_token": command["attempt"]["phase_token"],
            "device_boot_id": c["device_boot_id"], "device_timestamp": self.timestamp(),
            "receipt": {"kind": kind},
        }
        if kind == "completed":
            observation["receipt"]["device_reported_duration_ms"] = duration_ms
        return {"contract_version": VERSION, "observation": observation}

    @staticmethod
    def timestamp():
        return datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")

    def telemetry(self, accepting, descriptor=None, phase_progress=None):
        c = self.config
        active = self.active()
        result = {
            "contract_version": VERSION, "session_id": None, "execution_epoch_id": None,
            "device_id": c["device_id"], "device_boot_id": c["device_boot_id"],
            "device_timestamp": self.timestamp(), "accept_new_work": False,
            "device_state": "busy" if active else "idle",
            "active_command_id": active["command_id"] if active else None,
            "dataset_id": c["dataset_id"], "source_purpose": c["mode"],
        }
        if c["mode"] == "execution":
            result.update(session_id=c["identity"]["session_id"],
                          execution_epoch_id=c["identity"]["execution_epoch_id"],
                          accept_new_work=accepting and not active,
                          observation_probe_id=descriptor["probe_id"] if descriptor else None)
            if phase_progress is not None:
                if active is None or active["state"] != "started":
                    raise ProtocolError("phase report requires a live started command")
                result["phase_progress"] = phase_progress
        return result

    def pending_poll(self):
        return self.db.execute("SELECT * FROM poll_requests WHERE response IS NULL ORDER BY rowid LIMIT 1").fetchone()

    def prepare_poll(self, request):
        raw = encoded(request)
        with self.transaction():
            old = self.db.execute("SELECT * FROM poll_requests WHERE poll_id=?", (request["poll_request_id"],)).fetchone()
            if old:
                if old["request"] != raw:
                    raise ProtocolError("immutable poll request conflict")
            else:
                if self.pending_poll():
                    raise ProtocolError("previous poll outcome is unknown")
                self.db.execute("INSERT INTO poll_requests VALUES (?,?,NULL)", (request["poll_request_id"], raw))
        return raw

    def accept_poll(self, request, result, accepting, phase_progress=None):
        """Response, accepted receipt, and fresh probe sample commit together."""
        with self.transaction():
            row = self.db.execute("SELECT * FROM poll_requests WHERE poll_id=?", (request["poll_request_id"],)).fetchone()
            if not row or row["request"] != encoded(request):
                raise ProtocolError("poll request not persisted")
            if row["response"] is not None:
                if row["response"] != encoded(result):
                    raise ProtocolError("immutable poll response conflict")
                return
            command = result["command"]
            if command is not None:
                command_id = command["attempt"]["command_id"]
                old = self.command(command_id)
                if old and old["payload"] != encoded(command):
                    raise ProtocolError("immutable command payload conflict")
                if not old:
                    if self.active():
                        raise ProtocolError("device already owns an active command")
                    self.db.execute("INSERT INTO commands(command_id,payload,state) VALUES (?,?,'accepted')",
                                    (command_id, encoded(command)))
                    self.emit(self.receipt(command, "accepted"))
            descriptor = result["observation_probe"]
            if descriptor is not None:
                old = self.db.execute("SELECT probe_descriptor FROM event_outbox WHERE probe_id=?", (descriptor["probe_id"],)).fetchone()
                if old and old[0] != encoded(descriptor):
                    raise ProtocolError("immutable probe descriptor conflict")
                if not old:
                    self.emit(self.telemetry(accepting, descriptor, phase_progress), descriptor)
            self.db.execute("UPDATE poll_requests SET response=? WHERE poll_id=?",
                            (encoded(result), request["poll_request_id"]))

    def start_or_cancel(self, command_id, cancel, now_ns):
        with self.transaction():
            row = self.command(command_id)
            command = json.loads(row["payload"])
            if row["state"] == "accepted":
                state = "cancelled_not_started" if cancel else "started"
                self.db.execute("UPDATE commands SET state=?,started_monotonic_ns=? WHERE command_id=?",
                                (state, None if cancel else now_ns, command_id))
                self.emit(self.receipt(command, state))
            elif row["state"] in ("started", "paused") and cancel and not row["cancellation_rejected"]:
                self.db.execute("UPDATE commands SET cancellation_rejected=1 WHERE command_id=?", (command_id,))
                self.emit(self.receipt(command, "cancel_rejected_started"))

    def set_paused(self, command_id, paused):
        with self.transaction():
            row = self.command(command_id)
            expected = "started" if paused else "paused"
            if row["state"] != expected:
                raise ProtocolError("pause transition requires active phase")
            self.db.execute("UPDATE commands SET state=? WHERE command_id=?",
                            ("paused" if paused else "started", command_id))
            self.emit(self.receipt(json.loads(row["payload"]), "paused" if paused else "resumed"))

    def complete(self, command_id, elapsed_ms):
        with self.transaction():
            row = self.command(command_id)
            if row["state"] != "started":
                raise ProtocolError("only started work can complete")
            self.db.execute("UPDATE commands SET state='completed',completed_duration_ms=? WHERE command_id=?",
                            (elapsed_ms, command_id))
            self.emit(self.receipt(json.loads(row["payload"]), "completed", elapsed_ms))

    def sample_slot(self, slot, now_ns):
        with self.transaction():
            if self.db.execute("SELECT 1 FROM telemetry_samples WHERE slot=?", (slot,)).fetchone():
                return
            sequence = self.emit(self.telemetry(False))
            self.db.execute("INSERT INTO telemetry_samples VALUES (?,?,?)", (slot, sequence, now_ns))

    def telemetry_origin(self, origin_ns):
        self.db.execute("INSERT INTO telemetry_clock VALUES (1,?)", (origin_ns,))

    def telemetry_attempt(self, sequence, started_ns, finished_ns, outcome):
        self.db.execute("""INSERT INTO telemetry_attempts
            SELECT ?,COALESCE(MAX(attempt),0)+1,?,?,? FROM telemetry_attempts WHERE sequence=?""",
            (sequence, started_ns, finished_ns, outcome, sequence))

    def execution_send_attempt(self, sequence, started_ns, finished_ns, outcome):
        self.db.execute("""INSERT INTO execution_send_attempts
            SELECT ?,COALESCE(MAX(attempt),0)+1,?,?,? FROM execution_send_attempts WHERE sequence=?""",
            (sequence, started_ns, finished_ns, outcome, sequence))

    def first_unsent(self):
        return self.db.execute("SELECT * FROM event_outbox WHERE acknowledgement IS NULL ORDER BY sequence LIMIT 1").fetchone()

    def acknowledge(self, sequence, data):
        with self.transaction():
            row = self.first_unsent()
            if not row or row["sequence"] != sequence:
                raise ProtocolError("outbox FIFO acknowledgement mismatch")
            self.db.execute("UPDATE event_outbox SET acknowledgement=? WHERE sequence=?", (encoded(data), sequence))

    def acknowledge_telemetry(self, sequence, data):
        with self.transaction():
            row = self.db.execute("SELECT acknowledgement FROM event_outbox WHERE sequence=?", (sequence,)).fetchone()
            if row is None:
                raise ProtocolError("unknown telemetry sequence acknowledgement")
            if row["acknowledgement"] is not None:
                if row["acknowledgement"] != encoded(data):
                    raise ProtocolError("immutable telemetry acknowledgement conflict")
                return
            self.db.execute("UPDATE event_outbox SET acknowledgement=? WHERE sequence=?", (encoded(data), sequence))

    def can_shutdown(self):
        return not self.active() and not self.pending_poll() and not self.first_unsent()

    def mark_shutdown(self):
        with self.transaction():
            if not self.can_shutdown():
                raise JournalError("shutdown blocked by unsettled obligations")
            self.db.execute("UPDATE boot_state SET clean_shutdown=1")
