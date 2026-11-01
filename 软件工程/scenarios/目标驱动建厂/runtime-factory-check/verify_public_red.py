"""Compile the genuine public-dispatch tests using a recorded existing Core.
The one executed test is expected red before Root registers the producer.
No Cargo, no shadow Host, no edited dispatch or alternate implementation.
"""
import json
import os
from pathlib import Path
import subprocess
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
RECORDED=ROOT/'scenarios/图片时间轴/runtime/core-build.json'
record=json.loads(RECORDED.read_text(encoding='utf-8'))
core=ROOT/'target/debug/deps'/record['existing_core']
fingerprints=ROOT/'target/debug/.fingerprint'
core_fp=fingerprints/core.stem.removeprefix('lib').replace('_','-',1)
deps=json.loads((core_fp/'lib-sf_commands.json').read_text())['deps']
libraries={'sf_commands':core}
for name in ['sf_domain','serde_json','serde']:
    stamp=next(d[-1] for d in deps if d[1]==name).to_bytes(8,'little').hex()
    matches=[]
    for marker in fingerprints.glob(f"{name.replace('_','-') if name.startswith('sf_') else name}-*/lib-{name}"):
        path=ROOT/'target/debug/deps'/f"lib{name}-{marker.parent.name.rsplit('-',1)[1]}.rlib"
        if marker.read_text().strip()==stamp and path.is_file():matches.append(path)
    assert len(matches)==1,(name,matches)
    libraries[name]=matches[0]
command=[str(Path.home()/'.cargo/bin/rustc.exe'),'--edition=2024','--test',str(ROOT/'packages/commands/tests/factory_draft_check_host.rs'),'-L',f"dependency={ROOT/'target/debug/deps'}",'-o',str(HERE/'public-host-tests.exe')]
for name,path in libraries.items():command+=['--extern',f'{name}={path}']
env=os.environ.copy();env['CARGO_MANIFEST_DIR']=str(ROOT/'packages/commands')
build=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,encoding='utf-8')
(HERE/'public-red-build.stdout.txt').write_text(build.stdout,encoding='utf-8')
(HERE/'public-red-build.stderr.txt').write_text(build.stderr,encoding='utf-8')
receipt={'recorded_core':str(RECORDED),'libraries':{n:str(p) for n,p in libraries.items()},'compile_command':command,'compile_exit_code':build.returncode,'Cargo_used':False,'Root_registration_performed':False,'public_green_claimed':False}
if build.returncode:
    (HERE/'public-red.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2),encoding='utf-8')
    print(build.stderr);sys.exit(build.returncode)
run=subprocess.run([str(HERE/'public-host-tests.exe'),'--exact','public_check_complete_fixture_is_readonly_and_retains_old_run_and_original_orders','--nocapture'],cwd=ROOT,capture_output=True,text=True,encoding='utf-8')
(HERE/'public-red.stdout.txt').write_text(run.stdout,encoding='utf-8')
(HERE/'public-red.stderr.txt').write_text(run.stderr,encoding='utf-8')
receipt.update({'run_exit_code':run.returncode,'expected_pre_registration_red':run.returncode!=0 and 'Operation is not implemented by this Host' in run.stderr})
(HERE/'public-red.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2),encoding='utf-8')
print(run.stdout);print(run.stderr)
sys.exit(0 if receipt['expected_pre_registration_red'] else 1)
