"""Run one owned hidden Native case; reopening is a separate process and path."""
import argparse,copy,json,os,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];OUT=ROOT/'.work/cu-delivery'
parser=argparse.ArgumentParser();parser.add_argument('mode');parser.add_argument('--attempt',required=True);parser.add_argument('--reopen',action='store_true');args=parser.parse_args()
stage=OUT/'cases'/args.mode;case=json.loads((stage/'case.json').read_text(encoding='utf-8'))
if args.reopen:
    producer=stage/('actual-'+args.attempt);result=json.loads((producer/'workflow-result.json').read_text(encoding='utf-8'))
    data=json.loads((stage/'input.json').read_text(encoding='utf-8'));data['workflow']=result
    relocated=OUT/'relocated'/(args.mode+'-'+args.attempt);shutil.copytree(result['saved_directory'],relocated)
    stage=OUT/'cases'/(args.mode+'-reopen-'+args.attempt);stage.mkdir(parents=True,exist_ok=False)
    (stage/'input.json').write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8');case.update(mode='workflow-reopen',seed=str(relocated))
case['output']=str(stage/('actual-'+args.attempt));selected=stage/('case-'+args.attempt+'.json');selected.write_text(json.dumps(case,ensure_ascii=False,indent=2),encoding='utf-8')
with (stage/(args.attempt+'.stdout.txt')).open('wb') as a,(stage/(args.attempt+'.stderr.txt')).open('wb') as b:
    child=subprocess.Popen([str(OUT/'native/noncu-native.exe'),str(selected)],cwd=ROOT,env=dict(os.environ,TEMP=str(ROOT/'.work/tmp'),TMP=str(ROOT/'.work/tmp')),stdout=a,stderr=b,creationflags=subprocess.CREATE_NO_WINDOW)
    code=child.wait()
(stage/(args.attempt+'.process.json')).write_text(json.dumps({'pid':child.pid,'exit_code':code,'source':str(case['seed']),'output':case['output'],'normal_termination_observed':True,'hidden_without_input':True},ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps({'mode':case['mode'],'exit_code':code,'output':case['output']},ensure_ascii=False));raise SystemExit(code)
