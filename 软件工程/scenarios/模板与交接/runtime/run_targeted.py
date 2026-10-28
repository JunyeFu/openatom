"""Actual bounded module-kit + existing Commands DTO + single Engine probe.
No dependency installation or Cargo/config/lock mutation. W00 owns full native CI.
"""
from pathlib import Path
import json, os, subprocess, sys, time

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
EXEC=ROOT/'docs/04_开发与验收/整包派发/P4-01_整包执行.json'
CARGO=Path.home()/'.cargo/bin/cargo.exe'
RUSTC=Path.home()/'.cargo/bin/rustc.exe'


def run(label,command,env=None):
    started=time.time()
    r=subprocess.run([str(x) for x in command],cwd=ROOT,capture_output=True,env=env)
    (HERE/(label+'.stdout.txt')).write_bytes(r.stdout)
    (HERE/(label+'.stderr.txt')).write_bytes(r.stderr)
    d=json.loads(EXEC.read_text(encoding='utf-8'))
    d['implementation_progress']['runs'].append({'command':[str(x) for x in command], 'label':label,
        'round':'P4-01-acceptance-followup',
        'exit_code':r.returncode,'elapsed_s':time.time()-started,
        'raw_stdout':r.stdout.decode('utf-8','replace'),'raw_stderr':r.stderr.decode('utf-8','replace')})
    EXEC.write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(label,'exit',r.returncode)
    if r.returncode:
        print(r.stdout.decode('utf-8','replace')[-10000:]);print(r.stderr.decode('utf-8','replace')[-5000:]);sys.exit(r.returncode)
    return r


if '--sdk-only' in sys.argv:
    run('template-units',[CARGO,'test','-p','sf-module-kit','--locked','--test','template_units','--','--nocapture'])
    print((HERE/'template-units.stdout.txt').read_text(encoding='utf-8'))
    sys.exit(0)

if '--host-only' not in sys.argv:
    run('template-units',[CARGO,'test','-p','sf-module-kit','--locked','--test','template_units','--','--nocapture'])
    r=run('consumer-build',[CARGO,'build','-p','sf-commands','--locked','--message-format=json'])
    artifact_text=r.stdout.decode('utf-8','replace')
else:
    artifact_text=(HERE/'consumer-build.stdout.txt').read_text(encoding='utf-8')
libs={}
for line in artifact_text.splitlines():
    x=json.loads(line)
    if x.get('reason')=='compiler-artifact':
        for f in x['filenames']:
            if f.endswith('.rlib'):
                name=x['target']['name']
                # Cargo emits both host-build and runtime serde variants. The
                # alloc-enabled runtime is the one used by sf-domain/serde_json.
                if name!='serde' or 'alloc' in x['features'] or name not in libs:
                    libs[name]=f
exe=HERE/'engine-consumer.exe'
env=dict(os.environ);env['CARGO_MANIFEST_DIR']=str(HERE)
command=[RUSTC,'--edition=2024',HERE/'engine_consumer.rs','-o',exe,'-L','dependency='+str(ROOT/'target/debug/deps')]
for name in ['sf_domain','sf_module_kit','sf_simulation','sf_commands','serde_json','serde']:
    command += ['--extern',name+'='+libs[name]]
if '--host-only' not in sys.argv:
    run('consumer-link',command,env)
    run('engine-consumer',[exe])
    print((HERE/'engine-consumer-results.json').read_text(encoding='utf-8'))
host_command=list(command)
host_command[2]=str(HERE/'host_consumer.rs')
host_command[4]=str(HERE/'host-consumer.exe')
run('host-consumer-link',host_command,env)
run('host-consumer',[HERE/'host-consumer.exe'])
print((HERE/'host-consumer-results.json').read_text(encoding='utf-8'))
