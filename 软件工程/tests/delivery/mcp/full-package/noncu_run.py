"""Run one new hidden Native case and wait for normal termination."""
import argparse,json,os,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
parser=argparse.ArgumentParser()
parser.add_argument('mode',choices=['lifecycle','success','complete','shortfall','handoff','nine','generate','cancel'])
parser.add_argument('--attempt',required=True)
parser.add_argument('--executable',type=Path,required=True)
args=parser.parse_args()
stage=ROOT/'.work/delivery/cases'/args.mode
case=json.loads((stage/'case.json').read_text(encoding='utf-8'))
case['output']=str(stage/('actual-'+args.attempt))
selected=stage/('case-'+args.attempt+'.json')
selected.write_text(json.dumps(case,ensure_ascii=False,indent=2),encoding='utf-8')
temporary=ROOT/'.work/tmp';temporary.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,TEMP=str(temporary),TMP=str(temporary))
command=[str(args.executable.resolve()),str(selected)]
with (stage/(args.attempt+'.stdout.txt')).open('wb') as stdout,(stage/(args.attempt+'.stderr.txt')).open('wb') as stderr:
    child=subprocess.Popen(command,cwd=ROOT,env=env,stdout=stdout,stderr=stderr,creationflags=subprocess.CREATE_NO_WINDOW)
    print(json.dumps({'mode':args.mode,'pid':child.pid,'output':case['output']},ensure_ascii=False),flush=True)
    code=child.wait()
(stage/(args.attempt+'.process.json')).write_text(json.dumps({'command':command,'pid':child.pid,'exit_code':code,'visible':False,
    'URL':'about:blank','actual_product_frontend_executed':False,'normal_termination_observed':True},ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps({'mode':args.mode,'exit_code':code}),flush=True)
raise SystemExit(code)
