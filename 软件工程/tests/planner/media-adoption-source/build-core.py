"""Link the frozen real Core, prepare only owned adoption fixtures; never Cargo."""
import json, subprocess, struct, zlib
from pathlib import Path
root=Path.cwd(); folder=root/'tests/planner/media-adoption-source'; out=root/'scenarios/媒体表现/runtime-media-adoption-consumer'
out.mkdir(parents=True,exist_ok=True)
def png(rgb):
    def chunk(kind,data): return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',32,32,8,2,0,0,0))+chunk(b'IDAT',zlib.compress((b'\x00'+bytes(rgb)*32)*32))+chunk(b'IEND',b'')
template=json.loads((root/'modules/devices/template-support/media/module.json').read_text(encoding='utf-8'))['data']['definition']['payload']['data']
model=json.loads((root/'modules/media/p302-controlled/model.module.json').read_text(encoding='utf-8'))
def ref(name,version): return {'id':name,'version':version}
def package(name,version,presentation,assets,dependencies):
    rights=[{'asset_ref':a['id'],'source':'owned controlled consumer fixture; model from p302-controlled CC0','license':None,'redistribution_confirmed':False,'missing_reason':'no new redistribution grant'} for a in assets]
    return {'schema_version':'1.0','data':{'definition':{'id':name,'version':version,'title':name,'definition_state':'design','contribution':{'author':None,'license':None,'assets':rights},'parameters':[],'attribute_schema':[],'payload':{'category':'media','data':presentation}},'assets':assets,'files':[{'path':a['path'],'kind':'asset'} for a in assets],'dependencies':dependencies,'rights':rights,'issues':[]}}
for version,color in [('1.0.0',(255,0,0)),('2.0.0',(0,160,0))]:
    base=out/'packages'/version; base.mkdir(parents=True,exist_ok=True)
    seqref=ref('consumer.media.sequence',version); bridgeref=ref('consumer.media.bridge',version)
    a={'id':'consumer.a','path':'a.png','kind':'png'};b={'id':'consumer.b','path':'b.png','kind':'png'}
    sequence=json.loads(json.dumps(template)); sequence.update(kind='image_sequence',source={'kind':'image_sequence','data':[a,b]},nodes=[],timeline=[])
    sequence['lod'].update(detailed=None,simplified={'kind':'image_sequence','data':[a,b]})
    for phase in ['load','auto','unload','cycle']:
        sequence['timeline'].extend([{'id':phase+'a','phase_ref':phase,'start_progress':0,'end_progress':.5,'source':{'kind':'image_asset','data':a},'transition':'hold'},{'id':phase+'b','phase_ref':phase,'start_progress':.5,'end_progress':1,'source':{'kind':'image_asset','data':b},'transition':'hold'}])
    seqdir=base/'sequence';seqdir.mkdir(exist_ok=True);(seqdir/'module.json').write_text(json.dumps(package(seqref['id'],version,sequence,[a,b],[])),encoding='utf-8');(seqdir/'a.png').write_bytes(png(color));(seqdir/'b.png').write_bytes(png((0,0,255)))
    bridge=json.loads(json.dumps(template));bridge.update(source={'kind':'media_ref','data':seqref},timeline=[]);bridge['lod'].update(detailed=None,simplified={'kind':'media_ref','data':seqref})
    bridgedir=base/'bridge';bridgedir.mkdir(exist_ok=True);(bridgedir/'module.json').write_text(json.dumps(package(bridgeref['id'],version,bridge,[],[seqref])),encoding='utf-8')
    p=json.loads(json.dumps(model['definition']['payload']['data']));p['lod']['simplified']={'kind':'media_ref','data':bridgeref}
    asset=p['source']['data'];assets=[asset]
    rootdir=base/'model';rootdir.mkdir(exist_ok=True);(rootdir/'module.json').write_text(json.dumps(package('consumer.adopted.media',version,p,assets,[bridgeref])),encoding='utf-8');(rootdir/asset['path']).write_bytes((root/'modules/media/p302-controlled/station.glb').read_bytes())
    image=json.loads(json.dumps(sequence));image.update(kind='image',source={'kind':'asset','data':a});image['lod'].update(detailed=None,simplified=None)
    imagedir=base/'image';imagedir.mkdir(exist_ok=True);(imagedir/'module.json').write_text(json.dumps(package('consumer.adopted.image',version,image,[a,b],[])),encoding='utf-8');(imagedir/'a.png').write_bytes(png(color));(imagedir/'b.png').write_bytes(png((0,0,255)))
