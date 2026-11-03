"""Submit or verify one isolated 100-event SDK batch. The operator owns service restart."""
import argparse
import json
from pathlib import Path
import statistics
import sys
import time

from local import BASE, VERSION, fixture
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from integrations.devices.virtual_device import HttpClient, RetryableError


def encode(value):
    return json.dumps(value, separators=(",", ":")).encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=["send", "disconnected", "replay-read"])
    parser.add_argument("private_directory", type=Path)
    parser.add_argument("boot", help="fresh boot ID for one batch; reuse for its next two phases")
    args = parser.parse_args()
    principals = json.loads((args.private_directory / "credentials.json").read_bytes())["principals"]
    clients = {p["principal_id"]: HttpClient({"base_url": BASE, "token": p["token"]})
               for p in principals if p["principal_id"] in ("device-principal", "reader")}
    seed = fixture("TelemetryOnlyObservation")
    events = [dict(seed, dataset_id="telemetry-fixture", device_boot_id=args.boot,
                   device_sequence=i + 1) for i in range(100)]

    def send(event):
        return clients["device-principal"].request("POST", "/v2/devices/example/events", encode(event))

    if args.phase == "send":
        samples = []
        for event in events:
            start = time.perf_counter()
            send(event)
            samples.append(1000 * (time.perf_counter() - start))
        print(json.dumps({"count": len(samples), "median_ms": statistics.median(samples),
                          "p95_ms": sorted(samples)[94]}))
    elif args.phase == "disconnected":
        try:
            send(events[-1])
        except RetryableError:
            print("Retryable disconnect observed")
        else:
            raise AssertionError("Stop the owned test service before the disconnected check")
    else:
        send(events[-1])
        query = dict(dataset="telemetry", contract_version=VERSION, cloud_project_id="example",
                     dataset_id="telemetry-fixture", device_ids=["example"], aggregation="rows",
                     received_from="2020-01-01T00:00:00.000000Z", received_until="2090-01-01T00:00:00.000000Z",
                     page=dict(limit=100, cursor=None))
        found = []
        while True:
            data = clients["reader"].request("POST", "/v2/data/query", encode(query))["result"]["data"]
            found.extend(row["observation"] for row in data["rows"]
                         if row["observation"]["device_boot_id"] == args.boot)
            query["page"]["cursor"] = data["next_cursor"]
            if not query["page"]["cursor"]:
                break
        assert sorted(found, key=lambda event: event["device_sequence"]) == events
        print("All 100 original observations read back exactly once")


if __name__ == "__main__":
    main()
