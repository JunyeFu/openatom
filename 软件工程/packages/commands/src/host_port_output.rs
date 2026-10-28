//! Private UI projection of actual same-Run port output facts.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunPortOutput {
    pub run_id: RunId,
    pub at_ms: SimTimeMs,
    pub port: PortRef,
    pub item_type_ref: Option<SubjectRevisionRef>,
    pub completed_output: Quantity,
    pub accepted_output: Quantity,
}

/// Exact frozen declaration and positions in this immutable Run's actual history.
pub(super) struct PortOutputSource {
    unit: UnitId,
    records: Vec<usize>,
}

impl ProjectHost {
    pub fn read_run_port_output(
        &self,
        session_id: &ObjectId,
        project_id: &ProjectId,
        scope: &ApiScope,
        scope_revision: &Revision,
        run_id: &RunId,
        at_ms: &SimTimeMs,
        port: &PortRef,
    ) -> Result<RunPortOutput, ApiError> {
        let caller = self.sessions.get(session_id).ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "Unknown or revoked session",
                None,
            )
        })?;
        if !matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Port output reads require the actual UI session",
                None,
            ));
        }
        self.admit(
            caller,
            &Some(project_id.clone()),
            scope,
            &OperationName::GetRun,
        )?;
        let revision = match scope {
            ApiScope::Project => &self.project.as_ref().unwrap().revision,
            ApiScope::DefinitionDraft(id) => &self.definition_drafts[id].revision,
            ApiScope::Workspace => {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Port output reads require a Project or DefinitionDraft scope",
                    Some("/scope"),
                ));
            }
        };
        check_revision(scope_revision, revision, "/scope_revision")?;
        if matches!(scope, ApiScope::DefinitionDraft(_)) {
            let RunView::Summary(summary) = self.get_run(
                caller,
                scope,
                GetRunParams {
                    run_id: run_id.clone(),
                    query: RunQuery::Summary,
                },
            )?
            else {
                unreachable!()
            };
            if at_ms.0.value() < summary.window.start_ms.0.value()
                || summary
                    .computed_until_ms
                    .as_ref()
                    .is_none_or(|frontier| at_ms.0.value() > frontier.0.value())
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Requested time is outside the computed Run frontier",
                    Some("/at_ms"),
                ));
            }
            return Err(error(
                ErrorCode::UnsupportedCapability,
                "DefinitionCycle output records have no formal Project device instance port; this label is not applicable",
                Some("/port"),
            ));
        }
        let summary = self.frozen_run_summary(caller, scope, run_id, at_ms)?;
        self.stored_run_port_output(&summary, at_ms, port)
    }

    /// Shared exact producer after either the original UI gate or RenderView's
    /// own operation/Scope admission. No caller impersonation or extra grant.
    pub(super) fn stored_run_port_output(
        &self,
        summary: &RunSummary,
        at_ms: &SimTimeMs,
        port: &PortRef,
    ) -> Result<RunPortOutput, ApiError> {
        let run_id = &summary.run_id;
        let run = &self.runs[run_id];
        let key = serde_json::to_string(port).unwrap();
        let mut identities = run.observation_cache.lock().unwrap();
        if !identities.port_sources.contains_key(&key) {
            let project = self.frozen_run_project(summary)?;
            let device=project.devices.iter().find(|device|device.id==port.instance_id&&module_key(&device.module_ref)==module_key(&port.module_ref)).ok_or_else(||error(ErrorCode::MissingReference,"Port instance and exact module version are absent from this Run's frozen Project",Some("/port")))?;
            let package = self.frozen_run_module(summary, &device.module_ref)?;
            let ModulePayload::Devices(definition) = &package.definition.payload else {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Port must belong to a device definition",
                    Some("/port"),
                ));
            };
            let material = definition
                .ports
                .iter()
                .find(|p| {
                    p.id == port.port_id
                        && matches!(p.direction, Direction::Output | Direction::Bidirectional)
                })
                .and_then(|p| match &p.payload {
                    PortPayload::Material(m) => Some(m),
                    _ => None,
                })
                .ok_or_else(|| {
                    error(
                        ErrorCode::InvalidInput,
                        "Port must be a legal frozen material output",
                        Some("/port/port_id"),
                    )
                })?;
            let records=run.history.as_ref().and_then(|history|history.port_outputs.as_deref()).ok_or_else(||error(ErrorCode::UnsupportedCapability,"This Run has no actual port output history; final stock and Job states cannot supply the missing evidence",Some("/run_id")))?;
            let positions = records.iter().enumerate().filter(|(_, row)| row.3 == port.port_id
                && matches!(&row.2, JobExecutionRef::ProjectDevice { device_id, .. } if device_id == &port.instance_id))
                .map(|(offset, _)| offset).collect();
            identities.port_sources.insert(key.clone(), PortOutputSource { unit: material.unit.clone(), records: positions });
        }
        let source = &identities.port_sources[&key];
        let unit = source.unit.clone();
        let records = run.history.as_ref().unwrap().port_outputs.as_ref().unwrap();
        let selected: Vec<_> = source.records.iter().map(|offset| &records[*offset])
            .filter(|row| row.0.0.value() <= at_ms.0.value()).collect();
        let mut item_type = None;
        for row in &selected {
            if row.0.0.value() < run.result.window.start_ms.0.value()
                || !identities.matches_job_identity(&run.result.jobs, &row.1, &row.2)
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Port output record is outside its actual Run Job identity/window",
                    Some("/run_id"),
                ));
            }
            if row.5.unit != unit
                || item_type.as_ref().is_some_and(|t| !exact_data(t, &row.4))
            {
                return Err(error(
                    ErrorCode::UnsupportedCapability,
                    "This port has incompatible actual item types or units; a single quantity cannot combine them",
                    Some("/port"),
                ));
            }
            item_type = Some(row.4.clone());
        }
        let total = |transferred: bool| -> Result<Quantity, ApiError> {
            let rows: Vec<_> = selected.iter().filter(|r| r.6 == transferred).collect();
            let mut scale = rows
                .iter()
                .map(|r| r.5.value.decimal_places.value())
                .max()
                .unwrap_or(0);
            let mut sum: u128 = rows
                .iter()
                .map(|r| {
                    u128::from(r.5.value.coefficient.value())
                        * 10u128.pow(u32::from(scale - r.5.value.decimal_places.value()))
                })
                .sum();
            while scale > 0 && sum % 10 == 0 {
                sum /= 10;
                scale -= 1;
            }
            let coefficient = u64::try_from(sum)
                .ok()
                .and_then(|n| SafeUint::try_from(n).ok())
                .ok_or_else(|| {
                    error(
                        ErrorCode::UnsupportedCapability,
                        "Exact port quantity exceeds canonical numeric range; it cannot be rounded",
                        Some("/port"),
                    )
                })?;
            Ok(Quantity {
                value: DecimalAmount {
                    coefficient,
                    decimal_places: scale.try_into().unwrap(),
                },
                unit: unit.clone(),
            })
        };
        Ok(RunPortOutput {
            run_id: run_id.clone(),
            at_ms: at_ms.clone(),
            port: port.clone(),
            item_type_ref: item_type,
            completed_output: total(false)?,
            accepted_output: total(true)?,
        })
    }
}
