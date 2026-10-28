//! Bounded CSV + typed JSONL producer for H11. Directory authorization and publication
//! belong to the Host caller; this module accepts no path, credentials or HTTP client.
use serde::Serialize;
use serde_json::{Value, json};
use sf_domain::cloud_execution::*;
use sf_domain::{Issue, ObjectId, ProjectId};
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// Frozen UI query/selection and actual C12 provenance. UI pagination is discarded.
pub struct CloudDataExportInput {
    pub export_id: ObjectId,
    pub project_id: ProjectId,
    pub query: DaaSQuery,
    pub selection: CloudDataExportSelection,
    pub dataset_descriptor: Option<DatasetDescriptor>,
    pub generated_at_utc: CloudTimestamp,
}

/// Counts of complete pages accepted by both writers; issues count occurrences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloudDataExportCounts {
    pub page_count: u64,
    pub row_count: u64,
    pub quality_issue_occurrence_count: u64,
}

/// Read and write one page at a time. `progress` runs after both page streams flush,
/// reporting total written rows (including zero for an empty result).
///
/// The caller retains a failed/cancelled stage and publishes only after `Ok`, handle
/// close and its final cancellation/authorization check. Footer I/O failure is an
/// error even when a writer retained some or all uncommitted trailing bytes.
pub fn write_cloud_data_export<F, P>(
    input: &CloudDataExportInput,
    csv_output: &mut dyn Write,
    jsonl_output: &mut dyn Write,
    cancelled: &AtomicBool,
    mut progress: P,
    mut read_page: F,
) -> Result<CloudDataExportCounts, Issue>
where
    F: FnMut(&DaaSQuery) -> Result<DaaSQueryResult, Issue>,
    P: FnMut(u64),
{
    check_cancel(cancelled)?;
    let mut plan = Plan::new(input)?;
    let columns = columns(plan.kind);
    let header: CloudDataExportRecord = from_json(json!({
        "record_kind":"header", "contract_version":"sf.execution/2.0-draft11",
        "export_id":input.export_id, "project_id":input.project_id,
        "query":plan.canonical, "selection":input.selection,
        "generated_at_utc":input.generated_at_utc, "csv_columns":columns,
        "csv_cell_encoding":"json_value", "dataset_descriptor":input.dataset_descriptor
    }))?;
    let mut csv = csv::WriterBuilder::new().from_writer(csv_output);
    let mut jsonl = BufWriter::with_capacity(64 * 1024, jsonl_output);
    csv.write_record(columns).map_err(csv_error)?;
    record(&mut jsonl, &header)?;
    let mut counts = CloudDataExportCounts {
        page_count: 0,
        row_count: 0,
        quality_issue_occurrence_count: 0,
    };
    loop {
        check_cancel(cancelled)?;
        let result = read_page(&plan.request)?;
        check_cancel(cancelled)?;
        let (rows, issues, next) = plan.validate(&result, counts.row_count)?;
        let pages = add(counts.page_count, 1)?;
        let total = add(counts.row_count, rows as u64)?;
        let quality = add(counts.quality_issue_occurrence_count, issues as u64)?;
        match &result {
            DaaSQueryResult::Telemetry { rows, .. } => {
                for (i, row) in rows.iter().enumerate() {
                    check_cancel(cancelled)?;
                    write_cells(
                        &mut csv,
                        telemetry_cells(row, counts.row_count + i as u64, &plan.source_mode)?,
                    )?;
                }
            }
            DaaSQueryResult::Execution { rows, .. } => {
                for (i, row) in rows.iter().enumerate() {
                    check_cancel(cancelled)?;
                    write_cells(&mut csv, execution_cells(row, counts.row_count + i as u64)?)?;
                }
            }
        }
        check_cancel(cancelled)?;
        let page = CloudDataExportRecord::Page {
            export_id: input.export_id.clone(),
            page_index: from_json(json!(pages))?,
            result,
        };
        record(&mut jsonl, &page)?;
        csv.flush().map_err(io_error)?;
        jsonl.flush().map_err(io_error)?;
        counts = CloudDataExportCounts {
            page_count: pages,
            row_count: total,
            quality_issue_occurrence_count: quality,
        };
        progress(counts.row_count);
        check_cancel(cancelled)?;
        if let Some(cursor) = next {
            plan.set_cursor(cursor)?;
        } else {
            break;
        }
    }
    check_cancel(cancelled)?;
    let footer: CloudDataExportRecord =
        from_json(json!({"record_kind":"footer","export_id":input.export_id,
        "page_count":counts.page_count,"row_count":counts.row_count,
        "quality_issue_occurrence_count":counts.quality_issue_occurrence_count,"complete":true}))?;
    record(&mut jsonl, &footer)?;
    jsonl.flush().map_err(io_error)?;
    Ok(counts)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    TelemetryRows,
    Minute,
    ExecutionRows,
    Quantity,
    Commands,
    Waiting,
}
struct Plan {
    kind: Kind,
    canonical: Value,
    selection: Value,
    request: DaaSQuery,
    source_mode: Value,
    descriptor: Option<Value>,
    last_ingress: u64,
    last_group: Option<(CloudTimestamp, String, String)>,
}
impl Plan {
    fn new(input: &CloudDataExportInput) -> Result<Self, Issue> {
        let mut q = value(&input.query)?;
        let selection = value(&input.selection)?;
        q["page"] = json!({"limit":200,"cursor":null});
        let descriptor = input.dataset_descriptor.as_ref().map(value).transpose()?;
        let (kind, source_mode) = match (&input.query, &input.selection) {
            (
                DaaSQuery::Telemetry {
                    cloud_project_id,
                    dataset_id,
                    device_ids,
                    received_from,
                    received_until,
                    aggregation,
                    ..
                },
                CloudDataExportSelection::Telemetry {
                    cloud_project_id: cp,
                    dataset_id: ds,
                    selected_visible_frontier,
                    selected_device_ids,
                },
            ) => {
                if value(cloud_project_id)? != value(cp)?
                    || value(dataset_id)? != value(ds)?
                    || received_from.as_datetime() > received_until.as_datetime()
                {
                    return Err(invalid("query and displayed telemetry selection differ"));
                }
                let selected: Vec<String> = selected_device_ids
                    .iter()
                    .map(|v| v.as_str().to_owned())
                    .collect();
                if selected.windows(2).any(|v| v[0] >= v[1]) {
                    return Err(invalid("selected device IDs must be sorted and unique"));
                }
                let mut original: Vec<String> =
                    device_ids.iter().map(|v| v.as_str().to_owned()).collect();
                original.sort();
                original.dedup();
                if !original.is_empty() && original != selected {
                    return Err(invalid(
                        "displayed devices differ from original explicit filter",
                    ));
                }
                let d = descriptor
                    .as_ref()
                    .ok_or_else(|| invalid("telemetry requires actual C12 dataset descriptor"))?;
                if d["dataset_id"] != value(ds)?
                    || d["cloud_project_id"] != value(cp)?
                    || d["visible_frontier"].as_u64().unwrap() < selected_visible_frontier.value()
                {
                    return Err(invalid(
                        "C12 descriptor does not cover selected dataset/frontier",
                    ));
                }
                let mode = d["source_mode"].as_str().unwrap();
                let purpose = d["purpose"].as_str().unwrap();
                if (mode == "generated_load") != (purpose == "generated_load")
                    || (purpose == "execution") != (!d["session_id"].is_null())
                {
                    return Err(invalid(
                        "dataset source/purpose/session combination differs",
                    ));
                }
                q["device_ids"] = value(selected_device_ids)?;
                let kind = match aggregation {
                    DaaSQueryTelemetryAggregation::Rows => Kind::TelemetryRows,
                    DaaSQueryTelemetryAggregation::CountByDeviceMinute => Kind::Minute,
                };
                (kind, d["source_mode"].clone())
            }
            (
                DaaSQuery::Execution {
                    identity,
                    selection: original,
                    host_from_ms,
                    host_until_ms,
                    aggregation,
                    ..
                },
                CloudDataExportSelection::Execution {
                    identity: pin,
                    execution_revision,
                },
            ) => {
                if value(identity)? != value(pin)?
                    || value(&identity.project_id)? != value(&input.project_id)?
                    || host_from_ms.value() > host_until_ms.value()
                    || descriptor.is_some()
                {
                    return Err(invalid("execution identity, source or range differs"));
                }
                if let SnapshotSelection::Revision {
                    execution_revision: r,
                } = original
                {
                    if r.value() != execution_revision.value() {
                        return Err(invalid("explicit revision differs from displayed revision"));
                    }
                }
                q["selection"] = json!({"kind":"revision","execution_revision":execution_revision});
                let kind = match aggregation {
                    DaaSQueryExecutionAggregation::Rows => Kind::ExecutionRows,
                    DaaSQueryExecutionAggregation::AcceptedQuantity => Kind::Quantity,
                    DaaSQueryExecutionAggregation::CommandCounts => Kind::Commands,
                    DaaSQueryExecutionAggregation::WaitingTime => Kind::Waiting,
                };
                (kind, Value::Null)
            }
            _ => return Err(invalid("query and selection dataset differ")),
        };
        let mut request = from_json::<DaaSQuery>(q.clone())?;
        if matches!(kind, Kind::TelemetryRows | Kind::Minute) {
            let mut cursor = json!({"kind":if kind==Kind::Minute{"telemetry_aggregate"}else{"telemetry_rows"},"cursor_version":"1","dataset_id":selection["dataset_id"],"query_selection":q,"visible_frontier":selection["selected_visible_frontier"]});
            cursor[if kind == Kind::Minute {
                "after_row_index"
            } else {
                "after_ingress_id"
            }] = if kind == Kind::Minute {
                Value::Null
            } else {
                json!(0)
            };
            let cursor: TelemetryCursor = from_json(cursor)?;
            let mut first = q.clone();
            first["page"]["cursor"] = json!(base64_encode(
                &serde_json::to_vec(&cursor).map_err(json_error)?
            ));
            request = from_json(first)?;
        }
        Ok(Self {
            kind,
            canonical: q,
            selection,
            request,
            source_mode,
            descriptor,
            last_ingress: 0,
            last_group: None,
        })
    }
    fn set_cursor(&mut self, cursor: String) -> Result<(), Issue> {
        let mut q = self.canonical.clone();
        q["page"]["cursor"] = json!(cursor);
        self.request = from_json(q)?;
        Ok(())
    }
    fn validate(
        &mut self,
        result: &DaaSQueryResult,
        already: u64,
    ) -> Result<(usize, usize, Option<String>), Issue> {
        let (count, quality, next) = match result {
            DaaSQueryResult::Telemetry {
                rows,
                next_cursor,
                data_quality_issues,
                dataset_id,
                selected_visible_frontier,
                selected_device_ids,
                ..
            } => {
                if rows.len() > 200 {
                    return Err(invalid("page exceeds200 rows"));
                }
                if !matches!(self.kind, Kind::TelemetryRows | Kind::Minute)
                    || value(dataset_id)? != self.selection["dataset_id"]
                    || value(selected_visible_frontier)?
                        != self.selection["selected_visible_frontier"]
                    || value(selected_device_ids)? != self.selection["selected_device_ids"]
                {
                    return Err(invalid(
                        "page changed dataset, fixed frontier or device membership",
                    ));
                }
                for (i, row) in rows.iter().enumerate() {
                    self.telemetry_row(row, already + i as u64)?;
                }
                (
                    rows.len(),
                    data_quality_issues.len(),
                    next_cursor.as_ref().map(|v| v.as_str().to_owned()),
                )
            }
            DaaSQueryResult::Execution {
                rows,
                next_cursor,
                data_quality_issues,
                selected_epoch,
                selected_execution_revision,
                ..
            } => {
                if rows.len() > 200 {
                    return Err(invalid("page exceeds200 rows"));
                }
                if matches!(self.kind, Kind::TelemetryRows | Kind::Minute)
                    || value(selected_epoch)? != self.selection["identity"]["execution_epoch_id"]
                    || value(selected_execution_revision)? != self.selection["execution_revision"]
                {
                    return Err(invalid("page changed execution epoch/revision"));
                }
                for (i, row) in rows.iter().enumerate() {
                    self.execution_row(row, already + i as u64)?;
                }
                (
                    rows.len(),
                    data_quality_issues.len(),
                    next_cursor.as_ref().map(|v| v.as_str().to_owned()),
                )
            }
        };
        if let Some(next) = &next {
            if count == 0 {
                return Err(invalid("nonterminal page made no row progress"));
            }
            let cursor_json: Value = serde_json::from_slice(&base64_decode(next)?)
                .map_err(|_| invalid("invalid next cursor JSON"))?;
            if matches!(self.kind, Kind::TelemetryRows | Kind::Minute) {
                let _: TelemetryCursor = from_json(cursor_json.clone())?;
                if cursor_json["dataset_id"] != self.selection["dataset_id"]
                    || cursor_json["visible_frontier"]
                        != self.selection["selected_visible_frontier"]
                {
                    return Err(invalid("cursor changed telemetry source/frontier"));
                }
                let (kind, key, position) = if self.kind == Kind::Minute {
                    (
                        "telemetry_aggregate",
                        "after_row_index",
                        already + count as u64 - 1,
                    )
                } else {
                    ("telemetry_rows", "after_ingress_id", self.last_ingress)
                };
                if cursor_json["kind"] != kind || cursor_json[key] != json!(position) {
                    return Err(invalid("cursor did not point at actual last row"));
                }
            } else {
                let _: ExecutionCursor = from_json(cursor_json.clone())?;
                if cursor_json["identity"] != self.selection["identity"]
                    || cursor_json["execution_revision"] != self.selection["execution_revision"]
                    || cursor_json["after_row_index"] != json!(already + count as u64 - 1)
                {
                    return Err(invalid("cursor changed execution identity/revision/row"));
                }
            }
            if cursor_json["query_selection"] != self.canonical {
                return Err(invalid("cursor changed canonical filter or page size"));
            }
            if value(&self.request)?["page"]["cursor"] == json!(next) {
                return Err(invalid("cursor repeated without progress"));
            }
        }
        Ok((count, quality, next))
    }
    fn telemetry_row(
        &mut self,
        row: &DaaSQueryResultTelemetryRowsItem,
        index: u64,
    ) -> Result<(), Issue> {
        use DaaSQueryResultTelemetryRowsItem::*;
        match row {
            Telemetry {
                ingress_id,
                dataset_id,
                cloud_received_at,
                observation,
            } if self.kind == Kind::TelemetryRows => {
                if self.source_mode == "generated_load" {
                    return Err(invalid("telemetry row in generated source"));
                }
                let obs = value(observation)?;
                self.raw(
                    ingress_id.value(),
                    &value(dataset_id)?,
                    cloud_received_at,
                    &obs["device_id"],
                )?;
                let d = self.descriptor.as_ref().unwrap();
                if (d["purpose"] == "execution"
                    && (obs["source_purpose"] != "execution"
                        || obs["session_id"] != d["session_id"]))
                    || (d["purpose"] == "telemetry_only"
                        && obs["source_purpose"] != "telemetry_only")
                {
                    return Err(invalid("telemetry purpose/session differs from C12 source"));
                }
            }
            GeneratedLoad {
                ingress_id,
                dataset_id,
                cloud_received_at,
                event,
            } if self.kind == Kind::TelemetryRows => {
                if self.source_mode != "generated_load" {
                    return Err(invalid("generated row in live source"));
                }
                self.raw(
                    ingress_id.value(),
                    &value(dataset_id)?,
                    cloud_received_at,
                    &value(&event.device_ref)?,
                )?;
            }
            CountByDeviceMinute {
                device_id,
                source_mode,
                bucket_start,
                row_index,
                ..
            } if self.kind == Kind::Minute => {
                if row_index.value() != index
                    || value(source_mode)? != self.source_mode
                    || !self.member(&value(device_id)?)
                {
                    return Err(invalid("aggregate row index/device/source differs"));
                }
                let key = (
                    bucket_start.clone(),
                    device_id.as_str().to_owned(),
                    self.source_mode.as_str().unwrap().to_owned(),
                );
                if let Some(previous) = &self.last_group {
                    if (previous.0.as_datetime(), &previous.1, &previous.2)
                        >= (key.0.as_datetime(), &key.1, &key.2)
                    {
                        return Err(invalid("aggregate group order repeated or regressed"));
                    }
                }
                self.last_group = Some(key);
            }
            _ => return Err(invalid("row type differs from telemetry aggregation")),
        }
        Ok(())
    }
    fn member(&self, id: &Value) -> bool {
        self.selection["selected_device_ids"]
            .as_array()
            .unwrap()
            .contains(id)
    }
    fn raw(
        &mut self,
        ingress: u64,
        dataset: &Value,
        received: &CloudTimestamp,
        device: &Value,
    ) -> Result<(), Issue> {
        if ingress <= self.last_ingress
            || ingress
                > self.selection["selected_visible_frontier"]
                    .as_u64()
                    .unwrap()
            || *dataset != self.selection["dataset_id"]
            || !self.member(device)
        {
            return Err(invalid(
                "raw row source, ingress order or selected device differs",
            ));
        }
        if let DaaSQuery::Telemetry {
            received_from,
            received_until,
            ..
        } = &self.request
        {
            if received.as_datetime() < received_from.as_datetime()
                || received.as_datetime() >= received_until.as_datetime()
            {
                return Err(invalid("raw row outside original UTC interval"));
            }
        }
        self.last_ingress = ingress;
        Ok(())
    }
    fn execution_row(
        &self,
        row: &DaaSQueryResultExecutionRowsItem,
        index: u64,
    ) -> Result<(), Issue> {
        use DaaSQueryResultExecutionRowsItem::*;
        let (id, rev, row_index) = match row {
            ExecutionAttempt {
                identity,
                execution_revision,
                row_index,
                ..
            }
            | Acceptance {
                identity,
                execution_revision,
                row_index,
                ..
            } if self.kind == Kind::ExecutionRows => (identity, execution_revision, row_index),
            AcceptedQuantity {
                identity,
                execution_revision,
                row_index,
                host_from_ms,
                host_until_ms,
                ..
            } if self.kind == Kind::Quantity => {
                if value(host_from_ms)? != self.canonical["host_from_ms"]
                    || value(host_until_ms)? != self.canonical["host_until_ms"]
                {
                    return Err(invalid("quantity row changed Host interval"));
                }
                (identity, execution_revision, row_index)
            }
            CommandCount {
                identity,
                execution_revision,
                row_index,
                ..
            } if self.kind == Kind::Commands => (identity, execution_revision, row_index),
            WaitingTime {
                identity,
                execution_revision,
                row_index,
                ..
            } if self.kind == Kind::Waiting => (identity, execution_revision, row_index),
            _ => return Err(invalid("row type differs from execution aggregation")),
        };
        if value(id)? != self.selection["identity"]
            || value(rev)? != self.selection["execution_revision"]
            || row_index.value() != index
        {
            return Err(invalid(
                "execution row identity/revision or contiguous index differs",
            ));
        }
        Ok(())
    }
}

