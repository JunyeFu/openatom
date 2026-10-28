"""Bounded real HTTP/PostgreSQL checks for multistage physical device mapping."""
import copy, gzip, json, os, sys
from urllib.request import Request,urlopen
from pathlib import Path
import local as cloud
from acceptance import assert_contains

out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=False)
catalog = json.loads((cloud.WORK / 'component-bindings.json').read_text(encoding='utf-8'))
create = cloud.fixture('CloudSessionCreate')
first = cloud.fixture('CandidateBinding')
second = copy.deepcopy(first)
second.update(binding_id='second-stage-binding', phase_id='second-stage', nominal_work_ms=1400)
create['candidate_bindings'] = [first, second]
create['frozen_inputs']['candidate_bindings'] = [first, second]
catalog['device_bindings'][0]['capability_map'] = {'stage_bindings':[first,second]}
(out/'component-bindings.json').write_text(json.dumps(catalog), encoding='utf-8')
cloud.WORK = out
database='sfcloud_factory_'+cloud.dt.datetime.now().strftime('%Y%m%d%H%M%S')
cloud.sql('CREATE DATABASE '+database+';')
os.environ['SF_CLOUD_TEST_DATABASE']=database
cloud.run([str(cloud.EXE),'migrate'],env=cloud.environment())
def result(method,path,body=None,who='host',expected=200):
    response=cloud.request(method,path,body,who,expected)[0]
    return response['result']['data'] if expected==200 else response
