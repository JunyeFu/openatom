"""HTTP + persistent PostgreSQL tests; no mock transport/database/Host claims."""
import base64, copy, concurrent.futures, json, subprocess, time
from urllib.parse import urlencode
from local import *

def verify():
    database='sfcloud_component_'+dt.datetime.now().strftime('%Y%m%d%H%M%S')
    sql('CREATE DATABASE '+database+';')
    os.environ['SF_CLOUD_TEST_DATABASE']=database
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        list(pool.map(lambda _:run([str(EXE),'migrate'],env=environment()),range(2)))
    results=[]
    def check(name,fn):
        start=time.monotonic()
        try: fn();results.append(dict(name=name,passed=True,elapsed_ms=round((time.monotonic()-start)*1000,2)))
        except Exception as e:
            results.append(dict(name=name,passed=False,error=str(e)))
            (WORK/'http-acceptance.json').write_text(json.dumps(dict(passed=False,checks=results),ensure_ascii=False,indent=2),encoding='utf-8')
            raise
    identity=fixture('ExecutionIdentity');sid=identity['session_id'];epoch=identity['execution_epoch_id']
    q='?'+urlencode({'execution_epoch_id':epoch})
    create=fixture('CloudSessionCreate');binding=fixture('CandidateBinding');create['candidate_bindings']=[binding];create['frozen_inputs']['candidate_bindings']=[binding]
    path=f'/v2/projects/example/sessions/{sid}'
    boot=fixture('DeviceBootRegisterRequest')
    command=fixture('Command');cid=command['attempt']['command_id']
    poll=fixture('DevicePollRequest');poll['accept_commands']=False
    snap=fixture('SnapshotPublication');snap['snapshot']['devices']=[]
    def value(method,path,body=None,who='host',expected=200): return request(method,path,body,who,expected)[0]
    def result(*args,**kwargs): return value(*args,**kwargs)['result']['data']
    def is_error(method,path,body,who,status,code):
        assert value(method,path,body,who,status)['error']['code']==code
    def load_query(aggregation='rows',devices=None,limit=1):
        return dict(dataset='telemetry',contract_version=VERSION,cloud_project_id='example',dataset_id='load-fixture',device_ids=devices or [],received_from='2020-01-01T00:00:00.000000Z',received_until='2090-01-01T00:00:00.000000Z',aggregation=aggregation,page=dict(limit=limit,cursor=None))
    with Server():
        check('OP01 real DB/schema health204 empty',lambda:request('GET','/healthz',expected=204))
        check('C01 unauthenticated401',lambda:is_error('PUT',path,create,None,401,'unauthenticated'))
        check('C01 project scope403',lambda:is_error('PUT',path,create,'outside',403,'permission_denied'))
        check('C01 reader cannot mutate403',lambda:is_error('PUT',path,create,'reader',403,'permission_denied'))
        check('C01 fake grant and unknown fields422',lambda:is_error('PUT',path,dict(create,grant='all'),'host',422,'invalid_input'))
        check('C01 URL grant injection rejected422',lambda:is_error('PUT',path+'?grant=all',create,'host',422,'invalid_input'))
        check('C01 unsupported wire version422',lambda:is_error('PUT',path,dict(create,contract_version='sf.execution/wrong'),'host',422,'unsupported_contract'))
        check('C01 frozen input mismatch422',lambda:is_error('PUT',path,dict(create,frozen_inputs=dict(create['frozen_inputs'],identity=dict(identity,project_id='wrong'))),'host',422,'invalid_input'))
        def create_race():
            with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool: responses=list(pool.map(lambda _:result('PUT',path,create),range(4)))
            assert sum(x['storage_status']=='stored' for x in responses)==1
        no_device=copy.deepcopy(create);no_device['frozen_inputs']['project']['devices']=[]
        check('C01 frozen candidate device reference required422',lambda:is_error('PUT',path,no_device,'host',422,'invalid_input'))
        no_definition=copy.deepcopy(create);no_definition['frozen_inputs']['execution_definitions']=[]
        check('C01 frozen exact module definition required422',lambda:is_error('PUT',path,no_definition,'host',422,'invalid_input'))
        check('C01 concurrent atomic create and exact retries',create_race)
        check('C01 immutable exact retry',lambda:assert_equal(result('PUT',path,create)['storage_status'],'already_stored_same_payload'))
        changed=copy.deepcopy(create);changed['policy']['observation_max_age_ms']+=1
        check('C01 different immutable policy409',lambda:is_error('PUT',path,changed,'host',409,'immutable_payload_conflict'))
        other=copy.deepcopy(create);other['identity']['session_id']='another-session';other['frozen_inputs']['identity']=other['identity']
        check('C01 project active slot409',lambda:is_error('PUT','/v2/projects/example/sessions/another-session',other,'host',409,'active_session_conflict'))
        check('C12 exact project dataset catalog',lambda:assert_equal(len(result('GET','/v2/projects/example/datasets',who='reader')['datasets']),3))
        check('C13 actual active slot',lambda:assert_equal(result('GET','/v2/projects/example/active-execution',who='reader')['active_slot']['session_id'],sid))
        check('C16 frozen inputs immutable typed read',lambda:assert_contains(result('GET',f'/v2/sessions/{sid}/inputs'+q,who='reader'),create['frozen_inputs']))
        check('C16 wrong epoch409',lambda:is_error('GET',f'/v2/sessions/{sid}/inputs?execution_epoch_id=wrong',None,'reader',409,'epoch_mismatch'))
        check('C16 URL grant injection rejected422',lambda:is_error('GET',f'/v2/sessions/{sid}/inputs'+q+'&grant=all',None,'reader',422,'invalid_input'))
        check('C19 boot register',lambda:assert_equal(result('POST','/v2/devices/example/boots/register',boot,'device')['state'],'active'))
        check('C19 boot exact replay',lambda:result('POST','/v2/devices/example/boots/register',boot,'device'))
        check('C19 competing active boot409',lambda:is_error('POST','/v2/devices/example/boots/register',dict(boot,device_boot_id='other-boot'),'device',409,'device_boot_conflict'))
        device_view=fixture('ExecutionDeviceView');device_view.update(binding=binding,registered_device_boot_id=boot['device_boot_id'],pending_observation_probe=fixture('ObservationProbe'))
        snap['snapshot']['devices']=[device_view]
        check('C06 publish exact boot observation probe',lambda:result('PUT',f'/v2/sessions/{sid}/snapshots/0',snap))
        check('C02 command with wrong target boot409',lambda:is_error('PUT',f'/v2/sessions/{sid}/commands/{cid}',dict(command,target_device_boot_id='other-boot'),'host',409,'device_boot_mismatch'))
        check('C02 store actual command',lambda:result('PUT',f'/v2/sessions/{sid}/commands/{cid}',command))
        check('C02 exact command retry',lambda:assert_equal(result('PUT',f'/v2/sessions/{sid}/commands/{cid}',command)['storage_status'],'already_stored_same_payload'))
        altered=copy.deepcopy(command);altered['parameters']['nominal_work_ms']+=1
        check('C02 changed command409',lambda:is_error('PUT',f'/v2/sessions/{sid}/commands/{cid}',altered,'host',409,'immutable_payload_conflict'))
        second=copy.deepcopy(command);second['attempt']['command_id']='another-command';second['attempt']['attempt_ordinal']+=1
        check('C02 concurrent phase attempt blocked409',lambda:is_error('PUT',f'/v2/sessions/{sid}/commands/another-command',second,'host',409,'immutable_payload_conflict'))
        check('C03 accept_commands=false keeps command queued',lambda:assert_equal(result('POST','/v2/devices/example/poll',poll,'device')['state'],'empty'))
        check('C03 exact empty replay remains empty',lambda:assert_equal(result('POST','/v2/devices/example/poll',poll,'device')['state'],'empty'))
        check('C03 accept_commands=false still returns exact Host probe',lambda:assert_equal(result('POST','/v2/devices/example/poll',poll,'device')['observation_probe'],device_view['pending_observation_probe']))
        check('C03 poll identity mutation409',lambda:is_error('POST','/v2/devices/example/poll',dict(poll,accept_commands=True),'device',409,'immutable_payload_conflict'))
        poll2={}
        def claim_race():
            inputs=[dict(poll,poll_request_id='claim-'+str(i),accept_commands=True) for i in range(6)]
            with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool: responses=list(pool.map(lambda req:result('POST','/v2/devices/example/poll',req,'device'),inputs))
            assert sum(x['state']=='command' for x in responses)==1
            poll2.update(inputs[next(i for i,x in enumerate(responses) if x['state']=='command')])
        check('C03 six concurrent distinct polls claim exactly once',claim_race)
        check('C03 claims queued command once',lambda:assert_equal(result('POST','/v2/devices/example/poll',poll2,'device')['command'],command))
        check('C03 exact command replay',lambda:assert_equal(result('POST','/v2/devices/example/poll',poll2,'device')['command'],command))
        check('C03 new poll does not redispatch claimed command',lambda:assert_equal(result('POST','/v2/devices/example/poll',dict(poll2,poll_request_id='after-claim'),'device')['state'],'empty'))
        withdrawal=dict(contract_version=VERSION,identity=identity,command_id=cid)
        check('C09 claimed withdrawal needs device confirmation',lambda:assert_equal(result('POST',f'/v2/commands/{cid}/withdraw',withdrawal)['state'],'device_confirmation_required'))
        check('C14 real delivery/cancel fact, no Host confirmation invented',lambda:assert_equal(result('GET',f'/v2/commands/{cid}',who='reader')['cloud_delivery']['state'],'claimed'))
        receipt=fixture('CloudReceiptRequest');receipt['observation']['receipt']={'kind':'completed','device_reported_duration_ms':7}
        receipt_result=result('POST','/v2/devices/example/events',receipt,'device')
        def receipt_race():
            with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool: responses=list(pool.map(lambda _:result('POST','/v2/devices/example/events',receipt,'device'),range(6)))
            assert all(x==receipt_result for x in responses)
        check('C04 six concurrent exact retries share receipt/time/frontier',receipt_race)
        check('C04 exact receipt retry preserves original cloud time',lambda:assert_equal(result('POST','/v2/devices/example/events',receipt,'device'),receipt_result))
        mutated=copy.deepcopy(receipt);mutated['observation']['receipt']['device_reported_duration_ms']=8
        check('C04 same boot sequence different payload409',lambda:is_error('POST','/v2/devices/example/events',mutated,'device',409,'immutable_payload_conflict'))
        bad=copy.deepcopy(receipt);bad['observation']['device_sequence']+=1;bad['observation']['phase_token']='wrong'
        check('C04 receipt phase token mismatch422',lambda:is_error('POST','/v2/devices/example/events',bad,'device',422,'invalid_input'))
        check('C05 bounded committed ingress page',lambda:assert_equal(len(result('GET',f'/v2/sessions/{sid}/events'+q)['rows']),1))
        cq=dict(identity=identity,command_id=cid,page=dict(limit=1,cursor=None),contract_version=VERSION)
        check('C15 immutable raw command receipt',lambda:assert_equal(len(result('GET',f'/v2/commands/{cid}/receipts?'+urlencode({'query':json.dumps(cq)}),who='reader')['rows']),1))
        check('C06 publish prepared snapshot',lambda:result('PUT',f'/v2/sessions/{sid}/snapshots/0',snap))
        check('C06 exact publication retry',lambda:assert_equal(result('PUT',f'/v2/sessions/{sid}/snapshots/0',snap)['publication_status'],'already_stored_same_payload'))
        check('C06 same immutable snapshot tolerates refreshed CAS envelope',lambda:assert_equal(result('PUT',f'/v2/sessions/{sid}/snapshots/0',dict(snap,expected_published_revision=0))['publication_status'],'already_stored_same_payload'))
        check('C07 cloud read has no controls',lambda:assert_equal(result('GET',f'/v2/sessions/{sid}/snapshots/latest'+q,who='reader')['access']['allowed_session_controls'],[]))
        new=copy.deepcopy(snap);new['snapshot']['execution_revision']=2
        check('C06 incorrect CAS409',lambda:is_error('PUT',f'/v2/sessions/{sid}/snapshots/2',new,'host',409,'revision_conflict'))
        new['expected_published_revision']=0;new['snapshot']['coverage']['observed_until_host_ms']=100;new['snapshot']['coverage']['clock_started']=True
        logical=fixture('LogicalWorkView');logical['state']='queued'
        records=[dict(record_sequence=i,execution_revision=i,at_host_ms=i*10,object=dict(kind='logical_work',data=dict(logical,state='queued' if i==1 else 'reserved'))) for i in [1,2]]
        new['snapshot']['observation_frontier']=2;new['observation_records']=records
        broken=copy.deepcopy(new);broken['observation_records']=records[1:]
        check('C06 missing observation prefix rejects atomically422',lambda:is_error('PUT',f'/v2/sessions/{sid}/snapshots/2',broken,'host',422,'invalid_input'))
        check('C06 coalesced revisions publish full contiguous observation prefix',lambda:result('PUT',f'/v2/sessions/{sid}/snapshots/2',new))
        oq=dict(contract_version=VERSION,identity=identity,execution_revision=2,at_host_ms=15,objects=[],page=dict(limit=1,cursor=None))
        check('C17 selected revision as-of state',lambda:assert_equal(result('POST',f'/v2/sessions/{sid}/observations/query',oq,'reader')['objects'][0]['data']['state'],'queued'))
        hq=dict(contract_version=VERSION,identity=identity,execution_revision=2,host_from_ms=0,host_until_ms=100,objects=[],page=dict(limit=1,cursor=None))
        hist=result('POST',f'/v2/sessions/{sid}/history/query',hq,'reader');hq['page']['cursor']=hist['next_cursor']
        check('C18 contiguous history fixed F pagination',lambda:assert_equal(result('POST',f'/v2/sessions/{sid}/history/query',hq,'reader')['records'][0]['record_sequence'],2))
        closed=copy.deepcopy(hq);closed.update(host_from_ms=10,host_until_ms=10);closed['page']['cursor']=None
        check('C18 equal bounds include exact Host timestamp',lambda:assert_equal(result('POST',f'/v2/sessions/{sid}/history/query',closed,'reader')['records'][0]['record_sequence'],1))
        eq=dict(dataset='execution',contract_version=VERSION,identity=identity,selection=dict(kind='latest'),host_from_ms=0,host_until_ms=100,aggregation='rows',page=dict(limit=1,cursor=None))
        check('C08 selected execution snapshot revision',lambda:assert_equal(result('POST','/v2/data/query',eq,'reader')['selected_execution_revision'],2))
        # Dedicated generated component events use different IDs from the frozen 1M scale sample.
        batch=dict(contract_version=VERSION,dataset_id='load-fixture',batch_id='component-batch',source_mode='generated_load',events=[dict(device_ref='G000',device_sequence=20000+i,source_mode='generated_load',observed_at='2026-01-01T00:00:00.000000Z',metric='status_sample',value=i) for i in range(3)])
        check('C11 wrong role403',lambda:is_error('POST','/v2/datasets/load-fixture/load-batches',batch,'reader',403,'permission_denied'))
        check('C11 atomic load batch',lambda:assert_equal(result('POST','/v2/datasets/load-fixture/load-batches',batch,'load')['inserted_count'],3))
        check('C11 exact batch retry',lambda:assert_equal(result('POST','/v2/datasets/load-fixture/load-batches',batch,'load')['reused_count'],3))
        def batch_race():
            concurrent_batch=dict(batch,batch_id='concurrent-batch',events=[dict(batch['events'][0],device_sequence=40000)])
            with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool: responses=list(pool.map(lambda _:result('POST','/v2/datasets/load-fixture/load-batches',concurrent_batch,'load'),range(6)))
            assert sum(x['inserted_count'] for x in responses)==1 and sum(x['reused_count'] for x in responses)==5
        check('C11 six concurrent batches advance frontier once',batch_race)
        changed=copy.deepcopy(batch);changed['events'].reverse()
        check('C11 reordered retry409',lambda:is_error('POST','/v2/datasets/load-fixture/load-batches',changed,'load',409,'immutable_payload_conflict'))
        check('C11 another batch cannot reuse device sequence409',lambda:is_error('POST','/v2/datasets/load-fixture/load-batches',dict(batch,batch_id='another-batch'),'load',409,'immutable_payload_conflict'))
        query=load_query();first=result('POST','/v2/data/query',query,'reader');frontier=first['selected_visible_frontier'];query['page']['cursor']=first['next_cursor']
        more=dict(batch,batch_id='component-batch-2',events=[dict(batch['events'][0],device_sequence=30000)])
        result('POST','/v2/datasets/load-fixture/load-batches',more,'load')
        check('C08 raw pagination pins F despite later ingress',lambda:assert_equal(result('POST','/v2/data/query',query,'reader')['selected_visible_frontier'],frontier))
        changed=copy.deepcopy(query);changed['received_from']='2021-01-01T00:00:00.000000Z'
        check('C08 cursor filter tampering422',lambda:is_error('POST','/v2/data/query',changed,'reader',422,'invalid_input'))
        no_rows=load_query(devices=['unused'])
        check('C08 explicit authorized device without rows retained',lambda:assert_equal(result('POST','/v2/data/query',no_rows,'reader')['selected_device_ids'],['unused']))
        denied=load_query(devices=['not-authorized'])
        check('C08 explicit out of scope device403',lambda:is_error('POST','/v2/data/query',denied,'reader',403,'permission_denied'))
        same=load_query();same['received_until']=same['received_from']
        check('C08 equal half-open time range empty',lambda:assert_equal(result('POST','/v2/data/query',same,'reader')['rows'],[]))
        same['received_until']='2019-01-01T00:00:00.000000Z'
        check('C08 inverted time range422',lambda:is_error('POST','/v2/data/query',same,'reader',422,'invalid_input'))
        check('C08 UTC minute aggregate reads real events',lambda:assert_equal(sum(x['count'] for x in result('POST','/v2/data/query',load_query('count_by_device_minute',limit=200),'reader')['rows']),5))
        check('C10 live boot prevents premature release409',lambda:is_error('POST',f'/v2/sessions/{sid}/release-slot',dict(contract_version=VERSION,identity=identity,expected_published_revision=2),'host',409,'active_session_conflict'))
        terminal=copy.deepcopy(new);terminal['expected_published_revision']=2;terminal['snapshot']['execution_revision']=3;terminal['snapshot']['lifecycle']['state']='draining';terminal['observation_records']=[]
        terminal['as_cutoff_snapshot']=True;terminal['snapshot']['coverage'].update(requested_cutoff_reached=True,cutoff_snapshot_revision=3)
        attempt=fixture('AttemptView');attempt['attempt']=command['attempt'];attempt['state']='accepted';attempt['receipt_ids']=[receipt_result['receipt_id']];attempt['target_device_boot_id']=boot['device_boot_id'];terminal['snapshot']['attempts']=[attempt]
        admission=fixture('ReceiptAdmission');admission.update(receipt_id=receipt_result['receipt_id'],execution_revision=3,decision='applied',host_received_elapsed_ms=50,business_effect_committed=True);terminal['snapshot']['receipt_admissions']=[admission]
        accepted=fixture('AcceptanceRow')['event'];accepted.update(at_ms=50,quantity=dict(unit='piece',value=dict(coefficient=123,decimal_places=2)));terminal['snapshot']['business_facts']['acceptance_events']=[accepted]
        waiting=fixture('LogicalWorkView');waiting['state']='output_wait';terminal['snapshot']['logical_work']=[waiting]
        check('C06 terminal Host admission and command close atomic',lambda:result('PUT',f'/v2/sessions/{sid}/snapshots/3',terminal))
        metric=copy.deepcopy(eq);metric['selection']=dict(kind='revision',execution_revision=3);metric['aggregation']='accepted_quantity'
        check('C08 exact fixed-point accepted quantity from Host event',lambda:assert_equal(result('POST','/v2/data/query',metric,'reader')['rows'][0]['quantity'],accepted['quantity']))
        metric['aggregation']='command_counts'
        check('C08 command counts from selected Host attempt',lambda:assert_equal(result('POST','/v2/data/query',metric,'reader')['rows'][0]['count'],1))
        metric['aggregation']='waiting_time'
        check('C08 unknown Host waiting duration remains null',lambda:assert_equal(result('POST','/v2/data/query',metric,'reader')['rows'][0]['known_host_ms'],None))
        first=result('POST','/v2/data/query',eq,'reader');assert first['selected_execution_revision']==3
        completed=copy.deepcopy(terminal);completed['expected_published_revision']=3;completed['snapshot']['execution_revision']=4;completed['snapshot']['lifecycle']['state']='completed';completed['snapshot']['logical_work']=[];completed['as_cutoff_snapshot']=False
        changed_admission=copy.deepcopy(completed);changed_admission['snapshot']['receipt_admissions'][0]['business_effect_committed']=False
        check('C06 incremental validation rejects rewritten admission',lambda:is_error('PUT',f'/v2/sessions/{sid}/snapshots/4',changed_admission,'host',422,'invalid_input'))
        removed_admission=copy.deepcopy(completed);removed_admission['snapshot']['receipt_admissions']=[]
        check('C06 incremental validation rejects removed admission',lambda:is_error('PUT',f'/v2/sessions/{sid}/snapshots/4',removed_admission,'host',422,'invalid_input'))
        check('C06 complete drained snapshot4',lambda:result('PUT',f'/v2/sessions/{sid}/snapshots/4',completed))
        pinned=copy.deepcopy(eq);pinned['page']['cursor']=first['next_cursor']
        check('C08 latest cursor pins snapshot3 across newer publication4',lambda:assert_equal(result('POST','/v2/data/query',pinned,'reader')['selected_execution_revision'],3))
        check('C07 immutable cutoff remains snapshot3',lambda:assert_equal(result('GET',f'/v2/sessions/{sid}/snapshots/cutoff'+q,who='reader')['snapshot']['execution_revision'],3))
        check('C03 original poll keeps original probe across later snapshots',lambda:assert_equal(result('POST','/v2/devices/example/poll',poll,'device')['observation_probe'],device_view['pending_observation_probe']))
        retire=dict(contract_version=VERSION,session_id=sid,execution_epoch_id=epoch,expected_published_revision=4)
        check('C20 retire original boot at exact published revision',lambda:assert_equal(result('POST',f"/v2/devices/example/boots/{boot['device_boot_id']}/retire",retire)['state'],'retired'))
        check('C19 retired boot replay never resurrects',lambda:assert_equal(result('POST','/v2/devices/example/boots/register',boot,'device')['state'],'retired'))
        check('C03 retired boot exact poll replay preserved',lambda:assert_equal(result('POST','/v2/devices/example/poll',poll2,'device')['command'],command))
        check('C04 retired boot exact event replay preserves timestamp',lambda:assert_equal(result('POST','/v2/devices/example/events',receipt,'device'),receipt_result))
        check('C03 retired boot new poll409',lambda:is_error('POST','/v2/devices/example/poll',dict(poll2,poll_request_id='retired-new'),'device',409,'device_boot_mismatch'))
        check('C10 terminal/retired/settled release',lambda:assert_equal(result('POST',f'/v2/sessions/{sid}/release-slot',dict(contract_version=VERSION,identity=identity,expected_published_revision=4))['released'],True))
    # Restart the owned database and actual HTTP process, retaining the exact PG volume.
    run(['docker','restart',CONTAINER]);db_start()
    with Server():
        check('restart persistence exact C01 replay',lambda:assert_equal(result('PUT',path,create)['storage_status'],'already_stored_same_payload'))
        check('restart persistence immutable snapshot3',lambda:assert_equal(result('GET',f'/v2/sessions/{sid}/snapshots/3'+q,who='reader')['snapshot']['execution_revision'],3))
    from edge_acceptance import verify_edges
    verify_edges(check)
    receipt=dict(passed=True,scope='real Axum HTTP / dedicated PostgreSQL component acceptance; synthetic DTO fixtures, no Native/Host integration claim',checks=results,check_count=len(results),database=database,concurrent_migrate_processes=2,database_version=sql('SELECT version();').strip(),schema_tables=int(sql("SELECT count(*) FROM information_schema.tables WHERE table_schema='public';").strip()),service_stopped=True,database_preserved=True)
    (WORK/'http-acceptance.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2),encoding='utf-8');print(f"HTTP/PG checks passed: {len(results)}")

def assert_equal(actual,expected):
    assert actual==expected,(actual,expected)

def assert_contains(actual,expected):
    # Common domain DTOs serialize explicitly nullable optional fields on readback.
    if isinstance(expected,dict):
        for k,v in expected.items(): assert_contains(actual[k],v)
    elif isinstance(expected,list):
        assert len(actual)==len(expected)
        for a,b in zip(actual,expected): assert_contains(a,b)
    else: assert_equal(actual,expected)
