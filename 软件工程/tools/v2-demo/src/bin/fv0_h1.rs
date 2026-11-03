//! Offline FV0 H1 measurement. Uses the production archive reader and ProjectHost.
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    fs,
    io::{BufReader, BufWriter, Write},
    path::Path,
    time::Instant,
};

fn typed<T: DeserializeOwned>(v: Value) -> T {
    serde_json::from_value(v).expect("typed input")
}
fn id(s: &str) -> ObjectId {
    s.to_owned().try_into().unwrap()
}
fn read(p: &Path) -> Value {
    serde_json::from_reader(BufReader::new(fs::File::open(p).unwrap())).unwrap()
}
fn emit(phase: &str, start: Instant, detail: Value) {
    println!(
        "{}",
        json!({"phase":phase,"seconds":start.elapsed().as_secs_f64(),"pid":std::process::id(),"detail":detail})
    );
    std::io::stdout().flush().unwrap();
}
struct Host {
    host: ProjectHost,
    ui: ObjectId,
    project: Value,
    seq: u64,
}
impl Host {
    fn new() -> Self {
        let ui = id("fv0-h1-ui");
        Self {
            host: ProjectHost::new(
                id(&format!("fv0-h1-host-{}", std::process::id())),
                ui.clone(),
            ),
            ui,
            project: Value::Null,
            seq: 0,
        }
    }
    fn call(&mut self, project: bool, operation: &str, params: Value) -> Value {
        self.seq += 1;
        let request: ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":format!("fv0-h1-{}",self.seq),"project_id":if project{self.project.clone()}else{Value::Null},"scope":{"kind":if project{"project"}else{"workspace"}},"operation":operation,"params":params})).unwrap_or_else(|e| panic!("{operation} request: {e}"));
        let reply: Response<Value> = typed(self.host.dispatch(&self.ui, request));
        match reply.outcome {
            ApiOutcome::Ok(v) => v,
            other => panic!("{operation}: {other:?}"),
        }
    }
    fn finish(&mut self, project: bool, handle: &Value) -> Value {
        let state = self.io_result(project, handle);
        assert_eq!(state["task"]["state"], "completed", "task failed: {state}");
        state
    }
    fn io_result(&mut self, project: bool, handle: &Value) -> Value {
        let mut work = self.host.drain_io_work();
        assert_eq!(work.len(), 1);
        self.host.finish_io(work.pop().unwrap().run());
        self.call(project, "get_task", json!({"task_id":handle["id"]}))
    }
    fn open(&mut self, path: &Path) {
        let start = Instant::now();
        let selected = self
            .host
            .select_directory(&self.ui, FilePurpose::ImportPackage, path.to_owned())
            .unwrap();
        let handle=self.call(false,"import_package",json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}));
        let state = self.finish(false, &handle);
        let draft = &state["result"]["data"];
        let commit=self.call(false,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":handle["id"],"choices":{"chosen_policy":"open_project","objects_to_create":draft["proposed_objects"],"module_versions":draft["proposed_definitions"],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}));
        self.project = commit["project_id"].clone();
        emit("host_open", start, json!({"project_id":self.project}));
    }
    fn verify(&mut self, expected: &Value) {
        let start = Instant::now();
        let envelope=self.call(true,"get_execution_snapshot",json!({"execution_contract_version":"sf.execution/2.0-draft11","identity":expected["identity"],"selection":{"kind":"latest"}}));
        assert!(&envelope["snapshot"] == expected, "latest snapshot changed");
        assert_eq!(envelope["access"]["ownership"], "other_host");
        assert_eq!(envelope["access"]["allowed_session_controls"], json!([]));
        let mut cursor = Value::Null;
        let mut count = 0;
        loop {
            let page=self.call(true,"read_execution_view",json!({"execution_contract_version":"sf.execution/2.0-draft11","selection":{"kind":"history","query":{"contract_version":"sf.execution/2.0-draft11","identity":expected["identity"],"execution_revision":expected["execution_revision"],"objects":[],"host_from_ms":0,"host_until_ms":expected["coverage"]["observed_until_host_ms"],"page":{"limit":200,"cursor":cursor}}}}));
            count += page["data"]["records"].as_array().unwrap().len();
            cursor = page["data"]["next_cursor"].clone();
            if cursor.is_null() {
                break;
            }
        }
        assert_eq!(
            count as u64,
            expected["observation_frontier"].as_u64().unwrap()
        );
        emit(
            "verify_snapshot_and_all_history",
            start,
            json!({"revision":expected["execution_revision"],"history_records":count,"devices":expected["devices"].as_array().unwrap().len(),"snapshot_exact":true,"controls":[]}),
        );
    }
    fn save(&mut self, path: &Path) {
        let start = Instant::now();
        fs::create_dir(path).unwrap();
        let handle = self.queue_save(path);
        let state = self.finish(true, &handle);
        assert_eq!(state["result"]["kind"], "save");
        emit("host_save", start, json!({"saved":true}));
    }
    fn queue_save(&mut self, path: &Path) -> Value {
        let selected = self
            .host
            .select_directory(&self.ui, FilePurpose::SaveProject, path.to_owned())
            .unwrap();
        let context = self.call(true, "get_context", json!({"selection":[]}));
        self.call(
            true,
            "save_project",
            json!({"revision":context["project"]["revision"],"selected_directory_ref":selected}),
        )
    }
}
fn codec(input: &Path, output: &Path) {
    let start = Instant::now();
    let record: Versioned<StoredExecutionArchive> = serde_json::from_reader(BufReader::new(
        flate2::read::GzDecoder::new(fs::File::open(input).unwrap()),
    ))
    .expect("production archive types");
    emit(
        "decode",
        start,
        json!({"snapshots":record.data.snapshots.len(),"records":record.data.history.as_ref().unwrap().records.len()}),
    );
    let start = Instant::now();
    let raw = output.join("codec.json");
    let mut writer = BufWriter::new(fs::File::create_new(&raw).unwrap());
    serde_json::to_writer(&mut writer, &record).unwrap();
    writer.flush().unwrap();
    drop(writer);
    emit(
        "serialize",
        start,
        json!({"bytes":fs::metadata(&raw).unwrap().len()}),
    );
    let start = Instant::now();
    let dest = output.join("codec.json.gz");
    let mut encoder = flate2::write::GzEncoder::new(
        BufWriter::new(fs::File::create_new(&dest).unwrap()),
        flate2::Compression::fast(),
    );
    std::io::copy(
        &mut BufReader::new(fs::File::open(raw).unwrap()),
        &mut encoder,
    )
    .unwrap();
    encoder.finish().unwrap().flush().unwrap();
    emit(
        "compress",
        start,
        json!({"bytes":fs::metadata(dest).unwrap().len()}),
    );
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        eprintln!(
            "fv0_h1 codec INPUT_JSON_GZ OUTPUT_DIR | save PROJECT EXPECTED_SNAPSHOT NEW_DIR | reopen PROJECT EXPECTED_SNAPSHOT"
        );
        std::process::exit(2);
    }
    match a[1].as_str() {
        "codec" => codec(Path::new(&a[2]), Path::new(&a[3])),
        "save" | "reopen" => {
            let expected = read(Path::new(&a[3]));
            let mut host = Host::new();
            host.open(Path::new(&a[2]));
            host.verify(&expected);
            if a[1] == "save" {
                assert_eq!(a.len(), 5);
                host.save(Path::new(&a[4]));
            }
        }
        #[cfg(windows)]
        "check-failed-replace" => {
            assert_eq!(a.len(), 4, "expected a disposable copy and expected snapshot");
            use std::os::windows::fs::OpenOptionsExt;
            let copy = Path::new(&a[2]);
            let expected = read(Path::new(&a[3]));
            let root = copy.join("project.json");
            let before = fs::read(&root).unwrap();
            let mut host = Host::new();
            host.open(copy);
            host.verify(&expected);
            let held = fs::OpenOptions::new().read(true).share_mode(1).open(&root).unwrap();
            let handle = host.queue_save(copy);
            let state = host.io_result(true, &handle);
            assert_eq!(state["task"]["state"], "failed", "{state}");
            assert!(state.to_string().contains("old project ID/generation/revision verified"), "{state}");
            drop(held);
            assert_eq!(before, fs::read(root).unwrap());
            println!("Verified old root after controlled replacement failure");
        }
        _ => panic!("unknown mode"),
    }
}