checks=[]
with cloud.Server():
    sid=create['identity']['session_id']; endpoint='/v2/projects/example/sessions/'+sid
    assert result('PUT',endpoint,create)['storage_status']=='stored';checks.append('two stages admitted on one physical device')
    assert result('PUT',endpoint,create)['storage_status']=='already_stored_same_payload';checks.append('exact full input retry preserved')
    credentials=json.loads((cloud.PRIVATE/'credentials.json').read_text(encoding='utf-8'))
    token=next(p['token'] for p in credentials['principals'] if p['principal_id']=='host')
    request=Request(cloud.BASE+endpoint,data=gzip.compress(json.dumps(create).encode()),method='PUT',headers={'Authorization':'Bearer '+token,'Content-Type':'application/json','Content-Encoding':'gzip'})
    with urlopen(request,timeout=30) as response:compressed=json.load(response)
    assert compressed['result']['data']['storage_status']=='already_stored_same_payload';checks.append('gzip upload retains exact original input and identity')
    snapshot=cloud.fixture('SnapshotPublication');snapshot['snapshot']['devices']=[]
    revision=snapshot['snapshot']['execution_revision']
    snapshot_path='/v2/sessions/'+sid+'/snapshots/'+str(revision)
    request=Request(cloud.BASE+snapshot_path,data=gzip.compress(json.dumps(snapshot).encode()),method='PUT',headers={'Authorization':'Bearer '+token,'Content-Type':'application/json','Content-Encoding':'gzip'})
    with urlopen(request,timeout=30) as response:published=json.load(response)
    assert published['result']['data']['execution_revision']==revision
    assert result('PUT',snapshot_path,snapshot)['execution_revision']==revision
    checks.append('compressed snapshot and plain retry preserve the same revision')
    wrong=copy.deepcopy(create);wrong['frozen_inputs']['project']['title']='changed frozen title'
    result('PUT',endpoint,wrong,expected=409);checks.append('changed frozen input rejected on retry')
    inputs=result('GET','/v2/sessions/'+sid+'/inputs?execution_epoch_id='+create['identity']['execution_epoch_id'],who='reader')
    assert_contains(inputs,create['frozen_inputs']);checks.append('separate immutable input row reads back exactly')
    boot=cloud.fixture('DeviceBootRegisterRequest');result('POST','/v2/devices/example/boots/register',boot,'device')
    probe=cloud.fixture('ObservationProbe');probe.update(device_boot_id=boot['device_boot_id'],issued_execution_revision=revision+1,expires_at_host_ms=30000)
    view=cloud.fixture('ExecutionDeviceView');view.update(binding=first,registered_device_boot_id=boot['device_boot_id'],pending_observation_probe=probe)
    last_view=copy.deepcopy(view);last_view['binding']=second;probe=dict(probe,probe_id='last-stage-probe');last_view['pending_observation_probe']=probe
    snapshot['expected_published_revision']=revision;snapshot['snapshot']['execution_revision']=revision+1;snapshot['snapshot']['devices']=[view,last_view]
    result('PUT','/v2/sessions/'+sid+'/snapshots/'+str(revision+1),snapshot)
    for index,binding in enumerate([first,second]):
        command=cloud.fixture('Command');cid='stage-command-'+str(index)
        command['attempt'].update(command_id=cid,logical_job_id='stage-job',selected_binding_id=binding['binding_id'],phase_id=binding['phase_id'])
        command['parameters'].update(phase_id=binding['phase_id'],nominal_work_ms=binding['nominal_work_ms'])
        command['target_device_boot_id']=boot['device_boot_id']
        assert result('PUT','/v2/sessions/'+sid+'/commands/'+cid,command)['storage_status']=='stored'
        poll=dict(boot,poll_request_id='stage-poll-'+str(index),accept_commands=True)
        polled=result('POST','/v2/devices/example/poll',poll,'device')
        assert polled['observation_probe']==probe
        assert result('POST','/v2/devices/example/poll',poll,'device')==polled
        delivered=polled['command']
        assert delivered['attempt']['selected_binding_id']==binding['binding_id']
        assert delivered['target_device_boot_id']==boot['device_boot_id']
    checks.append('both exact stage commands delivered to the same registered boot')
    checks.append('device-specific probe projection and exact poll retry retain the original boot')
    bad=copy.deepcopy(command);bad['attempt']['command_id']='wrong-phase-command';bad['parameters']['nominal_work_ms']=9999
    result('PUT','/v2/sessions/'+sid+'/commands/wrong-phase-command',bad,expected=422);checks.append('wrong stage duration rejected')
    rows=cloud.sql("SELECT count(*) FROM device_boots;").strip();assert rows=='1'
    checks.append('one physical boot persisted for both stage bindings')
    # Bounded reproduction of the actual factory's 40k-record publication backlog.
    bulk=copy.deepcopy(snapshot);bulk['expected_published_revision']=revision+1
    bulk['snapshot']['execution_revision']=revision+2;bulk['snapshot']['observation_frontier']=40000
    bulk['snapshot']['coverage']['observed_until_host_ms']=1
    logical=cloud.fixture('LogicalWorkView')
    bulk['observation_records']=[dict(record_sequence=i,execution_revision=revision+2,at_host_ms=1,object=dict(kind='logical_work',data=logical)) for i in range(1,40001)]
    bulk_path='/v2/sessions/'+sid+'/snapshots/'+str(revision+2)
    broken=copy.deepcopy(bulk);broken['observation_records'].pop(100)
    result('PUT',bulk_path,broken,expected=422)
    assert result('PUT',bulk_path,bulk)['observation_frontier']==40000
    assert result('PUT',bulk_path,bulk)['publication_status']=='already_stored_same_payload'
    assert cloud.sql('SELECT count(*) FROM execution_observations;').strip()=='40000'
    checks.append('40000-record batch preserves contiguous prefix, atomic gap rejection and exact retry')
    transport=dict(id='factory-transport',supply_ref='factory-supply',cargo=['factory-cargo'],carrier_id='example',phase='loading',departed_ms=None,arrived_ms=None,received_ms=None,returned_ms=None)
    bulk['expected_published_revision']=revision+2;bulk['snapshot']['execution_revision']=revision+3
    bulk['snapshot']['observation_frontier']=40001;bulk['snapshot']['business_facts']['transport']=[transport]
    bulk['observation_records']=[dict(record_sequence=40001,execution_revision=revision+3,at_host_ms=1,object=dict(kind='transport',data=transport))]
    transport_path='/v2/sessions/'+sid+'/snapshots/'+str(revision+3)
    result('PUT',transport_path,bulk);assert result('PUT',transport_path,bulk)['publication_status']=='already_stored_same_payload'
    query=dict(contract_version=cloud.VERSION,identity=create['identity'],execution_revision=revision+3,at_host_ms=1,objects=[dict(kind='transport',id='factory-transport')],page=dict(limit=10,cursor=None))
    observed=result('POST','/v2/sessions/'+sid+'/observations/query',query,who='reader')
    assert observed['objects']==[dict(kind='transport',data=transport)]
    checks.append('transport observation persists and reads with the exact object identity and retry')
    cloud.run([str(cloud.EXE),'migrate'],env=cloud.environment())
    assert cloud.sql('SELECT string_agg(version::text,\',\' ORDER BY version) FROM schema_migrations;').strip()=='1,2'
    checks.append('schema 2 migration is repeatable and retains transport rows')
(out/'receipt.json').write_text(json.dumps(dict(passed=True,checks=checks,database=database,scope='real local HTTP/PostgreSQL component proof; no Native whole-factory claim'),indent=2),encoding='utf-8')
print('Actual HTTP/PostgreSQL factory checks passed:',len(checks))
