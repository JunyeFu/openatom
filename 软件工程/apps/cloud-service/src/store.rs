use super::*;

fn pack_frozen_inputs(value: &Value) -> Result<Value> {
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    serde_json::to_writer(&mut gzip, value).map_err(|_| Fault::invalid("cannot encode frozen inputs"))?;
    let bytes = gzip.finish().map_err(|_| Fault::invalid("cannot compress frozen inputs"))?;
    Ok(json!({"storage_encoding":"gzip-base64-v1","payload":URL_SAFE_NO_PAD.encode(bytes)}))
}
fn unpack_frozen_inputs(value: Value) -> Result<Value> {
    if value.get("storage_encoding").is_none() { return Ok(value); }
    if value["storage_encoding"] != "gzip-base64-v1" { return Err(Fault::invalid("unknown frozen input storage encoding")); }
    let bytes = URL_SAFE_NO_PAD.decode(text(&value,"payload")?).map_err(|_| Fault::invalid("invalid frozen input storage"))?;
    use std::io::Read;
    let mut decoded = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice()).take(128 * 1024 * 1024 + 1).read_to_end(&mut decoded)
        .map_err(|_| Fault::invalid("cannot decode frozen inputs"))?;
    if decoded.len() > 128 * 1024 * 1024 { return Err(Fault::invalid("frozen inputs exceed decoded limit")); }
    serde_json::from_slice(&decoded).map_err(|_| Fault::invalid("invalid stored frozen document"))
}

