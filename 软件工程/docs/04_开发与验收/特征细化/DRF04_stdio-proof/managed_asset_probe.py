"""Real Windows read-only file handle transfer; fixture image, not product render."""
import base64
import ctypes
from ctypes import wintypes
import json
from pathlib import Path
import subprocess
import sys

K=ctypes.WinDLL('kernel32',use_last_error=True)
K.CreateFileW.argtypes=[wintypes.LPCWSTR,wintypes.DWORD,wintypes.DWORD,ctypes.c_void_p,wintypes.DWORD,wintypes.DWORD,wintypes.HANDLE]
K.CreateFileW.restype=wintypes.HANDLE
K.OpenProcess.argtypes=[wintypes.DWORD,wintypes.BOOL,wintypes.DWORD]
K.OpenProcess.restype=wintypes.HANDLE
K.GetCurrentProcess.restype=wintypes.HANDLE
K.DuplicateHandle.argtypes=[wintypes.HANDLE,wintypes.HANDLE,wintypes.HANDLE,ctypes.POINTER(wintypes.HANDLE),wintypes.DWORD,wintypes.BOOL,wintypes.DWORD]
K.DuplicateHandle.restype=wintypes.BOOL
K.ReadFile.argtypes=[wintypes.HANDLE,ctypes.c_void_p,wintypes.DWORD,ctypes.POINTER(wintypes.DWORD),ctypes.c_void_p]
K.WriteFile.argtypes=K.ReadFile.argtypes
K.CloseHandle.argtypes=[wintypes.HANDLE]
K.CloseHandle.restype=wintypes.BOOL


def child_reader():
    # This fixture's parent supplies expected state at process launch. It is not
    # an API accepting grants, nor a real Host's artifact/permission registry.
    expected=json.loads(sys.argv[2])
    message=json.loads(sys.stdin.readline())
    handle=wintypes.HANDLE(message['handle_value'])
    identity_keys=['task_id','asset_id','scope','scope_revision']
    if any(message[k]!=expected[k] for k in identity_keys):
        K.CloseHandle(handle)
        print(json.dumps({'status':'PermissionDenied','returned_bytes':0}))
        return
    data=bytearray()
    while True:
        buffer=ctypes.create_string_buffer(4096); count=wintypes.DWORD()
        if not K.ReadFile(handle,buffer,len(buffer),ctypes.byref(count),None):
            raise ctypes.WinError(ctypes.get_last_error())
        if count.value==0: break
        data.extend(buffer.raw[:count.value])
    count=wintypes.DWORD(); attempted=ctypes.create_string_buffer(b'X')
    write_ok=bool(K.WriteFile(handle,attempted,1,ctypes.byref(count),None))
    write_error=0 if write_ok else ctypes.get_last_error()
    K.CloseHandle(handle)
    print(json.dumps({'status':'ok','image_base64':base64.b64encode(data).decode(),'returned_bytes':len(data),'write_succeeded':write_ok,'write_error':write_error,'scope_revision':message['scope_revision']}))


def main():
    root=Path(__file__).resolve().parents[4]
    out=Path(__file__).parent/'output'
    out.mkdir(exist_ok=True)
    # Own copy only; no write attempt against the existing shared fixture.
    image=out/'managed-fixture.png'
    source=(root/'tools/architecture-proof/assets/self-built-image-fixture.png').read_bytes()
    image.write_bytes(source)
    expected={'task_id':'fixture-render-task','asset_id':'fixture-image','scope':'fixture-project','scope_revision':7}
    results=[]
    for case in ('matching_scope','mismatched_scope_revision'):
        worker=subprocess.Popen([sys.executable,'-B','-X','utf8',str(Path(__file__).resolve()),'reader',json.dumps(expected)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8',creationflags=subprocess.CREATE_NO_WINDOW)
        source_handle=K.CreateFileW(str(image.resolve()),0x80000000,1,None,3,0x80,None)
        if source_handle==ctypes.c_void_p(-1).value: raise ctypes.WinError(ctypes.get_last_error())
        process=K.OpenProcess(0x0040,False,worker.pid)
        if not process: raise ctypes.WinError(ctypes.get_last_error())
        duplicate=wintypes.HANDLE()
        if not K.DuplicateHandle(K.GetCurrentProcess(),source_handle,process,ctypes.byref(duplicate),0x80000000,False,0):
            raise ctypes.WinError(ctypes.get_last_error())
        K.CloseHandle(source_handle);K.CloseHandle(process)
        message=dict(expected,handle_value=duplicate.value)
        if case=='mismatched_scope_revision':message['scope_revision']=8
        stdout,stderr=worker.communicate(json.dumps(message)+'\n')
        assert worker.returncode==0,stderr
        result=json.loads(stdout)
        if case=='matching_scope':
            assert base64.b64decode(result['image_base64'])==source
            assert result['write_succeeded'] is False and result['write_error']==5
            result.pop('image_base64')
            result['actual_readonly_duplicate_handle']=True
        else:assert result=={'status':'PermissionDenied','returned_bytes':0}
        results.append(dict(case=case,**result))
    assert image.read_bytes()==source
    record={'document_kind':'actual_Windows_handle_transfer_and_fixture_scope_guard_not_product_render',
            'status':'passed_native_readonly_asset_subprocess_and_scope_negative',
            'cases':results,'fixture_image_unchanged':True,
            'actual_renderer':None,'actual_Codex_call':None,'real_Host_result_registry':None,
            'actual_product_runtime':None,
            'limit':'real DuplicateHandle/ReadFile/WriteFile access check; fixture-selected task/scope only, not canonical IPC attachment or production authorization'}
    (out/'managed-asset-result.json').write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:record[k] for k in ('status','fixture_image_unchanged','actual_renderer','actual_product_runtime')},ensure_ascii=False))


if __name__=='__main__':
    if len(sys.argv)>1 and sys.argv[1]=='reader':child_reader()
    else:main()