fn columns(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::TelemetryRows => &[
            "export_row_index",
            "row_kind",
            "ingress_id",
            "dataset_id",
            "cloud_received_at_utc",
            "device_ref",
            "device_sequence",
            "source_mode",
            "payload_json",
        ],
        Kind::Minute => &[
            "export_row_index",
            "row_kind",
            "device_id",
            "source_mode",
            "bucket_start_utc",
            "count",
        ],
        Kind::ExecutionRows => &[
            "export_row_index",
            "row_kind",
            "session_id",
            "execution_epoch_id",
            "execution_revision",
            "source_row_index",
            "payload_json",
        ],
        Kind::Quantity => &[
            "export_row_index",
            "row_kind",
            "session_id",
            "execution_epoch_id",
            "execution_revision",
            "host_from_ms",
            "host_until_ms",
            "quantity_coefficient",
            "quantity_decimal_places",
            "quantity_unit",
        ],
        Kind::Commands => &[
            "export_row_index",
            "row_kind",
            "session_id",
            "execution_epoch_id",
            "execution_revision",
            "state",
            "count",
        ],
        Kind::Waiting => &[
            "export_row_index",
            "row_kind",
            "session_id",
            "execution_epoch_id",
            "execution_revision",
            "logical_job_id",
            "reason",
            "known_host_ms",
            "missing_conditions_json",
        ],
    }
}
fn telemetry_cells(
    row: &DaaSQueryResultTelemetryRowsItem,
    index: u64,
    mode: &Value,
) -> Result<Vec<Value>, Issue> {
    let v = value(row)?;
    match row {
        DaaSQueryResultTelemetryRowsItem::CountByDeviceMinute { .. } => Ok(vec![
            json!(index),
            v["kind"].clone(),
            v["device_id"].clone(),
            v["source_mode"].clone(),
            v["bucket_start"].clone(),
            v["count"].clone(),
        ]),
        _ => {
            let p = if v["kind"] == "generated_load" {
                &v["event"]
            } else {
                &v["observation"]
            };
            let device = if v["kind"] == "generated_load" {
                &p["device_ref"]
            } else {
                &p["device_id"]
            };
            Ok(vec![
                json!(index),
                v["kind"].clone(),
                v["ingress_id"].clone(),
                v["dataset_id"].clone(),
                v["cloud_received_at"].clone(),
                device.clone(),
                p["device_sequence"].clone(),
                mode.clone(),
                v.clone(),
            ])
        }
    }
}
fn execution_cells(
    row: &DaaSQueryResultExecutionRowsItem,
    index: u64,
) -> Result<Vec<Value>, Issue> {
    let v = value(row)?;
    let mut c = vec![
        json!(index),
        v["kind"].clone(),
        v["identity"]["session_id"].clone(),
        v["identity"]["execution_epoch_id"].clone(),
        v["execution_revision"].clone(),
    ];
    match row {
        DaaSQueryResultExecutionRowsItem::ExecutionAttempt { .. }
        | DaaSQueryResultExecutionRowsItem::Acceptance { .. } => {
            c.extend([v["row_index"].clone(), v.clone()])
        }
        DaaSQueryResultExecutionRowsItem::AcceptedQuantity { .. } => c.extend([
            v["host_from_ms"].clone(),
            v["host_until_ms"].clone(),
            v["quantity"]["value"]["coefficient"].clone(),
            v["quantity"]["value"]["decimal_places"].clone(),
            v["quantity"]["unit"].clone(),
        ]),
        DaaSQueryResultExecutionRowsItem::CommandCount { .. } => {
            c.extend([v["state"].clone(), v["count"].clone()])
        }
        DaaSQueryResultExecutionRowsItem::WaitingTime { .. } => c.extend([
            v["logical_job_id"].clone(),
            v["reason"].clone(),
            v["known_host_ms"].clone(),
            v["missing_conditions"].clone(),
        ]),
    }
    Ok(c)
}
fn write_cells(csv: &mut csv::Writer<&mut dyn Write>, cells: Vec<Value>) -> Result<(), Issue> {
    for cell in cells {
        csv.write_field(serde_json::to_string(&cell).map_err(json_error)?)
            .map_err(csv_error)?;
    }
    csv.write_record(std::iter::empty::<&str>())
        .map_err(csv_error)
}
fn record(writer: &mut dyn Write, record: &CloudDataExportRecord) -> Result<(), Issue> {
    serde_json::to_writer(&mut *writer, record).map_err(json_error)?;
    writer.write_all(b"\n").map_err(io_error)
}
fn value<T: Serialize>(v: &T) -> Result<Value, Issue> {
    serde_json::to_value(v).map_err(json_error)
}
fn from_json<T: serde::de::DeserializeOwned>(v: Value) -> Result<T, Issue> {
    serde_json::from_value(v).map_err(json_error)
}
fn add(a: u64, b: u64) -> Result<u64, Issue> {
    a.checked_add(b)
        .filter(|n| *n <= 9_007_199_254_740_991)
        .ok_or_else(|| invalid("export count exceeds wire integer range"))
}
fn failure(code: &str, message: impl Into<String>) -> Issue {
    Issue {
        code: code.into(),
        object_ref: None,
        field_path: None,
        message: message.into(),
        blocks: vec![],
    }
}
fn invalid(message: impl Into<String>) -> Issue {
    failure("cloud_data_export_invalid_page", message)
}
fn io_error(e: std::io::Error) -> Issue {
    failure("cloud_data_export_io", e.to_string())
}
fn csv_error(e: csv::Error) -> Issue {
    failure("cloud_data_export_io", e.to_string())
}
fn json_error(e: serde_json::Error) -> Issue {
    failure(
        if e.is_io() {
            "cloud_data_export_io"
        } else {
            "cloud_data_export_contract"
        },
        e.to_string(),
    )
}
fn check_cancel(flag: &AtomicBool) -> Result<(), Issue> {
    if flag.load(Ordering::Acquire) {
        Err(failure(
            "cloud_data_export_cancelled",
            "export cancelled before completion",
        ))
    } else {
        Ok(())
    }
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity((bytes.len() * 4 + 2) / 3);
    let (mut bits, mut count) = (0u32, 0u8);
    for b in bytes {
        bits = (bits << 8) | u32::from(*b);
        count += 8;
        while count >= 6 {
            count -= 6;
            out.push(BASE64[((bits >> count) & 63) as usize] as char);
        }
    }
    if count > 0 {
        out.push(BASE64[((bits << (6 - count)) & 63) as usize] as char);
    }
    out
}
fn base64_decode(text: &str) -> Result<Vec<u8>, Issue> {
    if text.len() % 4 == 1 {
        return Err(invalid("invalid base64url cursor length"));
    }
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut bits, mut count) = (0u32, 0u8);
    for b in text.bytes() {
        let v = BASE64
            .iter()
            .position(|v| *v == b)
            .ok_or_else(|| invalid("invalid base64url cursor character"))?;
        bits = (bits << 6) | v as u32;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push(((bits >> count) & 255) as u8);
        }
    }
    if count > 0 && (bits & ((1u32 << count) - 1)) != 0 {
        return Err(invalid("noncanonical base64url cursor tail"));
    }
    Ok(out)
}
