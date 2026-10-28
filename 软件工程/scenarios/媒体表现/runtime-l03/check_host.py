"""Compare actual fresh-Host results; prepare a read-only browser projection input."""
import json
from pathlib import Path

HERE=Path(__file__).resolve().parent
read=lambda p:json.loads(Path(p).read_text(encoding='utf-8-sig'))
fields=['acceptance_events','final_items','jobs','events','resource_claims','allocations','phase_work_intervals','waits','orders']
reports=[]
for name in ['baseline','robot-output-wait','sealer-output-wait','quality-pending','image','gif']:
    directory=HERE if name=='baseline' else HERE/('styles' if name in ['image','gif'] else 'variants')/name
    receipt=read(directory/'execution-current.json')
    receipt['archive_absolute_path']=receipt['archive_absolute_path'].removeprefix('\\\\?\\')
    execution=Path(receipt['archive_absolute_path']).parent
    receipt['native_requests']=str(execution/'native-requests.json')
    run=read(execution/'actual-run.json')
    expected=read(HERE/('engine-only.actual.json' if name in ['baseline','image','gif'] else name+'.engine.actual.json'))['run']
    checks={field:run[field]==expected[field] for field in fields}
    assert all(checks.values()),(name,checks)
    if name=='baseline':assert [e['at_ms'] for e in run['acceptance_events']]==[90000,130000,240000,255000,290000,330000]
    report=dict(case=name,kind='actual_Host_vs_independently_executed_same_Engine',checks=checks,
                actual_times_ms=[e['at_ms'] for e in run['acceptance_events']],actual_run_id=run['id'],archive_absolute_path=receipt['archive_absolute_path'])
    (directory/'host-business-check.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    receipt['business_equivalent']=True
    receipt['business_acceptance']='Nine full behavior/identity/resource/phase fields equal independently tested Engine result; baseline has all original six acceptance times. Industrial/human acceptance remains separate.'
    for path in [directory/'execution-current.json',execution/'receipt.json']:
        path.write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    modules=[]
    for path in (execution/'reopened').glob('*get_object.json'):
        value=read(path);request=value['request']['params']
        if request['object_id']['kind']=='module' and request['source']['kind']=='module_version':modules.append(value['response']['outcome']['data']['data'])
    snapshots=read(execution/'actual-run-snapshots.json')
    keep=[0,40000,42000,43000,47999,48000,48999,49000,52000,60000,82000,82999,84000,87999,88000,89999,90000,130000,183000,200000,203000,240000,255000,290000,330000]
    data=dict(kind='browser_projection_of_actual_fresh_Host_saved_Run_not_live_Native',archive_absolute_path=receipt['archive_absolute_path'],
              scope_revision=receipt['scope_revision'],project=read(execution/'actual-project-reopened.json')['data'],run=run,
              snapshots=[snapshot for snapshot in snapshots if snapshot['at_ms'] in keep],modules=modules)
    (directory/'browser-input.json').write_text(json.dumps(data,ensure_ascii=False,separators=(',',':')),encoding='utf-8')
    reports.append(report)
(HERE/'host-matrix.json').write_text(json.dumps(reports,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Actual Host baseline and three separate variants: all54 full behavior field comparisons passed.')