source=(root/'tests/planner/factory-actual-items/core_consumer.rs').read_text(encoding='utf-8').replace('scenarios/媒体表现/runtime-factory-items','scenarios/媒体表现/runtime-media-adoption-consumer')
prefix=source.split('fn main(){')[0]
main=source.split('fn main(){')[1].split(' if input["item_media_fixture"]')[0]
main=main.replace('scenarios/图片时间轴/runtime/fixed.input.json','scenarios/图片时间轴/runtime/phase.input.json')
tail=r'''
 for version in ["1.0.0","2.0.0"] {for kind in ["sequence","bridge","model","image"] {core.host.register_bundled_module_directory(&root.join("scenarios/媒体表现/runtime-media-adoption-consumer/packages").join(version).join(kind)).unwrap();}}
 let initial=core.snapshot();let dev=initial["project"]["devices"][0].clone();
 Core::data(core.request(json!({"kind":"project"}),"apply_commands",json!({"base_revision":initial["project"]["revision"],"commands":[{"kind":"adopt_module","data":{"module_ref":{"id":"consumer.adopted.media","version":"1.0.0"},"targets":[{"kind":"device","id":dev["id"]}],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}]})));
 let frozen=core.snapshot();let formal=core.run(json!({"kind":"project"}),frozen["scenarios"][0].clone(),frozen["project"]["revision"].clone(),json!({"kind":"project_scenario"}));core.formal=formal.clone();
 let package=core.object(json!({"kind":"project"}),"module",dev["module_ref"]["id"].as_str().unwrap(),json!({"kind":"module_version","data":{"module_ref":dev["module_ref"]}}));
 let mut definition=package["definition"].clone();definition["payload"]["data"]["presentation"]=frozen["project"]["devices"][0]["presentation_override"].clone();
 let commit=Core::data(core.request(json!({"kind":"project"}),"apply_commands",json!({"base_revision":frozen["project"]["revision"],"commands":[{"kind":"create_definition_draft","data":definition}]})));core.draft=commit["scope"]["data"]["id"].clone();
 let context=Core::data(core.request(json!({"kind":"definition_draft","data":core.draft}),"get_context",json!({"selection":[]})));
 let project_context=Core::data(core.request(json!({"kind":"project"}),"get_context",json!({"selection":[]})));
 let ready=json!({"ready":true,"snapshot":core.snapshot(),"formal_run":formal,"package":package,"draft":{"summary":context["definition"],"definition":definition,"history":commit["history"]},"context":context,"project_context":project_context,"capabilities":Core::data(core.request(json!({"kind":"workspace"}),"describe_capabilities",json!({}))),"producer_cut":"frozen runtime-media-adoption-source 2026-10-16T13:20:26+08"});
 write(&root.join("scenarios/媒体表现/runtime-media-adoption-consumer/bootstrap.actual.json"),&ready);println!("{ready}");io::stdout().flush().unwrap();
 for line in io::stdin().lock().lines(){let query:Value=serde_json::from_str(&line.unwrap()).unwrap();println!("{}",core.reply(query));io::stdout().flush().unwrap();}
}
'''
path=folder/'core_consumer.rs';path.write_text(prefix+'fn main(){'+main+tail,encoding='utf-8')
previous=out/'core-build.json'
assert previous.exists(), 'Use the already recorded Media producer cut; do not select an active Cargo library'
record=json.loads(previous.read_text(encoding='utf-8'));core=Path(next(s for s in record['command'] if s.startswith('sf_commands=')).split('=',1)[1])
assert core.stat().st_mtime==record['Core_mtime'], 'Recorded Media Core has changed: do not rebuild while another Cargo owner is active; existing fixed exe remains usable'
finger=json.loads((root/'target/debug/.fingerprint'/core.stem.removeprefix('lib').replace('_','-',1)/'lib-sf_commands.json').read_text(encoding='utf-8'));expected=next(d[-1] for d in finger['deps'] if d[1]=='serde_json').to_bytes(8,'little').hex()
matches=[p for p in (root/'target/debug/.fingerprint').glob('serde_json-*/lib-serde_json') if p.read_text(encoding='utf-8').strip()==expected];assert len(matches)==1
command=['C:/Users/fujunye/.cargo/bin/rustc.exe','--edition','2024',str(path),'-L','dependency=target/debug/deps','--extern',f'sf_commands={core}','--extern',f'serde_json={root/"target/debug/deps"/("lib"+matches[0].parent.name+".rlib")}','-o',str(out/'core_consumer.exe')]
r=subprocess.run(command,capture_output=True,text=True,encoding='utf-8');(out/'core-build.stdout.txt').write_text(r.stdout,encoding='utf-8');(out/'core-build.stderr.txt').write_text(r.stderr,encoding='utf-8');(out/'core-build.json').write_text(json.dumps({'command':command,'exit_code':r.returncode,'Core_mtime':core.stat().st_mtime,'cut':'Core media-adoption-source frozen 13:20:26; newest existing debug rlib, no Cargo','Host_Engine_logic_changed':False},ensure_ascii=False,indent=2),encoding='utf-8');raise SystemExit(r.returncode)
