"""Real failure/concurrency boundaries, preserving all test databases and rows."""
import base64
from local import *
from acceptance import assert_equal

def verify_edges(check):
    def data(method,path,v=None,who='host',expected=200): return request(method,path,v,who,expected)[0]
    def result(*args,**kwargs):return data(*args,**kwargs)['result']['data']
    def error(method,path,v,who,status,code):assert data(method,path,v,who,status)['error']['code']==code
    with Server():
        v=fixture('CloudSessionCreate');v['identity']['session_id']='withdraw-session';v['frozen_inputs']['identity']=v['identity'];binding=fixture('CandidateBinding');v['candidate_bindings']=[binding];v['frozen_inputs']['candidate_bindings']=[binding]
        result('PUT','/v2/projects/example/sessions/withdraw-session',v)
        boot=dict(contract_version=VERSION,session_id='withdraw-session',execution_epoch_id='example',device_boot_id='withdraw-boot')
        result('POST','/v2/devices/example/boots/register',boot,'device')
        command=fixture('Command');command['identity']=v['identity'];command['attempt'].update(command_id='withdraw-command',logical_job_id='withdraw-job');command['target_device_boot_id']='withdraw-boot'
        result('PUT','/v2/sessions/withdraw-session/commands/withdraw-command',command)
        w=dict(contract_version=VERSION,identity=v['identity'],command_id='withdraw-command')
        check('C09 queued withdrawal proves no delivery',lambda:assert_equal(result('POST','/v2/commands/withdraw-command/withdraw',w)['state'],'withdrawn_before_delivery'))
        poll=dict(boot,poll_request_id='withdrawn-poll',accept_commands=True)
        check('C03 withdrawn command is never dispatched',lambda:assert_equal(result('POST','/v2/devices/example/poll',poll,'device')['state'],'empty'))
        snap=fixture('SnapshotPublication');snap['snapshot']['identity']=v['identity'];snap['snapshot']['lifecycle']['state']='stopped';a=fixture('AttemptView');a.update(attempt=command['attempt'],state='cancelled_not_started',receipt_ids=[],target_device_boot_id='withdraw-boot');snap['snapshot']['attempts']=[a]
        claim=fixture('ProjectedResourceClaim');claim['claim'].update(units=1,intervals=[dict(start_ms=0,end_ms=0)],interval_end_causes=['observation_end']);claim['open_interval_index']=0;snap['snapshot']['business_facts']['resource_claims']=[claim]
        check('C06 queued withdrawal settles without invented device receipt',lambda:result('PUT','/v2/sessions/withdraw-session/snapshots/0',snap))
        retirement=dict(contract_version=VERSION,session_id='withdraw-session',execution_epoch_id='example',expected_published_revision=0)
        check('C20 open projected resource claim blocks retirement409',lambda:error('POST','/v2/devices/example/boots/withdraw-boot/retire',retirement,'host',409,'device_boot_conflict'))
        closed=copy.deepcopy(snap);closed['expected_published_revision']=0;closed['snapshot']['execution_revision']=1;closed['snapshot']['business_facts']['resource_claims'][0]['open_interval_index']=None;closed['snapshot']['business_facts']['resource_claims'][0]['claim']['interval_end_causes']=['released']
        check('C06 Host publishes actual resource closure at revision1',lambda:result('PUT','/v2/sessions/withdraw-session/snapshots/1',closed))
        retirement['expected_published_revision']=1
        result('POST','/v2/devices/example/boots/withdraw-boot/retire',retirement)
        check('C20 repeated retirement preserves original revision1',lambda:assert_equal(result('POST','/v2/devices/example/boots/withdraw-boot/retire',retirement)['retired_at_published_revision'],1))
        check('C20 changed retired revision conflicts',lambda:error('POST','/v2/devices/example/boots/withdraw-boot/retire',dict(retirement,expected_published_revision=2),'host',409,'revision_conflict'))
        result('POST','/v2/sessions/withdraw-session/release-slot',dict(contract_version=VERSION,identity=v['identity'],expected_published_revision=1))
        observation=fixture('TelemetryOnlyObservation');observation.update(dataset_id='telemetry-fixture',device_boot_id='telemetry-boot',device_sequence=1)
        ack=result('POST','/v2/devices/example/events',observation,'device')
        check('C04 telemetry_only stored without Host admission',lambda:assert_equal(ack['host_admission_state'],'not_applicable'))
        check('C04 telemetry_only exact retry preserves cloud time',lambda:assert_equal(result('POST','/v2/devices/example/events',observation,'device'),ack))
        check('C04 telemetry_only cannot inject active command',lambda:error('POST','/v2/devices/example/events',dict(observation,active_command_id='example'),'device',422,'invalid_input'))
        query=dict(dataset='telemetry',contract_version=VERSION,cloud_project_id='example',dataset_id='telemetry-fixture',device_ids=['example'],received_from='2020-01-01T00:00:00Z',received_until='2090-01-01T00:00:00Z',aggregation='rows',page=dict(limit=200,cursor=None))
        check('C08 telemetry_only read separated by dataset purpose',lambda:assert_equal(result('POST','/v2/data/query',query,'reader')['rows'][0]['observation'],observation))
        check('C05 telemetry_only never enters execution ingress',lambda:assert_equal(len(result('GET','/v2/sessions/example/events?execution_epoch_id=example')['rows']),1))
        query['dataset_id']='load-fixture';query['device_ids']=[];query['page']['limit']=1
        first=result('POST','/v2/data/query',query,'reader');query['page']['cursor']=first['next_cursor']
        stamp=first['rows'][0]['cloud_received_at'];instant=dt.datetime.fromisoformat(stamp.replace('Z','+00:00'))
        micro=copy.deepcopy(query);micro['page']['cursor']=None;micro.update(received_from=stamp,received_until=(instant+dt.timedelta(microseconds=1)).isoformat(timespec='microseconds'))
        check('C08 exact microsecond half-open boundary includes original batch',lambda:assert_equal(len(result('POST','/v2/data/query',dict(micro,page=dict(limit=200,cursor=None)),'reader')['rows']),3))
        offset=copy.deepcopy(micro);offset['received_from']=instant.astimezone(dt.timezone(dt.timedelta(hours=8))).isoformat(timespec='microseconds');offset['page']['limit']=200
        check('C08 offset normalization preserves microsecond filter',lambda:assert_equal(result('POST','/v2/data/query',offset,'reader')['rows'],result('POST','/v2/data/query',dict(micro,page=dict(limit=200,cursor=None)),'reader')['rows']))
        forged=copy.deepcopy(query);c=json.loads(base64.urlsafe_b64decode(forged['page']['cursor']+'==='));c['visible_frontier']+=100;forged['page']['cursor']=base64.urlsafe_b64encode(json.dumps(c).encode()).decode().rstrip('=')
        check('C08 unsigned cursor cannot exceed committed frontier',lambda:error('POST','/v2/data/query',forged,'reader',422,'invalid_input'))
        run(['docker','stop',CONTAINER])
        check('OP01 actual DB unavailable returns503 empty',lambda:assert_equal(request('GET','/healthz',expected=503)[0],None))
        db_start()
        check('OP01 actual DB restored returns204 empty',lambda:assert_equal(request('GET','/healthz',expected=204)[0],None))
    credentials=json.loads((PRIVATE/'credentials.json').read_text(encoding='utf-8'));catalog=credentials['principals']
    for p in catalog:
        if p['principal_id']=='reader':p['device_ids']=[x for x in p['device_ids'] if x!='G000']
    with Server({'SF_CLOUD_AUTH_TOKENS_JSON':json.dumps(catalog)}):
        check('C08 changed authorization cannot silently shrink cursor device set',lambda:error('POST','/v2/data/query',query,'reader',422,'invalid_input'))
    # Failed migration must leave no partially created schema; unknown version is preserved.
    original_db=os.environ['SF_CLOUD_TEST_DATABASE'];suffix=dt.datetime.now().strftime('%Y%m%d%H%M%S')
    for kind in ['rollback','unknown']:
        os.environ.pop('SF_CLOUD_TEST_DATABASE',None);db='sfcloud_'+kind+'_'+suffix;sql('CREATE DATABASE '+db+';');os.environ['SF_CLOUD_TEST_DATABASE']=db
        env=environment()
        if kind=='rollback':
            cfg=json.loads((WORK/'component-bindings.json').read_text());cfg['device_bindings'].append(copy.deepcopy(cfg['device_bindings'][0]));cfgpath=WORK/'migration-conflict-bindings.json';cfgpath.write_text(json.dumps(cfg),encoding='utf-8');env['SF_CLOUD_BINDINGS_FILE']=str(cfgpath)
        else:sql('CREATE TABLE schema_migrations(version integer PRIMARY KEY,applied_at timestamptz NOT NULL DEFAULT clock_timestamp()); INSERT INTO schema_migrations(version) VALUES(2);')
        failed=subprocess.run([str(EXE),'migrate'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        check('TX13 '+kind+' migration rejects with exit1',lambda:assert_equal(failed.returncode,1))
        if kind=='rollback':check('TX13 failed migration rolls back all16 tables',lambda:assert_equal(sql("SELECT count(*) FROM information_schema.tables WHERE table_schema='public';").strip(),'0'))
        else:check('TX13 unknown schema version2 remains unchanged',lambda:assert_equal(sql('SELECT version FROM schema_migrations;').strip(),'2'))
    os.environ['SF_CLOUD_TEST_DATABASE']=original_db
    credentials=json.loads((PRIVATE/'credentials.json').read_text(encoding='utf-8'));credentials['principals'][0]['grant']='all'
    bad_env=dict(environment(),SF_CLOUD_AUTH_TOKENS_JSON=json.dumps(credentials['principals']))
    denied=subprocess.run([str(EXE),'migrate'],env=bad_env,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    check('principal catalog cannot inject grant or unknown fields',lambda:assert_equal(denied.returncode,1))
    cfg=json.loads((WORK/'component-bindings.json').read_text());cfg['device_bindings'][0]['capability_map']['grant']='all';cfgpath=WORK/'unknown-capability-bindings.json';cfgpath.write_text(json.dumps(cfg),encoding='utf-8')
    denied=subprocess.run([str(EXE),'migrate'],env=dict(environment(),SF_CLOUD_BINDINGS_FILE=str(cfgpath)),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    check('fixed capability map rejects unknown grant field',lambda:assert_equal(denied.returncode,1))
