"""New canonical attachment identity/lifecycle subset; not actual Host or Codex."""
import copy
import ctypes
from ctypes import wintypes
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

HERE=Path(__file__).parent
ROOT=HERE.parents[3]
sys.path.insert(0,str(ROOT/'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator

# Reuse existing Win32 declarations; do not run/repeat its prior two-case main.
spec=importlib.util.spec_from_file_location('drf04_handle_primitives',HERE/'managed_asset_probe.py')
primitives=importlib.util.module_from_spec(spec);spec.loader.exec_module(primitives)
K=primitives.K
K.GetProcessId.argtypes=[wintypes.HANDLE];K.GetProcessId.restype=wintypes.DWORD
K.GetCurrentProcessId.restype=wintypes.DWORD
K.GetHandleInformation.argtypes=[wintypes.HANDLE,ctypes.POINTER(wintypes.DWORD)];K.GetHandleInformation.restype=wintypes.BOOL
K.GetFileSizeEx.argtypes=[wintypes.HANDLE,ctypes.POINTER(ctypes.c_longlong)];K.GetFileSizeEx.restype=wintypes.BOOL


def close_owned(handle):
    assert K.CloseHandle(handle)
    flags=wintypes.DWORD()
    still_valid=bool(K.GetHandleInformation(handle,ctypes.byref(flags)))
    error=0 if still_valid else ctypes.get_last_error()
    assert not still_valid and error==6
    return {'owned_handle_closed':True,'after_close_error':error}


def child():
    expected=json.loads(sys.argv[2])
    # Controlled fixture setup carries the actual duplicated ownership record.
    # It is NOT a public request accepting grants/handles from an Agent.
    setup=json.loads(sys.stdin.readline())
    line=sys.stdin.readline()
    if setup['owned_handle'] is None:
        print(json.dumps({'status':'PermissionDenied','returned_bytes':0,'handle_transferred':False}))
        return
    handle=wintypes.HANDLE(setup['owned_handle'])
    flags=wintypes.DWORD();assert K.GetHandleInformation(handle,ctypes.byref(flags))
    assert not flags.value & 1
    native_pid=K.GetCurrentProcessId();assert native_pid==setup['verified_bridge_pid']
    result={'returned_bytes':0,'actual_fixture_bridge_pid':native_pid,'non_inherited_handle':True}
    if not line:
        result.update(status='disconnected',**close_owned(handle))
        print(json.dumps(result));return
    frame=json.loads(line)
    data=frame['reply']['outcome']['data'];task=data['task']
    render=data['result']['data'];scene=render['scene'];attachment=frame['managed_assets'][0]
    context=frame['reply']['context']
    identities=[
        frame['host_instance_id']==expected['host_instance_id'],
        frame['client_session_id']==expected['client_session_id'],
        context['host_instance_id']==frame['host_instance_id'],
        context['session_id']==frame['client_session_id'],
        context['project_id']==expected['scope']['data'],
        context['scope']==expected['scope'],
        frame['originating_operation']=='get_task',
        data['originating_operation']=='render_view',
        task['state']=='completed',
        task['id']==attachment['task_id']==expected['task_id'],
        scene['file']['id']==attachment['asset_id']==expected['asset_id'],
        attachment['scope']==render['requested_scope']==expected['scope'],
        task['scope']=={'kind':'edit','data':expected['scope']},
        task['input_revision']==attachment['scope_revision']==render['requested_scope_revision']==scene['scene_revision']==expected['requested_revision'],
        attachment['handle_value']==setup['owned_handle'],
        attachment['mime_type']=='image/png' and scene['file']['kind']=='png',
    ]
    if not all(identities):
        result.update(status='PermissionDenied',**close_owned(handle))
        print(json.dumps(result));return
    size=ctypes.c_longlong();assert K.GetFileSizeEx(handle,ctypes.byref(size))
    if size.value!=attachment['byte_length']:
        result.update(status='IoFailure',**close_owned(handle))
        print(json.dumps(result));return
    read_count=0
    while read_count<size.value:
        buffer=ctypes.create_string_buffer(min(4096,size.value-read_count));n=wintypes.DWORD()
        assert K.ReadFile(handle,buffer,len(buffer),ctypes.byref(n),None)
        assert n.value>0
        read_count+=n.value
    result.update(status='ok',returned_bytes=read_count,requested_revision=render['requested_scope_revision'],
                  scene_revision=scene['scene_revision'],old_revision_result=frame['reply']['context']['scope_revision']!=render['requested_scope_revision'],
                  **close_owned(handle))
    print(json.dumps(result))


def main():
    case_doc=json.loads((HERE.parent/'DRF04_H04-02有限模型.json').read_text(encoding='utf-8'))
    source=json.loads((ROOT/'tools/type-contract-proof/ipc-managed-asset-cases.design.json').read_text(encoding='utf-8'))
    frame=copy.deepcopy(next(x['input'] for x in source['cases'] if x['id']=='IMA02'))
    task=frame['reply']['outcome']['data']['task'];render=frame['reply']['outcome']['data']['result']['data']
    task['input_revision']=7;render['requested_scope_revision']=7;render['scene']['scene_revision']=7;render['scene']['view']['project_revision']=7
    frame['reply']['context']['project_revision']=7;frame['reply']['context']['scope_revision']=7
    frame['managed_assets'][0]['scope_revision']=7
    # Never use the shape fixture's placeholder handle 12 or byte length 123.
    frame['managed_assets'][0]['handle_value']=0
    image=HERE/'output/managed-fixture.png'
    assert image.is_file()  # Existing self-built fixture is read only this round.
    schema_root=json.loads((ROOT/'tools/type-contract-proof/output/schemas/api-all-types.schema.json').read_text(encoding='utf-8'))
    schema={'$schema':schema_root['$schema'],'$defs':schema_root['$defs'],**schema_root['properties']['ipc_task_response']}
    validator=Draft202012Validator(schema)
    expected={'host_instance_id':frame['host_instance_id'],'client_session_id':frame['client_session_id'],
              'task_id':task['id'],'asset_id':render['scene']['file']['id'],'scope':render['requested_scope'],'requested_revision':7}
    records=[]
    for case in case_doc['cases']:
        worker=subprocess.Popen([sys.executable,'-B','-X','utf8',str(Path(__file__).resolve()),'reader',json.dumps(expected)],
            stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8',creationflags=subprocess.CREATE_NO_WINDOW)
        peer=K.OpenProcess(0x0040|0x1000,False,worker.pid)
        assert peer
        actual_pid=K.GetProcessId(peer);assert actual_pid==worker.pid
        recorded_pid=actual_pid+1 if case['event']=='wrong_registered_pid' else actual_pid
        payload=copy.deepcopy(frame);remote=None
        if recorded_pid==actual_pid:
            fh=K.CreateFileW(str(image.resolve()),0x80000000,1,None,3,0x80,None)
            assert fh!=ctypes.c_void_p(-1).value
            dup=wintypes.HANDLE()
            assert K.DuplicateHandle(K.GetCurrentProcess(),fh,peer,ctypes.byref(dup),0x80000000,False,0)
            K.CloseHandle(fh);remote=dup.value
            payload['managed_assets'][0]['handle_value']=remote
        K.CloseHandle(peer)
        payload['managed_assets'][0]['byte_length']=image.stat().st_size
        for change in case.get('changes',[]):
            keys=change['pointer'][1:].split('/');target=payload
            for key in keys[:-1]:target=target[int(key)] if isinstance(target,list) else target[key]
            if isinstance(target,list):target[int(keys[-1])]=change['value']
            else:target[keys[-1]]=change['value']
        if case['event']=='wrong_handle_value':payload['managed_assets'][0]['handle_value']=remote+4
        if case['event']=='wrong_byte_length':payload['managed_assets'][0]['byte_length']+=1
        validator.validate(payload)  # Real generated canonical schema, not stub DTO.
        setup={'owned_handle':remote,'verified_bridge_pid':actual_pid}
        stdin=json.dumps(setup)+'\n'
        if case['event']!='disconnect':stdin+=json.dumps(payload)+'\n'
        stdout,stderr=worker.communicate(stdin)
        assert worker.returncode==0,stderr
        result=json.loads(stdout)
        assert result['status']==case['expected']['status'],(case['id'],result)
        if case['expected']['returns_bytes']:assert result['returned_bytes']==image.stat().st_size
        else:assert result['returned_bytes']==0
        if remote is not None:assert result['owned_handle_closed'] and result['after_close_error']==6
        # Host-side business admission is a finite model, not an executed Engine.
        business={'state':'running','admitted':True,'cancel_calls':0,'resubmissions':0,'engine_instances':1}
        assert business==case_doc['independent_business_expected_after_event']
        records.append({'id':case['id'],'event':case['event'],'canonical_schema':'passed','native_result':result,
                        'business_model_after':business,'actual_Host_business_execution':None})
    record={'document_kind':'actual_canonical_schema_and_Windows_attachment_identity_lifecycle_fixture_not_product',
            'status':'passed_14_new_attachment_cases','cases':records,
            'old_trials_replayed':False,'old_readonly_access_evidence_ref':'docs/04_开发与验收/特征细化/DRF04_stdio-proof/output/managed-asset-result.json',
            'actual_subset':'canonical generated frame shape, OS process PID, duplicated read-only noninherited handle, identity comparison before bytes, actual CloseHandle/invalid-handle check and stdin EOF',
            'model_only':'trusted setup/session/task registry and noncancellation/resubmission/Engine counters; no real Host worker or named-pipe peer authentication in this new probe',
            'actual_Engine_processes_started':0,
            'actual_product_Host':None,'actual_renderer':None,'actual_Codex':None,'actual_product_runtime':None,'actual_result':None}
    (HERE/'output/attachment-lifecycle-result.json').write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'status':record['status'],'new_cases':len(records),'old_trials_replayed':False,'actual_product_runtime':None}))


if __name__=='__main__':
    if len(sys.argv)>1 and sys.argv[1]=='reader':child()
    else:main()
