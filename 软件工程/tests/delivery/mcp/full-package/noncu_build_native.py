"""Link the non-GUI driver to this copy's exact locked Native dependency closure."""
import argparse,json,os,re,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
parser=argparse.ArgumentParser()
parser.add_argument('--output',type=Path,required=True)
parser.add_argument('--build-log',type=Path,default=ROOT/'.work/delivery/commands/native-sdk/stdout.txt')
args=parser.parse_args()
out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
target=ROOT/'.work/target/debug'
build_events=[json.loads(line) for line in args.build_log.read_text(encoding='utf-8').splitlines()]
desktop_artifacts=[e for e in build_events if e.get('reason')=='compiler-artifact' and e.get('target',{}).get('name')=='sf_desktop_host']
assert len(desktop_artifacts)==1,desktop_artifacts
features=desktop_artifacts[0]['features']
matches=[p for p in (target/'.fingerprint').glob('sf-desktop-host-*/lib-sf_desktop_host.json') if sorted(json.loads(json.loads(p.read_text(encoding='utf-8'))['features']))==sorted(features)]
assert len(matches)==1,matches
fingerprint=json.loads(matches[0].read_text(encoding='utf-8'))
deps={entry[1]:entry[3].to_bytes(8,'little').hex() for entry in fingerprint['deps']}
native=ROOT/'apps/planner/src-tauri/src'
source=(native/'project_api.rs').read_text(encoding='utf-8')
for child in re.findall(r'pub\(super\) mod (\w+);',source):
    declaration=f'pub(super) mod {child};'
    assert source.count(declaration)==1
    source=source.replace(declaration,'#[path = '+json.dumps((native/'project_api'/(child+'.rs')).as_posix(),ensure_ascii=False)+']\n'+declaration)
source=source.replace('"../../../../','"'+ROOT.as_posix()+'/')
# Test-only trusted selection mirrors the existing dialog's Host::select_file;
# no public wire/interface or production function body is replaced.
source+='''\npub(super) fn noncu_select_file(state:&ProjectApiState,session:&sf_domain::ObjectId,purpose:sf_commands::FilePurpose,path:std::path::PathBuf)->Result<sf_commands::SelectedFileRef,sf_commands::ApiError>{
    state.host.lock().expect("controlled Host lock").select_file(session,purpose,path)
}\n'''
projection=out/'project_api.rs';projection.write_text(source,encoding='utf-8')
main=(Path(__file__).with_name('noncu_native.rs')).read_text(encoding='utf-8')
for module in ['agent_pipe','project_api','native_render','native_render_capture']:
    path=projection if module=='project_api' else native/(module+'.rs')
    main=main.replace('"../../../../apps/planner/src-tauri/src/'+module+'.rs"',json.dumps(path.as_posix(),ensure_ascii=False))
(out/'main.rs').write_text(main,encoding='utf-8')
command=['C:/Users/fujunye/.cargo/bin/rustc.exe','--edition=2024','--crate-name','noncu_native',str(out/'main.rs'),'-L','dependency='+str(target/'deps'),'-o',str(out/'noncu-native.exe')]
libraries={}
for name in ['sf_commands','sf_domain','serde','serde_json','tauri','tauri_plugin_dialog','tokio','windows_sys','webview2_com','windows']:
    matched=[p for p in (target/'.fingerprint').glob('*/lib-'+name) if p.read_text(encoding='utf-8').strip()==deps[name]]
    assert len(matched)==1,(name,matched)
    suffix=matched[0].parent.name.rsplit('-',1)[1]
    library=target/'deps'/f'lib{name}-{suffix}.rlib'
    assert library.is_file()
    command+=['--extern',name+'='+str(library)];libraries[name]=str(library)
desktop_outputs=[Path(e['out_dir']).parent/'output' for e in build_events if e.get('reason')=='build-script-executed' and 'sf-desktop-host' in e['package_id']]
assert len(desktop_outputs)==1,desktop_outputs
for event in build_events:
    if event.get('reason')=='build-script-executed':
        for path in event['linked_paths']: command+=['-L',path]
for feature in json.loads(fingerprint['features']): command+=['--cfg',f'feature="{feature}"']
for line in desktop_outputs[0].read_text(encoding='utf-8').splitlines():
    if line.startswith(('cargo:rustc-link-arg=','cargo:rustc-link-arg-bins=')): command+=['-C','link-arg='+line.split('=',1)[1]]
    elif line.startswith('cargo:rustc-cfg='): command+=['--cfg',line.split('=',1)[1]]
environment=dict(os.environ,CARGO_MANIFEST_DIR=str(ROOT/'apps/planner/src-tauri'),CARGO_PKG_NAME='sf-desktop-host',CARGO_PKG_VERSION='0.1.0',
    CARGO_PKG_DESCRIPTION='Controlled non-GUI protocol consumer',CARGO_PKG_AUTHORS='',OUT_DIR=str(desktop_outputs[0].parent/'out'))
with (out/'stdout.txt').open('wb') as stdout,(out/'stderr.txt').open('wb') as stderr:
    result=subprocess.run(command,cwd=ROOT,env=environment,stdout=stdout,stderr=stderr,creationflags=subprocess.CREATE_NO_WINDOW)
(out/'receipt.json').write_text(json.dumps({'command':command,'exit_code':result.returncode,'libraries':libraries,
    'production_function_bodies_unchanged':True,'private_projection':'explicit Rust child-module paths and trusted test-only select_file consumer',
    'public_API_or_Host_modified':False},ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps({'exit_code':result.returncode,'executable':str(out/'noncu-native.exe')},ensure_ascii=False))
raise SystemExit(result.returncode)
