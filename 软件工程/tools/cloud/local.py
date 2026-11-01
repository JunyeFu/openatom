"""Dedicated local PostgreSQL/HTTP component acceptance. Standard library only.

Credentials remain in an ACL-restricted private directory and process environments.
The fixed `example` project is the public DTO component fixture, not a Native run.
"""
from __future__ import annotations
import argparse, copy, datetime as dt, json, os, secrets, subprocess, sys, time
from pathlib import Path
from urllib.request import Request, urlopen
from urllib.error import HTTPError
from urllib.parse import urlencode

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / '.work/v2-implementation-20261026/cloud'
PRIVATE = WORK / 'private'
CONTAINER = 'sf-v2-cloud-db-20261026'
VOLUME = 'sf-v2-cloud-db-data-20261026'
EXE = Path(os.environ.get('SF_CLOUD_TEST_EXE',str(WORK / 'target/debug/sf-cloud-service.exe')))
VERSION = 'sf.execution/2.0-draft11'
BASE = 'http://127.0.0.1:23270'
FIXTURES = json.loads((ROOT / 'packages/domain/tests/fixtures/cloud_wire_cases.json').read_text(encoding='utf-8-sig'))

def fixture(name):
    value=copy.deepcopy(next(x['value'] for x in FIXTURES if x['definition'] == name and x['label'] == 'positive'))
    if name=='CloudSessionCreate':
        # Use real existing domain fixture types for frozen reference checks; this is still a component scenario.
        fixtures=ROOT/'tools/type-contract-proof/fixtures'
        project=json.loads((fixtures/'project.json').read_text(encoding='utf-8-sig'))['data']
        device=copy.deepcopy(project['devices'][0]);device.update(id='example',module_ref=dict(id='example',version='1.0.0'),role_bindings=dict(item_types={},resources={},targets={},ports={}),operation_binding=None)
        value['frozen_inputs']['project']['devices']=[device];value['frozen_inputs']['project']['halls']=copy.deepcopy(project['halls'])
        definition=json.loads((fixtures/'module-devices.json').read_text(encoding='utf-8-sig'))['data']['definition'];definition.update(id='example',version='1.0.0')
        value['frozen_inputs']['execution_definitions']=[definition]
    return value

def run(args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, text=True, encoding='utf-8', **kwargs).stdout

