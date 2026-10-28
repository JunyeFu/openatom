"""Independently receive one actual Native telemetry campaign and its hosted PG rows."""
import argparse
import collections
from contextlib import closing
import csv
import datetime as dt
import json
from pathlib import Path
import sqlite3
import subprocess

from hosted_scale import micros, write


def verify(args):
    out = args.campaign
    manifest = json.loads((out / 'native-result.json').read_text(encoding='utf-8'))
    count = manifest['required_monotonic_slots_per_child']
    children = manifest['children']
    assert manifest['actual_native_lifecycle_completed'] and not manifest['failed_or_aborted']
    assert len(children) == 10 and len({c['boot_id'] for c in children}) == 10
    assert len({(c['pid'], c['windows_creation_time_100ns'], c['spawn_sequence']) for c in children}) == 10
    expected = {}
    receipts = []
    for child in children:
        assert child['lifecycle'] == 'ExitedClean' and child['exit_code'] == 0 and child['shutdown_acknowledged']
        assert child['fault'] is None and child['status']['missed_slots'] == 0
        assert child['status']['pending_events'] == 0 and child['status']['shutdown_ready']
        journal = Path(child['journal_path'].removeprefix('\\\\?\\'))
        with closing(sqlite3.connect(journal.as_uri() + '?mode=ro', uri=True)) as db:
            origin = db.execute('SELECT origin_monotonic_ns FROM telemetry_clock').fetchone()[0]
            assert db.execute('SELECT clean_shutdown FROM boot_state').fetchone()[0] == 1
            assert db.execute('SELECT COUNT(*) FROM commands').fetchone()[0] == 0
            assert db.execute('SELECT COUNT(*) FROM poll_requests').fetchone()[0] == 0
            rows = db.execute('SELECT slot,sequence,sampled_monotonic_ns FROM telemetry_samples ORDER BY slot').fetchall()
            assert [(s, q) for s, q, _ in rows if s <= count] == [(n, n) for n in range(1, count + 1)]
            first_send_lags = []
            retries = 0
            for slot, sequence, sampled in rows:
                attempts = db.execute('SELECT attempt,started_monotonic_ns,finished_monotonic_ns,outcome FROM telemetry_attempts WHERE sequence=? ORDER BY attempt', (sequence,)).fetchall()
                assert attempts and [a[0] for a in attempts] == list(range(1, len(attempts) + 1))
                assert all(a[2] >= a[1] >= sampled for a in attempts)
                assert attempts[-1][3] == 'response_received'
                payload, ack = db.execute('SELECT payload,acknowledgement FROM event_outbox WHERE sequence=?', (sequence,)).fetchone()
                payload, ack = json.loads(payload), json.loads(ack)
                assert payload['device_boot_id'] == child['boot_id'] and payload['device_sequence'] == sequence
                assert payload['source_purpose'] == 'telemetry_only'
                assert all(payload[k] is None for k in ('session_id', 'execution_epoch_id', 'active_command_id'))
                assert not payload['accept_new_work'] and ack['storage_status'] == 'stored'
                if slot <= count:
                    assert origin + slot * 10**9 <= sampled <= attempts[0][1] < origin + (slot + 1) * 10**9
                    first_send_lags.append((attempts[0][1] - origin - slot * 10**9) / 10**6)
                    retries += len(attempts) - 1
                    expected[(child['device_id'], child['boot_id'], sequence)] = (payload, ack)
            receipts.append(dict(device_id=child['device_id'], boot_id=child['boot_id'], required_slots=count,
                actual_samples=len(rows), extra_samples=max(0, len(rows)-count), missed_slots=0,
                max_first_send_slot_lag_ms=max(first_send_lags), retry_attempts=retries,
                actual_last_sample_elapsed_ns=rows[count-1][2]-origin, clean_shutdown=True, native_exit_code=0))
    boots = ','.join("'" + c['boot_id'].replace("'", "''") + "'" for c in children)
    sql = ('COPY (SELECT device_ref,device_boot_id,device_sequence,receipt_id,cloud_received_at,typed_payload '
           'FROM device_events WHERE device_boot_id IN (' + boots + ') ORDER BY ingress_id) TO STDOUT WITH (FORMAT csv);')
    command = [args.kubectl, '--kubeconfig', str(args.kubeconfig), '--context', args.context,
        '--namespace', args.namespace, '--request-timeout=30s', 'exec', '-i', args.postgres_pod,
        '-c', 'postgresql', '--', 'psql', '-X', '-q', '-v', 'ON_ERROR_STOP=1', '-U', 'postgres', '-d', 'sfcloud', '-c', sql]
    raw = out / 'postgres-campaign-raw.csv'
    with raw.open('xb') as stream:
        result = subprocess.run(command, stdout=stream, stderr=subprocess.PIPE)
    assert result.returncode == 0, 'Actual PostgreSQL campaign export failed; retain original partial file'
    seen = set()
    buckets = collections.Counter()
    with raw.open(encoding='utf-8', newline='') as stream:
        for device, boot, sequence, receipt, received, payload in csv.reader(stream):
            key = (device, boot, int(sequence))
            if int(sequence) > count:
                continue
            assert key in expected and key not in seen
            seen.add(key)
            local, ack = expected[key]
            assert json.loads(payload) == local and receipt == ack['receipt_id']
            assert micros(received) == micros(ack['cloud_received_at'])
            buckets[(micros(received) // 60000000 * 60000000, device)] += 1
    assert seen == set(expected) and len(seen) == 10 * count
    receipt = dict(at=dt.datetime.now(dt.timezone.utc).isoformat(), status='actual_Native_and_hosted_telemetry_campaign_verified',
        mode=manifest['mode'], formal_1800_second_acceptance=count == 1800, rows=len(seen), devices=receipts,
        sampling_and_initial_request_in_every_actual_slot=True, PostgreSQL_payload_and_ACK_exact=True,
        independent_minute_counts=[dict(bucket_start_utc_us=b, device_id=d, count=n) for (b,d),n in sorted(buckets.items())],
        source_mode='software_in_loop', execution_receipts=0, actual_v2_product_acceptance=None)
    write(out / 'verification.json', receipt)
    print(json.dumps(dict(status=receipt['status'], rows=len(seen), formal_1800_second_acceptance=count == 1800)))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--campaign', type=Path, required=True)
    for name in ('kubectl', 'kubeconfig', 'context', 'namespace', 'postgres-pod'):
        parser.add_argument('--' + name, required=True)
    verify(parser.parse_args())
