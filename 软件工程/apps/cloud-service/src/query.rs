use super::store::{Session, dataset, get_query, identity, session, snapshot};
use super::*;

fn object_kind_rank(kind: &str) -> u8 {
    match kind {
        "item" => 0,
        "job" => 1,
        "device" => 2,
        "logical_work" => 3,
        "attempt" => 4,
        "transport" => 5,
        _ => unreachable!("validated execution object kind"),
    }
}

pub(super) fn check_get_identity(s: &Session, raw: &str) -> Result<()> {
    let q = get_query(raw)?;
    if let Some(v) = q.get("identity") {
        identity(
            s,
            &serde_json::from_str::<Value>(v)
                .map_err(|_| Fault::invalid("invalid identity query"))?,
        )?;
    }
    if (!q.contains_key("identity") && !q.contains_key("execution_epoch_id"))
        || q.get("execution_epoch_id")
            .is_some_and(|x| Some(x.as_str()) != s.identity["execution_epoch_id"].as_str())
    {
        return Err(Fault::conflict(
            "epoch_mismatch",
            "execution_epoch_id query required and must match",
        ));
    }
    Ok(())
}
pub(super) fn object_id(obj: &Value) -> Result<String> {
    let d = &obj["data"];
    let id = match text(obj, "kind")? {
        "device" => &d["binding"]["device_id"],
        "logical_work" => &d["logical_job_id"],
        "attempt" => &d["attempt"]["command_id"],
        "job" => &d["job_id"],
        "item" => &d["id"],
        "transport" => &d["id"],
        _ => return Err(Fault::invalid("invalid observation kind")),
    };
    id.as_str()
        .map(str::to_owned)
        .ok_or_else(|| Fault::invalid("observation has no indexed object identity"))
}
pub(super) fn event_row(r: &Row) -> Value {
    json!({"kind":r.get::<_,String>("event_kind"),"receipt_id":r.get::<_,String>("receipt_id"),"ingress_id":r.get::<_,i64>("ingress_id"),"cloud_received_at":stamp(r,"cloud_received_at"),"observation":r.get::<_,Value>("typed_payload")})
}
fn source_row(r: &Row, ds: &str) -> Value {
    let k: String = r.get("event_kind");
    let mut v = json!({"kind":k,"ingress_id":r.get::<_,i64>("ingress_id"),"dataset_id":ds,"cloud_received_at":stamp(r,"cloud_received_at")});
    v[if k == "generated_load" {
        "event"
    } else {
        "observation"
    }] = r.get("typed_payload");
    v
}
pub(super) async fn daas(t: &Transaction<'_>, p: &Principal, v: Value) -> Result<Value> {
    if v["dataset"] == "telemetry" {
        telemetry(t, p, v).await
    } else {
        execution(t, p, v).await
    }
}
async fn telemetry(t: &Transaction<'_>, p: &Principal, mut v: Value) -> Result<Value> {
    let ds = text(&v, "dataset_id")?.to_string();
    let d = dataset(t, p, &ds, false).await?;
    if v["cloud_project_id"] != d.get::<_, String>("project_id") {
        return Err(Fault::denied());
    }
    let from: CloudTimestamp = serde_json::from_value(v["received_from"].clone())
        .map_err(|_| Fault::invalid("invalid timestamp"))?;
    let until: CloudTimestamp = serde_json::from_value(v["received_until"].clone())
        .map_err(|_| Fault::invalid("invalid timestamp"))?;
    if from.as_datetime() > until.as_datetime() {
        return Err(Fault::invalid("received range inverted"));
    }
    let (limit, cursor) = page(&v)?;
    let current = d.get::<_, i64>("visible_frontier");
    let f = cursor
        .as_ref()
        .map(|x| num(x, "visible_frontier"))
        .transpose()?
        .unwrap_or(current);
    if f > current {
        return Err(Fault::invalid("cursor frontier exceeds committed prefix"));
    }
    let mut devices: Vec<String> = array(&v, "device_ids")?
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    devices.sort();
    devices.dedup();
    if devices.is_empty() {
        devices=t.query("SELECT DISTINCT device_ref FROM device_events WHERE dataset_id=$1 AND ingress_id<=$2 AND event_kind IN ('telemetry','generated_load') AND device_ref=ANY($3) ORDER BY device_ref",&[&ds,&f,&p.device_ids]).await?.iter().map(|r|r.get(0)).collect();
    } else {
        for device in &devices {
            p.device(device)?;
        }
    }
    v["device_ids"] = json!(devices);
    let selection = normalized_query(&v);
    let aggregate = v["aggregation"] == "count_by_device_minute";
    let after = if let Some(c) = &cursor {
        typed::<TelemetryCursor>(c.clone())?;
        if c["dataset_id"] != ds
            || c["query_selection"] != selection
            || c["kind"]
                != if aggregate {
                    "telemetry_aggregate"
                } else {
                    "telemetry_rows"
                }
        {
            return Err(Fault::invalid(
                "cursor does not match normalized query or current device scope",
            ));
        }
        if aggregate {
            c["after_row_index"].as_i64().unwrap_or(-1)
        } else {
            num(c, "after_ingress_id")?
        }
    } else {
        if aggregate { -1 } else { 0 }
    };
    if !aggregate && after > f {
        return Err(Fault::invalid("cursor position exceeds frontier"));
    }
    let rows = if aggregate {
        t.query("WITH groups AS (SELECT date_trunc('minute',cloud_received_at AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' bucket_start,device_ref,count(*) count FROM device_events WHERE dataset_id=$1 AND ingress_id<=$2 AND device_ref=ANY($3) AND cloud_received_at>=$4 AND cloud_received_at<$5 AND event_kind IN ('telemetry','generated_load') GROUP BY 1,2), numbered AS (SELECT *,row_number() OVER(ORDER BY bucket_start,device_ref)-1 row_index FROM groups) SELECT * FROM numbered WHERE row_index>$6 ORDER BY row_index LIMIT $7",&[&ds,&f,&devices,&from.as_datetime(),&until.as_datetime(),&after,&(limit+1)]).await?
    } else {
        t.query("SELECT * FROM device_events WHERE dataset_id=$1 AND ingress_id>$6 AND ingress_id<=$2 AND device_ref=ANY($3) AND cloud_received_at>=$4 AND cloud_received_at<$5 AND event_kind IN ('telemetry','generated_load') ORDER BY ingress_id LIMIT $7",&[&ds,&f,&devices,&from.as_datetime(),&until.as_datetime(),&after,&(limit+1)]).await?
    };
    let more = rows.len() > limit as usize;
    let out:Vec<Value>=rows.iter().take(limit as usize).map(|r|if aggregate {json!({"kind":"count_by_device_minute","device_id":r.get::<_,String>("device_ref"),"source_mode":d.get::<_,String>("source_mode"),"bucket_start":stamp(r,"bucket_start"),"count":r.get::<_,i64>("count"),"row_index":r.get::<_,i64>("row_index")})}else{source_row(r,&ds)}).collect();
    let next = if more {
        let last = out.last().unwrap();
        let mut c = json!({"kind":if aggregate{"telemetry_aggregate"}else{"telemetry_rows"},"cursor_version":"1","dataset_id":ds,"query_selection":selection,"visible_frontier":f});
        c[if aggregate {
            "after_row_index"
        } else {
            "after_ingress_id"
        }] = last[if aggregate { "row_index" } else { "ingress_id" }].clone();
        typed::<TelemetryCursor>(c.clone())?;
        Some(encode(&c))
    } else {
        None
    };
    success(
        "query_result",
        json!({"dataset":"telemetry","contract_version":CONTRACT,"selected_epoch":null,"selected_execution_revision":null,"rows":out,"next_cursor":next,"data_quality_issues":[],"dataset_id":ds,"selected_visible_frontier":f,"selected_device_ids":devices}),
    )
}
async fn execution(t: &Transaction<'_>, p: &Principal, v: Value) -> Result<Value> {
    let sid = text(&v["identity"], "session_id")?;
    let s = session(t, p, sid, false).await?;
    identity(&s, &v["identity"])?;
    let from = num(&v, "host_from_ms")?;
    let until = num(&v, "host_until_ms")?;
    if from > until {
        return Err(Fault::invalid("Host range inverted"));
    }
    let (limit, cursor) = page(&v)?;
    let selection = normalized_query(&v);
    let snap = if let Some(c) = &cursor {
        typed::<ExecutionCursor>(c.clone())?;
        if c["identity"] != s.identity || c["query_selection"] != selection {
            return Err(Fault::invalid("cursor query differs"));
        }
        snapshot(
            t,
            sid,
            &json!({"kind":"revision","execution_revision":c["execution_revision"]}),
        )
        .await?
    } else {
        snapshot(t, sid, &v["selection"]).await?
    };
    let rev = snap["execution_revision"].clone();
    let after = cursor
        .as_ref()
        .map(|c| num(c, "after_row_index"))
        .transpose()?
        .unwrap_or(-1);
    let mut out = Vec::new();
    match text(&v, "aggregation")? {
        "rows" => {
            for a in array(&snap, "attempts")? {
                let time = a["completed_at_host_ms"]
                    .as_i64()
                    .or(a["started_at_host_ms"].as_i64())
                    .unwrap_or(num(&a["dispatch_decision"], "decision_host_ms")?);
                if time >= from && time < until {
                    out.push(json!({"kind":"execution_attempt","identity":s.identity,"execution_revision":rev,"attempt":a,"row_index":out.len()}));
                }
            }
            for e in array(&snap["business_facts"], "acceptance_events")? {
                let at = num(e, "at_ms")?;
                if at >= from && at < until {
                    out.push(json!({"kind":"acceptance","identity":s.identity,"execution_revision":rev,"event":e,"row_index":out.len()}));
                }
            }
        }
        "accepted_quantity" => {
            // Quantity uses integer fixed point. Mixed units are separate rows; never float-round quantities.
            let mut sums: BTreeMap<String, (i128, u32)> = BTreeMap::new();
            for e in array(&snap["business_facts"], "acceptance_events")? {
                let at = num(e, "at_ms")?;
                if at < from || at >= until {
                    continue;
                }
                let q = &e["quantity"];
                let unit = text(q, "unit")?.to_string();
                let coef = q["value"]["coefficient"]
                    .as_i64()
                    .ok_or_else(|| Fault::invalid("invalid quantity coefficient"))?
                    as i128;
                let dp = num(&q["value"], "decimal_places")? as u32;
                let entry = sums.entry(unit).or_insert((0, dp));
                let scale = entry.1.max(dp);
                let left = 10_i128
                    .checked_pow(scale - entry.1)
                    .and_then(|p| entry.0.checked_mul(p));
                let right = 10_i128
                    .checked_pow(scale - dp)
                    .and_then(|p| coef.checked_mul(p));
                entry.0 = left
                    .and_then(|a| right.and_then(|b| a.checked_add(b)))
                    .ok_or_else(|| Fault::invalid("quantity aggregate overflow"))?;
                entry.1 = scale;
            }
            for (unit, (coefficient, dp)) in sums {
                let c = i64::try_from(coefficient)
                    .map_err(|_| Fault::invalid("quantity aggregate overflow"))?;
                out.push(json!({"kind":"accepted_quantity","identity":s.identity,"execution_revision":rev,"host_from_ms":from,"host_until_ms":until,"quantity":{"unit":unit,"value":{"coefficient":c,"decimal_places":dp}},"row_index":out.len()}));
            }
        }
        "command_counts" => {
            let mut counts = BTreeMap::new();
            for a in array(&snap, "attempts")? {
                let time = num(&a["dispatch_decision"], "decision_host_ms")?;
                if time >= from && time < until {
                    let state = text(a, "state")?;
                    let state = if matches!(state, "business_effect_committed" | "output_wait") {
                        "device_completed"
                    } else {
                        state
                    };
                    if state != "reserved" {
                        *counts.entry(state).or_insert(0_i64) += 1;
                    }
                }
            }
            for (state, count) in counts {
                out.push(json!({"kind":"command_count","identity":s.identity,"execution_revision":rev,"state":state,"count":count,"row_index":out.len()}));
            }
        }
        "waiting_time" => {
            // WorkTiming contains duration facts, not a waiting interval. Unknown waiting duration remains null.
            for w in array(&snap, "logical_work")? {
                if matches!(
                    w["state"].as_str(),
                    Some("queued" | "output_wait" | "reconciliation_required")
                ) {
                    let d = &w["latest_dispatch_decision"];
                    if let Some(at) = d["decision_host_ms"].as_i64() {
                        if at < from || at >= until {
                            continue;
                        }
                    }
                    let mut missing = array(&snap["coverage"], "data_quality_issues")?.clone();
                    let jobs: Vec<_> = array(&snap, "bound_jobs")?
                        .iter()
                        .filter(|j| j["logical_job_id"] == w["logical_job_id"])
                        .map(|j| &j["job_id"])
                        .collect();
                    for timing in array(&snap, "timings")? {
                        if jobs.contains(&&timing["job_id"]) {
                            for issue in array(timing, "missing_conditions")? {
                                if !missing.contains(issue) {
                                    missing.push(issue.clone());
                                }
                            }
                        }
                    }
                    out.push(json!({"kind":"waiting_time","identity":s.identity,"execution_revision":rev,"logical_job_id":w["logical_job_id"],"reason":w["state"],"known_host_ms":null,"missing_conditions":missing,"row_index":out.len()}));
                }
            }
        }
        _ => return Err(Fault::invalid("invalid execution aggregation")),
    }
    let all = out;
    let rows: Vec<_> = all
        .iter()
        .filter(|x| x["row_index"].as_i64().unwrap() > after)
        .take(limit as usize)
        .cloned()
        .collect();
    let last = rows.last().and_then(|x| x["row_index"].as_i64());
    let more = last.is_some_and(|x| x + 1 < all.len() as i64);
    let next = if more {
        Some(encode(
            &json!({"cursor_version":"1","identity":s.identity,"execution_revision":rev,"query_selection":selection,"after_row_index":last}),
        ))
    } else {
        None
    };
    success(
        "query_result",
        json!({"dataset":"execution","contract_version":CONTRACT,"selected_epoch":s.identity["execution_epoch_id"],"selected_execution_revision":rev,"rows":rows,"next_cursor":next,"data_quality_issues":snap["coverage"]["data_quality_issues"]}),
    )
}
pub(super) async fn receipts(
    t: &Transaction<'_>,
    p: &Principal,
    cid: &str,
    raw: &str,
) -> Result<Value> {
    p.role(&[Role::HostGateway, Role::ScopedReader])?;
    let params = get_query(raw)?;
    let v: Value = if let Some(q) = params.get("query") {
        serde_json::from_str(q).map_err(|_| Fault::invalid("invalid receipt query"))?
    } else {
        let mut obj = serde_json::Map::new();
        for (k, x) in params {
            obj.insert(
                k.clone(),
                if k == "identity" || k == "page" {
                    serde_json::from_str(&x)
                        .map_err(|_| Fault::invalid("invalid structured query field"))?
                } else {
                    json!(x)
                },
            );
        }
        Value::Object(obj)
    };
    let v = typed::<CommandReceiptsQuery>(v)?;
    if v["command_id"] != cid {
        return Err(Fault::invalid("command path differs"));
    }
    let sid = text(&v["identity"], "session_id")?;
    let s = session(t, p, sid, false).await?;
    identity(&s, &v["identity"])?;
    let r = t
        .query_opt(
            "SELECT 1 FROM commands WHERE command_id=$1 AND session_id=$2",
            &[&cid, &sid],
        )
        .await?;
    if r.is_none() {
        return Err(Fault::missing());
    }
    let d = t
        .query_one(
            "SELECT dataset_id,visible_frontier FROM datasets WHERE session_id=$1",
            &[&sid],
        )
        .await?;
    let ds: String = d.get(0);
    let current: i64 = d.get(1);
    let (limit, cursor) = page(&v)?;
    let selection = normalized_query(&v);
    let (f, after) = if let Some(c) = &cursor {
        typed::<CommandReceiptsCursor>(c.clone())?;
        if c["query_selection"] != selection || c["dataset_id"] != ds {
            return Err(Fault::invalid("cursor query differs"));
        }
        (num(c, "visible_frontier")?, num(c, "after_ingress_id")?)
    } else {
        (current, 0)
    };
    if f > current || after > f {
        return Err(Fault::invalid("invalid committed receipt prefix"));
    }
    let rs=t.query("SELECT * FROM device_events WHERE dataset_id=$1 AND command_id=$2 AND ingress_id>$3 AND ingress_id<=$4 AND event_kind='receipt' ORDER BY ingress_id LIMIT $5",&[&ds,&cid,&after,&f,&(limit+1)]).await?;
    let rows: Vec<_> = rs.iter().take(limit as usize).map(event_row).collect();
    let next = if rs.len() > limit as usize {
        Some(encode(
            &json!({"kind":"command_receipts","cursor_version":1,"query_selection":selection,"dataset_id":ds,"visible_frontier":f,"after_ingress_id":rows.last().unwrap()["ingress_id"]}),
        ))
    } else {
        None
    };
    success(
        "command_receipts",
        json!({"identity":s.identity,"command_id":cid,"dataset_id":ds,"visible_frontier":f,"rows":rows,"next_cursor":next}),
    )
}
pub(super) async fn observations(
    t: &Transaction<'_>,
    p: &Principal,
    sid: &str,
    mut v: Value,
    history: bool,
) -> Result<Value> {
    let s = session(t, p, sid, false).await?;
    identity(&s, &v["identity"])?;
    let mut objects = array(&v, "objects")?.clone();
    objects.sort_by_key(|x| {
        (
            object_kind_rank(x["kind"].as_str().unwrap()),
            x["id"].as_str().unwrap().to_owned(),
        )
    });
    objects.dedup();
    v["objects"] = json!(objects);
    let snap = snapshot(
        t,
        sid,
        &json!({"kind":"revision","execution_revision":v["execution_revision"]}),
    )
    .await?;
    let f = num(&snap, "observation_frontier")?;
    let until = num(&snap["coverage"], "observed_until_host_ms")?;
    if history {
        if num(&v, "host_from_ms")? > num(&v, "host_until_ms")? || num(&v, "host_until_ms")? > until
        {
            return Err(Fault::invalid("history range exceeds selected snapshot"));
        }
    } else if num(&v, "at_host_ms")? > until {
        return Err(Fault::invalid("observation time exceeds selected snapshot"));
    }
    let (limit, cursor) = page(&v)?;
    let selection = normalized_query(&v);
    let mut after = -1;
    if let Some(c) = &cursor {
        if history {
            typed::<ExecutionHistoryCursor>(c.clone())?;
            after = num(c, "after_record_sequence")?;
        } else {
            typed::<ExecutionObservationCursor>(c.clone())?;
            after = num(c, "after_row_index")?;
        }
        if c["query_selection"] != selection || num(c, "observation_frontier")? != f {
            return Err(Fault::invalid(
                "cursor differs from fixed observation selection",
            ));
        }
    }
    let states=t.query("SELECT typed_state FROM execution_observations WHERE session_id=$1 AND record_sequence<=$2 ORDER BY record_sequence",&[&sid,&f]).await?;
    let selected = |o: &Value| -> Result<bool> {
        let id = object_id(o)?;
        Ok(objects.is_empty()
            || objects
                .iter()
                .any(|x| x["kind"] == o["kind"] && x["id"] == id))
    };
    if history {
        let lo = num(&v, "host_from_ms")?;
        let hi = num(&v, "host_until_ms")?;
        let mut all = Vec::new();
        for row in states {
            let rec: Value = row.get(0);
            if num(&rec, "at_host_ms")? >= lo
                && num(&rec, "at_host_ms")? <= hi
                && selected(&rec["object"])?
                && num(&rec, "record_sequence")? > after
            {
                all.push(rec);
            }
        }
        let more = all.len() > limit as usize;
        let records: Vec<_> = all.into_iter().take(limit as usize).collect();
        let next = if more {
            Some(encode(
                &json!({"kind":"execution_history","cursor_version":1,"query_selection":selection,"observation_frontier":f,"after_record_sequence":records.last().unwrap()["record_sequence"]}),
            ))
        } else {
            None
        };
        success(
            "history",
            json!({"identity":s.identity,"execution_revision":v["execution_revision"],"observation_frontier":f,"host_from_ms":lo,"host_until_ms":hi,"records":records,"next_cursor":next}),
        )
    } else {
        let at = num(&v, "at_host_ms")?;
        let mut latest = BTreeMap::new();
        for row in states {
            let rec: Value = row.get(0);
            if num(&rec, "at_host_ms")? <= at && selected(&rec["object"])? {
                let o = &rec["object"];
                latest.insert((object_kind_rank(text(o, "kind")?), object_id(o)?), o.clone());
            }
        }
        let all: Vec<_> = latest.into_values().collect();
        let rows: Vec<_> = all
            .iter()
            .enumerate()
            .filter(|(i, _)| *i as i64 > after)
            .take(limit as usize)
            .map(|(_, x)| x.clone())
            .collect();
        let n = after + rows.len() as i64;
        let next = if n + 1 < all.len() as i64 {
            Some(encode(
                &json!({"kind":"execution_observations","cursor_version":1,"query_selection":selection,"observation_frontier":f,"after_row_index":n}),
            ))
        } else {
            None
        };
        success(
            "observations",
            json!({"identity":s.identity,"execution_revision":v["execution_revision"],"observation_frontier":f,"at_host_ms":at,"clock_started":snap["coverage"]["clock_started"],"objects":rows,"next_cursor":next}),
        )
    }
}
