//! Original Host owns the Engine; only a Native-held, non-wire lease admits Gateway work.
use super::*;
use crate::execution_projection::ProjectionError;
use crate::execution_session::ExecutionSession;
use sf_domain::cloud_execution::*;
use std::time::Instant;

/// Cannot be constructed from Renderer/Agent JSON, even with a copied identity.
pub struct ExecutionGatewayLease {
    pub(super) host_instance_id: ObjectId,
    pub(super) identity: ExecutionIdentity,
    pub(super) token: Arc<()>,
}

pub(super) struct HostedExecution {
    pub(super) core: ExecutionSession,
    pub(super) gateway_token: Arc<()>,
    pub(super) cleanup_complete: bool,
    pub(super) preparation_ready: bool,
    pub(super) grants: BTreeMap<ObjectId, ExecutionControlGrant>,
}

/// Trusted UI administration grants actions to one already authenticated caller.
/// This is a Rust-only capability, absent from all request DTOs.
pub struct ExecutionControlGrant {
    pub session_controls: Vec<ExecutionReadAccessAllowedSessionControlsItem>,
    pub device_acceptance_control_ids: Vec<ObjectId>,
}

fn projection_failure(e: ProjectionError) -> ApiError {
    error(
        ErrorCode::MissingReference,
        &format!("{}: {}", e.code, e.message),
        None,
    )
}
fn execution_terminal(core: &ExecutionSession) -> bool {
    matches!(
        core.phase,
        ExecutionSnapshotLifecycleState::Completed
            | ExecutionSnapshotLifecycleState::Stopped
            | ExecutionSnapshotLifecycleState::Failed
    )
}
fn uint(value: u64) -> SafeUint {
    value.try_into().expect("Host execution revision")
}


#[derive(Clone, Copy)]
pub(super) enum LocalExecutionSource<'a> {
    Live(&'a HostedExecution),
    Archive(&'a crate::execution_archive::ArchivedExecutionSource),
}
impl LocalExecutionSource<'_> {
    pub(super) fn frozen(&self) -> &ExecutionFrozenInputs {
        match self { Self::Live(v) => &v.core.frozen, Self::Archive(v) => &v.record.frozen_inputs }
    }
    pub(super) fn snapshot(&self, selection: &SnapshotSelection) -> Result<std::borrow::Cow<'_, ExecutionSnapshot>,ApiError> {
        match self {
            Self::Live(v) => v.core.projector.snapshot(selection).map_err(projection_failure),
            Self::Archive(v) => v.reader.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"Unbuilt preparation has no observed execution revision",None))?.snapshot(selection).map_err(projection_failure),
        }
    }
    pub(super) fn observation(&self, query: &ExecutionObservationQuery) -> Result<ExecutionObservationPage,ApiError> {
        match self {
            Self::Live(v) => v.core.projector.query_observations(query).map_err(projection_failure),
            Self::Archive(v) => v.reader.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"Execution observations are unavailable",None))?.query_observations(query).map_err(projection_failure),
        }
    }
    pub(super) fn history(&self, query: &ExecutionHistoryQuery) -> Result<ExecutionHistoryPage,ApiError> {
        match self {
            Self::Live(v) => v.core.projector.query_history(query).map_err(projection_failure),
            Self::Archive(v) => v.reader.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"Execution history is unavailable",None))?.query_history(query).map_err(projection_failure),
        }
    }
    pub(super) fn history_document(&self) -> Result<ExecutionObservationHistoryDocument,ApiError> {
        match self {
            Self::Live(v) => v.core.projector.history_document().map_err(projection_failure),
            Self::Archive(v) => v.record.history.as_deref().cloned().ok_or_else(||error(ErrorCode::MissingReference,"Execution history is unavailable",None)),
        }
    }
    pub(super) fn published_revision(&self) -> Option<SafeUint> {
        match self {
            Self::Live(v) => v.core.cloud_published_revision.map(uint),
            Self::Archive(v) => v.record.published_revisions.iter().max_by_key(|r|r.value()).cloned(),
        }
    }
}