pub(super) struct Session {
    pub cloud: String,
    pub identity: Value,
    pub payload: Value,
}
pub(super) async fn session(
    t: &Transaction<'_>,
    p: &Principal,
    sid: &str,
    lock: bool,
) -> Result<Session> {
    session_payload(t, p, sid, lock, None).await
}
async fn session_payload(
    t: &Transaction<'_>, p: &Principal, sid: &str, lock: bool, device: Option<&str>,
) -> Result<Session> {
    let r = t
        .query_opt(
            "SELECT s.project_id FROM execution_sessions s WHERE session_id=$1",
            &[&sid],
        )
        .await?
        .ok_or_else(Fault::missing)?;
    let cloud: String = r.get(0);
    p.project(&cloud)?;
    if lock {
        t.query_one(
            if device.is_some() {
                "SELECT project_id FROM cloud_projects WHERE project_id=$1 FOR SHARE"
            } else {
                "SELECT project_id FROM cloud_projects WHERE project_id=$1 FOR UPDATE"
            },
            &[&cloud],
        )
        .await?;
    }
    let sql = if lock {
        "SELECT immutable_payload FROM execution_sessions WHERE session_id=$1 FOR UPDATE"
    } else {
        "SELECT immutable_payload FROM execution_sessions WHERE session_id=$1"
    };
    let payload: Value = if let Some(device) = device {
        // Device polling and ingress need identity and membership, not every stage's ports.
        // Shared project/session locks exclude lifecycle writers while allowing
        // different devices to poll concurrently; binding() still locks each device.
        let sql = "SELECT jsonb_build_object('identity',immutable_payload->'identity','candidate_bindings',COALESCE((SELECT jsonb_agg(jsonb_build_object('device_id',b->'device_id')) FROM jsonb_array_elements(immutable_payload->'candidate_bindings') b WHERE b->>'device_id'=$2),'[]'::jsonb)) FROM execution_sessions WHERE session_id=$1 FOR SHARE";
        t.query_one(sql, &[&sid, &device]).await?.get(0)
    } else {
        t.query_one(sql, &[&sid]).await?.get(0)
    };
    Ok(Session {
        cloud,
        identity: payload["identity"].clone(),
        payload,
    })
}
pub(super) fn identity(s: &Session, v: &Value) -> Result<()> {
    if s.identity == *v {
        Ok(())
    } else {
        Err(Fault::conflict(
            "epoch_mismatch",
            "session identity or epoch differs",
        ))
    }
}
async fn active_slot(t: &Transaction<'_>, s: &Session) -> Result<()> {
    let r = t
        .query_opt(
            "SELECT session_id,execution_epoch_id FROM project_execution_slots WHERE project_id=$1",
            &[&s.cloud],
        )
        .await?
        .ok_or_else(|| {
            Fault::conflict(
                "active_session_conflict",
                "session no longer owns project slot",
            )
        })?;
    if r.get::<_, String>(0) != text(&s.identity, "session_id")?
        || r.get::<_, String>(1) != text(&s.identity, "execution_epoch_id")?
    {
        return Err(Fault::conflict(
            "active_session_conflict",
            "different active project session",
        ));
    }
    Ok(())
}
pub(super) async fn binding(
    t: &Transaction<'_>,
    s: &Session,
    device: &str,
    lock: bool,
) -> Result<String> {
    let sql = if lock {
        "SELECT binding_id FROM device_bindings WHERE project_id=$1 AND device_id=$2 FOR UPDATE"
    } else {
        "SELECT binding_id FROM device_bindings WHERE project_id=$1 AND device_id=$2"
    };
    let r = t
        .query_opt(sql, &[&s.cloud, &device])
        .await?
        .ok_or_else(Fault::missing)?;
    let id: String = r.get(0);
    if !array(&s.payload, "candidate_bindings")?
        .iter()
        .any(|v| v["device_id"] == device)
    {
        return Err(Fault::denied());
    }
    Ok(id)
}
async fn boot_active(t: &Transaction<'_>, s: &Session, binding: &str, boot: &str) -> Result<()> {
    let r=t.query_opt("SELECT session_id,execution_epoch_id,state FROM device_boots WHERE device_binding_id=$1 AND device_boot_id=$2",&[&binding,&boot]).await?.ok_or_else(||Fault::conflict("device_boot_mismatch","boot not registered"))?;
    if r.get::<_, String>(0) != text(&s.identity, "session_id")?
        || r.get::<_, String>(1) != text(&s.identity, "execution_epoch_id")?
        || r.get::<_, String>(2) != "active"
    {
        return Err(Fault::conflict(
            "device_boot_mismatch",
            "boot is not active for this session epoch",
        ));
    }
    Ok(())
}
pub(super) async fn dataset(
    t: &Transaction<'_>,
    p: &Principal,
    id: &str,
    lock: bool,
) -> Result<Row> {
    let sql = if lock {
        "SELECT * FROM datasets WHERE dataset_id=$1 FOR UPDATE"
    } else {
        "SELECT * FROM datasets WHERE dataset_id=$1"
    };
    let r = t.query_opt(sql, &[&id]).await?.ok_or_else(Fault::missing)?;
    p.project(&r.get::<_, String>("project_id"))?;
    if r.get::<_, String>("purpose") != "execution" {
        p.dataset(id)?;
    }
    Ok(r)
}
pub(super) async fn snapshot(t: &Transaction<'_>, sid: &str, selection: &Value) -> Result<Value> {
    let kind = text(selection, "kind")?;
    let row=match kind {
        "latest"=>t.query_opt("SELECT s.snapshot_payload FROM execution_snapshots s JOIN execution_heads h ON s.session_id=h.session_id AND s.execution_epoch_id=h.execution_epoch_id AND s.execution_revision=h.published_revision WHERE s.session_id=$1",&[&sid]).await?,
        "cutoff"=>t.query_opt("SELECT s.snapshot_payload FROM execution_snapshots s JOIN execution_cutoffs c ON s.session_id=c.session_id AND s.execution_epoch_id=c.execution_epoch_id AND s.execution_revision=c.snapshot_revision WHERE s.session_id=$1",&[&sid]).await?,
        "revision"=>t.query_opt("SELECT snapshot_payload FROM execution_snapshots WHERE session_id=$1 AND execution_revision=$2",&[&sid,&num(selection,"execution_revision")?]).await?,
        _=>return Err(Fault::invalid("invalid snapshot selection"))
    };
    Ok(row.ok_or_else(Fault::missing)?.get(0))
}
async fn stored_observations(t: &Transaction<'_>, sid: &str, records: &[Value]) -> Result<BTreeMap<i64, Value>> {
    let sequences = records.iter().map(|record| num(record, "record_sequence")).collect::<Result<Vec<_>>>()?;
    let mut stored = BTreeMap::new();
    for chunk in sequences.chunks(1000) {
        for row in t.query("SELECT record_sequence,typed_state FROM execution_observations WHERE session_id=$1 AND record_sequence=ANY($2)", &[&sid, &chunk]).await? {
            stored.insert(row.get(0), row.get(1));
        }
    }
    Ok(stored)
}
fn ack(v: &Value, status: &str) -> Value {
    json!({"session_id":v["identity"]["session_id"],"execution_epoch_id":v["identity"]["execution_epoch_id"],"execution_revision":v["execution_revision"],"publication_status":status,"observation_frontier":v["observation_frontier"]})
}
fn boot_value(device: &str, boot: &str, s: &Session, state: &str, rev: Value) -> Value {
    json!({"device_id":device,"session_id":s.identity["session_id"],"execution_epoch_id":s.identity["execution_epoch_id"],"device_boot_id":boot,"state":state,"retired_at_published_revision":rev})
}
pub(super) fn settled(v: &Value) -> bool {
    matches!(
        v["lifecycle"]["state"].as_str(),
        Some("completed" | "stopped" | "failed")
    ) && v["lifecycle"]["unresolved_command_ids"]
        .as_array()
        .is_some_and(|v| v.is_empty())
        && v["pending_reservations"]
            .as_array()
            .is_some_and(|v| v.is_empty())
        && v["business_facts"]["pending_output_refs"]
            .as_array()
            .is_some_and(|v| v.is_empty())
        && v["business_facts"]["resource_claims"]
            .as_array()
            .is_some_and(|claims| {
                claims
                    .iter()
                    .all(|claim| claim["open_interval_index"].is_null())
            })
        && v["bound_jobs"].as_array().is_some_and(|v| {
            v.iter()
                .all(|x| matches!(x["state"].as_str(), Some("completed" | "cancelled")))
        })
        && v["attempts"].as_array().is_some_and(|v| {
            v.iter().all(|x| {
                matches!(
                    x["state"].as_str(),
                    Some("accepted" | "business_effect_committed" | "cancelled_not_started" | "device_failed")
                )
            })
        })
        && v["logical_work"].as_array().is_some_and(|v| {
            v.iter().all(|x| {
                !matches!(
                    x["state"].as_str(),
                    Some(
                        "reserved"
                            | "published"
                            | "device_accepted"
                            | "device_started"
                            | "device_completed"
                            | "output_wait"
                            | "reconciliation_required"
                    )
                )
            })
        })
}
fn revision(expected: &Value, actual: &Value) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        let mut e = Fault::conflict("revision_conflict", "published revision changed");
        e.expected = expected.clone();
        e.actual = actual.clone();
        Err(e)
    }
}
pub(super) fn unescape(s: &str, query: bool) -> Result<String> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            if i + 2 >= b.len() {
                return Err(Fault::invalid("invalid URL encoding"));
            }
            let h = std::str::from_utf8(&b[i + 1..i + 3])
                .map_err(|_| Fault::invalid("invalid URL encoding"))?;
            out.push(
                u8::from_str_radix(h, 16)
                    .map_err(|_| Fault::invalid("invalid URL encoding"))?,
            );
            i += 3;
        } else {
            out.push(if query && b[i] == b'+' { b' ' } else { b[i] });
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| Fault::invalid("invalid URL encoding"))
}
pub(super) fn get_query(raw: &str) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for pair in raw.split('&').filter(|x| !x.is_empty()) {
        let (k, v) = pair
            .split_once('=')
            .ok_or_else(|| Fault::invalid("query requires key=value"))?;
        if out.insert(unescape(k, true)?, unescape(v, true)?).is_some() {
            return Err(Fault::invalid("duplicate query field"));
        }
    }
    Ok(out)
}
pub(super) async fn route(
    t: &Transaction<'_>,
    p: &Principal,
    m: &str,
    path: &[&str],
    raw: &str,
    v: Value,
    h: &mut HeaderMap,
) -> Result<Value> {
    if m == "GET" {
        let allowed: &[&str] = match path {
            ["v2", "sessions", _, "events"] => &[
                "identity",
                "execution_epoch_id",
                "after_ingress_id",
                "limit",
                "visible_frontier",
            ],
            ["v2", "sessions", _, "snapshots", _] | ["v2", "sessions", _, "inputs"] => {
                &["identity", "execution_epoch_id"]
            }
            ["v2", "commands", _, "receipts"] => &[
                "query",
                "identity",
                "command_id",
                "page",
                "contract_version",
            ],
            _ => &[],
        };
        let params = get_query(raw)?;
        if params.keys().any(|k| !allowed.contains(&k.as_str()))
            || (params.contains_key("query") && params.len() != 1)
        {
            return Err(Fault::invalid("unknown or mixed URL query fields"));
        }
    } else if !raw.is_empty() {
        return Err(Fault::invalid(
            "operation accepts its typed JSON body without URL query fields",
        ));
    }
    match (m, path) {
        ("PUT", ["v2", "projects", project, "sessions", sid]) => {
            create(t, p, project, sid, typed::<CloudSessionCreate>(v)?).await
        }
        ("PUT", ["v2", "sessions", sid, "commands", cid]) => {
            command(t, p, sid, cid, typed::<Command>(v)?).await
        }
        ("POST", ["v2", "devices", device, "poll"]) => {
            poll(t, p, device, typed::<DevicePollRequest>(v)?).await
        }
        ("POST", ["v2", "devices", device, "events"]) => event(t, p, device, v).await,
        ("GET", ["v2", "sessions", sid, "events"]) => events(t, p, sid, raw, h).await,
        ("PUT", ["v2", "sessions", sid, "snapshots", rev]) => {
            publish(t, p, sid, rev, typed::<SnapshotPublication>(v)?).await
        }
        ("GET", ["v2", "sessions", sid, "snapshots", selector]) => {
            p.role(&[Role::HostGateway, Role::ScopedReader, Role::Operator])?;
            let s = session(t, p, sid, false).await?;
            query::check_get_identity(&s, raw)?;
            let selection = if *selector == "latest" || *selector == "cutoff" {
                json!({"kind":selector})
            } else {
                json!({"kind":"revision","execution_revision":selector.parse::<i64>().map_err(|_|Fault::invalid("invalid revision"))?})
            };
            let snap = snapshot(t, sid, &selection).await?;
            let r = snap["execution_revision"].clone();
            success(
                "snapshot",
                json!({"snapshot":snap,"delivery":{"read_origin":"cloud","local_execution_revision":null,"cloud_published_revision":r,"relation":"unknown"},"access":{"ownership":"other_host","allowed_session_controls":[],"device_acceptance_control_ids":[]}}),
            )
        }
        ("POST", ["v2", "data", "query"]) => {
            p.role(&[Role::HostGateway, Role::ScopedReader, Role::Operator])?;
            query::daas(t, p, typed::<DaaSQuery>(v)?).await
        }
        ("POST", ["v2", "commands", cid, "withdraw"]) => {
            withdraw(t, p, cid, typed::<CloudWithdrawRequest>(v)?).await
        }
        ("POST", ["v2", "sessions", sid, "release-slot"]) => {
            release(t, p, sid, typed::<CloudReleaseRequest>(v)?).await
        }
        ("POST", ["v2", "datasets", ds, "load-batches"]) => {
            load(t, p, ds, typed::<LoadBatch>(v)?).await
        }
        ("GET", ["v2", "projects", project, "datasets"]) => {
            p.role(&[Role::HostGateway, Role::ScopedReader, Role::Operator])?;
            p.project(project)?;
            let rows = t
                .query(
                    "SELECT * FROM datasets WHERE project_id=$1 ORDER BY dataset_id",
                    &[project],
                )
                .await?;
            let mut out = Vec::new();
            for r in rows {
                let id: String = r.get("dataset_id");
                if r.get::<_, String>("purpose") == "execution" || p.dataset_ids.contains(&id) {
                    out.push(json!({"dataset_id":id,"cloud_project_id":project,"source_mode":r.get::<_,String>("source_mode"),"session_id":r.get::<_,Option<String>>("session_id"),"visible_frontier":r.get::<_,i64>("visible_frontier"),"purpose":r.get::<_,String>("purpose")}));
                }
            }
            success(
                "dataset_catalog",
                json!({"cloud_project_id":project,"datasets":out}),
            )
        }
        ("GET", ["v2", "projects", project, "active-execution"]) => {
            p.role(&[Role::HostGateway, Role::ScopedReader, Role::Operator])?;
            p.project(project)?;
            let r=t.query_opt("SELECT x.*,s.owner_host_context,h.published_revision FROM project_execution_slots x JOIN execution_sessions s USING(session_id) LEFT JOIN execution_heads h USING(session_id) WHERE x.project_id=$1",&[project]).await?;
            let slot = if let Some(r) = r {
                let sid: String = r.get("session_id");
                let s = session(t, p, &sid, false).await?;
                let mut boots = Vec::new();
                for b in t.query("SELECT b.*,d.device_id FROM device_boots b JOIN device_bindings d ON b.device_binding_id=d.binding_id WHERE b.session_id=$1 ORDER BY d.device_id,b.device_boot_id",&[&sid]).await?{boots.push(boot_value(&b.get::<_,String>("device_id"),&b.get::<_,String>("device_boot_id"),&s,&b.get::<_,String>("state"),json!(b.get::<_,Option<i64>>("retired_at_published_revision"))));}
                json!({"session_id":sid,"execution_epoch_id":r.get::<_,String>("execution_epoch_id"),"owner_host_context":r.get::<_,String>("owner_host_context"),"state":r.get::<_,String>("state"),"published_revision":r.get::<_,Option<i64>>("published_revision"),"device_boots":boots})
            } else {
                Value::Null
            };
            success(
                "active_execution",
                json!({"cloud_project_id":project,"active_slot":slot}),
            )
        }
        ("GET", ["v2", "commands", cid]) => read_command(t, p, cid).await,
        ("GET", ["v2", "commands", cid, "receipts"]) => query::receipts(t, p, cid, raw).await,
        ("GET", ["v2", "sessions", sid, "inputs"]) => {
            p.role(&[Role::HostGateway, Role::ScopedReader])?;
            let s = session(t, p, sid, false).await?;
            query::check_get_identity(&s, raw)?;
            let frozen: Value = t.query_one("SELECT frozen_inputs FROM execution_inputs WHERE session_id=$1", &[&sid]).await?.get(0);
            success("inputs", unpack_frozen_inputs(frozen)?)
        }
        ("POST", ["v2", "sessions", sid, "observations", "query"]) => {
            p.role(&[Role::HostGateway, Role::ScopedReader])?;
            query::observations(t, p, sid, typed::<ExecutionObservationQuery>(v)?, false).await
        }
        ("POST", ["v2", "sessions", sid, "history", "query"]) => {
            p.role(&[Role::HostGateway, Role::ScopedReader])?;
            query::observations(t, p, sid, typed::<ExecutionHistoryQuery>(v)?, true).await
        }
        ("POST", ["v2", "devices", device, "boots", "register"]) => {
            register(t, p, device, typed::<DeviceBootRegisterRequest>(v)?).await
        }
        ("POST", ["v2", "devices", device, "boots", boot, "retire"]) => {
            retire(t, p, device, boot, typed::<DeviceBootRetireRequest>(v)?).await
        }
        _ => Err(Fault::invalid("unknown operation or HTTP method")),
    }
}
async fn create(
    t: &Transaction<'_>,
    p: &Principal,
    project: &str,
    sid: &str,
    v: Value,
) -> Result<Value> {
    p.role(&[Role::HostGateway])?;
    p.project(project)?;
    let r = t
        .query_opt(
            "SELECT local_project_id FROM cloud_projects WHERE project_id=$1 FOR UPDATE",
            &[&project],
        )
        .await?
        .ok_or_else(Fault::missing)?;
    if v["identity"]["session_id"] != sid
        || v["identity"]["project_id"] != r.get::<_, String>(0)
        || v["frozen_inputs"]["identity"] != v["identity"]
        || v["frozen_inputs"]["project_input_ref"] != v["project_input_ref"]
        || v["frozen_inputs"]["candidate_bindings"] != v["candidate_bindings"]
        || v["frozen_inputs"]["project"]["id"] != v["identity"]["project_id"]
        || v["project_input_ref"]["project_id"] != v["identity"]["project_id"]
        || v["frozen_inputs"]["project"]["revision"] != v["project_input_ref"]["project_revision"]
        || v["frozen_inputs"]["scenario"]["id"] != v["project_input_ref"]["scenario_id"]
        || v["frozen_inputs"]["scenario"]["revision"] != v["project_input_ref"]["scenario_revision"]
        || v["frozen_inputs"]["scenario"]["project_revision"]
            != v["project_input_ref"]["project_revision"]
    {
        return Err(Fault::invalid(
            "frozen inputs differ from session identity or input reference",
        ));
    }
    let ds = format!("execution-{sid}");
    if let Some(r) = t
        .query_opt(
            "SELECT immutable_payload,project_id FROM execution_sessions WHERE session_id=$1",
            &[&sid],
        )
        .await?
    {
        if r.get::<_, String>(1) != project {
            return Err(Fault::denied());
        }
        let mut original: Value = r.get(0);
        if original.get("frozen_inputs").is_none() {
            original["frozen_inputs"] = unpack_frozen_inputs(t.query_one("SELECT frozen_inputs FROM execution_inputs WHERE session_id=$1", &[&sid]).await?.get(0))?;
        }
        same(&original, &v)?;
        return success(
            "session_stored",
            json!({"session_id":sid,"execution_epoch_id":v["identity"]["execution_epoch_id"],"storage_status":"already_stored_same_payload","dataset_id":ds}),
        );
    }
    if t.query_opt(
        "SELECT session_id FROM project_execution_slots WHERE project_id=$1",
        &[&project],
    )
    .await?
    .is_some()
    {
        return Err(Fault::conflict(
            "active_session_conflict",
            "project already has an active session",
        ));
    }
    let mut seen = BTreeSet::new();
    for b in array(&v, "candidate_bindings")? {
        let id = text(b, "binding_id")?;
        if !seen.insert(id) {
            return Err(Fault::invalid("duplicate candidate binding"));
        }
        let frozen = &v["frozen_inputs"];
        if b["transport_supply_id"].is_null() && !array(&frozen["project"], "devices")?
            .iter()
            .any(|d| d["id"] == b["device_id"] && d["module_ref"] == b["device_module_ref"])
        {
            return Err(Fault::invalid(
                "candidate device or exact module reference absent from frozen project",
            ));
        }
        for reference in [&b["device_module_ref"], &b["operation_ref"]] {
            let definitions = array(frozen, "execution_definitions")?
                .iter()
                .chain(array(frozen, "modules")?.iter().map(|m| &m["definition"]));
            if !definitions
                .into_iter()
                .any(|d| d["id"] == reference["id"] && d["version"] == reference["version"])
            {
                return Err(Fault::invalid(
                    "candidate exact definition absent from frozen inputs",
                ));
            }
        }
        let r=t.query_opt("SELECT binding_id,device_id,definition_ref,capability_map FROM device_bindings WHERE device_id=$1 AND project_id=$2",&[&text(b,"device_id")?,&project]).await?.ok_or_else(Fault::missing)?;
        let cap: Value = r.get(3);
        if let Some(stages) = cap.get("stage_bindings") {
            if !stages.as_array().is_some_and(|items| items.contains(b)) {
                return Err(Fault::invalid("candidate differs from fixed stage mapping"));
            }
            continue;
        }
        if id != r.get::<_, String>(0) || b["device_id"] != r.get::<_, String>(1) || b["device_module_ref"] != r.get::<_, Value>(2)
        {
            return Err(Fault::invalid(
                "candidate differs from fixed device mapping",
            ));
        }
        if !matches!(cap["action"].as_str(), Some("process_unit" | "prepare_equipment" | "transport_cargo"))
            || cap["operation_ref"] != b["operation_ref"]
            || cap["execution_capacity_ref"] != b["execution_capacity_ref"]
        {
            return Err(Fault::invalid(
                "candidate capability differs from fixed mapping",
            ));
        }
    }
    let ep = text(&v["identity"], "execution_epoch_id")?;
    let owner = text(&v["identity"], "host_context_session_id")?;
    // Frozen inputs already have their own immutable row. Keep high-frequency device
    // polling from decoding the complete factory geometry on every transaction.
    let header = Value::Object(v.as_object().unwrap().iter().filter(|(key,_)| key.as_str() != "frozen_inputs").map(|(key,value)| (key.clone(),value.clone())).collect());
    t.execute("INSERT INTO execution_sessions(session_id,project_id,execution_epoch_id,owner_host_context,source_input_ref,policy,immutable_payload) VALUES($1,$2,$3,$4,$5,$6,$7)",&[&sid,&project,&ep,&owner,&v["project_input_ref"],&v["policy"],&header]).await?;
    t.execute(
        "INSERT INTO execution_inputs VALUES($1,$2,$3)",
        &[&sid, &ep, &pack_frozen_inputs(&v["frozen_inputs"])?],
    )
    .await?;
    let source: String = t
        .query_opt(
            "SELECT source_mode FROM datasets WHERE project_id=$1 AND purpose='telemetry_only'",
            &[&project],
        )
        .await?
        .ok_or_else(Fault::missing)?
        .get(0);
    t.execute("INSERT INTO datasets(dataset_id,project_id,source_mode,purpose,session_id) VALUES($1,$2,$3,'execution',$4)",&[&ds,&project,&source,&sid]).await?;
    t.execute(
        "INSERT INTO project_execution_slots VALUES($1,$2,$3,'active')",
        &[&project, &sid, &ep],
    )
    .await?;
    success(
        "session_stored",
        json!({"session_id":sid,"execution_epoch_id":ep,"storage_status":"stored","dataset_id":ds}),
    )
}
async fn command(
    t: &Transaction<'_>,
    p: &Principal,
    sid: &str,
    cid: &str,
    v: Value,
) -> Result<Value> {
    p.role(&[Role::HostGateway])?;
    let s = session(t, p, sid, true).await?;
    identity(&s, &v["identity"])?;
    if v["attempt"]["command_id"] != cid {
        return Err(Fault::invalid("command path differs"));
    }
    let b = binding(t, &s, text(&v, "device_id")?, true).await?;
    if let Some(r) = t
        .query_opt(
            "SELECT immutable_payload FROM commands WHERE command_id=$1",
            &[&cid],
        )
        .await?
    {
        same(&r.get(0), &v)?;
        return success(
            "command_stored",
            json!({"command_id":cid,"storage_status":"already_stored_same_payload"}),
        );
    }
    active_slot(t, &s).await?;
    boot_active(t, &s, &b, text(&v, "target_device_boot_id")?).await?;
    let candidate = array(&s.payload, "candidate_bindings")?
        .iter()
        .find(|x| x["binding_id"] == v["attempt"]["selected_binding_id"] && x["device_id"] == v["device_id"])
        .ok_or_else(|| Fault::invalid("selected stage absent from frozen candidates"))?;
    let capability: Value = t.query_one("SELECT capability_map FROM device_bindings WHERE binding_id=$1", &[&b]).await?.get(0);
    let expected_action = if let Some(stages) = capability.get("stage_bindings") {
        if !stages.as_array().is_some_and(|items| items.contains(candidate)) {
            return Err(Fault::invalid("selected stage differs from fixed capability"));
        }
        if !candidate["transport_supply_id"].is_null() { "transport_cargo" }
        else if !candidate["setup_transition"].is_null() { "prepare_equipment" }
        else { "process_unit" }
    } else {
        if v["attempt"]["selected_binding_id"] != b { return Err(Fault::invalid("selected binding differs")); }
        capability["action"].as_str().ok_or_else(|| Fault::invalid("missing action"))?
    };
    if v["action"] != expected_action
        || v["parameters"]["phase_id"] != v["attempt"]["phase_id"]
        || v["parameters"]["phase_id"] != candidate["phase_id"]
        || v["parameters"]["nominal_work_ms"] != candidate["nominal_work_ms"]
        || v["parameters"]["setup_transition"] != candidate["setup_transition"]
    {
        return Err(Fault::invalid(
            "command phase/capability differs from frozen binding",
        ));
    }
    t.execute("INSERT INTO commands(command_id,session_id,execution_epoch_id,logical_job_id,phase_id,attempt_ordinal,device_binding_id,immutable_payload,target_device_boot_id,delivery_state) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,'queued')",&[&cid,&sid,&text(&v["identity"],"execution_epoch_id")?,&text(&v["attempt"],"logical_job_id")?,&text(&v["attempt"],"phase_id")?,&num(&v["attempt"],"attempt_ordinal")?,&b,&v,&text(&v,"target_device_boot_id")?]).await?;
    success(
        "command_stored",
        json!({"command_id":cid,"storage_status":"stored"}),
    )
}
async fn register(t: &Transaction<'_>, p: &Principal, device: &str, v: Value) -> Result<Value> {
    p.role(&[Role::Device])?;
    p.device(device)?;
    let sid = text(&v, "session_id")?;
    let s = session(t, p, sid, true).await?;
    if v["execution_epoch_id"] != s.identity["execution_epoch_id"] {
        return Err(Fault::conflict("epoch_mismatch", "wrong epoch"));
    }
    let b = binding(t, &s, device, true).await?;
    let boot = text(&v, "device_boot_id")?;
    if let Some(r) = t
        .query_opt(
            "SELECT * FROM device_boots WHERE device_binding_id=$1 AND device_boot_id=$2",
            &[&b, &boot],
        )
        .await?
    {
        if r.get::<_, String>("session_id") != sid
            || r.get::<_, String>("execution_epoch_id") != text(&v, "execution_epoch_id")?
        {
            return Err(Fault::conflict(
                "device_boot_conflict",
                "boot already belongs to a different epoch",
            ));
        }
        return success(
            "device_boot",
            boot_value(
                device,
                boot,
                &s,
                &r.get::<_, String>("state"),
                json!(r.get::<_, Option<i64>>("retired_at_published_revision")),
            ),
        );
    }
    active_slot(t, &s).await?;
    if t.query_opt(
        "SELECT 1 FROM device_boots WHERE device_binding_id=$1 AND state='active'",
        &[&b],
    )
    .await?
    .is_some()
    {
        return Err(Fault::conflict(
            "device_boot_conflict",
            "another boot is active",
        ));
    }
    t.execute(
        "INSERT INTO device_boots VALUES($1,$2,$3,$4,'active',NULL)",
        &[&b, &boot, &sid, &text(&v, "execution_epoch_id")?],
    )
    .await?;
    success(
        "device_boot",
        boot_value(device, boot, &s, "active", Value::Null),
    )
}
async fn poll(t: &Transaction<'_>, p: &Principal, device: &str, v: Value) -> Result<Value> {
    p.role(&[Role::Device])?;
    p.device(device)?;
    let sid = text(&v, "session_id")?;
    let s = session_payload(t, p, sid, true, Some(device)).await?;
    if v["execution_epoch_id"] != s.identity["execution_epoch_id"] {
        return Err(Fault::conflict("epoch_mismatch", "wrong epoch"));
    }
    let b = binding(t, &s, device, true).await?;
    let boot = text(&v, "device_boot_id")?;
    let poll = text(&v, "poll_request_id")?;
    if let Some(r)=t.query_opt("SELECT immutable_request,response_payload FROM device_poll_requests WHERE binding_id=$1 AND device_boot_id=$2 AND poll_request_id=$3",&[&b,&boot,&poll]).await?{same(&r.get(0),&v)?;return success("device_poll",r.get(1));}
    active_slot(t, &s).await?;
    boot_active(t, &s, &b, boot).await?;
    let mut probe = Value::Null;
    if let Some(row) = t.query_opt(
        "SELECT d->'pending_observation_probe' FROM execution_heads h JOIN execution_snapshots s ON s.session_id=h.session_id AND s.execution_epoch_id=h.execution_epoch_id AND s.execution_revision=h.published_revision CROSS JOIN LATERAL jsonb_array_elements(s.snapshot_payload->'devices') WITH ORDINALITY AS views(d,ordinal) WHERE h.session_id=$1 AND d->'binding'->>'device_id'=$2 AND d->>'registered_device_boot_id'=$3 AND d->'pending_observation_probe'->>'device_id'=$2 AND d->'pending_observation_probe'->>'device_boot_id'=$3 ORDER BY ordinal DESC LIMIT 1",
        &[&sid, &device, &boot],
    ).await? { probe = row.get(0); }
    let command = if v["accept_commands"] == true {
        t.query_opt("UPDATE commands SET delivery_state='claimed' WHERE command_id=(SELECT command_id FROM commands WHERE session_id=$1 AND device_binding_id=$2 AND target_device_boot_id=$3 AND delivery_state='queued' AND closed_by_host_at IS NULL ORDER BY created_at,command_id LIMIT 1 FOR UPDATE) RETURNING immutable_payload",&[&sid,&b,&boot]).await?.map(|r|r.get::<_,Value>(0))
    } else {
        None
    };
    let outcome = if command.is_some() {
        "command"
    } else {
        "empty"
    };
    let cid = command
        .as_ref()
        .map(|v| text(&v["attempt"], "command_id").unwrap().to_string());
    let data =
        json!({"state":outcome,"poll_request_id":poll,"command":command,"observation_probe":probe});
    t.execute(
        "INSERT INTO device_poll_requests VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        &[
            &b,
            &boot,
            &poll,
            &sid,
            &text(&v, "execution_epoch_id")?,
            &cid,
            &outcome,
            &v,
            &probe,
            &data,
        ],
    )
    .await?;
    success("device_poll", data)
}
async fn event(t: &Transaction<'_>, p: &Principal, device: &str, v: Value) -> Result<Value> {
    p.role(&[Role::Device])?;
    p.device(device)?;
    let (obs, kind) = if v.get("observation").is_some() {
        let v = typed::<CloudReceiptRequest>(v)?;
        (v["observation"].clone(), "receipt")
    } else {
        (typed::<TelemetryObservation>(v)?, "telemetry")
    };
    if obs["device_id"] != device {
        return Err(Fault::invalid("device path differs"));
    }
    if !obs["phase_progress"].is_null() && (obs["device_state"] != "busy" || obs["active_command_id"].is_null()) {
        return Err(Fault::invalid("phase report requires an active busy command"));
    }
    let execution = !obs["session_id"].is_null();
    let boot = text(&obs, "device_boot_id")?;
    let sequence = num(&obs, "device_sequence")?;
    let (b, ds, s) = if execution {
        let s = session_payload(t, p, text(&obs, "session_id")?, true, Some(device)).await?;
        if obs["execution_epoch_id"] != s.identity["execution_epoch_id"] {
            return Err(Fault::conflict("epoch_mismatch", "wrong epoch"));
        }
        let b = binding(t, &s, device, true).await?;
        let ds: String = t
            .query_one(
                "SELECT dataset_id FROM datasets WHERE session_id=$1",
                &[&text(&obs, "session_id")?],
            )
            .await?
            .get(0);
        if kind == "telemetry" && obs["dataset_id"] != ds {
            return Err(Fault::invalid("execution dataset differs"));
        }
        (b, ds, Some(s))
    } else {
        if kind != "telemetry" || obs["source_purpose"] != "telemetry_only" {
            return Err(Fault::invalid("telemetry-only endpoint forbids receipts"));
        }
        let r=t.query_opt("SELECT binding_id,project_id FROM device_bindings WHERE device_id=$1 AND principal_scope_ref=$2 FOR UPDATE",&[&device,&p.principal_id]).await?.ok_or_else(Fault::denied)?;
        p.project(&r.get::<_, String>(1))?;
        (r.get(0), text(&obs, "dataset_id")?.to_string(), None)
    };
    if let Some(r)=t.query_opt("SELECT * FROM device_events WHERE device_binding_id=$1 AND device_boot_id=$2 AND device_sequence=$3",&[&b,&boot,&sequence]).await?{
        same(&r.get("typed_payload"),&obs)?;if r.get::<_,String>("dataset_id")!=ds||r.get::<_,String>("event_kind")!=kind{return Err(Fault::conflict("immutable_payload_conflict","sequence already used in another dataset or event kind"));}
        return success("event_stored",json!({"receipt_id":r.get::<_,String>("receipt_id"),"cloud_received_at":stamp(&r,"cloud_received_at"),"storage_status":"stored","host_admission_state":if execution{"pending"}else{"not_applicable"}}));
    }
    if let Some(s) = &s {
        active_slot(t, s).await?;
        boot_active(t, s, &b, boot).await?;
    }
    let d = dataset(t, p, &ds, true).await?;
    if (execution && d.get::<_, String>("purpose") != "execution")
        || (!execution && d.get::<_, String>("purpose") != "telemetry_only")
    {
        return Err(Fault::denied());
    }
    if kind == "receipt" {
        let r = t
            .query_opt(
                "SELECT immutable_payload FROM commands WHERE command_id=$1",
                &[&text(&obs, "command_id")?],
            )
            .await?
            .ok_or_else(Fault::missing)?;
        let c: Value = r.get(0);
        if c["identity"]["session_id"] != obs["session_id"]
            || c["identity"]["execution_epoch_id"] != obs["execution_epoch_id"]
            || c["device_id"] != device
            || c["target_device_boot_id"] != boot
            || c["attempt"]["phase_id"] != obs["phase_id"]
            || c["attempt"]["phase_token"] != obs["phase_token"]
        {
            return Err(Fault::invalid("receipt differs from immutable command"));
        }
    }
    let ingress = d.get::<_, i64>("next_ingress_sequence") + 1;
    let receipt = format!("event-{ds}-{ingress}");
    let now: chrono::DateTime<chrono::Utc> =
        t.query_one("SELECT clock_timestamp()", &[]).await?.get(0);
    let sid = obs["session_id"].as_str();
    let epoch = obs["execution_epoch_id"].as_str();
    let cid = if kind == "receipt" {
        obs["command_id"].as_str()
    } else {
        None
    };
    t.execute("INSERT INTO device_events(receipt_id,dataset_id,ingress_id,device_ref,device_binding_id,device_boot_id,device_sequence,session_id,execution_epoch_id,event_kind,typed_payload,cloud_received_at,command_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",&[&receipt,&ds,&ingress,&device,&b,&boot,&sequence,&sid,&epoch,&kind,&obs,&now,&cid]).await?;
    t.execute(
        "UPDATE datasets SET next_ingress_sequence=$2,visible_frontier=$2 WHERE dataset_id=$1",
        &[&ds, &ingress],
    )
    .await?;
    success(
        "event_stored",
        json!({"receipt_id":receipt,"cloud_received_at":now.to_rfc3339_opts(chrono::SecondsFormat::Micros,true),"storage_status":"stored","host_admission_state":if execution{"pending"}else{"not_applicable"}}),
    )
}
async fn load(t: &Transaction<'_>, p: &Principal, ds: &str, v: Value) -> Result<Value> {
    p.role(&[Role::GeneratedLoadProducer])?;
    p.dataset(ds)?;
    if v["dataset_id"] != ds {
        return Err(Fault::invalid("dataset path differs"));
    }
    let d = dataset(t, p, ds, true).await?;
    if d.get::<_, String>("purpose") != "generated_load" {
        return Err(Fault::denied());
    }
    let events = array(&v, "events")?;
    let batch = text(&v, "batch_id")?;
    let old=t.query("SELECT typed_payload FROM device_events WHERE dataset_id=$1 AND batch_id=$2 ORDER BY batch_ordinal",&[&ds,&batch]).await?;
    if !old.is_empty() {
        same(
            &json!(old.iter().map(|x| x.get::<_, Value>(0)).collect::<Vec<_>>()),
            &v["events"],
        )?;
        return success(
            "load_ingested",
            json!({"dataset_id":ds,"batch_id":batch,"inserted_count":0,"reused_count":events.len()}),
        );
    }
    let mut pairs = BTreeSet::new();
    for e in events {
        if !pairs.insert((text(e, "device_ref")?, num(e, "device_sequence")?)) {
            return Err(Fault::invalid("duplicate device sequence within batch"));
        }
    }
    let count = events.len() as i64;
    let base = d.get::<_, i64>("next_ingress_sequence");
    let now: chrono::DateTime<chrono::Utc> =
        t.query_one("SELECT clock_timestamp()", &[]).await?.get(0);
    t.execute("INSERT INTO device_events(receipt_id,dataset_id,ingress_id,device_ref,device_sequence,event_kind,typed_payload,cloud_received_at,batch_id,batch_ordinal) SELECT 'event-'||$1||'-'||($3+x.n)::text,$1,$3+x.n,x.e->>'device_ref',(x.e->>'device_sequence')::bigint,'generated_load',x.e,$4,$2,x.n-1 FROM jsonb_array_elements($5::jsonb) WITH ORDINALITY x(e,n)",&[&ds,&batch,&base,&now,&v["events"]]).await?;
    t.execute(
        "UPDATE datasets SET next_ingress_sequence=$2,visible_frontier=$2 WHERE dataset_id=$1",
        &[&ds, &(base + count)],
    )
    .await?;
    success(
        "load_ingested",
        json!({"dataset_id":ds,"batch_id":batch,"inserted_count":count,"reused_count":0}),
    )
}
async fn events(
    t: &Transaction<'_>,
    p: &Principal,
    sid: &str,
    raw: &str,
    h: &mut HeaderMap,
) -> Result<Value> {
    p.role(&[Role::HostGateway])?;
    let s = session(t, p, sid, false).await?;
    let q = get_query(raw)?;
    query::check_get_identity(&s, raw)?;
    let after = q
        .get("after_ingress_id")
        .map(|v| v.parse::<i64>())
        .transpose()
        .map_err(|_| Fault::invalid("invalid ingress cursor"))?
        .unwrap_or(0);
    let limit = q
        .get("limit")
        .map(|v| v.parse::<i64>())
        .transpose()
        .map_err(|_| Fault::invalid("invalid limit"))?
        .unwrap_or(200);
    if after < 0 || !(1..=200).contains(&limit) {
        return Err(Fault::invalid("invalid events page"));
    }
    let r = t
        .query_one(
            "SELECT dataset_id,visible_frontier FROM datasets WHERE session_id=$1",
            &[&sid],
        )
        .await?;
    let ds: String = r.get(0);
    let current: i64 = r.get(1);
    let f = q
        .get("visible_frontier")
        .map(|x| x.parse::<i64>())
        .transpose()
        .map_err(|_| Fault::invalid("invalid frontier"))?
        .unwrap_or(current);
    if f < 0 || f > current || after > f {
        return Err(Fault::invalid("invalid frontier"));
    }
    h.insert("x-sf-visible-frontier", f.to_string().parse().unwrap());
    let rows=t.query("SELECT * FROM device_events WHERE dataset_id=$1 AND ingress_id>$2 AND ingress_id<=$3 ORDER BY ingress_id LIMIT $4",&[&ds,&after,&f,&(limit+1)]).await?;
    let more = rows.len() > limit as usize;
    let out: Vec<_> = rows
        .iter()
        .take(limit as usize)
        .map(query::event_row)
        .collect();
    let next = if more {
        out.last().map(|v| v["ingress_id"].clone())
    } else {
        None
    };
    success(
        "events_page",
        json!({"identity":s.identity,"rows":out,"next_ingress_id":next}),
    )
}
async fn publish(
    t: &Transaction<'_>,
    p: &Principal,
    sid: &str,
    rev: &str,
    v: Value,
) -> Result<Value> {
    p.role(&[Role::HostGateway])?;
    let s = session(t, p, sid, true).await?;
    let snap = &v["snapshot"];
    identity(&s, &snap["identity"])?;
    let r = num(snap, "execution_revision")?;
    if rev.parse::<i64>().ok() != Some(r) {
        return Err(Fault::invalid("revision path differs"));
    }
    if snap["project_input_ref"] != s.payload["project_input_ref"]
        || snap["policy"] != s.payload["policy"]
    {
        return Err(Fault::invalid("snapshot changed frozen inputs/policy"));
    }
    if let Some(old)=t.query_opt("SELECT snapshot_payload,publication_payload FROM execution_snapshots WHERE session_id=$1 AND execution_revision=$2",&[&sid,&r]).await?{
        same(&old.get(0),snap)?;let original:Value=old.get(1);same(&original["as_cutoff_snapshot"],&v["as_cutoff_snapshot"])?;
        let existing = stored_observations(t, sid, array(&v,"observation_records")?).await?;
        for rec in array(&v,"observation_records")?{let n=num(rec,"record_sequence")?;if n>num(snap,"observation_frontier")?{return Err(Fault::invalid("record exceeds immutable snapshot prefix"));}same(existing.get(&n).ok_or_else(Fault::missing)?,rec)?;}
        return success("snapshot_published",ack(snap,"already_stored_same_payload"));
    }
    active_slot(t, &s).await?;
    t.query("SELECT b.binding_id FROM device_bindings b WHERE b.project_id=$1 ORDER BY b.binding_id FOR UPDATE",&[&s.cloud]).await?;
    t.query(
        "SELECT command_id FROM commands WHERE session_id=$1 ORDER BY command_id FOR UPDATE",
        &[&sid],
    )
    .await?;
    let head = t
        .query_opt(
            "SELECT published_revision FROM execution_heads WHERE session_id=$1 FOR UPDATE",
            &[&sid],
        )
        .await?;
    let actual = head.as_ref().map(|x| x.get::<_, i64>(0));
    revision(&v["expected_published_revision"], &json!(actual))?;
    if actual.is_some_and(|a| r <= a) {
        return Err(Fault::invalid("revision must advance"));
    }
    let old = if actual.is_some() {
        Some(snapshot(t, sid, &json!({"kind":"latest"})).await?)
    } else {
        None
    };
    let prev = old
        .as_ref()
        .map(|x| num(x, "observation_frontier").unwrap())
        .unwrap_or(0);
    let next = num(snap, "observation_frontier")?;
    let until = num(&snap["coverage"], "observed_until_host_ms")?;
    if next < prev
        || old
            .as_ref()
            .is_some_and(|x| x["coverage"]["observed_until_host_ms"].as_i64().unwrap() > until)
    {
        return Err(Fault::invalid("observation prefix or Host clock regressed"));
    }
    let mut new_records = BTreeMap::new();
    let existing = stored_observations(t, sid, array(&v, "observation_records")?).await?;
    for rec in array(&v, "observation_records")? {
        let n = num(rec, "record_sequence")?;
        if n == 0
            || n > next
            || num(rec, "execution_revision")? > r
            || num(rec, "at_host_ms")? > until
        {
            return Err(Fault::invalid("observation exceeds snapshot prefix"));
        }
        if let Some(old) = existing.get(&n) { same(old, rec)?; }
        else if new_records.insert(n,rec).is_some() { return Err(Fault::invalid("duplicate record sequence")); }
    }
    if new_records.len() as i64 != next - prev
        || new_records
            .keys()
            .enumerate()
            .any(|(i, n)| *n != prev + i as i64 + 1)
    {
        return Err(Fault::invalid(
            "observation records must form complete contiguous published prefix",
        ));
    }
    if let Some(old) = &old {
        for a in array(old, "receipt_admissions")? {
            if !array(snap, "receipt_admissions")?.contains(a) {
                return Err(Fault::invalid(
                    "snapshot dropped an existing Host admission prefix",
                ));
            }
        }
    }
    for d in array(snap, "devices")? {
        let b = &d["binding"];
        if !array(&s.payload, "candidate_bindings")?.contains(b) {
            return Err(Fault::invalid("snapshot device changed frozen binding"));
        }
        if let Some(probe) = d["pending_observation_probe"].as_object() {
            let q = Value::Object(probe.clone());
            if q["device_id"] != b["device_id"]
                || q["device_boot_id"] != d["registered_device_boot_id"]
                || num(&q, "issued_execution_revision")? > r
                || num(&q, "issued_at_host_ms")? > until
                || num(&q, "expires_at_host_ms")? < num(&q, "issued_at_host_ms")?
            {
                return Err(Fault::invalid(
                    "snapshot probe differs from selected device boot or Host revision",
                ));
            }
        }
    }
    let mut rows = Vec::with_capacity(new_records.len());
    for (n, rec) in new_records {
        let obj = &rec["object"];
        let kind = text(obj, "kind")?;
        let id = query::object_id(obj)?;
        rows.push(json!({"record_sequence":n,"execution_revision":num(rec,"execution_revision")?,
            "at_host_ms":num(rec,"at_host_ms")?,"kind":kind,"id":id,"typed_state":rec}));
    }
    for chunk in rows.chunks(1000) {
        t.execute("INSERT INTO execution_observations SELECT $1,$2,r.record_sequence,r.execution_revision,r.at_host_ms,r.kind,r.id,r.typed_state FROM jsonb_to_recordset($3) AS r(record_sequence bigint,execution_revision bigint,at_host_ms bigint,kind text,id text,typed_state jsonb)",
            &[&sid,&text(&s.identity,"execution_epoch_id")?,&json!(chunk)]).await?;
    }
    for a in array(snap, "receipt_admissions")? {
        // The immutable previous publication already verified these exact rows.
        // Its complete prefix was checked above; only additions need database reads.
        if old.as_ref().is_some_and(|previous| previous["receipt_admissions"].as_array().unwrap().contains(a)) {
            continue;
        }
        if num(a, "execution_revision")? > r || num(a, "host_received_elapsed_ms")? > until {
            return Err(Fault::invalid("admission exceeds snapshot"));
        }
        let event=t.query_opt("SELECT session_id,execution_epoch_id,event_kind FROM device_events WHERE receipt_id=$1",&[&text(a,"receipt_id")?]).await?.ok_or_else(Fault::missing)?;
        if event.get::<_, Option<String>>(0).as_deref() != Some(sid)
            || event.get::<_, Option<String>>(1).as_deref()
                != s.identity["execution_epoch_id"].as_str()
        {
            return Err(Fault::invalid("admission references another session"));
        }
        let ar = num(a, "execution_revision")?;
        if let Some(old)=t.query_opt("SELECT typed_payload FROM receipt_admissions WHERE receipt_id=$1 AND execution_revision=$2",&[&text(a,"receipt_id")?,&ar]).await?{same(&old.get(0),a)?;}else{t.execute("INSERT INTO receipt_admissions VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",&[&text(a,"receipt_id")?,&ar,&sid,&text(&s.identity,"execution_epoch_id")?,&text(a,"decision")?,&num(a,"host_admission_sequence")?,&num(a,"host_received_elapsed_ms")?,&a["business_effect_committed"].as_bool().unwrap(),&a["reason"].as_str(),&a]).await?;}
    }
    let ep = text(&s.identity, "execution_epoch_id")?;
    t.execute("INSERT INTO execution_snapshots(session_id,execution_epoch_id,execution_revision,snapshot_payload,publication_payload,host_observed_until_ms,observation_frontier) VALUES($1,$2,$3,$4,$5,$6,$7)",&[&sid,&ep,&r,&snap,&v,&until,&next]).await?;
    if v["as_cutoff_snapshot"] == true {
        if snap["coverage"]["requested_cutoff_reached"] != true
            || snap["coverage"]["cutoff_snapshot_revision"] != r
            || until < num(&s.payload["policy"], "report_cutoff_ms")?
        {
            return Err(Fault::invalid("invalid cutoff snapshot"));
        }
        t.execute(
            "INSERT INTO execution_cutoffs VALUES($1,$2,$3,$4)",
            &[
                &sid,
                &num(&s.payload["policy"], "report_cutoff_ms")?,
                &ep,
                &r,
            ],
        )
        .await?;
    }
    t.execute("INSERT INTO execution_heads VALUES($1,$2,$3) ON CONFLICT(session_id) DO UPDATE SET published_revision=excluded.published_revision",&[&sid,&ep,&r]).await?;
    if snap["lifecycle"]["state"] == "needs_resolution" {
        t.execute(
            "UPDATE project_execution_slots SET state='needs_resolution' WHERE project_id=$1",
            &[&s.cloud],
        )
        .await?;
    }
    close_completed_commands(t, &sid, snap, old.as_ref(), None).await?;
    success("snapshot_published", ack(snap, "stored"))
}
// Physical stage completion closes its command even when the product is an
// intermediate result. Final acceptance remains a separate Host business fact.
async fn close_completed_commands(t: &Transaction<'_>, sid: &str, snap: &Value,
    old: Option<&Value>, only_device: Option<&str>) -> Result<()> {
    for a in array(snap, "attempts")? {
        if only_device.is_some_and(|device| a["device_id"] != device) { continue; }
        if old.as_ref().is_some_and(|previous| previous["attempts"].as_array().unwrap().contains(a)) {
            continue;
        }
        if matches!(
            a["state"].as_str(),
            Some("accepted" | "business_effect_committed" | "device_completed" | "cancelled_not_started" | "device_failed")
        ) {
            let cid = text(&a["attempt"], "command_id")?;
            let row=t.query_opt("SELECT immutable_payload,delivery_state FROM commands WHERE command_id=$1 AND session_id=$2",&[&cid,&sid]).await?.ok_or_else(Fault::missing)?;
            let c: Value = row.get(0);
            if c["attempt"] != a["attempt"]
                || c["target_device_boot_id"] != a["target_device_boot_id"]
                || c["device_id"] != a["device_id"]
            {
                return Err(Fault::invalid(
                    "terminal attempt differs from stored command",
                ));
            }
            let receipts = array(a, "receipt_ids")?;
            let withdrawn =
                a["state"] == "cancelled_not_started" && row.get::<_, String>(1) == "withdrawn";
            if !withdrawn {
                let expected = match a["state"].as_str().unwrap() {
                    "accepted" | "business_effect_committed" | "device_completed" => "completed",
                    "device_failed" => "failed",
                    _ => "cancelled_not_started",
                };
                let mut terminal = false;
                for id in receipts {
                    let e=t.query_opt("SELECT typed_payload FROM device_events WHERE receipt_id=$1 AND command_id=$2 AND event_kind='receipt'",&[&id.as_str().unwrap(),&cid]).await?.ok_or_else(Fault::missing)?;
                    let e: Value = e.get(0);
                    if e["receipt"]["kind"] == expected
                        && array(snap, "receipt_admissions")?.iter().any(|v| {
                            v["receipt_id"] == *id
                                && v["decision"] == "applied"
                                && (!matches!(a["state"].as_str(), Some("accepted" | "business_effect_committed" | "device_completed"))
                                    || v["business_effect_committed"] == true)
                        })
                    {
                        terminal = true;
                    }
                }
                if !terminal {
                    return Err(Fault::invalid(
                        "terminal command needs matching Host-admitted terminal receipt",
                    ));
                }
            }
            for id in receipts {
                if !array(snap, "receipt_admissions")?
                    .iter()
                    .any(|v| v["receipt_id"] == *id && v["decision"] == "applied")
                {
                    return Err(Fault::invalid(
                        "terminal receipt has not been applied by Host",
                    ));
                }
            }
            t.execute("UPDATE commands SET closed_by_host_at=clock_timestamp() WHERE command_id=$1 AND closed_by_host_at IS NULL",&[&cid]).await?;
        }
    }
    Ok(())
}
async fn withdraw(t: &Transaction<'_>, p: &Principal, cid: &str, v: Value) -> Result<Value> {
    p.role(&[Role::HostGateway])?;
    let s = session(t, p, text(&v["identity"], "session_id")?, true).await?;
    identity(&s, &v["identity"])?;
    if v["command_id"] != cid {
        return Err(Fault::invalid("command path differs"));
    }
    let r=t.query_opt("SELECT delivery_state,closed_by_host_at,device_binding_id FROM commands WHERE command_id=$1 AND session_id=$2",&[&cid,&text(&s.identity,"session_id")?]).await?.ok_or_else(Fault::missing)?;
    t.query_one(
        "SELECT binding_id FROM device_bindings WHERE binding_id=$1 FOR UPDATE",
        &[&r.get::<_, String>(2)],
    )
    .await?;
    let state = if r
        .get::<_, Option<chrono::DateTime<chrono::Utc>>>(1)
        .is_some()
    {
        "already_settled"
    } else if r.get::<_, String>(0) == "claimed" {
        t.execute(
            "UPDATE commands SET cancel_requested=true WHERE command_id=$1",
            &[&cid],
        )
        .await?;
        "device_confirmation_required"
    } else {
        t.execute(
            "UPDATE commands SET delivery_state='withdrawn' WHERE command_id=$1",
            &[&cid],
        )
        .await?;
        "withdrawn_before_delivery"
    };
    success("withdrawal", json!({"command_id":cid,"state":state}))
}
async fn retire(
    t: &Transaction<'_>,
    p: &Principal,
    device: &str,
    boot: &str,
    v: Value,
) -> Result<Value> {
    p.role(&[Role::HostGateway])?;
    let sid = text(&v, "session_id")?;
    let s = session(t, p, sid, true).await?;
    if v["execution_epoch_id"] != s.identity["execution_epoch_id"] {
        return Err(Fault::conflict("epoch_mismatch", "wrong epoch"));
    }
    let b = binding(t, &s, device, true).await?;
    let r=t.query_opt("SELECT * FROM device_boots WHERE device_binding_id=$1 AND device_boot_id=$2 FOR UPDATE",&[&b,&boot]).await?.ok_or_else(Fault::missing)?;
    if r.get::<_, String>("session_id") != sid
        || r.get::<_, String>("execution_epoch_id") != text(&v, "execution_epoch_id")?
    {
        return Err(Fault::conflict("device_boot_mismatch", "wrong boot epoch"));
    }
    if r.get::<_, String>("state") == "retired" {
        revision(
            &v["expected_published_revision"],
            &json!(r.get::<_, Option<i64>>("retired_at_published_revision")),
        )?;
        return success(
            "device_boot",
            boot_value(
                device,
                boot,
                &s,
                "retired",
                v["expected_published_revision"].clone(),
            ),
        );
    }
    let snap = snapshot(t, sid, &json!({"kind":"latest"})).await?;
    revision(
        &v["expected_published_revision"],
        &snap["execution_revision"],
    )?;
    if !settled(&snap) { return Err(Fault::conflict("device_boot_conflict", "published session remains unsettled")); }
    if t.query_opt("SELECT 1 FROM commands WHERE device_binding_id=$1 AND target_device_boot_id=$2 AND closed_by_host_at IS NULL LIMIT 1", &[&b,&boot]).await?.is_some() {
        // Repair only the observed older publisher omission, using the exact
        // current snapshot and matching Host-admitted terminal receipts.
        close_completed_commands(t, &sid, &snap, None, Some(device)).await?;
        if t.query_opt("SELECT 1 FROM commands WHERE device_binding_id=$1 AND target_device_boot_id=$2 AND closed_by_host_at IS NULL LIMIT 1", &[&b,&boot]).await?.is_some() {
            return Err(Fault::conflict("device_boot_conflict", "boot commands remain unsettled"));
        }
    }
    t.execute("UPDATE device_boots SET state='retired',retired_at_published_revision=$3 WHERE device_binding_id=$1 AND device_boot_id=$2",&[&b,&boot,&num(&v,"expected_published_revision")?]).await?;
    success(
        "device_boot",
        boot_value(
            device,
            boot,
            &s,
            "retired",
            v["expected_published_revision"].clone(),
        ),
    )
}
async fn release(t: &Transaction<'_>, p: &Principal, sid: &str, v: Value) -> Result<Value> {
    p.role(&[Role::HostGateway])?;
    let s = session(t, p, sid, true).await?;
    identity(&s, &v["identity"])?;
    let snap = snapshot(t, sid, &json!({"kind":"latest"})).await?;
    revision(
        &v["expected_published_revision"],
        &snap["execution_revision"],
    )?;
    if !settled(&snap)||t.query_opt("SELECT 1 FROM device_boots WHERE session_id=$1 AND state='active' UNION ALL SELECT 1 FROM commands WHERE session_id=$1 AND closed_by_host_at IS NULL LIMIT 1",&[&sid]).await?.is_some(){return Err(Fault::conflict("active_session_conflict","published session, commands or boots remain unsettled"));}
    t.execute(
        "DELETE FROM project_execution_slots WHERE project_id=$1 AND session_id=$2",
        &[&s.cloud, &sid],
    )
    .await?;
    success(
        "slot_released",
        json!({"session_id":sid,"execution_epoch_id":s.identity["execution_epoch_id"],"released":true}),
    )
}
async fn read_command(t: &Transaction<'_>, p: &Principal, cid: &str) -> Result<Value> {
    p.role(&[
        Role::HostGateway,
        Role::ScopedReader,
        Role::Operator,
        Role::Device,
    ])?;
    let r = t
        .query_opt("SELECT * FROM commands WHERE command_id=$1", &[&cid])
        .await?
        .ok_or_else(Fault::missing)?;
    let c: Value = r.get("immutable_payload");
    session(t, p, text(&c["identity"], "session_id")?, false).await?;
    if p.role == Role::Device {
        p.device(text(&c, "device_id")?)?;
    }
    let head = t
        .query_opt(
            "SELECT published_revision FROM execution_heads WHERE session_id=$1",
            &[&text(&c["identity"], "session_id")?],
        )
        .await?
        .map(|r| r.get::<_, i64>(0));
    let admission=t.query_opt("SELECT a.typed_payload FROM receipt_admissions a JOIN device_events e USING(receipt_id) WHERE e.command_id=$1 AND a.execution_revision<=$2 ORDER BY a.execution_revision DESC,a.host_admission_sequence DESC LIMIT 1",&[&cid,&head.unwrap_or(-1)]).await?.map(|r|r.get::<_,Value>(0));
    let now: chrono::DateTime<chrono::Utc> =
        t.query_one("SELECT clock_timestamp()", &[]).await?.get(0);
    success(
        "command",
        json!({"command":c,"cloud_delivery":{"state":r.get::<_,String>("delivery_state"),"cancel_requested":r.get::<_,bool>("cancel_requested"),"observed_at":now.to_rfc3339_opts(chrono::SecondsFormat::Micros,true)},"host_confirmation":{"published_execution_revision":head,"latest_admission":admission}}),
    )
}
