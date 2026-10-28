"""Run the original frozen 1M/30+30 workload against an actual hosted service.

Uses existing private role credentials. Large raw evidence belongs in --output,
which must be outside the source checkout. No dependency installation or DB reset.
"""
import argparse, array, collections, csv, datetime as dt, http.client, json
import ssl, subprocess, time
from pathlib import Path
from urllib.parse import urlsplit

VERSION = 'sf.execution/2.0-draft11'
EPOCH = dt.datetime(1970, 1, 1, tzinfo=dt.timezone.utc)

def micros(value):
    delta = dt.datetime.fromisoformat(value.replace('Z', '+00:00')).astimezone(dt.timezone.utc) - EPOCH
    return (delta.days * 86400 + delta.seconds) * 1000000 + delta.microseconds

def iso(value):
    return (EPOCH + dt.timedelta(microseconds=value)).isoformat(timespec='microseconds').replace('+00:00', 'Z')

def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')

class Hosted:
    def __init__(self, origin, private):
        parsed = urlsplit(origin)
        assert parsed.scheme == 'https' and parsed.path in ('', '/') and not parsed.query and not parsed.fragment and not parsed.username
        self.connection = http.client.HTTPSConnection(parsed.hostname, parsed.port or 443,
            context=ssl.create_default_context(), timeout=30)
        self.config = json.loads(private.read_text(encoding='utf-8'))
        self.project = self.config['cloud_project_id']
        self.dataset = self.config['load_dataset_id']

    def request(self, method, path, role='scoped_reader', body=None):
        token = next(v['token'] for v in self.config['principals'] if v['role'] == role)
        headers = {'Authorization': 'Bearer ' + token}
        payload = None
        if body is not None:
            payload = json.dumps(body, separators=(',', ':')).encode('utf-8')
            headers['Content-Type'] = 'application/json'
        self.connection.request(method, path, body=payload, headers=headers)
        response = self.connection.getresponse()
        raw = response.read()
        if response.status != 200:
            raise RuntimeError(f'Actual hosted request returned HTTP{response.status}; retain this batch/query identity')
        result = json.loads(raw)
        return result['result']['data']

    def catalog(self):
        return self.request('GET', '/v2/projects/' + self.project + '/datasets')

def batch(dataset, index):
    events = []
    start = micros('2026-01-01T00:00:00.000000Z')
    for n in range(index * 1000, (index + 1) * 1000):
        device, sequence = n % 100, n // 100 + 1
        events.append(dict(device_ref=f'G{device:03}', device_sequence=sequence,
            source_mode='generated_load', observed_at=iso(start + (sequence - 1) * 1000000),
            metric='status_sample', value=(device + sequence) % 1000))
    return dict(contract_version=VERSION, dataset_id=dataset, batch_id=f'batch-load-{index:04}',
                source_mode='generated_load', events=events)

def load(api, out):
    assert not (out / 'load-progress.json').exists(), 'Inspect the original progress before replaying any batch'
    before = api.catalog()
    descriptor = next(v for v in before['datasets'] if v['dataset_id'] == api.dataset)
    assert descriptor['purpose'] == descriptor['source_mode'] == 'generated_load' and descriptor['visible_frontier'] == 0
    manifest = {'at': dt.datetime.now(dt.timezone.utc).isoformat(), 'project_id': api.project,
        'dataset_id': api.dataset, 'source_mode': 'generated_load', 'device_count': 100,
        'event_count': 1000000, 'batch_count': 1000, 'batch_size': 1000,
        'formula': 'n=0..999999; G(n%100);seq=n//100+1;observed=2026-01-01UTC+(seq-1)s;value=(device+seq)%1000;batch=batch-load-(n//1000)',
        'original_workload': 'tools/cloud/scale.py; identical formula/rank selection and 30+30 groups',
        'before': descriptor, 'physical_or_telemetry_run_claim': False}
    write(out / 'load-manifest.json', manifest)
    started = time.monotonic()
    for index in range(1000):
        payload = batch(api.dataset, index)
        ack = api.request('POST', '/v2/datasets/' + api.dataset + '/load-batches', 'generated_load_producer', payload)
        assert ack['inserted_count'] == 1000 and ack['reused_count'] == 0
        progress = {'at': dt.datetime.now(dt.timezone.utc).isoformat(), 'batch_index_completed': index,
            'batches_completed': index + 1, 'inserted_rows_acknowledged': (index + 1) * 1000,
            'elapsed_ms': (time.monotonic() - started) * 1000}
        write(out / 'load-progress.json', progress)
        if index % 100 == 99:
            print(json.dumps(progress), flush=True)
    replays = []
    for index in [0, 999]:
        ack = api.request('POST', '/v2/datasets/' + api.dataset + '/load-batches',
            'generated_load_producer', batch(api.dataset, index))
        assert ack['inserted_count'] == 0 and ack['reused_count'] == 1000
        replays.append({'original_batch_index': index, 'inserted': 0, 'reused': 1000})
    after = next(v for v in api.catalog()['datasets'] if v['dataset_id'] == api.dataset)
    assert after['visible_frontier'] == 1000000
    write(out / 'load-receipt.json', dict(manifest, status='actual_hosted_1M_ingested',
        after=after, exact_original_replays=replays, elapsed_ms=(time.monotonic() - started) * 1000,
        independent_oracle='pending actual PostgreSQL raw export'))
    print(json.dumps({'status': 'actual_hosted_1M_ingested', 'visible_frontier': 1000000}), flush=True)

