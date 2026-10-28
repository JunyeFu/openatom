"""Frozen DS 1M/30 raw/30 minute groups; independent exported-row Python oracle."""
import array, base64, collections, csv, datetime as dt, json, os, subprocess, time
from local import *

EPOCH=dt.datetime(1970,1,1,tzinfo=dt.timezone.utc)
def micros(text):
    delta=dt.datetime.fromisoformat(text.replace('Z','+00:00')).astimezone(dt.timezone.utc)-EPOCH
    return (delta.days*86400+delta.seconds)*1000000+delta.microseconds
def iso(value): return (EPOCH+dt.timedelta(microseconds=value)).isoformat(timespec='microseconds').replace('+00:00','Z')

def verify_scale():
    os.environ.pop('SF_CLOUD_TEST_DATABASE',None)
    exists=sql("SELECT datname FROM pg_database WHERE datname='sfcloud_scale';").strip()
    if not exists: sql('CREATE DATABASE sfcloud_scale;')
    os.environ['SF_CLOUD_TEST_DATABASE']='sfcloud_scale'
    run([str(EXE),'migrate'],env=environment())
    out=WORK/'scale';out.mkdir(exist_ok=True)
    with Server():
        initial=int(sql("SELECT visible_frontier FROM datasets WHERE dataset_id='load-fixture';").strip())
        assert initial in range(0,1000001,1000), 'existing dataset does not match whole frozen batches'
        started=time.monotonic()
        for batch_index in range(1000):
            events=[]
            for n in range(batch_index*1000,(batch_index+1)*1000):
                d=n%100;seq=n//100+1
                events.append(dict(device_ref=f'G{d:03}',device_sequence=seq,source_mode='generated_load',observed_at=iso(micros('2026-01-01T00:00:00.000000Z')+(seq-1)*1000000),metric='status_sample',value=(d+seq)%1000))
            v=dict(contract_version=VERSION,dataset_id='load-fixture',batch_id=f'batch-load-{batch_index:04}',source_mode='generated_load',events=events)
            ack=request('POST','/v2/datasets/load-fixture/load-batches',v,'load')[0]['result']['data']
            assert ack['inserted_count']+ack['reused_count']==1000
            if batch_index%100==99: print(f'generated batches verified: {batch_index+1}/1000',flush=True)
        ingest_ms=(time.monotonic()-started)*1000
        f0=int(sql("SELECT visible_frontier FROM datasets WHERE dataset_id='load-fixture';").strip());assert f0==1000000
        # Export raw rows, no SQL aggregates or REST are used as the expected-data oracle.
        statement="COPY (SELECT ingress_id,device_ref,device_sequence,cloud_received_at,batch_id,batch_ordinal,typed_payload FROM device_events WHERE dataset_id='load-fixture' ORDER BY ingress_id) TO STDOUT WITH (FORMAT csv);"
        with (out/'postgres-raw.csv').open('wb') as stream:
            subprocess.run(['docker','exec','-i',CONTAINER,'psql','-X','-q','-v','ON_ERROR_STOP=1','-U','sfcloud','-d','sfcloud_scale','-c',statement],check=True,stdout=stream,stderr=subprocess.PIPE)
        times=array.array('q');devices=array.array('B');seen=bytearray(1000000);batch_times={};start_observed=micros('2026-01-01T00:00:00.000000Z')
        with (out/'postgres-raw.csv').open(encoding='utf-8',newline='') as stream:
            for count,row in enumerate(csv.reader(stream),1):
                ingress,device,seq,received,batch,ordinal,payload=row;event=json.loads(payload);d=int(device[1:]);seq=int(seq);n=(seq-1)*100+d
                assert 0<=n<1000000 and not seen[n];seen[n]=1
                assert int(ingress)==n+1==count and batch==f'batch-load-{n//1000:04}' and int(ordinal)==n%1000
                assert event==dict(device_ref=device,device_sequence=seq,source_mode='generated_load',observed_at=iso(start_observed+(seq-1)*1000000),metric='status_sample',value=(d+seq)%1000)
                timestamp=micros(received);old=batch_times.setdefault(batch,timestamp);assert old==timestamp
                times.append(timestamp);devices.append(d)
        assert len(times)==1000000 and all(seen)
        print('independent 1M formula/identity/microsecond export verified',flush=True)
        # The manifest is saved before warmups or measured HTTP requests.
        ordered=sorted(range(1000000),key=lambda i:(times[i],i+1));by_device=[[] for _ in range(100)]
        for i in ordered: by_device[devices[i]].append(i)
        manifest=[]
        for j in range(30):
            selected=by_device[j];lo=times[selected[1000+10*j]];hi=times[selected[1099+10*j]]+1
            manifest.append(dict(id=f'R{j:02}',kind='raw',devices=[f'G{j:03}'],received_from=iso(lo),received_until=iso(hi),rank_from=1000+10*j,rank_until=1099+10*j))
            lo=times[ordered[10000*j]];hi=times[ordered[999999-10000*j]]+1
            manifest.append(dict(id=f'A{j:02}',kind='aggregate',devices=[f'G{x:03}' for x in range(100)],received_from=iso(lo),received_until=iso(hi),rank_from=10000*j,rank_until=999999-10000*j))
        (out/'query-manifest.json').write_text(json.dumps(dict(dataset_id='load-fixture',visible_frontier=f0,groups=manifest),indent=2),encoding='utf-8')
        expected={}
        for entry in manifest:
            lo=micros(entry['received_from']);hi=micros(entry['received_until'])
            if entry['kind']=='raw':
                wanted=int(entry['devices'][0][1:]);expected[entry['id']]=[i+1 for i in by_device[wanted] if lo<=times[i]<hi];expected[entry['id']].sort()
            else:
                groups=collections.Counter((times[i]//60000000*60000000,devices[i]) for i in range(1000000) if lo<=times[i]<hi)
                expected[entry['id']]=[(iso(bucket),f'G{d:03}',count) for (bucket,d),count in sorted(groups.items())]
        def execute(entry):
            query=dict(dataset='telemetry',contract_version=VERSION,cloud_project_id='example',dataset_id='load-fixture',device_ids=entry['devices'],received_from=entry['received_from'],received_until=entry['received_until'],aggregation='rows' if entry['kind']=='raw' else 'count_by_device_minute',page=dict(limit=200,cursor=None))
            rows=[];pages=0;cursors=set();start=time.monotonic_ns()
            while True:
                result=request('POST','/v2/data/query',query,'reader')[0]['result']['data'];pages+=1
                assert result['selected_visible_frontier']==f0 and result['selected_device_ids']==entry['devices'] and result['data_quality_issues']==[]
                rows.extend(result['rows']);cursor=result['next_cursor']
                if cursor is None: break
                assert cursor not in cursors;cursors.add(cursor);query['page']['cursor']=cursor
            elapsed=(time.monotonic_ns()-start)/1000000
            actual=[r['ingress_id'] for r in rows] if entry['kind']=='raw' else [(r['bucket_start'],r['device_id'],r['count']) for r in rows]
            assert actual==expected[entry['id']],(entry['id'],len(actual),len(expected[entry['id']]))
            if entry['kind']=='aggregate':
                assert [r['row_index'] for r in rows]==list(range(len(rows))) and all(r['source_mode']=='generated_load' for r in rows)
            return dict(id=entry['id'],kind=entry['kind'],passed=True,elapsed_ms=elapsed,pages=pages,rows=len(rows))
        execute(manifest[0]);execute(manifest[1])
        outcomes=[]
        for entry in manifest:
            try: outcomes.append(execute(entry))
            except Exception as e: outcomes.append(dict(id=entry['id'],kind=entry['kind'],passed=False,error=str(e)))
            print(entry['id'],outcomes[-1].get('elapsed_ms'),outcomes[-1]['passed'],flush=True)
            (out/'query-results.partial.json').write_text(json.dumps(outcomes,indent=2),encoding='utf-8')
        p95={kind:sorted(x['elapsed_ms'] for x in outcomes if x['kind']==kind and x['passed'])[28] if sum(x['kind']==kind and x['passed'] for x in outcomes)==30 else None for kind in ['raw','aggregate']}
        receipt=dict(database='sfcloud_scale',dataset_id='load-fixture',exported_rows=len(times),visible_frontier=f0,ingest_and_exact_replay_ms=ingest_ms,independent_oracle='PostgreSQL raw COPY + Python bitset and frozen formula; actual UTC microsecond timestamps and Python grouping',manifest_frozen_before_queries=True,outcomes=outcomes,p95_ms=p95,correctness_passed=all(x['passed'] for x in outcomes),performance_passed=all(x is not None and x<=2000 for x in p95.values()),ten_producers_native='not executed; root integration scope')
        (out/'receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2),encoding='utf-8');print(json.dumps({k:receipt[k] for k in ['exported_rows','correctness_passed','performance_passed','p95_ms']}),flush=True)