def prepare():
    PRIVATE.mkdir(parents=True, exist_ok=True)
    if os.name == 'nt':
        script='''$taskAcl=[System.Security.AccessControl.DirectorySecurity]::new();
$taskAcl.SetAccessRuleProtection($true,$false);
$taskSid=[System.Security.Principal.WindowsIdentity]::GetCurrent().User;
$taskSystem=[System.Security.Principal.SecurityIdentifier]::new('S-1-5-18');
$taskInheritance=[System.Security.AccessControl.InheritanceFlags]'ContainerInherit,ObjectInherit';
foreach($taskPrincipal in @($taskSid,$taskSystem)){$taskAcl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new($taskPrincipal,'FullControl',$taskInheritance,'None','Allow'))};
[System.IO.Directory]::SetAccessControl($env:SF_CLOUD_PRIVATE_DIR,$taskAcl)'''
        subprocess.run(['powershell','-NoProfile','-NonInteractive','-Command',script],env=dict(os.environ,SF_CLOUD_PRIVATE_DIR=str(PRIVATE)),check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    if (PRIVATE/'credentials.json').exists():
        return
    devices = ['example','unused'] + [f'G{i:03}' for i in range(100)]
    roles = {'host':'host_gateway','reader':'scoped_reader','operator':'operator','device':'device','load':'generated_load_producer','outside':'scoped_reader'}
    principals = []
    for name,role in roles.items():
        principals.append(dict(token=secrets.token_hex(32),principal_id='device-principal' if name=='device' else name,role=role,project_ids=['other' if name=='outside' else 'example'],device_ids=['example'] if name=='device' else devices,dataset_ids=['load-fixture'] if name=='load' else ['telemetry-fixture','load-fixture']))
    password = secrets.token_hex(24)
    (PRIVATE/'credentials.json').write_text(json.dumps({'password':password,'principals':principals}), encoding='utf-8')
    (PRIVATE/'postgres.env').write_text(f'POSTGRES_USER=sfcloud\nPOSTGRES_DB=sfcloud\nPOSTGRES_PASSWORD={password}\n', encoding='utf-8')
    b=fixture('CandidateBinding')
    config=dict(projects=[dict(cloud_project_id='example',local_project_id='example',owner_scope_ref='host'),dict(cloud_project_id='other',local_project_id='other',owner_scope_ref='outside')],device_bindings=[dict(binding_id=b['binding_id'],cloud_project_id='example',device_id=b['device_id'],principal_scope_ref='device-principal',definition_ref=b['device_module_ref'],capability_map=dict(action='process_unit',operation_ref=b['operation_ref'],execution_capacity_ref=b['execution_capacity_ref']))],datasets=[dict(dataset_id='telemetry-fixture',cloud_project_id='example',source_mode='software_in_loop',purpose='telemetry_only'),dict(dataset_id='load-fixture',cloud_project_id='example',source_mode='generated_load',purpose='generated_load')])
    (WORK/'component-bindings.json').write_text(json.dumps(config,indent=2),encoding='utf-8')

def environment():
    c=json.loads((PRIVATE/'credentials.json').read_text(encoding='utf-8'))
    db=os.environ.get('SF_CLOUD_TEST_DATABASE','sfcloud')
    assert db.replace('_','').isalnum()
    return dict(os.environ,DATABASE_URL=f"postgresql://sfcloud:{c['password']}@127.0.0.1:23271/{db}",SF_CLOUD_AUTH_TOKENS_JSON=json.dumps(c['principals']),SF_CLOUD_BINDINGS_FILE=str(WORK/'component-bindings.json'),SF_CLOUD_LOCAL_NO_TLS='1',SF_CLOUD_LISTEN_ADDRESS='127.0.0.1',PORT='23270')

def db_start():
    existing=run(['docker','ps','-a','--filter','name=^/'+CONTAINER+'$','--format','{{.Names}}']).strip()
    if existing:
        inspect=json.loads(run(['docker','inspect',CONTAINER]))[0]
        assert inspect['Config']['Labels'].get('sf.scope')=='v2-cloud-20261026', 'container ownership mismatch'
        assert inspect['HostConfig']['PortBindings']['5432/tcp']==[{'HostIp':'127.0.0.1','HostPort':'23271'}]
        if not inspect['State']['Running']: run(['docker','start',CONTAINER])
    else:
        # No existing volume is adopted merely because its name matches.
        if run(['docker','volume','ls','--filter','name=^'+VOLUME+'$','--format','{{.Name}}']).strip():
            assert json.loads(run(['docker','volume','inspect',VOLUME]))[0]['Labels'].get('sf.scope')=='v2-cloud-20261026', 'volume ownership mismatch'
        else: run(['docker','volume','create','--label','sf.scope=v2-cloud-20261026',VOLUME])
        run(['docker','run','-d','--name',CONTAINER,'--label','sf.scope=v2-cloud-20261026','--env-file',str(PRIVATE/'postgres.env'),'-p','127.0.0.1:23271:5432','-v',VOLUME+':/var/lib/postgresql','postgres:18'])
    for _ in range(60):
        result=subprocess.run(['docker','exec',CONTAINER,'pg_isready','-U','sfcloud','-d','sfcloud'],capture_output=True)
        if result.returncode==0: return
        time.sleep(.5)
    raise RuntimeError('dedicated Postgres not ready')

def sql(statement):
    return run(['docker','exec','-i',CONTAINER,'psql','-X','-q','-v','ON_ERROR_STOP=1','-U','sfcloud','-d',os.environ.get('SF_CLOUD_TEST_DATABASE','sfcloud'),'-At'],input=statement)

def request(method,path,body=None,who='host',expected=200):
    credentials=json.loads((PRIVATE/'credentials.json').read_text(encoding='utf-8'))
    token=next(x['token'] for x in credentials['principals'] if x['principal_id']==('device-principal' if who=='device' else who)) if who else None
    headers={'Content-Type':'application/json'}
    if token: headers['Authorization']='Bearer '+token
    req=Request(BASE+path,data=None if body is None else json.dumps(body,separators=(',',':')).encode(),method=method,headers=headers)
    try:
        response=urlopen(req,timeout=30);status=response.status;data=response.read();h=dict(response.headers)
    except HTTPError as e:
        status=e.code;data=e.read();h=dict(e.headers)
    parsed=json.loads(data) if data else None
    assert status==expected, (method,path,status,expected,parsed)
    return parsed,h

class Server:
    def __init__(self,env_override=None): self.env_override=env_override or {}
    def __enter__(self):
        self.log=(WORK/'server.log').open('ab');self.process=subprocess.Popen([str(EXE),'serve'],env=dict(environment(),**self.env_override),stdout=self.log,stderr=self.log,creationflags=subprocess.CREATE_NO_WINDOW if os.name=='nt' else 0)
        for _ in range(100):
            if self.process.poll() is not None: raise RuntimeError('owned service exited; inspect server.log')
            try:
                r=urlopen(BASE+'/healthz',timeout=1)
                if r.status==204: return self
            except OSError: pass
            time.sleep(.1)
        raise RuntimeError('healthz did not become ready')
    def __exit__(self,*args):
        self.process.terminate();self.process.wait(timeout=15);self.log.close()

def main():
    parser=argparse.ArgumentParser();parser.add_argument('command',choices=['prepare','db-start','migrate','verify','scale']);args=parser.parse_args()
    WORK.mkdir(parents=True,exist_ok=True)
    if args.command=='prepare': prepare()
    elif args.command=='db-start': db_start()
    elif args.command=='migrate': print(run([str(EXE),'migrate'],env=environment()).strip())
    elif args.command=='verify':
        from acceptance import verify
        verify()
    elif args.command=='scale':
        from scale import verify_scale
        verify_scale()

if __name__=='__main__': main()
