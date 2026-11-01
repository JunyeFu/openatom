#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use base64::{Engine, engine::general_purpose::STANDARD};
use rmcp::{ServiceExt, handler::server::wrapper::Parameters, model::{CallToolRequestParams, CallToolResult, ContentBlock as Content}, tool, tool_router, transport::{TokioChildProcess, stdio}};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashMap, ffi::c_void, io, mem::size_of, os::windows::io::AsRawHandle, path::PathBuf, ptr, sync::{Arc, Barrier, Mutex, atomic::{AtomicBool, Ordering}}, time::Duration};
use tauri::State;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::windows::named_pipe::{ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions}, sync::{Mutex as AsyncMutex, Notify, watch}};
use windows_sys::Win32::{Foundation::{CloseHandle, GetHandleInformation, LocalFree}, Security::{Authorization::{ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW}, GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_GROUPS, TOKEN_QUERY, TokenGroups}, System::{Pipes::{GetNamedPipeClientProcessId, GetNamedPipeClientSessionId, GetNamedPipeServerProcessId}, RemoteDesktop::ProcessIdToSessionId, Threading::{GetCurrentProcess, GetExitCodeProcess, OpenProcess, OpenProcessToken, WaitForSingleObject}}};
type AnyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Serialize)]
struct Snapshot { scope: String, revision: u64, x_m: f64, object_id: String, task_id: String }
#[derive(Clone, Serialize)]
struct WorkState { status: String, completed_steps: u32, input_revision: u64, error: Option<String> }
struct Work { cancelled: AtomicBool, tx: watch::Sender<WorkState> }
struct Data {
    revision: u64, x_m: f64, frozen: Option<Snapshot>, capture: Option<String>,
    edits: Vec<Value>, agent: Option<Value>, bridge_pid: u32, pipe_peer: Value,
    pipe_eof: bool, pipe_closed_error: i32, pipe_close_waits: u32, agent_started: bool, work: HashMap<String, Arc<Work>>,
}
struct OwnedProcess(usize);
impl Drop for OwnedProcess { fn drop(&mut self) { unsafe { CloseHandle(self.0 as *mut c_void); } } }
struct Lab { data: Mutex<Data>, race: Barrier, capture_ready: Notify, agent_done: Notify, output: PathBuf }
impl Lab {
    fn context(&self) -> Value {
        let d = self.data.lock().unwrap();
        json!({"scope":"P1", "revision":d.revision, "x_m":d.x_m, "object_id":"D1", "host_pid":std::process::id()})
    }
    fn edit(&self, source: &str, revision: u64, x_m: f64) -> Value {
        self.race.wait();
        let mut d = self.data.lock().unwrap();
        let answer = if d.revision != revision {
            json!({"ok":false,"error":"RevisionConflict","current_revision":d.revision,"source":source})
        } else {
            d.revision += 1; d.x_m = x_m;
            json!({"ok":true,"revision":d.revision,"x_m":d.x_m,"source":source})
        };
        d.edits.push(answer.clone()); answer
    }
}
fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }
fn logon_sid() -> io::Result<String> {
    unsafe {
        let mut token = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 { return Err(io::Error::last_os_error()); }
        let mut length = 0;
        GetTokenInformation(token, TokenGroups, ptr::null_mut(), 0, &mut length);
        let mut buffer = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
        let ok = GetTokenInformation(token, TokenGroups, buffer.as_mut_ptr().cast(), length, &mut length);
        let err = io::Error::last_os_error(); CloseHandle(token);
        if ok == 0 { return Err(err); }
        let groups = &*buffer.as_ptr().cast::<TOKEN_GROUPS>();
        let entry = std::slice::from_raw_parts(groups.Groups.as_ptr(), groups.GroupCount as usize).iter()
            .find(|g| g.Attributes & 0xC0000000 == 0xC0000000).ok_or_else(|| io::Error::other("No actual logon SID"))?;
        let mut text = ptr::null_mut();
        if ConvertSidToStringSidW(entry.Sid, &mut text) == 0 { return Err(io::Error::last_os_error()); }
        let mut n = 0; while *text.add(n) != 0 { n += 1; }
        let sid = String::from_utf16_lossy(std::slice::from_raw_parts(text, n)); LocalFree(text.cast()); Ok(sid)
    }
}
fn make_pipe(name: &str) -> io::Result<NamedPipeServer> {
    let sddl = wide(&format!("D:P(A;;GA;;;SY)(A;;GRGW;;;{})", logon_sid()?));
    unsafe {
        let mut descriptor = ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.as_ptr(), 1, &mut descriptor, ptr::null_mut()) == 0 { return Err(io::Error::last_os_error()); }
        let mut attrs = SECURITY_ATTRIBUTES { nLength:size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor:descriptor, bInheritHandle:0 };
        let pipe = ServerOptions::new().first_pipe_instance(true).reject_remote_clients(true)
            .create_with_security_attributes_raw(name, (&mut attrs as *mut SECURITY_ATTRIBUTES).cast::<c_void>());
        LocalFree(descriptor); pipe
    }
}
async fn write_frame<S: AsyncWriteExt + Unpin>(s: &mut S, v: &Value) -> io::Result<()> {
    let bytes = serde_json::to_vec(v)?; s.write_u32_le(bytes.len() as u32).await?; s.write_all(&bytes).await?; s.flush().await
}
async fn read_frame<S: AsyncReadExt + Unpin>(s: &mut S) -> AnyResult<Option<Value>> {
    let size = match s.read_u32_le().await { Ok(n) => n, Err(e) if e.kind()==io::ErrorKind::UnexpectedEof => return Ok(None), Err(e) => return Err(e.into()) };
    if size > 1_048_576 { return Err(io::Error::other("Fixture frame cap").into()); }
    let mut data=vec![0u8;size as usize]; s.read_exact(&mut data).await?; Ok(Some(serde_json::from_slice(&data)?))
}
async fn pipe_host(mut pipe: NamedPipeServer, lab: Arc<Lab>) -> AnyResult<()> {
    pipe.connect().await?;
    let raw=pipe.as_raw_handle() as usize; let mut peer_pid=0; let mut peer_session=0; let mut own_session=0;
    unsafe {
        if GetNamedPipeClientProcessId(raw as *mut c_void, &mut peer_pid)==0 || GetNamedPipeClientSessionId(raw as *mut c_void, &mut peer_session)==0 || ProcessIdToSessionId(std::process::id(), &mut own_session)==0 { return Err(io::Error::last_os_error().into()); }
    }
    {
        let mut d=lab.data.lock().unwrap();
        if peer_pid != d.bridge_pid || peer_session != own_session { return Err(io::Error::other("Actual bridge peer mismatch").into()); }
        d.pipe_peer=json!({"actual_host_pid":std::process::id(),"actual_bridge_pid":peer_pid,"actual_session":peer_session,"current_logon_sid":logon_sid()?,"explicit_logon_DACL":true});
    }
    while let Some(q)=read_frame(&mut pipe).await? {
        let op=q["op"].as_str().unwrap_or("");
        let answer=match op {
            "hello" => json!({"ok":q["expected_host"]==format!("S05-host-{}",std::process::id()),"actual_host_pid":std::process::id()}),
            "edit" => {
                let l=lab.clone(); let revision=q["revision"].as_u64().unwrap(); let x=q["x_m"].as_f64().unwrap();
                tokio::task::spawn_blocking(move || l.edit("actual_mcp_bridge",revision,x)).await?
            },
            "capture" => {
                if q["scope"]!="P1" { json!({"ok":false,"error":"ScopeMismatch"}) }
                else if q["revision"]!=7 { json!({"ok":false,"error":"FrozenRevisionMismatch"}) }
                else {
                    lab.capture_ready.notified().await;
                    let d=lab.data.lock().unwrap();
                    json!({"ok":true,"scope":"P1","task_id":"capture-r7","asset_id":"fixture-r7-png","scene_revision":7,"requested_revision":7,"current_revision":d.revision,"png_base64":d.capture.as_ref().unwrap()})
                }
            },
            _ => json!({"ok":false,"error":"UnsupportedFixtureOperation"})
        };
        write_frame(&mut pipe,&answer).await?;
    }
    drop(pipe);
    // First actual attempt showed Mio's cancelled overlapped IO keeps an Arc after Drop.
    // Await only this known handle's release; no transport/tool retries or foreign closes.
    for waits in 0..=100 {
        let mut flags=0;
        let invalid=unsafe { GetHandleInformation(raw as *mut c_void,&mut flags)==0 };
        let close_error=io::Error::last_os_error().raw_os_error().unwrap_or(0);
        if invalid && close_error==6 {
            let mut d=lab.data.lock().unwrap(); d.pipe_eof=true; d.pipe_closed_error=close_error; d.pipe_close_waits=waits; return Ok(());
        }
        if waits==100 { return Err(io::Error::other("Known own pipe handle release not observed after IO completion wait").into()); }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    unreachable!()
}
#[derive(Clone)]
struct Bridge { pipe: Arc<AsyncMutex<NamedPipeClient>> }
impl Bridge {
    async fn call(&self,q:Value) -> Result<Value,rmcp::ErrorData> {
        let mut pipe=self.pipe.lock().await;
        write_frame(&mut *pipe,&q).await.map_err(|e|rmcp::ErrorData::internal_error(e.to_string(),None))?;
        read_frame(&mut *pipe).await.map_err(|e|rmcp::ErrorData::internal_error(e.to_string(),None))?.ok_or_else(||rmcp::ErrorData::internal_error("Actual pipe EOF",None))
    }
}
#[derive(Deserialize,schemars::JsonSchema)]
struct EditInput { base_revision:u64, x_m:f64 }
#[derive(Deserialize,schemars::JsonSchema)]
struct CaptureInput { scope:String, revision:u64 }
#[tool_router(server_handler)]
impl Bridge {
    #[tool(description="Edit isolated shared desktop fixture, no product operation")]
    async fn fixture_edit(&self,Parameters(p):Parameters<EditInput>) -> Result<String,rmcp::ErrorData> {
        Ok(self.call(json!({"op":"edit","revision":p.base_revision,"x_m":p.x_m})).await?.to_string())
    }
    #[tool(description="Return actual frozen WebView2 PNG from the isolated Host fixture")]
    async fn fixture_capture(&self,Parameters(p):Parameters<CaptureInput>) -> Result<CallToolResult,rmcp::ErrorData> {
        let mut response=self.call(json!({"op":"capture","scope":p.scope,"revision":p.revision})).await?;
        if response["ok"]==false { return Ok(CallToolResult::error(vec![Content::text(response.to_string())])); }
        let png=response.as_object_mut().unwrap().remove("png_base64").unwrap().as_str().unwrap().to_string();
        Ok(CallToolResult::success(vec![Content::text(response.to_string()),Content::image(png,"image/png")]))
    }
}
fn mcp_text(r:&CallToolResult) -> AnyResult<Value> {
    let text=r.content.iter().find_map(|c|c.as_text()).ok_or_else(||io::Error::other("No real MCP text"))?;
    Ok(serde_json::from_str(&text.text)?)
}
async fn agent_client(lab:Arc<Lab>, pipe:NamedPipeServer, name:String) -> AnyResult<Value> {
    let mut command=tokio::process::Command::new(std::env::current_exe()?);
    command.arg("bridge").arg(name).arg(std::process::id().to_string()).creation_flags(0x08000000);
    let transport=TokioChildProcess::new(command)?;
    let bridge_pid=transport.id().ok_or_else(||io::Error::other("No actual bridge PID"))?;
    lab.data.lock().unwrap().bridge_pid=bridge_pid;
    let process_value=unsafe { OpenProcess(0x1000|0x00100000,0,bridge_pid) } as usize;
    if process_value==0 { return Err(io::Error::last_os_error().into()); }
    let process=OwnedProcess(process_value);
    let pipe_task=tokio::spawn(pipe_host(pipe,lab.clone()));
    let client=().serve(transport).await?;
    let tools=client.list_all_tools().await?;
    let names:Vec<_>=tools.iter().map(|t|t.name.to_string()).collect();
    let edit=mcp_text(&client.call_tool(CallToolRequestParams::new("fixture_edit").with_arguments(json!({"base_revision":7,"x_m":8.0}).as_object().unwrap().clone())).await?)?;
    let mut rejections=Vec::new();
    for (scope,revision) in [("foreign",7),("P1",8)] {
        let r=client.call_tool(CallToolRequestParams::new("fixture_capture").with_arguments(json!({"scope":scope,"revision":revision}).as_object().unwrap().clone())).await?;
        if r.is_error!=Some(true) { return Err(io::Error::other("MCP wrong scope/revision admitted").into()); }
        rejections.push(mcp_text(&r)?);
    }
    let result=client.call_tool(CallToolRequestParams::new("fixture_capture").with_arguments(json!({"scope":"P1","revision":7}).as_object().unwrap().clone())).await?;
    let raw=serde_json::to_value(&result)?;
    let image=raw["content"].as_array().unwrap().iter().find(|c|c["type"]=="image").ok_or_else(||io::Error::other("No actual MCP image"))?;
    let bytes=STANDARD.decode(image["data"].as_str().unwrap())?;
    std::fs::write(lab.output.join("mcp-r7.png"),&bytes)?;
    std::fs::write(lab.output.join("actual-mcp-capture.json"),serde_json::to_vec_pretty(&raw)?)?;
    let meta=mcp_text(&result)?;
    client.cancel().await?;
    pipe_task.await??;
    let exit_code=tokio::task::spawn_blocking(move || -> io::Result<u32> {
        let handle=process.0 as *mut c_void;
        unsafe {
            let wait=WaitForSingleObject(handle,5000); let mut code=0;
            let ok=GetExitCodeProcess(handle,&mut code);
            if wait!=0 || ok==0 || code!=0 { return Err(io::Error::other(format!("Owned bridge exit wait={wait} code={code}"))); }
            Ok(code)
        }
    }).await??;
    Ok(json!({"status":"passed","official_sdk":"rmcp3.5.1","bridge_pid":bridge_pid,"discovered_tools":names,"edit_result":edit,"rejections":rejections,"image_metadata":meta,"actual_image_bytes":bytes.len(),"bridge_exit_code":exit_code,"bridge_creation_flags":"CREATE_NO_WINDOW","pipe_eof":true}))
}
#[tauri::command]
fn get_context(lab:State<'_,Arc<Lab>>) -> Value { lab.context() }
#[tauri::command]
fn freeze_render(lab:State<'_,Arc<Lab>>,scope:String,revision:u64) -> Result<Snapshot,String> {
    let mut d=lab.data.lock().unwrap();
    if scope!="P1" { return Err("ScopeMismatch".into()); }
    if revision!=d.revision { return Err("RevisionConflict".into()); }
    let s=Snapshot { scope,revision,x_m:d.x_m,object_id:"D1".into(),task_id:"capture-r7".into() };
    d.frozen=Some(s.clone()); Ok(s)
}
#[tauri::command]
async fn start_agent(lab:State<'_,Arc<Lab>>) -> Result<Value,String> {
    let l=lab.inner().clone(); let name=format!(r"\\.\pipe\drf04-s05-desktop-{}",std::process::id());
    let pipe=make_pipe(&name).map_err(|e|e.to_string())?;
    l.data.lock().unwrap().agent_started=true;
    tauri::async_runtime::spawn(async move {
        let result=agent_client(l.clone(),pipe,name).await;
        let value=match result { Ok(v)=>v, Err(e)=>json!({"status":"failed","error":e.to_string()}) };
        l.data.lock().unwrap().agent=Some(value); l.agent_done.notify_one();
    });
    Ok(json!({"started":true}))
}
#[tauri::command]
async fn ui_edit(lab:State<'_,Arc<Lab>>,base_revision:u64,x_m:f64) -> Result<Value,String> {
    let l=lab.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.edit("actual_tauri_UI_invoke",base_revision,x_m)).await.map_err(|e|e.to_string())
}
#[derive(Deserialize)]
struct CapturePacket { task_id:String, scope:String, requested_revision:u64, scene_revision:u64, png_data_url:String }
fn png_bytes(data:&str) -> Result<(String,Vec<u8>),String> {
    let encoded=data.strip_prefix("data:image/png;base64,").ok_or("InvalidPngDataUrl")?;
    let bytes=STANDARD.decode(encoded).map_err(|e|e.to_string())?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") { return Err("InvalidPngSignature".into()); }
    Ok((encoded.to_string(),bytes))
}
#[tauri::command]
fn receive_capture(lab:State<'_,Arc<Lab>>,packet:CapturePacket) -> Result<Value,String> {
    let mut d=lab.data.lock().unwrap();
    let s=d.frozen.as_ref().ok_or("NoFrozenTask")?;
    if packet.scope!=s.scope { return Err("ScopeMismatch".into()); }
    if packet.task_id!=s.task_id { return Err("TaskMismatch".into()); }
    if packet.requested_revision!=s.revision || packet.scene_revision!=s.revision { return Err("FrozenRevisionMismatch".into()); }
    let scene_revision=s.revision;
    let (b64,bytes)=png_bytes(&packet.png_data_url)?;
    std::fs::write(lab.output.join("frozen-r7.png"),&bytes).map_err(|e|e.to_string())?;
    d.capture=Some(b64); lab.capture_ready.notify_one();
    Ok(json!({"status":"Completed","scene_revision":scene_revision,"current_revision":d.revision,"asset_id":"fixture-r7-png","actual_png_bytes":bytes.len()}))
}
#[tauri::command]
fn save_current(lab:State<'_,Arc<Lab>>,png_data_url:String) -> Result<Value,String> {
    let (_,bytes)=png_bytes(&png_data_url)?;
    std::fs::write(lab.output.join("current-r8.png"),&bytes).map_err(|e|e.to_string())?;
    Ok(lab.context())
}
fn terminal(status:&str) -> bool { matches!(status,"Completed"|"Cancelled"|"Failed") }
#[tauri::command]
fn start_work(lab:State<'_,Arc<Lab>>,mode:String) -> Result<String,String> {
    if mode!="cancel" && mode!="failure" { return Err("UnsupportedFixtureMode".into()); }
    let l=lab.inner().clone(); let mut d=l.data.lock().unwrap(); let id=format!("work-{}",mode);
    let (tx,_)=watch::channel(WorkState{status:"Running".into(),completed_steps:0,input_revision:d.revision,error:None});
    let work=Arc::new(Work{cancelled:AtomicBool::new(false),tx}); d.work.insert(id.clone(),work.clone()); drop(d);
    tauri::async_runtime::spawn(async move {
        for step in 1..=5 {
            tokio::time::sleep(Duration::from_millis(40)).await;
            if work.cancelled.load(Ordering::SeqCst) {
                work.tx.send_modify(|s|s.status="Cancelled".into()); return;
            }
            if mode=="failure" && step==2 {
                work.tx.send_modify(|s|{s.status="Failed".into();s.error=Some("ObservedFixtureWorkerFailureAtStep2".into());}); return;
            }
            work.tx.send_modify(|s|{s.completed_steps=step;if step==5{s.status="Completed".into();}});
        }
    });
    Ok(id)
}
#[tauri::command]
async fn wait_work(lab:State<'_,Arc<Lab>>,task_id:String,first_progress:bool) -> Result<WorkState,String> {
    let work=lab.data.lock().unwrap().work.get(&task_id).cloned().ok_or("UnknownTask")?;
    let mut rx=work.tx.subscribe();
    loop {
        let value=rx.borrow().clone();
        if terminal(&value.status) || (first_progress && value.completed_steps>0) { return Ok(value); }
        rx.changed().await.map_err(|e|e.to_string())?;
    }
}
#[tauri::command]
fn cancel_work(lab:State<'_,Arc<Lab>>,task_id:String) -> Result<WorkState,String> {
    let work=lab.data.lock().unwrap().work.get(&task_id).cloned().ok_or("UnknownTask")?;
    work.tx.send_modify(|s|{if !terminal(&s.status){work.cancelled.store(true,Ordering::SeqCst);s.status="CancelRequested".into();}});
    Ok(work.tx.borrow().clone())
}
#[tauri::command]
async fn finish_probe(app:tauri::AppHandle,lab:State<'_,Arc<Lab>>,report:Value) -> Result<(),String> {
    let started=lab.data.lock().unwrap().agent_started;
    if started { lab.agent_done.notified().await; }
    let d=lab.data.lock().unwrap();
    let passed=report["status"]=="passed" && d.agent.as_ref().map(|v|v["status"]=="passed").unwrap_or(false)
        && d.revision==8 && d.x_m==8.0 && d.edits.len()==2 && d.edits.iter().filter(|v|v["ok"]==true).count()==1
        && d.pipe_eof && d.pipe_closed_error==6;
    let result=json!({"document_kind":"S05_actual_isolated_Tauri_WebView2_Three_shared_Rust_MCP_trial_not_product",
        "status":if passed{"passed"}else{"failed"},"tauri_version":"2.12.3","three_package_version":"0.186.1",
        "host_pid":std::process::id(),"UI_report":report,"native_agent":d.agent,"actual_edits":d.edits,"actual_pipe_peer":d.pipe_peer,
        "current":{"scope":"P1","revision":d.revision,"x_m":d.x_m},"frozen":d.frozen,
        "owned_pipe_eof":d.pipe_eof,"owned_pipe_after_close_error":d.pipe_closed_error,"owned_pipe_actual_10ms_release_waits":d.pipe_close_waits,
        "actual_product_runtime":null,"actual_18_product_operations":null,"actual_Engine":null});
    std::fs::write(lab.output.join("desktop-result.json"),serde_json::to_vec_pretty(&result).unwrap()).map_err(|e|e.to_string())?;
    app.exit(if passed {0}else{2}); Ok(())
}
async fn bridge_main(name:String,host_pid:u32) -> AnyResult<()> {
    let mut pipe=ClientOptions::new().open(name)?;
    let mut actual=0;
    if unsafe{GetNamedPipeServerProcessId(pipe.as_raw_handle(),&mut actual)}==0 || actual!=host_pid { return Err(io::Error::other("Actual server PID mismatch").into()); }
    write_frame(&mut pipe,&json!({"op":"hello","expected_host":format!("S05-host-{host_pid}")})).await?;
    let welcome=read_frame(&mut pipe).await?.ok_or_else(||io::Error::other("No actual welcome"))?;
    if welcome["ok"]!=true { return Err(io::Error::other("Actual instance rejected").into()); }
    let server=Bridge{pipe:Arc::new(AsyncMutex::new(pipe))}.serve(stdio()).await?; server.waiting().await?; Ok(())
}
fn main() {
    let args:Vec<_>=std::env::args().collect();
    if args.get(1).map(String::as_str)==Some("bridge") {
        let runtime=tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        if let Err(e)=runtime.block_on(bridge_main(args[2].clone(),args[3].parse().unwrap())) { eprintln!("{e}"); std::process::exit(3); }
        return;
    }
    let output=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output"); std::fs::create_dir_all(&output).unwrap();
    let lab=Arc::new(Lab{data:Mutex::new(Data{revision:7,x_m:2.0,frozen:None,capture:None,edits:Vec::new(),agent:None,bridge_pid:0,pipe_peer:Value::Null,pipe_eof:false,pipe_closed_error:0,pipe_close_waits:0,agent_started:false,work:HashMap::new()}),race:Barrier::new(2),capture_ready:Notify::new(),agent_done:Notify::new(),output:output.clone()});
    tauri::Builder::default().manage(lab)
        .invoke_handler(tauri::generate_handler![get_context,freeze_render,start_agent,ui_edit,receive_capture,save_current,start_work,wait_work,cancel_work,finish_probe])
        .setup(move|app| { tauri::WebviewWindowBuilder::new(app,"main",tauri::WebviewUrl::App("index.html".into()))
            .title("DRF04 S05 isolated proof").inner_size(700.0,450.0).visible(false).data_directory(output.join("webview2-profile")).build()?; Ok(()) })
        .run(tauri::generate_context!()).expect("Actual isolated Tauri runtime failed");
}
