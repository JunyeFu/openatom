"""Select real work/wait intervals from the unchanged saved factory Run."""
import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    run = json.loads((args.source / 'actual-connected-run.json').read_text(encoding='utf-8'))
    jobs = {j['id']: j for j in run['jobs'] if j['execution_ref']['kind'] == 'project_device'}
    first = {}
    for job in jobs.values():
        key = (job['execution_ref']['data']['device_id'], job['operation_ref']['id'], job['operation_ref']['version'])
        first.setdefault(key, job)
    samples = []
    for job in first.values():
        work = [w for w in run['phase_work_intervals'] if w['job_id'] == job['id'] and w['interval']['end_ms'] - w['interval']['start_ms'] >= 2]
        longest = max(work, key=lambda w: w['interval']['end_ms'] - w['interval']['start_ms'])
        for row in work if 'sf.paperfactory.bag.operation.' in job['operation_ref']['id'] else [longest]:
            at = (row['interval']['start_ms'] + row['interval']['end_ms']) // 2
            samples.append(dict(kind='working', at_ms=at, job_id=job['id'], device_id=job['execution_ref']['data']['device_id'], operation_ref=job['operation_ref'],
                                phase_id=row['phase_id'], effective_ms=row['effective_progress_start_ms'] + at - row['interval']['start_ms']))
    waits = {}
    for row in run['waits']:
        job = jobs.get(row['object_ref']['id'])
        if not job or job['started_at_ms'] is None or row['interval']['end_ms'] - row['interval']['start_ms'] < 3:
            continue
        at = (row['interval']['start_ms'] + row['interval']['end_ms']) // 2
        if at < job['started_at_ms']:
            continue
        reasons = tuple(sorted({r['kind'] for r in row['reasons']}))
        key = (job['execution_ref']['data']['device_id'], reasons)
        if key in waits:
            continue
        waits[key] = True
        # Two interior points must retain the same effective work during a wait.
        for at in [row['interval']['start_ms'] + 1, row['interval']['end_ms'] - 1]:
            if at >= job['started_at_ms']:
                samples.append(dict(kind='waiting', at_ms=at, job_id=job['id'], device_id=job['execution_ref']['data']['device_id'], operation_ref=job['operation_ref'], reasons=reasons))
    times = sorted({0, run['window']['end_ms'], *(s['at_ms'] for s in samples)})
    plan = dict(run_id=run['id'], source=str(args.source), selection='First actual Job per device/operation; recorded work interval midpoints and started waiting intervals',
                devices=len({s['device_id'] for s in samples if s['kind'] == 'working'}), work_samples=sum(s['kind'] == 'working' for s in samples), wait_samples=sum(s['kind'] == 'waiting' for s in samples),
                samples=samples, times=times, whole_factory_accepted=False)
    with args.output.open('x', encoding='utf-8') as stream:
        json.dump(plan, stream, ensure_ascii=False, indent=2)
    print(f"Selected {plan['devices']} devices, {plan['work_samples']} work/{plan['wait_samples']} wait samples at {len(times)} actual times")


if __name__ == '__main__':
    main()
