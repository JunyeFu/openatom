mod query;
mod store;
use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use deadpool_postgres::{Manager, Pool};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sf_domain::cloud_execution::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio_postgres::{Row, Transaction};
pub const CONTRACT: &str = "sf.execution/2.0-draft11";
type Result<T> = std::result::Result<T, Fault>;

#[derive(Debug)]
struct Fault {
    status: StatusCode,
    code: &'static str,
    message: String,
    expected: Value,
    actual: Value,
}
impl Fault {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            expected: Value::Null,
            actual: Value::Null,
        }
    }
    fn invalid(s: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_input", s)
    }
    fn conflict(code: &'static str, s: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, s)
    }
    fn denied() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "permission_denied",
            "principal scope does not permit this operation",
        )
    }
    fn missing() -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "missing_reference",
            "referenced object does not exist",
        )
    }
}
impl From<tokio_postgres::Error> for Fault {
    fn from(e: tokio_postgres::Error) -> Self {
        if let Some(d) = e.as_db_error() {
            if d.code() == &tokio_postgres::error::SqlState::UNIQUE_VIOLATION {
                return Self::conflict(
                    "immutable_payload_conflict",
                    "immutable identity or active ownership conflict",
                );
            }
        }
        // Connection strings and server diagnostic detail are never sent to clients.
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "io_failure",
            "database operation failed",
        )
    }
}
impl IntoResponse for Fault {
    fn into_response(self) -> Response {
        (self.status,Json(json!({"contract_version":CONTRACT,"error":{"code":self.code,"message":self.message,"field_errors":[],"expected_revision":self.expected,"actual_revision":self.actual}}))).into_response()
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Principal {
    token: String,
    principal_id: String,
    role: Role,
    project_ids: Vec<String>,
    device_ids: Vec<String>,
    dataset_ids: Vec<String>,
}
#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Role {
    ScopedReader,
    Operator,
    HostGateway,
    Device,
    GeneratedLoadProducer,
}
impl Principal {
    fn role(&self, allowed: &[Role]) -> Result<()> {
        if allowed.contains(&self.role) {
            Ok(())
        } else {
            Err(Fault::denied())
        }
    }
    fn project(&self, p: &str) -> Result<()> {
        if self.project_ids.iter().any(|v| v == p) {
            Ok(())
        } else {
            Err(Fault::denied())
        }
    }
    fn device(&self, d: &str) -> Result<()> {
        if self.device_ids.iter().any(|v| v == d) {
            Ok(())
        } else {
            Err(Fault::denied())
        }
    }
    fn dataset(&self, d: &str) -> Result<()> {
        if self.dataset_ids.iter().any(|v| v == d) {
            Ok(())
        } else {
            Err(Fault::denied())
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bindings {
    projects: Vec<Project>,
    device_bindings: Vec<Binding>,
    datasets: Vec<Dataset>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Project {
    cloud_project_id: String,
    local_project_id: String,
    owner_scope_ref: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    binding_id: String,
    cloud_project_id: String,
    device_id: String,
    principal_scope_ref: String,
    definition_ref: Value,
    capability_map: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CapabilityMap {
    action: String,
    operation_ref: sf_domain::ModuleRef,
    execution_capacity_ref: sf_domain::ObjectRef,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StageCapabilities {
    stage_bindings: Vec<sf_domain::cloud_execution::CandidateBinding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dataset {
    dataset_id: String,
    cloud_project_id: String,
    source_mode: String,
    purpose: String,
}
struct App {
    pool: Pool,
    principals: Vec<Principal>,
    bindings: Bindings,
    draining: AtomicBool,
}

pub async fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let command = std::env::args().nth(1).unwrap_or_default();
    if command != "migrate" && command != "serve" {
        return Err("usage: sf-cloud-service migrate|serve".into());
    }
    let bindings: Bindings = serde_json::from_str(&std::fs::read_to_string(std::env::var(
        "SF_CLOUD_BINDINGS_FILE",
    )?)?)?;
    let principals: Vec<Principal> =
        serde_json::from_str(&std::env::var("SF_CLOUD_AUTH_TOKENS_JSON")?)?;
    validate_config(&bindings, &principals)?;
    let mut pg: tokio_postgres::Config = std::env::var("DATABASE_URL")?.parse()?;
    let local=pg.get_hosts().iter().all(|h|matches!(h,tokio_postgres::config::Host::Tcp(s) if s=="127.0.0.1" || s=="::1" || s=="localhost")) && !pg.get_hosts().is_empty();
    let manager = if local && std::env::var("SF_CLOUD_LOCAL_NO_TLS").as_deref() == Ok("1") {
        pg.ssl_mode(tokio_postgres::config::SslMode::Disable);
        Manager::new(pg, tokio_postgres::NoTls)
    } else {
        pg.ssl_mode(tokio_postgres::config::SslMode::Require);
        let tls = native_tls::TlsConnector::builder()
            .min_protocol_version(Some(native_tls::Protocol::Tlsv12))
            .build()?;
        Manager::new(pg, postgres_native_tls::MakeTlsConnector::new(tls))
    };
    let pool = Pool::builder(manager).max_size(16).build()?;
    if command == "migrate" {
        migrate(&pool, &bindings)
            .await
            .map_err(|_| "migration failed")?;
        println!("schema version 2 ready");
        return Ok(());
    }
    let app = Arc::new(App {
        pool,
        principals,
        bindings,
        draining: AtomicBool::new(false),
    });
    let port = std::env::var("PORT").unwrap_or_else(|_| "23270".into());
    let bind = std::env::var("SF_CLOUD_LISTEN_ADDRESS").unwrap_or_else(|_| "0.0.0.0".into());
    let listener = tokio::net::TcpListener::bind(format!("{bind}:{port}")).await?;
    let router = Router::new().fallback(any(handle)).with_state(app.clone());
    use std::future::IntoFuture;
    let stop = Arc::new(tokio::sync::Notify::new());
    let signal_stop = stop.clone();
    let signal_app = app.clone();
    let server = axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = shutdown_signal().await;
            signal_app.draining.store(true, Ordering::SeqCst);
            signal_stop.notify_one();
        })
        .into_future();
    tokio::pin!(server);
    tokio::select! {
        result=&mut server=>result?,
        _=stop.notified()=>{match tokio::time::timeout(std::time::Duration::from_secs(20),&mut server).await {Ok(result)=>result?,Err(_)=>app.pool.close()}}
    }
    Ok(())
}
async fn shutdown_signal() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {_=terminate.recv()=>Ok(()),result=tokio::signal::ctrl_c()=>result}
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await
    }
}
fn validate_config(b: &Bindings, p: &[Principal]) -> std::result::Result<(), &'static str> {
    if b.projects.is_empty() || b.datasets.len() < 2 || p.is_empty() {
        return Err("missing fixed configuration");
    }
    let projects: BTreeSet<_> = b
        .projects
        .iter()
        .map(|x| x.cloud_project_id.as_str())
        .collect();
    if projects.len() != b.projects.len()
        || b.projects
            .iter()
            .any(|x| x.local_project_id.is_empty() || x.owner_scope_ref.is_empty())
    {
        return Err("invalid project configuration");
    }
    let devices: BTreeSet<_> = b
        .device_bindings
        .iter()
        .map(|x| x.device_id.as_str())
        .collect();
    let datasets: BTreeSet<_> = b.datasets.iter().map(|x| x.dataset_id.as_str()).collect();
    if datasets.len() != b.datasets.len()
        || b.datasets.iter().any(|x| {
            !projects.contains(x.cloud_project_id.as_str())
                || !matches!(
                    (x.purpose.as_str(), x.source_mode.as_str()),
                    ("telemetry_only", "software_in_loop" | "physical_device")
                        | ("generated_load", "generated_load")
                )
        })
        || !b.datasets.iter().any(|x| x.purpose == "telemetry_only")
        || !b.datasets.iter().any(|x| x.purpose == "generated_load")
    {
        return Err("invalid fixed dataset configuration");
    }
    if b.device_bindings.iter().any(|x| {
        x.binding_id.is_empty()
            || x.device_id.is_empty()
            || !projects.contains(x.cloud_project_id.as_str())
            || x.principal_scope_ref.is_empty()
            || serde_json::from_value::<sf_domain::ModuleRef>(x.definition_ref.clone()).is_err()
            || if x.capability_map.get("stage_bindings").is_some() {
                serde_json::from_value::<StageCapabilities>(x.capability_map.clone()).map(|stages| {
                    let mut ids = BTreeSet::new();
                    stages.stage_bindings.is_empty() || stages.stage_bindings.iter().any(|b|
                        b.device_id.as_str() != x.device_id || !ids.insert(b.binding_id.as_str()))
                }).unwrap_or(true)
            } else { serde_json::from_value::<CapabilityMap>(x.capability_map.clone())
                .map(|cap| {
                    let _ = (&cap.operation_ref, &cap.execution_capacity_ref);
                    !matches!(cap.action.as_str(), "process_unit" | "prepare_equipment" | "transport_cargo")
                })
                .unwrap_or(true) }
    }) {
        return Err("invalid binding configuration");
    }
    let mut tokens = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for x in p {
        if x.token.is_empty()
            || x.principal_id.is_empty()
            || !tokens.insert(&x.token)
            || !ids.insert(&x.principal_id)
            || x.project_ids.is_empty()
            || x.project_ids.iter().any(|v| !projects.contains(v.as_str()))
        {
            return Err("invalid principal catalog");
        }
        if x.role == Role::Device
            && (x.device_ids.len() != 1
                || !devices.contains(x.device_ids[0].as_str())
                || !b.device_bindings.iter().any(|v| {
                    v.device_id == x.device_ids[0]
                        && v.principal_scope_ref == x.principal_id
                        && x.project_ids.contains(&v.cloud_project_id)
                }))
        {
            return Err("invalid device principal");
        }
        if x.role == Role::GeneratedLoadProducer
            && (x.dataset_ids.len() != 1
                || !b.datasets.iter().any(|v| {
                    v.dataset_id == x.dataset_ids[0]
                        && v.purpose == "generated_load"
                        && x.project_ids.contains(&v.cloud_project_id)
                }))
        {
            return Err("invalid load principal");
        }
    }
    Ok(())
}
async fn migrate(pool: &Pool, b: &Bindings) -> Result<()> {
    let mut c = pool.get().await.map_err(|_| {
        Fault::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "io_failure",
            "database unavailable",
        )
    })?;
    let t = c.transaction().await?;
    t.batch_execute("SELECT pg_advisory_xact_lock(53023001); CREATE TABLE IF NOT EXISTS schema_migrations(version integer PRIMARY KEY, applied_at timestamptz(6) NOT NULL DEFAULT clock_timestamp());").await?;
    let versions = t
        .query(
            "SELECT version FROM schema_migrations ORDER BY version",
            &[],
        )
        .await?;
    if versions.is_empty() {
        t.batch_execute(include_str!("../migrations/001.sql"))
            .await?;
        for x in &b.projects {
            t.execute("INSERT INTO cloud_projects(project_id,local_project_id,owner_scope_ref) VALUES($1,$2,$3)",&[&x.cloud_project_id,&x.local_project_id,&x.owner_scope_ref]).await?;
        }
        for x in &b.device_bindings {
            t.execute(
                "INSERT INTO device_bindings VALUES($1,$2,$3,$4,$5,$6)",
                &[
                    &x.binding_id,
                    &x.cloud_project_id,
                    &x.device_id,
                    &x.principal_scope_ref,
                    &x.definition_ref,
                    &x.capability_map,
                ],
            )
            .await?;
        }
        for x in &b.datasets {
            t.execute("INSERT INTO datasets(dataset_id,project_id,source_mode,purpose) VALUES($1,$2,$3,$4)",&[&x.dataset_id,&x.cloud_project_id,&x.source_mode,&x.purpose]).await?;
        }
        t.execute("INSERT INTO schema_migrations(version) VALUES(1)", &[])
            .await?;
    } else if !matches!(versions.iter().map(|row| row.get::<_, i32>(0)).collect::<Vec<_>>().as_slice(), [1] | [1, 2]) {
        return Err(Fault::invalid("unsupported database schema version"));
    }
    if !versions.iter().any(|row| row.get::<_, i32>(0) == 2) {
        t.batch_execute(include_str!("../migrations/002_transport_observations.sql")).await?;
        t.execute("INSERT INTO schema_migrations(version) VALUES(2)", &[]).await?;
    }
    // A running deployment cannot silently consume a different fixed catalog.
    catalog_matches(&t, b).await?;
    t.commit().await?;
    Ok(())
}
async fn catalog_matches(t: &Transaction<'_>, b: &Bindings) -> Result<()> {
    for x in &b.projects {
        let r = t
            .query_opt(
                "SELECT local_project_id,owner_scope_ref FROM cloud_projects WHERE project_id=$1",
                &[&x.cloud_project_id],
            )
            .await?
            .ok_or_else(Fault::missing)?;
        if r.get::<_, String>(0) != x.local_project_id || r.get::<_, String>(1) != x.owner_scope_ref
        {
            return Err(Fault::invalid(
                "fixed project catalog differs from database",
            ));
        }
    }
    for x in &b.device_bindings {
        let r=t.query_opt("SELECT project_id,device_id,principal_scope_ref,definition_ref,capability_map FROM device_bindings WHERE binding_id=$1",&[&x.binding_id]).await?.ok_or_else(Fault::missing)?;
        if r.get::<_, String>(0) != x.cloud_project_id
            || r.get::<_, String>(1) != x.device_id
            || r.get::<_, String>(2) != x.principal_scope_ref
            || r.get::<_, Value>(3) != x.definition_ref
            || r.get::<_, Value>(4) != x.capability_map
        {
            return Err(Fault::invalid("fixed device catalog differs from database"));
        }
    }
    for x in &b.datasets {
        let r=t.query_opt("SELECT project_id,source_mode,purpose,session_id FROM datasets WHERE dataset_id=$1",&[&x.dataset_id]).await?.ok_or_else(Fault::missing)?;
        if r.get::<_, String>(0) != x.cloud_project_id
            || r.get::<_, String>(1) != x.source_mode
            || r.get::<_, String>(2) != x.purpose
            || r.get::<_, Option<String>>(3).is_some()
        {
            return Err(Fault::invalid(
                "fixed dataset catalog differs from database",
            ));
        }
    }
    Ok(())
}
fn typed<T: DeserializeOwned + Serialize>(v: Value) -> Result<Value> {
    if v.get("contract_version").is_some_and(|x| x != CONTRACT) {
        return Err(Fault::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unsupported_contract",
            "unsupported execution contract",
        ));
    }
    serde_json::from_value::<T>(v)
        .and_then(serde_json::to_value)
        .map_err(|e| Fault::invalid(format!("typed payload: {e}")))
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .ok_or_else(|| Fault::invalid(format!("missing {k}")))
}
fn num(v: &Value, k: &str) -> Result<i64> {
    v[k].as_i64()
        .filter(|x| *x >= 0 && *x <= 9007199254740991)
        .ok_or_else(|| Fault::invalid(format!("invalid {k}")))
}
fn array<'a>(v: &'a Value, k: &str) -> Result<&'a Vec<Value>> {
    v[k].as_array()
        .ok_or_else(|| Fault::invalid(format!("missing {k}")))
}
fn same(a: &Value, b: &Value) -> Result<()> {
    if a == b {
        Ok(())
    } else {
        Err(Fault::conflict(
            "immutable_payload_conflict",
            "identity already exists with different payload",
        ))
    }
}
fn success(kind: &str, data: Value) -> Result<Value> {
    let v = json!({"contract_version":CONTRACT,"result":{"kind":kind,"data":data}});
    // The known inputs variant contains large byte arrays. Decode its data
    // directly so the adjacent response tag does not buffer another JSON tree.
    let checked = if kind == "inputs" {
        <ExecutionFrozenInputs as serde::Deserialize>::deserialize(&v["result"]["data"]).map(|_| ())
    } else {
        <CloudResponse as serde::Deserialize>::deserialize(&v).map(|_| ())
    };
    checked.map_err(|e| {
        eprintln!("response DTO validation failed: {e}");
        Fault::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "io_failure",
            "response contract validation failed",
        )
    })?;
    Ok(v)
}
fn encode(v: &Value) -> String {
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(v).unwrap())
}
fn decode(s: &str) -> Result<Value> {
    serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(s)
            .map_err(|_| Fault::invalid("invalid cursor"))?,
    )
    .map_err(|_| Fault::invalid("invalid cursor"))
}
fn normalized_query(v: &Value) -> Value {
    let mut v = v.clone();
    v["page"]["cursor"] = Value::Null;
    v
}
fn page(v: &Value) -> Result<(i64, Option<Value>)> {
    let limit = num(&v["page"], "limit")?;
    if !(1..=200).contains(&limit) {
        return Err(Fault::invalid("page limit must be 1..200"));
    }
    Ok((limit, v["page"]["cursor"].as_str().map(decode).transpose()?))
}
fn stamp(row: &Row, name: &str) -> Value {
    json!(
        row.get::<_, chrono::DateTime<chrono::Utc>>(name)
            .to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
    )
}

