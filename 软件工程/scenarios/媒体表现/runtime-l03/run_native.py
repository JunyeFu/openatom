"""Run the existing Native two-view technical gate on an explicitly chosen candidate.
This is Native Host/byte/GPU/CapturePreview evidence, not the formal visible NFR matrix.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
from datetime import datetime

HERE=Path(__file__).resolve().parent
read=lambda p:json.loads(Path(p).read_text(encoding='utf-8-sig'))
write=lambda p,v:Path(p).write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
parser=argparse.ArgumentParser()
parser.add_argument('--candidate',type=Path,required=True,help='Exact already built portable directory, supplied by its owner')
parser.add_argument('--cases',nargs='+',default=['grip','seal','pallet','lod','region','robot-output-wait','sealer-output-wait','quality-pending','image','gif'])
args=parser.parse_args()
source=args.candidate.resolve()
assert (source/'superfactory.exe').is_file(),source
output=HERE/('native-'+datetime.now().strftime('%Y%m%d-%H%M%S'))
output.mkdir()
candidate=output/'candidate'
shutil.copytree(source,candidate)
cases={
    'grip':('baseline',[(43000,'detailed','R01'),(48000,'detailed','R01')]),
    'seal':('baseline',[(87999,'detailed','CASE01'),(88000,'detailed','CASE01')]),
    'pallet':('baseline',[(90000,'detailed','PAL01'),(330000,'detailed','PAL01')]),
    'lod':('baseline',[(330000,'detailed','line'),(330000,'simplified','line')]),
    'region':('baseline',[(330000,'detailed','line'),(330000,'region','line')]),
    'robot-output-wait':('robot-output-wait',[(48000,'detailed','R01'),(60000,'detailed','R01')]),
    'sealer-output-wait':('sealer-output-wait',[(90000,'detailed','CASE01'),(330000,'detailed','CASE01')]),
    'quality-pending':('quality-pending',[(183000,'detailed','V01'),(200000,'detailed','V01')]),
    'image':('image',[(43000,'detailed','ASM01'),(60000,'detailed','ASM01')]),
    'gif':('gif',[(43000,'detailed','ASM01'),(60000,'detailed','ASM01')]),
}
positions=dict(R01=(1.7,-.9),CASE01=(12.7,0),PAL01=(14.9,0),V01=(5.6,0),ASM01=(0,0))
records=[]
for name in args.cases:
    group,points=cases[name]
    directory=HERE if group=='baseline' else HERE/('styles' if group in ['image','gif'] else 'variants')/group
    receipt=read(directory/'execution-current.json')
    assert receipt['actual_host'] and receipt['fresh_host_reopen'] and receipt['business_equivalent']
    views=[]
    for at,lod,focus in points:
        x,y=positions.get(focus,(7.5,0));x+=2;y+=4
        camera=dict(unit='meter',coordinate_frame='right_handed_z_up',zoom=1,
                    eye=dict(x=10,y=-13,z=13) if focus=='line' else dict(x=x+2.5,y=y-3.5,z=2.5),target=dict(x=x,y=y,z=.8))
        views.append(dict(kind='line_canvas',canvas='line_canvas',hall_id='H01',object_ref=dict(kind='line',id='L03'),
                          project_revision=receipt['frozen_project_revision'],run_id=receipt['run_id'],view_time_ms=at,lod=lod,camera=camera))
    view_file=output/(name+'-views.json');write(view_file,views)
    # Per-process WebView2 UDF prevents reuse of another window's profile/process.
    # Microsoft CreateCoreWebView2EnvironmentWithOptions documents this override.
    env=dict(os.environ,WEBVIEW2_USER_DATA_FOLDER=str(output/('webview-'+name)))
    command=[str(candidate/'superfactory.exe'),'--p3-04-capture-smoke','--capture-input',receipt['archive_absolute_path'],
             '--capture-output',str(output/name),'--capture-views',str(view_file)]
    with (output/(name+'.stdout.txt')).open('w',encoding='utf-8') as stdout,(output/(name+'.stderr.txt')).open('w',encoding='utf-8') as stderr:
        process=subprocess.Popen(command,cwd=candidate,env=env,stdout=stdout,stderr=stderr,creationflags=subprocess.CREATE_NO_WINDOW)
        write(output/'active-process.json',dict(case=name,pid=process.pid,command=command,owned_executable=str(candidate/'superfactory.exe')))
        code=process.wait()
    result_file=output/name/'capture-result.json'
    result=read(result_file) if result_file.is_file() else read(output/name/'failure.json')
    record=dict(case=name,exit_code=code,passed=code==0 and result.get('passed') is True,result=str(result_file),
                actual_run_id=receipt['run_id'],archive=receipt['archive_absolute_path'])
    records.append(record)
    write(output/'matrix.json',dict(kind='actual_selected_candidate_Native_technical_capture_not_public_Task_or_NFR',candidate_source=str(source),records=records,
                                  formal_NFR=None,industrial_or_human_acceptance=None,whole_task_done=False))
    print(json.dumps(record,ensure_ascii=False),flush=True)
    if not record['passed']:raise SystemExit(1)
write(HERE/'native-current.json',dict(output=str(output),candidate_source=str(source),records=records,whole_task_done=False))