def oracle(api, out, args):
    receipt = json.loads((out / 'load-receipt.json').read_text(encoding='utf-8'))
    assert receipt['dataset_id'] == api.dataset and receipt['after']['visible_frontier'] == 1000000
    assert all([args.kubectl, args.kubeconfig, args.context, args.namespace, args.postgres_pod])
    dataset = "'" + api.dataset.replace("'", "''") + "'"
    statement = 'COPY (SELECT ingress_id,device_ref,device_sequence,cloud_received_at,batch_id,batch_ordinal,typed_payload FROM device_events WHERE dataset_id=' + dataset + ' ORDER BY ingress_id) TO STDOUT WITH (FORMAT csv);'
    command = [args.kubectl, '--kubeconfig', str(args.kubeconfig), '--context', args.context, '--namespace', args.namespace,
        '--request-timeout=30s', 'exec', '-i', args.postgres_pod, '-c', 'postgresql', '--',
        'psql', '-X', '-q', '-v', 'ON_ERROR_STOP=1', '-U', 'postgres', '-d', 'sfcloud', '-c', statement]
    raw_path = out / 'postgres-raw.csv'
    with raw_path.open('xb') as stream:
        result = subprocess.run(command, stdout=stream, stderr=subprocess.PIPE)
    assert result.returncode == 0, 'Original PG export failed; retain partial raw file and inspect the same operation'
    print(json.dumps({'status': 'actual_PG_COPY_completed', 'bytes': raw_path.stat().st_size}), flush=True)
    times = array.array('q'); devices = array.array('B'); seen = bytearray(1000000); batch_times = {}
    observed_start = micros('2026-01-01T00:00:00.000000Z')
    with raw_path.open(encoding='utf-8', newline='') as stream:
        for count, row in enumerate(csv.reader(stream), 1):
            ingress, device, sequence, received, batch_id, ordinal, payload = row
            event = json.loads(payload); device_number = int(device[1:]); sequence = int(sequence)
            n = (sequence - 1) * 100 + device_number
            assert 0 <= n < 1000000 and not seen[n]; seen[n] = 1
            assert device == f'G{device_number:03}' and int(ingress) == n + 1 == count
            assert batch_id == f'batch-load-{n // 1000:04}' and int(ordinal) == n % 1000
            assert event == dict(device_ref=device, device_sequence=sequence, source_mode='generated_load',
                observed_at=iso(observed_start + (sequence - 1) * 1000000), metric='status_sample', value=(device_number + sequence) % 1000)
            timestamp = micros(received)
            assert batch_times.setdefault(batch_id, timestamp) == timestamp
            times.append(timestamp); devices.append(device_number)
    assert len(times) == 1000000 and all(seen)
    ordered = sorted(range(1000000), key=lambda i: (times[i], i + 1)); by_device = [[] for _ in range(100)]
    for i in ordered: by_device[devices[i]].append(i)
    groups = []
    for j in range(30):
        selected = by_device[j]
        groups.append(dict(id=f'R{j:02}', kind='raw', devices=[f'G{j:03}'],
            received_from=iso(times[selected[1000 + 10 * j]]), received_until=iso(times[selected[1099 + 10 * j]] + 1),
            rank_from=1000 + 10 * j, rank_until=1099 + 10 * j))
        groups.append(dict(id=f'A{j:02}', kind='aggregate', devices=[f'G{x:03}' for x in range(100)],
            received_from=iso(times[ordered[10000 * j]]), received_until=iso(times[ordered[999999 - 10000 * j]] + 1),
            rank_from=10000 * j, rank_until=999999 - 10000 * j))
    write(out / 'query-manifest.json', dict(at=dt.datetime.now(dt.timezone.utc).isoformat(), dataset_id=api.dataset,
        project_id=api.project, visible_frontier=1000000, groups=groups, frozen_before_queries=True))
    expected = {}
    for entry in groups:
        lo, hi = micros(entry['received_from']), micros(entry['received_until'])
        if entry['kind'] == 'raw':
            wanted = int(entry['devices'][0][1:])
            expected[entry['id']] = sorted(i + 1 for i in by_device[wanted] if lo <= times[i] < hi)
        else:
            counts = collections.Counter((times[i] // 60000000 * 60000000, devices[i]) for i in range(1000000) if lo <= times[i] < hi)
            expected[entry['id']] = [(iso(bucket), f'G{device:03}', count) for (bucket, device), count in sorted(counts.items())]
    write(out / 'independent-expected.json', expected)
    write(out / 'oracle-receipt.json', {'status': 'actual_hosted_PG_raw_1M_independently_verified',
        'rows': len(times), 'devices': 100, 'batch_count': len(batch_times), 'raw_bytes': raw_path.stat().st_size,
        'original_formula_exact': True, 'microsecond_times_preserved': True, 'query_groups': 60,
        'HTTP_or_SQL_aggregates_used_as_expected': False})
    print(json.dumps({'status': 'independent_1M_oracle_and_original60_groups_frozen', 'rows': 1000000}), flush=True)

def queries(api, out):
    manifest = json.loads((out / 'query-manifest.json').read_text())
    expected = json.loads((out / 'independent-expected.json').read_text())
    assert manifest['dataset_id'] == api.dataset and manifest['project_id'] == api.project
    assert not (out / 'query-results.json').exists(), 'Retain the original formal measurements'
    def execute(entry):
        query = dict(dataset='telemetry', contract_version=VERSION, cloud_project_id=api.project,
            dataset_id=api.dataset, device_ids=entry['devices'], received_from=entry['received_from'],
            received_until=entry['received_until'], aggregation='rows' if entry['kind'] == 'raw' else 'count_by_device_minute', page=dict(limit=200, cursor=None))
        rows = []; pages = 0; cursors = set(); started_at = dt.datetime.now(dt.timezone.utc).isoformat(); started = time.monotonic_ns()
        while True:
            result = api.request('POST', '/v2/data/query', body=query); pages += 1
            assert result['selected_visible_frontier'] == 1000000 and result['selected_device_ids'] == entry['devices'] and result['data_quality_issues'] == []
            rows.extend(result['rows']); cursor = result['next_cursor']
            if cursor is None: break
            encoded = json.dumps(cursor, sort_keys=True); assert encoded not in cursors; cursors.add(encoded)
            query['page']['cursor'] = cursor
        elapsed = (time.monotonic_ns() - started) / 1000000
        actual = [r['ingress_id'] for r in rows] if entry['kind'] == 'raw' else [[r['bucket_start'], r['device_id'], r['count']] for r in rows]
        assert actual == expected[entry['id']], f"Independent expected rows differ: {entry['id']}"
        if entry['kind'] == 'aggregate':
            assert [r['row_index'] for r in rows] == list(range(len(rows))) and all(r['source_mode'] == 'generated_load' for r in rows)
        return dict(id=entry['id'], kind=entry['kind'], passed=True, started_at=started_at,
            finished_at=dt.datetime.now(dt.timezone.utc).isoformat(), elapsed_ms=elapsed, pages=pages, rows=len(rows))
    warmups = [execute(manifest['groups'][0]), execute(manifest['groups'][1])]
    write(out / 'query-warmups.json', warmups)
    results = []
    for entry in manifest['groups']:
        started = time.monotonic_ns(); started_at = dt.datetime.now(dt.timezone.utc).isoformat()
        try:
            outcome = execute(entry)
        except (AssertionError, RuntimeError, OSError, http.client.HTTPException) as error:
            outcome = dict(id=entry['id'], kind=entry['kind'], passed=False, started_at=started_at,
                finished_at=dt.datetime.now(dt.timezone.utc).isoformat(), elapsed_ms=(time.monotonic_ns()-started)/1000000,
                error_kind=type(error).__name__)
            api.connection.close()
        results.append(outcome); write(out / 'query-results.partial.json', results)
        print(json.dumps(results[-1]), flush=True)
    p95 = {kind: sorted(v['elapsed_ms'] for v in results if v['kind'] == kind)[28] for kind in ['raw', 'aggregate']}
    write(out / 'query-results.json', dict(dataset_id=api.dataset, visible_frontier=1000000,
        outcomes=results, p95_ms=p95, correctness_passed=all(v['passed'] for v in results), performance_passed=all(v <= 2000 for v in p95.values()),
        scope='Actual hosted original60 groups; collector identity/load interval must be recorded separately'))
    print(json.dumps({'status': 'actual_hosted60_query_groups_completed', 'p95_ms': p95}), flush=True)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['load', 'oracle', 'queries'])
    parser.add_argument('--origin', required=True); parser.add_argument('--private-config', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--kubectl'); parser.add_argument('--kubeconfig', type=Path)
    parser.add_argument('--context')
    parser.add_argument('--namespace'); parser.add_argument('--postgres-pod')
    args = parser.parse_args(); args.output.mkdir(parents=True, exist_ok=True)
    assert not args.output.resolve().is_relative_to(Path(__file__).resolve().parents[3]), 'Large evidence must stay outside checkout'
    api = Hosted(args.origin, args.private_config)
    try:
        if args.phase == 'load': load(api, args.output)
        elif args.phase == 'oracle': oracle(api, args.output, args)
        else: queries(api, args.output)
    finally: api.connection.close()

if __name__ == '__main__': main()
