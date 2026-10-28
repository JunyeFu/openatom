"""Build a fresh real Host-saved example; preserve all existing output directories."""
import argparse,datetime as dt,json,os,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]

def main():
    p=argparse.ArgumentParser();p.add_argument('new_output',type=Path);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
    output=a.new_output.resolve();evidence=a.evidence.resolve()
    if output.exists():raise FileExistsError('new output required; no existing example/Run will be overwritten')
    evidence.mkdir(parents=True,exist_ok=True)
    target=Path(os.environ['TEMP'])/'sf-v2-demo-package-20261027/target';exe=target/'debug/sf-v2-demo-package.exe'
    commands=[['cargo','build','--manifest-path',str(ROOT/'tools/v2-demo/Cargo.toml'),'--target-dir',str(target)],
      [str(exe),'build',str(ROOT/'resources/v2-demo/source'),str(output),str(evidence/'build-receipt.json')],
      [str(exe),'verify',str(output),str(evidence/'reopened')],
      ['node',str(ROOT/'tools/v2-demo/check-assets.mjs'),str(evidence/'reopened'),str(evidence)]]
    results=[]
    for i,cmd in enumerate(commands):
        with (evidence/f'command-{i+1}.log').open('wb') as log:result=subprocess.run(cmd,cwd=ROOT,stdout=log,stderr=log,creationflags=subprocess.CREATE_NO_WINDOW if os.name=='nt' else 0)
        results.append(dict(command=cmd,exit_code=result.returncode))
        if result.returncode:
            (evidence/'generation-commands.json').write_text(json.dumps(results,indent=2),encoding='utf-8');raise RuntimeError(f'command{i+1} failed; inspect its private log')
    (evidence/'generation-commands.json').write_text(json.dumps(results,indent=2),encoding='utf-8');print('Real Host build, save, fresh-process reopen, prepared Engine and asset decode passed')

if __name__=='__main__':main()
