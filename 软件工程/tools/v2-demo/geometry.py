"""Repeat geometry upgrade through the real Host; output and old Run are never overwritten."""
import argparse,json,os,subprocess
from pathlib import Path
from prepare_geometry import ROOT,prepare
def main():
 p=argparse.ArgumentParser();p.add_argument('new_bundle',type=Path);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
 bundle=a.new_bundle.resolve();evidence=a.evidence.resolve()
 if bundle.exists():raise FileExistsError('Fresh bundle path required; preserve all existing saved/frozen versions')
 evidence.mkdir(parents=True,exist_ok=True);source=prepare(bundle)
 target=Path(os.environ['TEMP'])/'sf-v2-demo-geometry-20261027/target';exe=target/'debug/sf-v2-demo-package.exe'
 commands=[['cargo','build','--manifest-path',str(ROOT/'tools/v2-demo/Cargo.toml'),'--target-dir',str(target),'--offline'],
  [str(exe),'geometry-build',str(ROOT/'resources/v2-demo/project'),str(source),str(bundle/'project'),str(evidence/'build-receipt.json')],
  [str(exe),'geometry-verify',str(bundle/'project'),str(ROOT/'resources/v2-demo/project'),str(evidence/'reopened')],
  ['node',str(ROOT/'tools/v2-demo/check-assets.mjs'),str(evidence/'reopened'),str(evidence)]]
 results=[]
 for i,cmd in enumerate(commands):
  with (evidence/f'command-{i+1}.log').open('wb') as log:r=subprocess.run(cmd,cwd=ROOT,stdout=log,stderr=log,creationflags=subprocess.CREATE_NO_WINDOW if os.name=='nt' else 0)
  results.append({'command':cmd,'exit_code':r.returncode})
  (evidence/'generation-commands.json').write_text(json.dumps(results,indent=2),encoding='utf-8')
  if r.returncode:raise RuntimeError(f'Geometry command {i+1} failed; original archive retained; inspect evidence log')
 print('New Host save/reopen, old Run preservation, actual external Engine/Projector and exact asset parse passed')
if __name__=='__main__':main()
