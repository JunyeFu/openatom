//! Additional protocol combinations from the frozen connector contract.
use serde_json::{Value, json};
use sf_mcp_bridge::{decode::decode, framing::{MAX_FRAME_BYTES, read_frame, write_frame}};
use tokio::io::{AsyncWriteExt, duplex};

#[tokio::test]
async fn fragmented_and_adjacent_frames_preserve_utf8_and_exact_maximum_boundary() {
    let messages = vec![json!({"中文":"协议","n":1}), json!("x".repeat(MAX_FRAME_BYTES - 2))];
    let (mut writer, mut reader) = duplex(97);
    let expected = messages.clone();
    let send = tokio::spawn(async move {
        for message in messages {
            let bytes = serde_json::to_vec(&message).unwrap();
            writer.write_all(&(bytes.len() as u32).to_le_bytes()[..2]).await.unwrap();
            writer.write_all(&(bytes.len() as u32).to_le_bytes()[2..]).await.unwrap();
            for chunk in bytes.chunks(67) { writer.write_all(chunk).await.unwrap(); }
        }
    });
    for expected in expected { assert_eq!(read_frame(&mut reader).await.unwrap(), expected); }
    send.await.unwrap();
    assert_eq!(read_frame(&mut reader).await.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);
}

#[tokio::test]
async fn empty_oversize_truncated_and_invalid_json_frames_fail_without_a_value() {
    let cases = vec![
        0u32.to_le_bytes().to_vec(),
        ((MAX_FRAME_BYTES + 1) as u32).to_le_bytes().to_vec(),
        vec![1, 0],
        [5u32.to_le_bytes().to_vec(), b"{}".to_vec()].concat(),
        [1u32.to_le_bytes().to_vec(), vec![0xff]].concat(),
        [3u32.to_le_bytes().to_vec(), b"{xx".to_vec()].concat(),
    ];
    for bytes in cases {
        let (mut writer, mut reader) = duplex(32);
        writer.write_all(&bytes).await.unwrap();
        writer.shutdown().await.unwrap();
        assert!(read_frame(&mut reader).await.is_err());
    }
    let (mut writer, _) = duplex(8);
    assert_eq!(write_frame(&mut writer, &json!("x".repeat(MAX_FRAME_BYTES - 1))).await.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
}

fn inline() -> (Value, Value) {
    let request = json!({"api_version":"0.1","request_id":"inline","scope":{"kind":"workspace"},"project_id":null,
        "operation":"get_context","params":{"selection":[]}});
    let frame = json!({"wire_version":"1","host_instance_id":"host","client_session_id":"session","originating_operation":"get_context",
        "reply":{"api_version":"0.1","request_id":"inline","context":null,"outcome":{"status":"error","data":{
            "code":"PermissionDenied","message":"scope withheld","object_refs":[],"field_errors":[],"expected_revision":null,"actual_revision":null}}},
        "managed_results":null,"managed_assets":null});
    (request,frame)
}

#[test]
fn inline_correlation_and_unexpected_leases_are_rejected_before_acquisition() {
    for (pointer,value) in [
        ("/wire_version",json!("2")),("/host_instance_id",json!("foreign")),
        ("/client_session_id",json!("foreign")),("/originating_operation",json!("get_object")),
        ("/reply/request_id",json!("old")),("/reply/api_version",json!("other")),
        ("/managed_results",json!([{"result_id":"unexpected","handle_value":71}])),
    ] {
        let (request,mut frame)=inline();
        *frame.pointer_mut(pointer).unwrap()=value;
        assert!(decode(frame,&request,&json!("host"),&json!("session"),|_,_|panic!("unbound lease")).is_err(),"{pointer}");
    }
}

#[cfg(windows)]
#[test]
fn owned_os_leases_close_on_nonzero_offset_inheritance_and_foreign_announced_identity() {
    use sf_mcp_bridge::{managed_io::{ReceivedLease,publish_readonly},pipe_os::{self,Peer,ProcessHandle}};
    use std::{fs::File,io::{Seek,SeekFrom},os::windows::io::{FromRawHandle,IntoRawHandle}};
    use windows_sys::Win32::{Foundation::{GetHandleInformation,SetHandleInformation,HANDLE_FLAG_INHERIT},
        System::Threading::{OpenProcess,PROCESS_DUP_HANDLE,PROCESS_QUERY_LIMITED_INFORMATION}};
    let identity=pipe_os::current_identity().unwrap();
    let process=unsafe{OpenProcess(PROCESS_DUP_HANDLE|PROCESS_QUERY_LIMITED_INFORMATION,0,std::process::id())};
    assert!(!process.is_null());
    let peer=Peer{identity:identity.clone(),process:ProcessHandle(process)};
    let bytes=b"native read-only lease";
    for case in ["offset","inherited","announced"] {
        let handle=publish_readonly(bytes,&peer,&identity).unwrap();
        if case=="offset" {
            let mut file=unsafe{File::from_raw_handle(handle as usize as _)};
            file.seek(SeekFrom::Start(1)).unwrap();
            assert_eq!(file.into_raw_handle() as usize as u64,handle);
        }
        if case=="inherited" {
            assert_ne!(unsafe{SetHandleInformation(handle as usize as _,HANDLE_FLAG_INHERIT,HANDLE_FLAG_INHERIT)},0);
            assert!(unsafe{ReceivedLease::receive(handle)}.is_err());
        } else {
            let lease=unsafe{ReceivedLease::receive(handle)}.unwrap();
            assert!(lease.read_exact_original(if case=="announced" {handle+4} else {handle},bytes.len() as u64,||false).is_err());
        }
        let mut flags=0;
        assert_eq!(unsafe{GetHandleInformation(handle as usize as _,&mut flags)},0,"{case}: known lease must close");
    }
}