async fn handle(State(app): State<Arc<App>>, req: Request) -> Response {
    if app.draining.load(Ordering::SeqCst) {
        return if req.uri().path() == "/healthz" {
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        } else {
            Fault::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "io_failure",
                "service is draining",
            )
            .into_response()
        };
    }
    if req.uri().path() == "/healthz" {
        if req.method() != axum::http::Method::GET {
            return StatusCode::METHOD_NOT_ALLOWED.into_response();
        }
        let healthy = async {
            let mut c = app
                .pool
                .get()
                .await
                .map_err(|_| Fault::invalid("database unavailable"))?;
            let t = c.transaction().await?;
            let r = t
                .query_one(
                    "SELECT (SELECT array_agg(version ORDER BY version)=ARRAY[1,2] FROM schema_migrations),1",
                    &[],
                )
                .await?;
            if !r.get::<_, bool>(0) {
                return Err(Fault::invalid("schema not ready"));
            }
            catalog_matches(&t, &app.bindings).await?;
            t.commit().await?;
            Ok::<_, Fault>(())
        }
        .await
        .is_ok();
        return if healthy {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        }
        .into_response();
    }
    match dispatch(&app, req).await {
        Ok((v, h)) => (h, Json(v)).into_response(),
        Err(e) => {
            eprintln!("cloud_request_failed status={} code={} message={}", e.status.as_u16(), e.code, e.message);
            e.into_response()
        }
    }
}
async fn dispatch(app: &App, req: Request) -> Result<(Value, HeaderMap)> {
    let token = req
        .headers()
        .get("authorization")
        .and_then(|x| x.to_str().ok())
        .and_then(|x| x.strip_prefix("Bearer "))
        .ok_or_else(|| {
            Fault::new(
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "Bearer authentication required",
            )
        })?;
    let p = app
        .principals
        .iter()
        .find(|p| p.token == token)
        .ok_or_else(|| {
            Fault::new(
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "unknown Bearer principal",
            )
        })?;
    let method = req.method().as_str().to_owned();
    let path = req.uri().path().to_owned();
    let query = req.uri().query().unwrap_or("").to_owned();
    // Full-factory frozen modules contain the exact modeled geometry, over 48 MiB.
    // Keep the ordinary event/query bound; only authenticated session/snapshot uploads need more.
    let large_execution_document = method == "PUT" && (path.contains("/sessions/") || path.contains("/snapshots/"));
    let encoding = req.headers().get("Content-Encoding").map(|v| v.to_str().unwrap_or("")).unwrap_or("").to_owned();
    if !encoding.is_empty() && !(large_execution_document && encoding == "gzip") {
        return Err(Fault::invalid("unsupported request encoding"));
    }
    let bytes = to_bytes(req.into_body(), if large_execution_document { 128 * 1024 * 1024 } else { 16 * 1024 * 1024 })
        .await
        .map_err(|_| Fault::invalid("request body exceeds limit"))?;
    let decoded_body;
    let bytes = if encoding == "gzip" {
        use std::io::Read;
        let mut decoded = Vec::new();
        flate2::read::GzDecoder::new(bytes.as_ref()).take(128 * 1024 * 1024 + 1).read_to_end(&mut decoded)
            .map_err(|_| Fault::invalid("invalid compressed execution document"))?;
        if decoded.len() > 128 * 1024 * 1024 { return Err(Fault::invalid("execution document exceeds decoded limit")); }
        decoded_body = decoded;
        decoded_body.as_slice()
    } else { bytes.as_ref() };
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice::<Value>(&bytes).map_err(|_| Fault::invalid("invalid JSON body"))?
    };
    let decoded = path.trim_matches('/').split('/').map(|s| store::unescape(s, false)).collect::<Result<Vec<_>>>()?;
    let segments: Vec<_> = decoded.iter().map(String::as_str).collect();
    let mut c = app.pool.get().await.map_err(|_| {
        Fault::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "io_failure",
            "database unavailable",
        )
    })?;
    let t = c
        .build_transaction()
        .isolation_level(tokio_postgres::IsolationLevel::ReadCommitted)
        .start()
        .await?;
    let mut headers = HeaderMap::new();
    let value = store::route(&t, p, &method, &segments, &query, body, &mut headers).await?;
    t.commit().await?;
    Ok((value, headers))
}

#[cfg(test)]
mod path_tests {
    use super::store::unescape;
    #[test]
    fn opaque_path_ids_decode_once_and_keep_literal_plus_and_encoded_slash() {
        assert_eq!(unescape("one%2Ftwo+%25%E8%AE%BE%E5%A4%87", false).unwrap(), "one/two+%设备");
        assert_eq!(unescape("a+b%2Bc", true).unwrap(), "a b+c");
        assert_eq!(unescape("%252F", false).unwrap(), "%2F");
        for invalid in ["%", "%1", "%GG", "%FF"] {
            assert!(unescape(invalid, false).is_err(), "{invalid}");
        }
    }
}
