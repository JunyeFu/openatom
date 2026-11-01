"""Owned short-lived subprocess used only by component unittest."""
import json
import os
from pathlib import Path
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
from test_device import ComponentCloud, config, sample_command
from integrations.devices.virtual_device import Device

c = json.loads(sys.stdin.readline())
cloud = ComponentCloud(c)
d = Device(c, cloud)
cloud.device = d
if sys.argv[1] == "crash":
    d.register()
    cloud.commands.append(sample_command(c))
    d.poll()
    d.progress()
    os._exit(17)  # Deliberately crash only this owned component-test process.
elif sys.argv[1] == "telemetry":
    end = time.monotonic() + 2.12
    while time.monotonic() < end:
        d.step()
        time.sleep(0.005)
    while not d.shutdown():
        d.step()
        time.sleep(0.005)
    print(json.dumps(d.status()))
    d.close()
elif sys.argv[1] == "execution":
    d.register()
    cloud.commands.append(sample_command(c))
    d.poll()
    while d.journal.active():
        d.step()
        time.sleep(0.005)
    while not d.shutdown():
        d.step()
        time.sleep(0.005)
    rows = [json.loads(r[0]) for r in d.journal.db.execute("SELECT payload FROM event_outbox ORDER BY sequence")]
    duration = d.journal.command("command-a")["completed_duration_ms"]
    print(json.dumps({"device_id": c["device_id"], "duration_ms": duration,
                      "kinds": [r["observation"]["receipt"]["kind"] for r in rows]}))
    d.close()
else:
    raise SystemExit(2)