impl ProjectHost {
    pub(super) fn local_execution_source(&self, identity: &ExecutionIdentity) -> Result<LocalExecutionSource<'_>,ApiError> {
        if self.project.as_ref().is_none_or(|p|p.id!=identity.project_id) {
            return Err(error(ErrorCode::PermissionDenied,"Execution is outside the active project",None));
        }
        if let Some(entry)=self.executions.get(&identity.session_id).filter(|entry|exact_data(&entry.core.identity,identity)) {
            return Ok(LocalExecutionSource::Live(entry));
        }
        self.backing.as_ref().and_then(|a|a.execution_sources.get(&crate::execution_archive::key(identity)))
            .map(|v|LocalExecutionSource::Archive(v.as_ref()))
            .ok_or_else(||error(ErrorCode::MissingReference,"Original local execution/epoch is unavailable",None))
    }

    pub(super) fn mark_execution_content_changed(&mut self,identity:&ExecutionIdentity) {
        if self.project.as_ref().is_some_and(|p|p.id==identity.project_id) {
            self.content_generation+=1;
            self.dirty=true;
        }
    }
    fn mutate_gateway<T>(&mut self,lease:&ExecutionGatewayLease,action:impl FnOnce(&mut ExecutionSession)->Result<T,ApiError>)->Result<T,ApiError> {
        let core=self.gateway_core(lease)?;
        let before=(core.revision,core.cloud_published_revision);
        let result=action(core);
        let changed=before!=(core.revision,core.cloud_published_revision);
        if changed {self.mark_execution_content_changed(&lease.identity);}
        result
    }

    /// H01's trusted Native worker supplies frozen inputs; this Host constructs
    /// the original Engine. This does not complete H01 or assert r0 publication.
    pub fn register_native_execution(
        &mut self,
        authorizer_ui: &ObjectId,
        frozen: ExecutionFrozenInputs,
        policy: ExecutionPolicy,
        dataset_id: CloudDatasetId,
        connection_ref: ObjectId,
    ) -> Result<ExecutionGatewayLease, ApiError> {
        self.require_registered_ui(authorizer_ui)?;
        let project = self
            .project
            .as_ref()
            .ok_or_else(|| error(ErrorCode::NoActiveProject, "No active project", None))?;
        let scenario = self.scenarios.get(&frozen.scenario.id).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Frozen scenario is not registered",
                None,
            )
        })?;
        if frozen.identity.host_context_session_id != self.host_instance_id
            || frozen.identity.project_id != project.id
            || !exact_data(project, &frozen.project)
            || !exact_data(scenario, &frozen.scenario)
        {
            return Err(error(
                ErrorCode::RevisionConflict,
                "Prepared input no longer matches its original Host/project/scenario",
                None,
            ));
        }
        if !self.execution_blockers().is_empty() {
            return Err(error(
                ErrorCode::InvalidInput,
                "An original execution still has production or cleanup obligations",
                None,
            ));
        }
        if self.executions.contains_key(&frozen.identity.session_id)
            || self.backing.as_ref().is_some_and(|a|a.execution_sources.contains_key(&crate::execution_archive::key(&frozen.identity))) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Execution identity is already registered; query the original request",
                None,
            ));
        }
        let core = ExecutionSession::prepare(frozen, policy, dataset_id, connection_ref)?;
        let token = Arc::new(());
        let lease = ExecutionGatewayLease {
            host_instance_id: self.host_instance_id.clone(),
            identity: core.identity.clone(),
            token: token.clone(),
        };
        self.executions.insert(
            core.identity.session_id.clone(),
            HostedExecution {
                core,
                gateway_token: token,
                cleanup_complete: false,
                preparation_ready: false,
                grants: BTreeMap::new(),
            },
        );
        self.mark_execution_content_changed(&lease.identity);
        Ok(lease)
    }

    pub(super) fn execution(&self, identity: &ExecutionIdentity) -> Result<&HostedExecution, ApiError> {
        if self
            .project
            .as_ref()
            .is_none_or(|p| p.id != identity.project_id)
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Execution is outside the active project",
                None,
            ));
        }
        self.executions
            .get(&identity.session_id)
            .filter(|entry| exact_data(&entry.core.identity, identity))
            .ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Original local execution/epoch is unavailable",
                    None,
                )
            })
    }

    fn gateway_core(
        &mut self,
        lease: &ExecutionGatewayLease,
    ) -> Result<&mut ExecutionSession, ApiError> {
        let entry = self.execution(&lease.identity)?;
        if lease.host_instance_id != self.host_instance_id
            || !Arc::ptr_eq(&lease.token, &entry.gateway_token)
            || entry.cleanup_complete
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Native Gateway lease is not current",
                None,
            ));
        }
        Ok(&mut self
            .executions
            .get_mut(&lease.identity.session_id)
            .unwrap()
            .core)
    }

    pub fn execution_gateway_register_boot(
        &mut self,
        lease: &ExecutionGatewayLease,
        device: &ObjectId,
        boot: DeviceBootId,
        now: Instant,
    ) -> Result<(), ApiError> {
        self.mutate_gateway(lease,|core|core.register_boot(device,boot,now))
    }
    pub fn execution_gateway_tick(
        &mut self,
        lease: &ExecutionGatewayLease,
        now: Instant,
    ) -> Result<(), ApiError> {
        self.mutate_gateway(lease,|core|core.tick(now))
    }
    pub fn execution_gateway_cloud_work(
        &mut self,
        lease: &ExecutionGatewayLease,
    ) -> Result<CloudWorkBatch, ApiError> {
        Ok(self.gateway_core(lease)?.cloud_work())
    }
    pub fn execution_gateway_next_cloud_work(
        &mut self,
        lease: &ExecutionGatewayLease,
    ) -> Result<Option<CloudWorkItem>, ApiError> {
        let core = self.gateway_core(lease)?;
        core.coalesce_pending_publications();
        Ok(core.pending_cloud_work.first().cloned())
    }
    pub fn execution_gateway_snapshot(&mut self, lease: &ExecutionGatewayLease) -> Result<ExecutionSnapshot, ApiError> {
        self.gateway_core(lease)?.projector.snapshot(&SnapshotSelection::Latest {}).map(std::borrow::Cow::into_owned).map_err(projection_failure)
    }
    pub fn execution_gateway_acknowledge(
        &mut self,
        lease: &ExecutionGatewayLease,
        ack: CloudEffectAck,
        now: Instant,
    ) -> Result<(), ApiError> {
        self.mutate_gateway(lease,|core|core.acknowledge_cloud_work(ack,now))
    }
    pub fn execution_gateway_receipt(
        &mut self,
        lease: &ExecutionGatewayLease,
        receipt_id: ObjectId,
        received_at: CloudTimestamp,
        receipt: DeviceReceipt,
        now: Instant,
    ) -> Result<ReceiptAdmission, ApiError> {
        self.mutate_gateway(lease,|core|core.apply_receipt(receipt_id,received_at,receipt,now))
    }
    pub fn execution_gateway_observation(
        &mut self,
        lease: &ExecutionGatewayLease,
        receipt_id: ObjectId,
        received_at: CloudTimestamp,
        observation: ExecutionTelemetryObservation,
        now: Instant,
    ) -> Result<ReceiptAdmission, ApiError> {
        self.mutate_gateway(lease,|core|core.apply_observation(receipt_id,received_at,observation,now))
    }

    /// Called only after Native has checked the owned children and the original
    /// cloud slot/boots. A pending or unknown publication cannot become ready.
    pub fn finish_native_execution_preparation(
        &mut self,
        lease: &ExecutionGatewayLease,
    ) -> Result<(), ApiError> {
        let core = self.gateway_core(lease)?;
        if !matches!(core.phase, ExecutionSnapshotLifecycleState::Prepared)
            || core.clock_anchor.is_some()
            || core.devices.values().any(|d| d.boot.is_none())
            || core.cloud_published_revision != Some(core.revision)
            || !core.pending_cloud_work.is_empty()
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Original boots and prepared publication are not confirmed",
                None,
            ));
        }
        self.executions
            .get_mut(&lease.identity.session_id)
            .unwrap()
            .preparation_ready = true;
        Ok(())
    }

    /// Preparation cancellation retains the same Engine/identity and leaves all
    /// publication, child retirement and cloud release work to the owned actor.
    pub fn stop_native_execution_preparation(
        &mut self,
        lease: &ExecutionGatewayLease,
        original_request: ObjectId,
    ) -> Result<ControlAck, ApiError> {
        if self.execution(&lease.identity)?.preparation_ready {
            return Err(error(
                ErrorCode::InvalidInput,
                "Preparation already completed; its cancellation cannot stop production",
                None,
            ));
        }
        let caller = self.host_instance_id.clone();
        let core = self.gateway_core(lease)?;
        core.control(
            caller,
            original_request,
            core.revision,
            ControlAction::StopDispatch {},
            Instant::now(),
        )
    }

    /// The owned Native runtime calls this after actual child retirement and
    /// C20/C10 release. It cannot erase unresolved Engine/publication obligations.
    pub fn finish_native_execution_cleanup(
        &mut self,
        lease: &ExecutionGatewayLease,
    ) -> Result<(), ApiError> {
        let core = self.gateway_core(lease)?;
        if !execution_terminal(core)
            || !core.observe()?.drained
            || !core.pending_cloud_work.is_empty()
            || core.cloud_published_revision != Some(core.revision)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Original production or publication is not settled",
                None,
            ));
        }
        self.executions
            .get_mut(&lease.identity.session_id)
            .unwrap()
            .cleanup_complete = true;
        Ok(())
    }

    pub fn grant_execution_controls(
        &mut self,
        authorizer_ui: &ObjectId,
        caller_session: &ObjectId,
        identity: &ExecutionIdentity,
        grant: ExecutionControlGrant,
    ) -> Result<(), ApiError> {
        self.require_registered_ui(authorizer_ui)?;
        let caller = self.sessions.get(caller_session).ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "Caller is not registered",
                None,
            )
        })?;
        self.admit(
            caller,
            &Some(identity.project_id.clone()),
            &ApiScope::Project,
            &OperationName::ControlExecutionSession,
        )?;
        let entry = self.execution(identity)?;
        if grant
            .device_acceptance_control_ids
            .iter()
            .any(|id| !entry.core.devices.contains_key(id))
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Device grant is outside the original frozen binding set",
                None,
            ));
        }
        self.executions
            .get_mut(&identity.session_id)
            .unwrap()
            .grants
            .insert(caller_session.clone(), grant);
        Ok(())
    }

    fn execution_control_authorized(
        &self,
        caller: &HostCallContext,
        entry: &HostedExecution,
        action: &ControlAction,
    ) -> bool {
        if !caller
            .allowed_operations
            .iter()
            .any(|op| matches!(op, OperationName::ControlExecutionSession))
        {
            return false;
        }
        if matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return true;
        }
        let Some(grant) = entry.grants.get(&caller.session_id) else {
            return false;
        };
        match action {
            ControlAction::SetDeviceAcceptance { device_id, .. } => {
                grant.device_acceptance_control_ids.contains(device_id)
            }
            _ => grant.session_controls.iter().any(|allowed| {
                matches!(
                    (allowed, action),
                    (
                        ExecutionReadAccessAllowedSessionControlsItem::Run,
                        ControlAction::Run {}
                    ) | (
                        ExecutionReadAccessAllowedSessionControlsItem::PauseDispatch,
                        ControlAction::PauseDispatch {}
                    ) | (
                        ExecutionReadAccessAllowedSessionControlsItem::ResumeDispatch,
                        ControlAction::ResumeDispatch {}
                    ) | (
                        ExecutionReadAccessAllowedSessionControlsItem::StopDispatch,
                        ControlAction::StopDispatch {}
                    ) | (
                        ExecutionReadAccessAllowedSessionControlsItem::RequestReconciliation,
                        ControlAction::RequestReconciliation {}
                    )
                )
            }),
        }
    }

    pub(super) fn read_execution_snapshot(
        &self,
        caller: &HostCallContext,
        _: &ApiScope,
        params: HostReadRequestParams,
    ) -> Result<ExecutionReadEnvelope, ApiError> {
        let source = self.local_execution_source(&params.identity)?;
        let snapshot = source.snapshot(&params.selection)?.into_owned();
        let entry = match source {
            LocalExecutionSource::Live(entry) => entry,
            LocalExecutionSource::Archive(_) => {
                return Ok(ExecutionReadEnvelope {
                    snapshot,
                    access: ExecutionReadAccess {
                        ownership: if params.identity.host_context_session_id==self.host_instance_id {ExecutionReadAccessOwnership::Unverified} else {ExecutionReadAccessOwnership::OtherHost},
                        allowed_session_controls:vec![], device_acceptance_control_ids:vec![],
                    },
                    delivery: ExecutionReadEnvelopeDelivery {
                        read_origin:ExecutionReadEnvelopeDeliveryReadOrigin::Host,
                        local_execution_revision:Some(source.snapshot(&SnapshotSelection::Latest {})?.execution_revision.clone()),
                        cloud_published_revision:source.published_revision(),
                        relation:ExecutionReadEnvelopeDeliveryRelation::Unknown,
                    },
                });
            }
        };
        let mut access = ExecutionReadAccess {
            ownership: ExecutionReadAccessOwnership::CurrentHostOwner,
            allowed_session_controls: vec![],
            device_acceptance_control_ids: vec![],
        };
        if matches!(params.selection, SnapshotSelection::Latest {})
            && entry.preparation_ready
            && !entry.cleanup_complete
            && !execution_terminal(&entry.core)
        {
            let choices = match entry.core.phase {
                ExecutionSnapshotLifecycleState::Prepared => {
                    vec![ControlAction::Run {}, ControlAction::StopDispatch {}]
                }
                ExecutionSnapshotLifecycleState::Running => vec![
                    ControlAction::PauseDispatch {},
                    ControlAction::StopDispatch {},
                    ControlAction::RequestReconciliation {},
                ],
                ExecutionSnapshotLifecycleState::DispatchPaused => vec![
                    ControlAction::ResumeDispatch {},
                    ControlAction::StopDispatch {},
                    ControlAction::RequestReconciliation {},
                ],
                ExecutionSnapshotLifecycleState::NeedsResolution => {
                    let mut controls = vec![
                        ControlAction::StopDispatch {},
                        ControlAction::RequestReconciliation {},
                    ];
                    if !entry.core.observe()?.attempts.iter().any(|attempt| {
                        matches!(
                            attempt.state,
                            sf_simulation::ExternalAttemptState::Failed
                                | sf_simulation::ExternalAttemptState::ReconciliationRequired
                        )
                    }) {
                        controls.push(ControlAction::ResumeDispatch {});
                    }
                    controls
                }
                _ => vec![ControlAction::RequestReconciliation {}],
            };
            for action in choices {
                if self.execution_control_authorized(caller, entry, &action) {
                    access.allowed_session_controls.push(match action {
                        ControlAction::Run {} => ExecutionReadAccessAllowedSessionControlsItem::Run,
                        ControlAction::PauseDispatch {} => {
                            ExecutionReadAccessAllowedSessionControlsItem::PauseDispatch
                        }
                        ControlAction::ResumeDispatch {} => {
                            ExecutionReadAccessAllowedSessionControlsItem::ResumeDispatch
                        }
                        ControlAction::StopDispatch {} => {
                            ExecutionReadAccessAllowedSessionControlsItem::StopDispatch
                        }
                        ControlAction::RequestReconciliation {} => {
                            ExecutionReadAccessAllowedSessionControlsItem::RequestReconciliation
                        }
                        ControlAction::SetDeviceAcceptance { .. } => unreachable!(),
                    });
                }
            }
            access.device_acceptance_control_ids = entry
                .core
                .devices
                .keys()
                .filter(|id| {
                    self.execution_control_authorized(
                        caller,
                        entry,
                        &ControlAction::SetDeviceAcceptance {
                            device_id: (*id).clone(),
                            accept_new_work: false,
                        },
                    )
                })
                .cloned()
                .collect();
        }
        Ok(ExecutionReadEnvelope {
            snapshot,
            access,
            delivery: ExecutionReadEnvelopeDelivery {
                read_origin: ExecutionReadEnvelopeDeliveryReadOrigin::Host,
                local_execution_revision: Some(uint(entry.core.revision)),
                cloud_published_revision: entry.core.cloud_published_revision.map(uint),
                relation: if entry.core.cloud_published_revision == Some(entry.core.revision) {
                    ExecutionReadEnvelopeDeliveryRelation::Synced
                } else {
                    ExecutionReadEnvelopeDeliveryRelation::PublicationPending
                },
            },
        })
    }

    pub(super) fn dispatch_execution_control(
        &mut self,
        session: &ObjectId,
        request: HostControlRequest,
    ) -> Value {
        let request = request.into_inner();
        let result = (|| {
            let caller = self.sessions.get(session).cloned().ok_or_else(|| {
                error(
                    ErrorCode::PermissionDenied,
                    "Unknown or revoked session",
                    None,
                )
            })?;
            self.admit(
                &caller,
                &request.project_id,
                &request.scope,
                &OperationName::ControlExecutionSession,
            )?;
            let entry = self.execution(&request.params.identity)?;
            if !entry.preparation_ready {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "The original preparation Task has not established ready devices and publication",
                    None,
                ));
            }
            if !self.execution_control_authorized(&caller, entry, &request.params.action) {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Caller has no grant for this execution action/device",
                    None,
                ));
            }
            let core = &mut self
                .executions
                .get_mut(&request.params.identity.session_id)
                .unwrap()
                .core;
            let before=(core.revision,core.cloud_published_revision);
            let result=core.control(
                caller.session_id,
                request.request_id.clone(),
                request.params.expected_execution_revision.value(),
                request.params.action,
                Instant::now(),
            );
            let changed=before!=(core.revision,core.cloud_published_revision);
            if changed {self.mark_execution_content_changed(&request.params.identity);}
            result
        })();
        self.respond(request.request_id, session, &request.scope, result)
    }

    pub(super) fn read_execution_view(
        &self,
        caller: &HostCallContext,
        _: &ApiScope,
        params: HostExecutionViewRequestParams,
    ) -> Result<ExecutionViewRead, ApiError> {
        Ok(match params.selection {
            ExecutionViewSelection::Inputs { identity } => ExecutionViewRead::Inputs {
                data: self.local_execution_source(&identity)?.frozen().clone(),
            },
            ExecutionViewSelection::Observations { query } => ExecutionViewRead::Observations {
                data: self.local_execution_source(&query.identity)?.observation(&query)?,
            },
            ExecutionViewSelection::History { query } => ExecutionViewRead::History {
                data: self.local_execution_source(&query.identity)?.history(&query)?,
            },
            ExecutionViewSelection::Request {
                originating_request_id,
            } => {
                let key = (caller.session_id.clone(), originating_request_id.clone());
                let data = if let Some(start) = self.registered_execution_start(caller, &originating_request_id)? {
                    start
                } else if let Some(registration) = self.cloud_read_requests.get(&key) {
                    let task =
                        self.owned_task(caller, &ApiScope::Project, &registration.task_id)?;
                    ExecutionRequestStatus::RegisteredCloudRead(RegisteredCloudReadRequest {
                        host_context_session_id: self.host_instance_id.clone(),
                        state: RegisteredCloudReadRequestState::Registered,
                        operation: RegisteredCloudReadRequestOperation::ReadCloudProduction,
                        task: serde_json::from_value(
                            serde_json::to_value(current_task_state(task).task).unwrap(),
                        )
                        .expect("Cloud Task registry has strict handle"),
                        originating_request_id,
                    })
                } else if let Some(export) = self.registered_execution_export(caller, &originating_request_id)? {
                    export
                } else {
                    ExecutionRequestStatus::NotFound(ExecutionRequestNotFound {
                        host_context_session_id: self.host_instance_id.clone(),
                        state: ExecutionRequestNotFoundState::NotFound,
                        operation: (),
                        task: (),
                        originating_request_id,
                    })
                };
                ExecutionViewRead::Request { data }
            }
            ExecutionViewSelection::Catalog {} => {
                let project_id = self
                    .project
                    .as_ref()
                    .ok_or_else(|| error(ErrorCode::NoActiveProject, "No active project", None))?
                    .id
                    .clone();
                let mut entries:Vec<_> = self
                    .executions
                    .values()
                    .filter(|entry| entry.core.identity.project_id == project_id)
                    .map(|entry| {
                        let s = entry
                            .core
                            .projector
                            .snapshot(&SnapshotSelection::Latest {})
                            .expect("Registered Engine has original r0");
                        ExecutionSourceSummary {
                            identity: s.identity.clone(),
                            execution_revision: s.execution_revision.clone(),
                            project_input_ref: s.project_input_ref.clone(),
                            observed_until_host_ms: s.coverage.observed_until_host_ms.clone(),
                            clock_started: s.coverage.clock_started,
                            lifecycle_state: s.lifecycle.state.clone(),
                            goal_at_snapshot: s.goal_at_snapshot.clone(),
                            active_owner: !entry.cleanup_complete,
                        }
                    })
                    .collect();
                if let Some(archive)=&self.backing {
                    for source in archive.execution_sources.values() {
                        if source.record.frozen_inputs.identity.project_id!=project_id || self.executions.values().any(|entry|exact_data(&entry.core.identity,&source.record.frozen_inputs.identity)) {continue;}
                        let Some(reader)=&source.reader else {continue};
                        let s=reader.snapshot(&SnapshotSelection::Latest {}).map_err(projection_failure)?;
                        entries.push(ExecutionSourceSummary {identity:s.identity.clone(),execution_revision:s.execution_revision.clone(),
                            project_input_ref:s.project_input_ref.clone(),observed_until_host_ms:s.coverage.observed_until_host_ms.clone(),
                            clock_started:s.coverage.clock_started,lifecycle_state:s.lifecycle.state.clone(),goal_at_snapshot:s.goal_at_snapshot.clone(),active_owner:false});
                    }
                }
                ExecutionViewRead::Catalog {
                    data: LocalExecutionCatalog {
                        project_id,
                        entries,
                    },
                }
            }
        })
    }

    pub(super) fn execution_blockers(&self) -> Vec<ExecutionCloseBlocker> {
        let mut blockers = vec![];
        for entry in self.executions.values() {
            let core = &entry.core;
            let snapshot = core
                .projector
                .snapshot(&SnapshotSelection::Latest {})
                .expect("Registered Engine has r0");
            let reason = if !execution_terminal(core) {
                "nonterminal_session"
            } else if !snapshot.lifecycle.unresolved_command_ids.is_empty()
                || core
                    .pending_cloud_work
                    .iter()
                    .any(|w| !matches!(w, CloudWorkItem::PublishSnapshot { .. }))
            {
                "unresolved_command"
            } else if core.cloud_published_revision != Some(core.revision) {
                "unpublished_final_result"
            } else if !entry.cleanup_complete {
                "native_cleanup_pending"
            } else {
                continue;
            };
            blockers.push(ExecutionCloseBlocker {
                session_id: core.identity.session_id.clone(),
                execution_epoch_id: core.identity.execution_epoch_id.clone(),
                execution_revision: uint(core.revision),
                reason: reason.to_owned(),
            });
        }
        blockers
    }

    pub(super) fn execution_control_capability(&self, caller: &HostCallContext) -> bool {
        self.executions.values().any(|entry| {
            entry.preparation_ready
                && !entry.cleanup_complete
                && (matches!(caller.origin, CallerOrigin::InteractiveUi)
                    || entry.grants.contains_key(&caller.session_id))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(value: &str) -> ObjectId {
        value.to_owned().try_into().unwrap()
    }
    fn setup() -> (ProjectHost, ObjectId, ExecutionGatewayLease) {
        let fixture = crate::execution_session::tests::session();
        let ui = id("native-ui");
        let mut host =
            ProjectHost::new(fixture.identity.host_context_session_id.clone(), ui.clone());
        host.project = Some(fixture.frozen.project.clone());
        host.scenarios.insert(
            fixture.frozen.scenario.id.clone(),
            fixture.frozen.scenario.clone(),
        );
        host.switch_epoch(&fixture.identity.project_id);
        let lease = host
            .register_native_execution(
                &ui,
                fixture.frozen,
                fixture.policy,
                fixture.dataset_id.unwrap(),
                fixture.provenance.cloud_connection_ref,
            )
            .unwrap();
        (host, ui, lease)
    }
    fn request(
        host: &mut ProjectHost,
        caller: &ObjectId,
        request_id: &str,
        operation: &str,
        params: Value,
    ) -> Value {
        let p = host.project.as_ref().unwrap().id.clone();
        host.dispatch(caller, serde_json::from_value(json!({"api_version":"0.1","request_id":request_id,"project_id":p,"scope":{"kind":"project"},"operation":operation,"params":params})).unwrap())
    }
    fn read(
        host: &mut ProjectHost,
        caller: &ObjectId,
        identity: &ExecutionIdentity,
        selection: Value,
    ) -> ExecutionReadEnvelope {
        let value = request(
            host,
            caller,
            "read",
            "get_execution_snapshot",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":identity,"selection":selection}),
        );
        assert_eq!(value["outcome"]["status"], "ok", "{value}");
        serde_json::from_value(value["outcome"]["data"].clone()).unwrap()
    }
    fn control(
        host: &mut ProjectHost,
        caller: &ObjectId,
        identity: &ExecutionIdentity,
        request_id: &str,
        expected: u64,
        action: Value,
    ) -> Value {
        request(
            host,
            caller,
            request_id,
            "control_execution_session",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":identity,"expected_execution_revision":expected,"action":action}),
        )
    }
    fn external(host: &mut ProjectHost, caller: &str) -> ObjectId {
        let caller = id(caller);
        host.register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: caller.clone(),
            granted_project: host.project.as_ref().map(|p| p.id.clone()),
            allowed_operations: vec![
                OperationName::GetExecutionSnapshot,
                OperationName::ReadExecutionView,
                OperationName::ControlExecutionSession,
            ],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
        caller
    }
    /// Only models a successful cloud acknowledgement for finite Host-boundary
    /// tests. Actual C02/HTTP and owned-process cleanup are separate acceptance.
    fn acknowledge_publications(host: &mut ProjectHost, lease: &ExecutionGatewayLease) {
        for work in host.execution_gateway_cloud_work(lease).unwrap().items {
            let CloudWorkItem::PublishSnapshot {
                work_id,
                publication,
            } = work
            else {
                panic!("Prepared stop cannot dispatch commands")
            };
            let ack = CloudEffectAck::SnapshotPublished {
                work_id,
                ack: PublishAck {
                    session_id: publication.snapshot.identity.session_id,
                    execution_epoch_id: publication.snapshot.identity.execution_epoch_id,
                    execution_revision: publication.snapshot.execution_revision,
                    publication_status: PublishAckPublicationStatus::Stored,
                    observation_frontier: publication.snapshot.observation_frontier,
                },
            };
            host.execution_gateway_acknowledge(lease, ack, Instant::now())
                .unwrap();
        }
    }
    fn ready_native(host: &mut ProjectHost, lease: &ExecutionGatewayLease) -> u64 {
        assert!(host.finish_native_execution_preparation(lease).is_err());
        for device in ["L1-D01", "L1-D02"] {
            host.execution_gateway_register_boot(
                lease,
                &id(device),
                format!("native-{device}").try_into().unwrap(),
                Instant::now(),
            )
            .unwrap();
        }
        assert!(
            host.finish_native_execution_preparation(lease).is_err(),
            "Stored boots alone do not publish the prepared result"
        );
        acknowledge_publications(host, lease);
        host.finish_native_execution_preparation(lease).unwrap();
        host.execution(&lease.identity).unwrap().core.revision
    }

    #[test]
    fn original_prepared_engine_is_readable_but_never_claimed_published_or_running() {
        let (mut host, ui, lease) = setup();
        let identity = lease.identity.clone();
        let original = host.project.as_ref().unwrap().revision.clone();
        let view = read(&mut host, &ui, &identity, json!({"kind":"latest"}));
        assert_eq!(view.snapshot.logical_work.len(), 6);
        assert!(view.snapshot.bound_jobs.is_empty());
        assert!(!view.snapshot.coverage.clock_started);
        assert_eq!(view.snapshot.coverage.observed_until_host_ms.value(), 0);
        assert!(view.delivery.cloud_published_revision.is_none());
        assert!(matches!(
            view.delivery.relation,
            ExecutionReadEnvelopeDeliveryRelation::PublicationPending
        ));
        assert!(view.access.allowed_session_controls.is_empty());
        assert_eq!(
            host.project.as_ref().unwrap().revision.0.value(),
            original.0.value()
        );
        assert!(
            host.tasks.is_empty(),
            "Production is not a never-ending preparation Task"
        );
        let revision = ready_native(&mut host, &lease);
        let ready = read(&mut host, &ui, &identity, json!({"kind":"latest"}));
        assert_eq!(ready.access.allowed_session_controls.len(), 2);
        let run = control(
            &mut host,
            &ui,
            &identity,
            "activate-original",
            revision,
            json!({"kind":"run"}),
        );
        assert_eq!(run["outcome"]["status"], "ok", "{run}");
        let live = read(&mut host, &ui, &identity, json!({"kind":"latest"}));
        assert!(live.snapshot.coverage.clock_started);
        let historical = read(
            &mut host,
            &ui,
            &identity,
            json!({"kind":"revision","execution_revision":0}),
        );
        assert!(!historical.snapshot.coverage.clock_started);
        assert!(historical.access.allowed_session_controls.is_empty());
        assert!(historical.access.device_acceptance_control_ids.is_empty());
    }

    #[test]
    fn actual_caller_action_and_device_grants_are_separate_from_project_read_access() {
        let (mut host, ui, lease) = setup();
        let identity = lease.identity.clone();
        let prepared_revision = ready_native(&mut host, &lease);
        let agent = external(&mut host, "bounded-agent");
        let view = read(&mut host, &agent, &identity, json!({"kind":"latest"}));
        assert!(view.access.allowed_session_controls.is_empty());
        let denied = control(
            &mut host,
            &agent,
            &identity,
            "before-grant",
            prepared_revision,
            json!({"kind":"run"}),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
        host.grant_execution_controls(
            &ui,
            &agent,
            &identity,
            ExecutionControlGrant {
                session_controls: vec![
                    ExecutionReadAccessAllowedSessionControlsItem::PauseDispatch,
                ],
                device_acceptance_control_ids: vec![id("L1-D01")],
            },
        )
        .unwrap();
        let first = control(
            &mut host,
            &ui,
            &identity,
            "shared-request-id",
            prepared_revision,
            json!({"kind":"run"}),
        );
        assert_eq!(first["outcome"]["status"], "ok", "{first}");
        let revision = first["outcome"]["data"]["execution_revision"]
            .as_u64()
            .unwrap();
        let second = control(
            &mut host,
            &agent,
            &identity,
            "shared-request-id",
            revision,
            json!({"kind":"pause_dispatch"}),
        );
        assert_eq!(second["outcome"]["status"], "ok", "{second}");
        assert!(
            second["outcome"]["data"]["execution_revision"]
                .as_u64()
                .unwrap()
                > revision
        );
        let duplicate = control(
            &mut host,
            &agent,
            &identity,
            "shared-request-id",
            revision,
            json!({"kind":"pause_dispatch"}),
        );
        assert_eq!(
            second["outcome"], duplicate["outcome"],
            "Exact replay keeps original acknowledgement"
        );
        let disallowed = control(
            &mut host,
            &agent,
            &identity,
            "other-device",
            2,
            json!({"kind":"set_device_acceptance","device_id":"L1-D02","accept_new_work":false}),
        );
        assert_eq!(disallowed["outcome"]["data"]["code"], "PermissionDenied");
        let changed = control(
            &mut host,
            &agent,
            &identity,
            "shared-request-id",
            revision,
            json!({"kind":"resume_dispatch"}),
        );
        assert_eq!(changed["outcome"]["status"], "error");
    }

    #[test]
    fn opaque_gateway_lease_cannot_be_replaced_by_copied_wire_identity() {
        let (mut host, ui, lease) = setup();
        let identity = lease.identity.clone();
        let forged = ExecutionGatewayLease {
            host_instance_id: host.host_instance_id.clone(),
            identity: identity.clone(),
            token: Arc::new(()),
        };
        assert!(host.execution_gateway_cloud_work(&forged).is_err());
        assert!(
            host.execution_gateway_register_boot(
                &lease,
                &id("unbound-device"),
                "wrong".to_owned().try_into().unwrap(),
                Instant::now()
            )
            .is_err()
        );
        host.execution_gateway_register_boot(
            &lease,
            &id("L1-D01"),
            "actual-native-boot".to_owned().try_into().unwrap(),
            Instant::now(),
        )
        .unwrap();
        let view = read(&mut host, &ui, &identity, json!({"kind":"latest"}));
        assert_eq!(
            view.snapshot
                .devices
                .iter()
                .find(|d| d.binding.device_id == id("L1-D01"))
                .unwrap()
                .registered_device_boot_id
                .as_ref()
                .unwrap()
                .as_str(),
            "actual-native-boot"
        );
        let wire = request(
            &mut host,
            &ui,
            "wire-gateway",
            "get_execution_cloud_work",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":identity}),
        );
        assert_eq!(wire["outcome"]["data"]["code"], "PermissionDenied");
    }

    #[test]
    fn prepared_stop_keeps_clock_zero_and_close_waits_for_publication_and_native_cleanup() {
        let (mut host, ui, lease) = setup();
        let identity = lease.identity.clone();
        let before = host.desktop_close_state();
        assert_eq!(before.execution_blockers[0].reason, "nonterminal_session");
        assert!(
            host.finish_desktop_close(&before, true, || panic!(
                "Must not destroy active production"
            ))
            .is_err()
        );
        assert!(
            host.check_departure(&DepartureDecision::DiscardAfterTasksTerminal)
                .is_err()
        );
        let prepared_revision = ready_native(&mut host, &lease);
        let actual_before_control = read(&mut host, &ui, &identity, json!({"kind":"latest"}));
        let stopped = control(
            &mut host,
            &ui,
            &identity,
            "stop-prepared",
            prepared_revision,
            json!({"kind":"stop_dispatch"}),
        );
        assert_eq!(stopped["outcome"]["status"], "ok", "{stopped}");
        std::fs::write(std::env::temp_dir().join("sf-v2-actual-control-ack-20261027.json"),serde_json::to_vec_pretty(&json!({"scope":"actual original Host component control response, not Native/HTTPS", "before":actual_before_control,"response":stopped})).unwrap()).unwrap();
        let view = read(&mut host, &ui, &identity, json!({"kind":"latest"}));
        assert!(matches!(
            view.snapshot.lifecycle.state,
            ExecutionSnapshotLifecycleState::Stopped
        ));
        assert!(!view.snapshot.coverage.clock_started);
        assert_eq!(view.snapshot.coverage.observed_until_host_ms.value(), 0);
        assert!(view.snapshot.bound_jobs.is_empty());
        assert!(host.finish_native_execution_cleanup(&lease).is_err());
        assert_eq!(
            host.desktop_close_state().execution_blockers[0].reason,
            "unpublished_final_result"
        );
        acknowledge_publications(&mut host, &lease);
        assert_eq!(
            host.desktop_close_state().execution_blockers[0].reason,
            "native_cleanup_pending"
        );
        host.finish_native_execution_cleanup(&lease).unwrap();
        assert!(
            host.execution_gateway_tick(&lease, Instant::now()).is_err(),
            "Retired lease cannot mutate the Engine"
        );
        assert!(
            host.check_departure(&DepartureDecision::DiscardAfterTasksTerminal)
                .is_ok()
        );
        let clean = host.desktop_close_state();
        assert!(clean.execution_blockers.is_empty());
        assert!(clean.unsaved, "Execution changes are now persisted project content");
        assert!(host.finish_desktop_close(&clean, false, || Ok(())).is_err());
        host.finish_desktop_close(&clean, true, || Ok(())).unwrap();
    }

    #[test]
    fn exact_epoch_and_original_revision_have_no_cloud_or_latest_fallback() {
        let (mut host, ui, lease) = setup();
        let mut identity = lease.identity.clone();
        identity.execution_epoch_id = "not-the-original-epoch".to_owned().try_into().unwrap();
        let missing = request(
            &mut host,
            &ui,
            "missing-epoch",
            "get_execution_snapshot",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":identity,"selection":{"kind":"latest"}}),
        );
        assert_eq!(missing["outcome"]["data"]["code"], "MissingReference");
        let missing = request(
            &mut host,
            &ui,
            "missing-r",
            "get_execution_snapshot",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":lease.identity,"selection":{"kind":"revision","execution_revision":9000}}),
        );
        assert_eq!(missing["outcome"]["data"]["code"], "MissingReference");
        assert!(host.drain_io_work().is_empty());
    }

    #[test]
    fn preparation_unknown_cannot_run_and_cancellation_retains_zero_clock_cleanup_obligations() {
        let (mut host, ui, lease) = setup();
        let caller = host.sessions[&ui].clone();
        assert!(!host.execution_control_capability(&caller));
        let start = control(
            &mut host,
            &ui,
            &lease.identity,
            "too-early",
            0,
            json!({"kind":"run"}),
        );
        assert_eq!(start["outcome"]["data"]["code"], "InvalidInput");
        host.stop_native_execution_preparation(&lease, id("cancel-original-preparation"))
            .unwrap();
        let result = read(&mut host, &ui, &lease.identity, json!({"kind":"latest"}));
        assert!(!result.snapshot.coverage.clock_started);
        assert!(matches!(
            result.snapshot.lifecycle.state,
            ExecutionSnapshotLifecycleState::Stopped
        ));
        assert!(result.snapshot.bound_jobs.is_empty());
        assert!(!host.desktop_close_state().execution_blockers.is_empty());
        acknowledge_publications(&mut host, &lease);
        host.finish_native_execution_cleanup(&lease).unwrap();
        assert!(host.desktop_close_state().execution_blockers.is_empty());
    }

    #[test]
    fn unresolved_actual_failed_attempt_does_not_offer_resume_as_an_allowed_control() {
        let (mut host, ui, lease) = setup();
        let revision = ready_native(&mut host, &lease);
        let started = control(
            &mut host,
            &ui,
            &lease.identity,
            "run-before-fault",
            revision,
            json!({"kind":"run"}),
        );
        assert_eq!(started["outcome"]["status"], "ok", "{started}");
        let core = &host.execution(&lease.identity).unwrap().core;
        let anchor = core.clock_anchor.unwrap();
        let slot = &core.devices[&id("L1-D01")];
        let observation: ExecutionTelemetryObservation = serde_json::from_value(json!({
            "contract_version":EXECUTION_CONTRACT_VERSION,"session_id":lease.identity.session_id,"execution_epoch_id":lease.identity.execution_epoch_id,
            "device_id":"L1-D01","device_boot_id":slot.boot,"device_sequence":1,"device_timestamp":"component-device-clock",
            "accept_new_work":true,"device_state":"idle","active_command_id":null,"dataset_id":core.dataset_id,"source_purpose":"execution","observation_probe_id":slot.probe.as_ref().unwrap().probe_id
        })).unwrap();
        let cloud_time: CloudTimestamp = "2026-10-27T00:00:00Z".to_owned().try_into().unwrap();
        host.execution_gateway_observation(
            &lease,
            id("actual-observation"),
            cloud_time.clone(),
            observation,
            anchor + std::time::Duration::from_millis(1),
        )
        .unwrap();
        let command = host
            .execution_gateway_cloud_work(&lease)
            .unwrap()
            .items
            .into_iter()
            .find_map(|work| match work {
                CloudWorkItem::PublishCommand { command, .. } => Some(command),
                _ => None,
            })
            .unwrap();
        for (sequence, payload) in [
            (2, json!({"kind":"started"})),
            (
                3,
                json!({"kind":"failed","code":"motor","message":"observed component fault"}),
            ),
        ] {
            let receipt: DeviceReceipt = serde_json::from_value(json!({"session_id":lease.identity.session_id,"execution_epoch_id":lease.identity.execution_epoch_id,
                "command_id":command.attempt.command_id,"device_id":command.device_id,"phase_id":command.attempt.phase_id,"phase_token":command.attempt.phase_token,
                "device_boot_id":command.target_device_boot_id,"device_sequence":sequence,"device_timestamp":"component-device-clock","receipt":payload})).unwrap();
            host.execution_gateway_receipt(
                &lease,
                id(&format!("fault-receipt-{sequence}")),
                cloud_time.clone(),
                receipt,
                anchor + std::time::Duration::from_millis(sequence),
            )
            .unwrap();
        }
        let view = read(&mut host, &ui, &lease.identity, json!({"kind":"latest"}));
        assert!(matches!(
            view.snapshot.lifecycle.state,
            ExecutionSnapshotLifecycleState::NeedsResolution
        ));
        assert!(
            !view
                .access
                .allowed_session_controls
                .iter()
                .any(|c| matches!(
                    c,
                    ExecutionReadAccessAllowedSessionControlsItem::ResumeDispatch
                ))
        );
        assert!(!view.snapshot.lifecycle.unresolved_command_ids.is_empty());
        assert!(host.finish_native_execution_cleanup(&lease).is_err());
    }

    #[test]
    fn request_recovery_uses_actual_caller_registry_and_read_only_history_uses_frozen_identity() {
        let (mut host, ui, lease) = setup();
        let project = host.project.as_ref().unwrap().clone();
        let scenario = host.scenarios.values().next().unwrap().clone();
        let response = request(
            &mut host,
            &ui,
            "same-id",
            "read_cloud_production",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"configuration","base_revision":project.revision,"scenario_id":scenario.id,"scenario_revision":scenario.revision}}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let query = json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"request","originating_request_id":"same-id"}});
        let recovered = request(
            &mut host,
            &ui,
            "recover",
            "read_execution_view",
            query.clone(),
        );
        assert_eq!(
            recovered["outcome"]["data"]["data"]["task"]["id"],
            response["outcome"]["data"]["id"]
        );
        let agent = external(&mut host, "different-caller");
        let hidden = request(&mut host, &agent, "recover", "read_execution_view", query);
        assert_eq!(hidden["outcome"]["data"]["data"]["state"], "not_found");
        let inputs = request(
            &mut host,
            &ui,
            "inputs",
            "read_execution_view",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"inputs","identity":lease.identity}}),
        );
        assert_eq!(
            inputs["outcome"]["data"]["data"]["project"]["id"],
            json!(project.id)
        );
        let catalog = request(
            &mut host,
            &ui,
            "catalog",
            "read_execution_view",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"catalog"}}),
        );
        assert_eq!(
            catalog["outcome"]["data"]["data"]["entries"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let observations = request(
            &mut host,
            &ui,
            "observations",
            "read_execution_view",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"observations","query":{
            "contract_version":EXECUTION_CONTRACT_VERSION,"identity":lease.identity,"execution_revision":0,"at_host_ms":0,"objects":[],"page":{"limit":200,"cursor":null}}}}),
        );
        assert_eq!(observations["outcome"]["status"], "ok", "{observations}");
    }
}
