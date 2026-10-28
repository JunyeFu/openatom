"""AQ-01 finite canonical-Schema and actual Windows cross-process handle trial."""
from pathlib import Path
import ctypes as C
from ctypes import wintypes as W
import copy, json, os, subprocess, sys, tempfile

HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE/'python-libs'))
from jsonschema import Draft202012Validator
OUT=HERE/'output'

def kernel():
    k=C.WinDLL('kernel32',use_last_error=True)
    k.GetCurrentProcess.restype=W.HANDLE
    k.OpenProcess.argtypes=[W.DWORD,W.BOOL,W.DWORD];k.OpenProcess.restype=W.HANDLE
    k.GetHandleInformation.argtypes=[W.HANDLE,C.POINTER(W.DWORD)]
    k.CloseHandle.argtypes=[W.HANDLE]
    k.DuplicateHandle.argtypes=[W.HANDLE,W.HANDLE,W.HANDLE,C.POINTER(W.HANDLE),W.DWORD,W.BOOL,W.DWORD]
    k.WriteFile.argtypes=[W.HANDLE,C.c_void_p,W.DWORD,C.POINTER(W.DWORD),C.c_void_p]
    return k

def child():
    import msvcrt
    k=kernel();print('READY',flush=True)
    q=json.loads(sys.stdin.readline());h=q['handle'];flags=W.DWORD()
    assert k.GetHandleInformation(h,C.byref(flags)) and not flags.value&1
    count=W.DWORD();buf=C.create_string_buffer(b'x')
    denied=not k.WriteFile(h,buf,1,C.byref(count),None) and C.get_last_error()==5
    if q['cancel']:
        assert k.CloseHandle(h)
        assert not k.GetHandleInformation(h,C.byref(flags))
        print(json.dumps({'cancelled':True,'closed':True,'write_denied':denied}),flush=True);return
    fd=msvcrt.open_osfhandle(h,os.O_RDONLY|os.O_BINARY)
    with os.fdopen(fd,'rb') as stream:raw=stream.read(q['length']+1)
    closed=not k.GetHandleInformation(h,C.byref(flags))
    result={'bytes':len(raw),'closed':closed,'write_denied':denied,'exact_length':len(raw)==q['length']}
    if result['exact_length']:
        try:result['payload']=json.loads(raw)
        except json.JSONDecodeError:result['decode_error']=True
    print(json.dumps(result,separators=(',',':')),flush=True)

def transfer(data,declared_length,cancel=False,disconnect=False):
    import msvcrt
    k=kernel()
    with tempfile.TemporaryDirectory(prefix='sf-aq01-') as tmp:
        path=Path(tmp)/'response.json';path.write_bytes(data)
        process=subprocess.Popen([sys.executable,'-B',str(Path(__file__).resolve()),'--child'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8')
        assert process.stdout.readline().strip()=='READY'
        target=k.OpenProcess(0x40,False,process.pid);assert target
        try:
            with path.open('rb') as source:
                handle=W.HANDLE()
                assert k.DuplicateHandle(k.GetCurrentProcess(),msvcrt.get_osfhandle(source.fileno()),target,C.byref(handle),0x80000000,False,0)
                if disconnect:
                    process.terminate();process.communicate();return {'peer_exited':process.returncode is not None}
                text,error=process.communicate(json.dumps({'handle':handle.value,'length':declared_length,'cancel':cancel})+'\n')
                assert process.returncode==0,error
                return json.loads(text)
        finally:k.CloseHandle(target)

def main():
    load=lambda n:json.loads((OUT/n).read_text(encoding='utf-8-sig'))
    schema=load('api-schemas/get_run.response.schema.json');validator=Draft202012Validator(schema)
    full=load('aq01-full-response.json');outer=load('aq01-managed-run.response.json');frame=load('aq01-managed-run.frame.json')
    validator.validate(full);validator.validate(outer)
    bundle=load('schemas/api-all-types.schema.json')
    defs=bundle['$defs']
    Draft202012Validator(dict(bundle['properties']['ipc_run_response'], **{'$defs':defs})).validate(frame)
    assert frame['reply']==outer and len(json.dumps(frame,separators=(',',':')).encode())<=1048576
    for row in load('aq01-negative-responses.json'):assert list(validator.iter_errors(row['response'])),row['id']
    raw=(OUT/'aq01-full-response.json').read_bytes();ref=outer['outcome']['data']
    assert len(raw)==ref['byte_length']>1048576
    def correlated(f):
        metadata=f['reply']['outcome']['data'];attachments=f.get('managed_results') or []
        return f['originating_operation']=='get_run' and metadata['response_schema_ref']=='tools/type-contract-proof/output/api-schemas/get_run.response.schema.json' and len(attachments)==1 and attachments[0]['result_id']==metadata['result_id']
    assert correlated(frame)
    bad=copy.deepcopy(frame);bad['reply']['outcome']['data']['response_schema_ref']='other.response.schema.json';assert not correlated(bad)
    bad=copy.deepcopy(frame);bad['managed_results'][0]['result_id']='OTHER';assert not correlated(bad)
    bad=copy.deepcopy(frame);bad['originating_operation']='get_task';assert not correlated(bad)
    def accept(payload,request_id=outer['request_id'],run_id='R1'):
        validator.validate(payload)
        return payload['outcome']['status']!='managed' and payload['request_id']==request_id and payload['context']==outer['context'] and payload['outcome']['data']['data']['id']==run_id
    normal=transfer(raw,len(raw));assert normal['payload']==full and normal['closed'] and normal['write_denied'] and accept(normal['payload'])
    short=transfer(raw,len(raw)+1);assert not short['exact_length'] and short['closed']
    cancelled=transfer(raw,len(raw),cancel=True);assert cancelled['cancelled'] and cancelled['closed']
    disconnected=transfer(raw,len(raw),disconnect=True);assert disconnected['peer_exited']
    malformed=transfer(b'{',1);assert malformed['decode_error'] and malformed['closed']
    wrong=copy.deepcopy(full);wrong['request_id']='OTHER';assert not accept(wrong)
    wrong=copy.deepcopy(full);wrong['outcome']['data']['data']['id']='R2';assert not accept(wrong)
    assert not accept(outer)
    embedded={'type':'resource','resource':{'uri':'superfactory-result://'+ref['result_id'],'mimeType':'application/json','text':raw.decode('utf-8')}}
    assert json.loads(embedded['resource']['text'])==full
    result={'status':'passed_canonical_schema_and_actual_Windows_cross_process_readonly_transfer','payload_bytes':len(raw),'frame_bytes':len((OUT/'aq01-managed-run.frame.json').read_bytes()),'frame_cap':1048576,'schema_positives':3,'schema_negatives':3,'actual_handle_cases':['complete_bytes_and_write_denied','short_read_cleanup','cancel_cleanup','peer_disconnect','decode_failure_cleanup'],'finite_identity_rejections':['wrong_request','wrong_run','nested_managed','wrong_schema','wrong_result_attachment','wrong_operation'],'MCP_embedded_resource_roundtrip':True,'actual_product_Host_or_Codex_client':False,'actual_product_runtime':None}
    (OUT/'aq01-transport-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result,ensure_ascii=False))

if __name__=='__main__':
    child() if '--child' in sys.argv else main()
