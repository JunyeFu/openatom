//! Frozen, read-only rendering through the existing Host task queue.
//! Only a trusted in-process backend supplies pixels; no PNG/path enters the API.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RenderInputSource {
    Project,
    DefinitionDraft(DefinitionScopeIdentity),
    Run {
        input: RunInputRef,
        definition: Option<DefinitionScopeIdentity>,
    },
}

/// Actual source identity checked again against the backend's completed frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderFrame {
    pub task_id: TaskId,
    pub epoch: u64,
    pub project_id: ProjectId,
    pub scope: EditScope,
    pub scope_revision: Revision,
    pub source: RenderInputSource,
    pub request: RenderViewParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrozenRenderAsset {
    pub source: ObjectSource,
    pub asset: AssetRef,
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrozenRenderFile {
    pub file: FileRef,
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderPackagingInput {
    pub artifact: ArtifactRecord,
    pub result: PackagingResult,
    pub job: PackageStructureJob,
    /// None is the explicitly requested flat design state, never sample zero.
    pub sample: Option<PackagingProcessSample>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderComparisonInput {
    pub project: ProjectState,
    pub scenario: ScenarioInput,
    pub run: RunResult,
}

/// Private renderer DTO. Serialize omits all bytes; a task-bound byte callback
/// can return only an exact listed source/AssetRef or FileRef below.
#[derive(Debug, Clone, Serialize)]
pub struct FrozenRenderInput {
    pub project: ProjectState,
    pub scenarios: Vec<ScenarioInput>,
    pub modules: Vec<ModulePackage>,
    pub definition: Option<ModuleDefinition>,
    pub template_subset: Option<TemplateSubset>,
    pub run: Option<RunResult>,
    pub active_job_id: Option<ObjectId>,
    pub objects_at: Vec<DomainObject>,
    pub port_outputs: Vec<sf_simulation::PortOutputRecord>,
    pub port_output_summaries: Vec<RunPortOutput>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub port_output_failures: Vec<(PortRef, ApiError)>,
    pub packaging: Option<RenderPackagingInput>,
    pub comparison: Option<RunComparison>,
    pub comparison_runs: Vec<RenderComparisonInput>,
    pub assets: Vec<FrozenRenderAsset>,
    pub files: Vec<FrozenRenderFile>,
    pub issues: Vec<Issue>,
}

impl FrozenRenderInput {
    pub fn read_asset(&self, source: &ObjectSource, asset: &AssetRef) -> Result<Vec<u8>, ApiError> {
        self.assets
            .iter()
            .find(|file| exact_data(&file.source, source) && exact_data(&file.asset, asset))
            .map(|file| file.bytes.clone())
            .ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Asset is outside this task's actual frozen exact-source byte closure",
                    Some("/asset"),
                )
            })
    }
    pub fn read_file(&self, reference: &FileRef) -> Result<Vec<u8>, ApiError> {
        self.files
            .iter()
            .find(|file| exact_data(&file.file, reference))
            .map(|file| file.bytes.clone())
            .ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "File is outside this task's actual frozen file closure",
                    Some("/file"),
                )
            })
    }
}

#[derive(Clone)]
pub struct RenderJob {
    pub frame: RenderFrame,
    pub input: Arc<FrozenRenderInput>,
    /// Host-owned CSS viewport; Native still verifies the real window/DPR/PNG.
    pub viewport_css: (u32, u32),
    /// Native checks immediately BEFORE showing. Revocation does not cancel an
    /// admitted CaptureOnly task or erase a display which already happened.
    pub show_authorized: Arc<AtomicBool>,
}

pub struct RenderedCapture {
    pub frame: RenderFrame,
    pub scene_id: ObjectId,
    pub image_id: AssetId,
    pub mime_type: String,
    pub bytes: Vec<u8>,
    pub visible_objects: Vec<ObjectRef>,
    pub issues: Vec<Issue>,
}

pub trait RenderBackend: Send + Sync {
    /// Only actually wired product views belong here; never the proposed matrix.
    fn supported_kinds(&self) -> Vec<RenderViewKind>;
    fn render(&self, job: RenderJob, cancelled: &AtomicBool) -> Result<RenderedCapture, ApiError>;
}

#[derive(Clone)]
pub struct RenderedImage {
    pub asset: AssetRef,
    pub mime_type: String,
    pub bytes: Arc<[u8]>,
}

struct RenderEntry {
    owner: ObjectId,
    job: RenderJob,
    image: Option<RenderedImage>,
}

#[derive(Default)]
pub(super) struct RenderState {
    backend: Option<Arc<dyn RenderBackend>>,
    entries: BTreeMap<TaskId, RenderEntry>,
}

pub(super) struct RenderWork {
    backend: Arc<dyn RenderBackend>,
    job: RenderJob,
}

impl RenderWork {
    pub(super) fn with_report_viewport(mut self, viewport: (u32, u32)) -> Self {
        self.job.viewport_css = viewport;
        self
    }

    pub(super) fn capture_for_report(self, cancelled: &AtomicBool) -> Result<RenderedCapture, ApiError> {
        let job = self.job.clone();
        let mut capture = self.run(cancelled)?;
        validate_capture(&job, &capture)?;
        if cancelled.load(Ordering::Acquire) {
            return Err(render_cancelled());
        }
        let mut issues = job.input.issues.clone();
        issues.append(&mut capture.issues);
        capture.issues = issues;
        Ok(capture)
    }

    pub(super) fn run(self, cancelled: &AtomicBool) -> Result<RenderedCapture, ApiError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(render_cancelled());
        }
        if matches!(
            self.job.frame.request.delivery,
            RenderDelivery::ShowAndCapture
        ) && !self.job.show_authorized.load(Ordering::Acquire)
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "View control expired before actual display; ShowAndCapture is not CaptureOnly",
                None,
            ));
        }
        self.backend.render(self.job, cancelled)
    }
}

fn render_cancelled() -> ApiError {
    error(
        ErrorCode::Cancelled,
        "Rendering was cancelled before result commit",
        None,
    )
}
fn invalid_render(message: &str, path: &str) -> ApiError {
    error(ErrorCode::InvalidInput, message, Some(path))
}

impl ProjectHost {
    /// Trusted Native setup only; it does not grant callers any operation.
    pub fn set_render_backend(&mut self, backend: Arc<dyn RenderBackend>) {
        self.render_state.backend = Some(backend);
    }

    pub(super) fn rendering_available(&self) -> bool {
        self.render_state
            .backend
            .as_ref()
            .is_some_and(|backend| !backend.supported_kinds().is_empty())
    }

    /// Called by the original authority-update/revoke/epoch-switch hooks.
    pub(super) fn refresh_render_controls(&self) {
        for entry in self.render_state.entries.values() {
            let allowed = entry.job.frame.epoch == self.epoch
                && self.sessions.get(&entry.owner).is_some_and(|caller| {
                    self.admit(
                        caller,
                        &Some(entry.job.frame.project_id.clone()),
                        &render_api_scope(&entry.job.frame.scope),
                        &OperationName::RenderView,
                    )
                    .is_ok()
                        && (matches!(caller.origin, CallerOrigin::InteractiveUi)
                            || caller.allow_view_control)
                });
            entry.job.show_authorized.store(allowed, Ordering::Release);
        }
    }

    /// The external caller receives this image only via its original GetTask
    /// authority/owner/Scope. File paths supplied by a client are never accepted.
    pub fn read_rendered_image(
        &self,
        session: &ObjectId,
        project_id: &ProjectId,
        scope: &ApiScope,
        task_id: &TaskId,
        image_id: &AssetId,
    ) -> Result<RenderedImage, ApiError> {
        let caller = self.sessions.get(session).ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "Unknown or revoked session",
                None,
            )
        })?;
        self.admit(
            caller,
            &Some(project_id.clone()),
            scope,
            &OperationName::GetTask,
        )?;
        let task = self.owned_task(caller, scope, task_id)?;
        if !matches!(task.state.task.state, ComputationStatus::Completed)
            || !matches!(task.state.result, Some(TaskOutput::Render(_)))
        {
            return Err(invalid_render(
                "Task has no completed actual rendering image",
                "/task_id",
            ));
        }
        self.render_state
            .entries
            .get(task_id)
            .and_then(|entry| entry.image.as_ref())
            .filter(|image| &image.asset.id == image_id)
            .cloned()
            .ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Exact image does not belong to this rendering Task",
                    Some("/image_id"),
                )
            })
    }
}

fn render_api_scope(scope: &EditScope) -> ApiScope {
    match scope {
        EditScope::Project(_) => ApiScope::Project,
        EditScope::DefinitionDraft(identity) => ApiScope::DefinitionDraft(identity.id.clone()),
    }
}

impl ProjectHost {
    pub(super) fn render_scope_available(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
    ) -> bool {
        let Some(backend) = &self.render_state.backend else {
            return false;
        };
        self.admit(
            caller,
            &self.project.as_ref().map(|p| p.id.clone()),
            scope,
            &OperationName::RenderView,
        )
        .is_ok()
            && backend.supported_kinds().iter().any(|kind| match scope {
                ApiScope::Project => true,
                ApiScope::Workspace => false,
                ApiScope::DefinitionDraft(id) => match kind {
                    RenderViewKind::ModulePreview => true,
                    RenderViewKind::DeviceDetail | RenderViewKind::DeviceCyclePreview =>
                        matches!(self.definition_drafts[id].definition.payload, ModulePayload::Devices(_)),
                    RenderViewKind::PackagingWorkbench | RenderViewKind::PackagingManufacturing =>
                        self.definition_drafts[id].professional.artifacts.values().any(|artifact|
                            matches!(artifact.input_ref.kind, DocumentKind::PackagingJob)
                            && matches!(artifact.result_ref.kind, DocumentKind::PackagingResult)),
                    RenderViewKind::TimelineSnapshot => self.runs.values().any(|run|
                        matches!(&run.scope,EditScope::DefinitionDraft(identity)if &identity.id==id)),
                    _ => false,
                },
            })
    }

    pub(super) fn render_view(
        &mut self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: RenderViewParams,
    ) -> Result<TaskHandle, ApiError> {
        self.admit(
            caller,
            &self.project.as_ref().map(|p| p.id.clone()),
            scope,
            &OperationName::RenderView,
        )?;
        let id = TaskId::try_from(format!("{}:task:{}", self.host_instance_id.as_str(), self.next_task)).unwrap();
        let work = self.prepare_render_work(caller, scope, params, id.clone())?;
        let job = work.job.clone();
        let handle = self.enqueue_io(
            caller,
            TaskScope::Edit(job.frame.scope.clone()),
            OperationName::RenderView,
            job.frame.scope_revision.clone(),
            None,
            IoInput::Render(Box::new(work)),
        );
        self.render_state.entries.insert(id, RenderEntry {
            owner: caller.session_id.clone(), job, image: None,
        });
        Ok(handle)
    }

    /// The caller has already admitted RenderView or ExportEvaluation. Report
    /// captures are private CaptureOnly work, not additional public Tasks/grants.
    pub(super) fn prepare_render_work(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
        mut params: RenderViewParams,
        id: TaskId,
    ) -> Result<RenderWork, ApiError> {
        let current = self.project.as_ref().expect("admitted Project");
        let (edit_scope, revision) = match scope {
            ApiScope::Project => (
                EditScope::Project(current.id.clone()),
                current.revision.clone(),
            ),
            ApiScope::DefinitionDraft(id) => {
                let draft = &self.definition_drafts[id];
                (
                    EditScope::DefinitionDraft(DefinitionScopeIdentity {
                        id: id.clone(),
                        revision: draft.revision.clone(),
                    }),
                    draft.revision.clone(),
                )
            }
            ApiScope::Workspace => {
                return Err(invalid_render(
                    "Rendering requires an actual Project or DefinitionDraft",
                    "/scope",
                ));
            }
        };
        check_revision(&params.revision, &revision, "/params/revision")?;
        if matches!(params.delivery, RenderDelivery::ShowAndCapture)
            && !matches!(caller.origin, CallerOrigin::InteractiveUi)
            && !caller.allow_view_control
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "ShowAndCapture needs independently granted current view control",
                None,
            ));
        }
        if let (Some(a), Some(b)) = (&params.object_id, &params.view.object_ref) {
            if !same_ref(a, b) {
                return Err(invalid_render(
                    "The two rendering targets disagree",
                    "/params/object_id",
                ));
            }
        }
        let target = params
            .object_id
            .clone()
            .or_else(|| params.view.object_ref.clone());
        // Normalize only two equivalent target carriers, never a guessed target.
        params.view.object_ref = target.clone();
        if params.view.run_id.is_some() != params.view.view_time_ms.is_some() {
            return Err(invalid_render(
                "Run and exact view time must be supplied together",
                "/params/view",
            ));
        }
        if params.view.run_id.is_some() && params.preview.is_some() {
            return Err(invalid_render(
                "Declared preview cannot be mixed with actual Run",
                "/params/preview",
            ));
        }
        if params.module_ref.is_some() && !matches!(params.view.kind, RenderViewKind::ModulePreview)
        {
            return Err(invalid_render(
                "module_ref applies only to module_preview",
                "/params/module_ref",
            ));
        }
        if params.comparison.is_some() && !matches!(params.view.kind, RenderViewKind::Comparison) {
            return Err(invalid_render(
                "comparison applies only to comparison",
                "/params/comparison",
            ));
        }
        let (mut input, source) = self.freeze_render_source(caller, scope, &params)?;
        check_revision(
            &params.view.project_revision,
            &input.project.revision,
            "/params/view/project_revision",
        )?;
        self.prepare_render_kind(caller, scope, &mut params, target.as_ref(), &mut input)?;
        let backend = self
            .render_state
            .backend
            .clone()
            .ok_or_else(|| unsupported("No actual product rendering backend is registered"))?;
        if !backend
            .supported_kinds()
            .iter()
            .any(|kind| same(kind, &params.view.kind))
        {
            return Err(unsupported(
                "This view kind has no actual registered product rendering consumer",
            ));
        }
        let show_authorized = Arc::new(AtomicBool::new(
            matches!(caller.origin, CallerOrigin::InteractiveUi) || caller.allow_view_control,
        ));
        let job = RenderJob {
            frame: RenderFrame {
                task_id: id.clone(),
                epoch: self.epoch,
                project_id: current.id.clone(),
                scope: edit_scope.clone(),
                scope_revision: revision.clone(),
                source,
                request: params,
            },
            input: Arc::new(input),
            viewport_css: (1440, 900),
            show_authorized,
        };
        Ok(RenderWork { backend, job })
    }

    fn freeze_render_source(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: &RenderViewParams,
    ) -> Result<(FrozenRenderInput, RenderInputSource), ApiError> {
        let current = self.project.as_ref().expect("admitted Project");
        let mut input = FrozenRenderInput {
            project: current.clone(),
            scenarios: vec![],
            modules: vec![],
            definition: None,
            template_subset: None,
            run: None,
            active_job_id: None,
            objects_at: vec![],
            port_outputs: vec![],
            port_output_summaries: vec![],
            port_output_failures: vec![],
            packaging: None,
            comparison: None,
            comparison_runs: vec![],
            assets: vec![],
            files: vec![],
            issues: vec![],
        };
        if let (Some(id), Some(at)) = (&params.view.run_id, &params.view.view_time_ms) {
            let RunView::Full(run) = self.get_run(
                caller,
                scope,
                GetRunParams {
                    run_id: id.clone(),
                    query: RunQuery::FullResult,
                },
            )?
            else {
                unreachable!()
            };
            if at.0.value() < run.window.start_ms.0.value()
                || run
                    .computed_until_ms
                    .as_ref()
                    .is_none_or(|end| at.0.value() > end.0.value())
            {
                return Err(invalid_render(
                    "Requested rendering time is outside the computed Run frontier",
                    "/params/view/view_time_ms",
                ));
            }
            let mut definition_identity = None;
            match scope {
                ApiScope::Project => {
                    let summary = run_summary(&run);
                    input.project = self.frozen_run_project(&summary)?;
                    input.scenarios.push(self.frozen_run_scenario(&summary)?);
                    let mut seen = BTreeSet::new();
                    for reference in input.project.module_refs.iter().chain(
                        input
                            .project
                            .devices
                            .iter()
                            .flat_map(|d| &d.route_operation_refs),
                    ) {
                        if seen.insert(module_key(reference)) {
                            input
                                .modules
                                .push(self.frozen_run_module(&summary, reference)?);
                        }
                    }
                }
                ApiScope::DefinitionDraft(draft) => {
                    let frozen = self
                        .definition_cycle_inputs
                        .get(id)
                        .filter(|f| &f.identity.id == draft)
                        .ok_or_else(|| {
                            error(
                                ErrorCode::MissingReference,
                                "Actual DefinitionCycle frozen inputs are absent",
                                Some("/params/view/run_id"),
                            )
                        })?;
                    input.project = frozen.input.project.clone();
                    input.scenarios.push(frozen.input.scenario.clone());
                    input.modules = frozen.packages.values().cloned().collect();
                    input.definition = Some(frozen.definition.clone());
                    definition_identity = Some(frozen.identity.clone());
                }
                _ => unreachable!(),
            }
            let stored = &self.runs[id];
            input.port_outputs = stored
                .history
                .as_ref()
                .and_then(|h| h.port_outputs.as_ref())
                .map(|r| {
                    r.iter()
                        .filter(|r| r.0.0.value() <= at.0.value())
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            if matches!(
                params.view.kind,
                RenderViewKind::LineCanvas
                    | RenderViewKind::MultiHallCanvas
                    | RenderViewKind::DeviceDetail
                    | RenderViewKind::DeviceCyclePreview
                    | RenderViewKind::TimelineSnapshot
            ) {
                let mut selectors = vec![vec![]];
                // Legacy stored Runs can retain Items without Job history.
                // Static device_detail needs the exact frozen Device declaration,
                // not invented final-state animation. Other dynamic views retain
                // the original explicit ObjectsAt rejection for missing Jobs.
                let static_without_job_history = matches!(
                    params.view.kind,
                    RenderViewKind::DeviceDetail
                        | RenderViewKind::LineCanvas
                        | RenderViewKind::MultiHallCanvas
                ) && stored.engine.is_none()
                    && stored
                        .history
                        .as_ref()
                        .and_then(|h| h.job_records.as_ref())
                        .is_none();
                if static_without_job_history {
                    input.issues.push(render_issue("missing_job_history",
                        "Actual stage, active Job and Job attachment are unknown: this stored Run has no Job history; showing exact frozen geometry and known Item history without inferred animation"));
                }
                if !run.jobs.is_empty() && !static_without_job_history {
                    selectors.push(
                        run.jobs
                            .iter()
                            .map(|j| ObjectRef {
                                kind: ObjectKind::Job,
                                id: j.id.clone(),
                            })
                            .collect(),
                    );
                }
                for selected in selectors {
                    let mut after_index = None;
                    loop {
                        let RunView::Objects(page) = self.get_run(
                            caller,
                            scope,
                            GetRunParams {
                                run_id: id.clone(),
                                query: RunQuery::ObjectsAt {
                                    at_ms: at.clone(),
                                    objects: selected.clone(),
                                    page: PageRequest {
                                        after_index,
                                        limit: SafeUint::try_from(1000).unwrap(),
                                    },
                                },
                            },
                        )?
                        else {
                            unreachable!()
                        };
                        input.objects_at.extend(page.objects);
                        after_index = page.next_index;
                        if after_index.is_none() {
                            break;
                        }
                    }
                }
            }
            if matches!(scope, ApiScope::Project) {
                if stored
                    .history
                    .as_ref()
                    .and_then(|h| h.port_outputs.as_ref())
                    .is_none()
                {
                    input.issues.push(render_issue("missing_port_output_history","Actual output and acceptance quantities are unknown: this Run has no port output history; final stock and Jobs cannot substitute"));
                } else {
                    for device in &input.project.devices {
                        let package = input
                            .modules
                            .iter()
                            .find(|p| {
                                p.definition.id == device.module_ref.id
                                    && p.definition.version == device.module_ref.version
                            })
                            .ok_or_else(|| {
                                error(
                                    ErrorCode::MissingReference,
                                    "Frozen device package is absent for actual port summary",
                                    Some("/params/view/run_id"),
                                )
                            })?;
                        let ModulePayload::Devices(definition) = &package.definition.payload else {
                            continue;
                        };
                        for port in definition.ports.iter().filter(|p| {
                            matches!(p.direction, Direction::Output | Direction::Bidirectional)
                                && matches!(p.payload, PortPayload::Material(_))
                        }) {
                            let reference = PortRef {
                                instance_id: device.id.clone(),
                                module_ref: device.module_ref.clone(),
                                port_id: port.id.clone(),
                            };
                            match self.stored_run_port_output(&run_summary(&run), at, &reference) {
                                Ok(value) => input.port_output_summaries.push(value),
                                Err(failure)
                                    if matches!(failure.code, ErrorCode::UnsupportedCapability) =>
                                {
                                    input.issues.push(render_issue(
                                        "unavailable_port_output",
                                        &failure.message,
                                    ));
                                    input.port_output_failures.push((reference, failure));
                                }
                                Err(failure) => return Err(failure),
                            }
                        }
                    }
                }
            } else {
                input.issues.push(render_issue("port_output_not_applicable","DefinitionCycle output records do not identify a formal Project device port; Project port labels are not applicable"));
            }
            let reference = run.input_ref.clone();
            input.run = Some(*run);
            return Ok((
                input,
                RenderInputSource::Run {
                    input: reference,
                    definition: definition_identity,
                },
            ));
        }
        let source = match scope {
            ApiScope::Project => {
                input.scenarios = self.scenarios.values().cloned().collect();
                RenderInputSource::Project
            }
            ApiScope::DefinitionDraft(id) => {
                let draft = &self.definition_drafts[id];
                input.definition = Some(draft.definition.clone());
                if let Some(composition) = &draft.composition {
                    input.template_subset = Some(composition.subset.clone());
                }
                RenderInputSource::DefinitionDraft(DefinitionScopeIdentity {
                    id: id.clone(),
                    revision: draft.revision.clone(),
                })
            }
            _ => unreachable!(),
        };
        Ok((input, source))
    }
}

fn render_issue(code: &str, message: &str) -> Issue {
    Issue {
        code: format!("render.{code}"),
        object_ref: None,
        field_path: None,
        message: message.into(),
        blocks: vec![],
    }
}

impl ProjectHost {
    fn prepare_render_kind(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: &mut RenderViewParams,
        target: Option<&ObjectRef>,
        input: &mut FrozenRenderInput,
    ) -> Result<(), ApiError> {
        use RenderViewKind::*;
        let project_scope = matches!(scope, ApiScope::Project);
        let kind = params.view.kind.clone();
        if !project_scope && matches!(kind, LineCanvas | MultiHallCanvas | Comparison) {
            return Err(invalid_render(
                "This view kind requires Project scope",
                "/scope",
            ));
        }
        if params.preview.is_some()
            && matches!(
                kind,
                LineCanvas
                    | MultiHallCanvas
                    | PackagingManufacturing
                    | TimelineSnapshot
                    | Comparison
            )
        {
            return Err(invalid_render(
                "This view kind does not accept a declared preview",
                "/params/preview",
            ));
        }
        if params.view.run_id.is_some()
            && matches!(
                kind,
                PackagingWorkbench | PackagingManufacturing | Comparison
            )
        {
            return Err(invalid_render(
                "This view is not a factory Run projection",
                "/params/view/run_id",
            ));
        }
        match kind {
            LineCanvas => {
                if !matches!(params.view.canvas, CanvasKind::LineCanvas) {
                    return Err(invalid_render(
                        "line_canvas needs line_canvas coordinates",
                        "/params/view/canvas",
                    ));
                }
                let line = if let Some(target) = target {
                    if !matches!(target.kind, ObjectKind::Line) {
                        return Err(invalid_render(
                            "line_canvas target must be an actual Line",
                            "/params/object_id",
                        ));
                    }
                    Some(
                        input
                            .project
                            .lines
                            .iter()
                            .find(|line| line.id == target.id)
                            .ok_or_else(|| {
                                missing(target, "Selected Line is absent from this exact Project")
                            })?,
                    )
                } else {
                    None
                };
                if let Some(hall) = &params.view.hall_id {
                    if !input.project.halls.iter().any(|value| &value.id == hall) {
                        return Err(invalid_render(
                            "Selected hall is absent from this exact Project",
                            "/params/view/hall_id",
                        ));
                    }
                } else {
                    match input.project.halls.as_slice() {
                        [hall] => params.view.hall_id = Some(hall.id.clone()),
                        [] => input.issues.push(render_issue(
                            "empty_project",
                            "The actual Project has no halls; this is an empty view",
                        )),
                        _ => {
                            return Err(invalid_render(
                                "Multiple halls require an explicit hall, no first-hall fallback",
                                "/params/view/hall_id",
                            ));
                        }
                    }
                }
                if line.is_some_and(|line| Some(&line.hall_id) != params.view.hall_id.as_ref()) {
                    return Err(invalid_render(
                        "Selected Line is not in the selected hall",
                        "/params/object_id",
                    ));
                }
                self.render_project_modules(scope, input)?;
            }
            MultiHallCanvas => {
                if !matches!(params.view.canvas, CanvasKind::MultiHallCanvas)
                    || params.view.hall_id.is_some()
                {
                    return Err(invalid_render(
                        "multi_hall_canvas needs its canvas and no selected hall",
                        "/params/view",
                    ));
                }
                if target.is_some_and(|target| {
                    !matches!(target.kind, ObjectKind::Project)
                        || target.id.as_str() != input.project.id.as_str()
                }) {
                    return Err(invalid_render(
                        "multi_hall_canvas target must be this Project",
                        "/params/object_id",
                    ));
                }
                self.render_project_modules(scope, input)?;
            }
            DeviceDetail | DeviceCyclePreview => {
                if matches!(kind, DeviceCyclePreview)
                    && input.run.is_none()
                    && params.preview.as_ref().is_none_or(|p| {
                        p.phase_id.is_none() && p.progress.is_none() && p.pose_name.is_none()
                    })
                {
                    return Err(invalid_render(
                        "A declared cycle needs explicit phase/progress/pose; rendering does not start a Run",
                        "/params/preview",
                    ));
                }
                let target = target.ok_or_else(|| {
                    invalid_render(
                        "Device view needs an explicit exact target",
                        "/params/object_id",
                    )
                })?;
                let reference = match scope {
                    ApiScope::Project => {
                        if !matches!(target.kind, ObjectKind::Device) {
                            return Err(invalid_render(
                                "Project Device view needs a real Device",
                                "/params/object_id",
                            ));
                        }
                        input
                            .project
                            .devices
                            .iter()
                            .find(|d| d.id == target.id)
                            .map(|d| d.module_ref.clone())
                            .ok_or_else(|| {
                                missing(target, "Device is absent from the selected exact source")
                            })?
                    }
                    ApiScope::DefinitionDraft(id) => {
                        if !matches!(target.kind, ObjectKind::DefinitionDraft)
                            || target.id.as_str() != id.as_str()
                        {
                            return Err(invalid_render(
                                "Draft Device view must select its opaque actual Draft",
                                "/params/object_id",
                            ));
                        }
                        let definition =
                            input.definition.as_ref().expect("actual Draft definition");
                        if !matches!(definition.payload, ModulePayload::Devices(_)) {
                            return Err(unsupported(
                                "This Draft is not a Devices definition; no fictitious device is created for rendering",
                            ));
                        }
                        ModuleRef {
                            id: definition.id.clone(),
                            version: definition.version.clone(),
                        }
                    }
                    _ => unreachable!(),
                };
                self.render_module_closure(scope, &reference, input)?;
                if project_scope {
                    let references: Vec<_> = input
                        .project
                        .devices
                        .iter()
                        .find(|device| device.id == target.id)
                        .and_then(|device| device.presentation_override.as_ref())
                        .into_iter()
                        .flat_map(|presentation| {
                            std::iter::once(&presentation.source)
                                .chain(presentation.lod.detailed.iter())
                                .chain(presentation.lod.simplified.iter())
                        })
                        .filter_map(|locator| match locator {
                            MediaLocator::MediaRef(reference) => Some(reference.clone()),
                            _ => None,
                        })
                        .collect();
                    for reference in references {
                        self.render_module_closure(scope, &reference, input)?;
                    }
                }
                if input.run.is_some() {
                    let matches: Vec<_> = input
                        .objects_at
                        .iter()
                        .filter_map(|object| {
                            if let DomainObject::Job(job) = object {
                                Some(job)
                            } else {
                                None
                            }
                        })
                        .filter(|job| {
                            !matches!(job.state, JobStatus::Completed | JobStatus::Cancelled)
                                && job.started_at_ms.is_some()
                        })
                        .filter(|job| match &job.execution_ref {
                            JobExecutionRef::ProjectDevice { device_id, .. } => {
                                if project_scope {
                                    device_id == &target.id
                                } else {
                                    input.project.devices.iter().any(|d| {
                                        &d.id == device_id
                                            && module_key(&d.module_ref) == module_key(&reference)
                                    })
                                }
                            }
                            JobExecutionRef::DefinitionCycle { module_ref } => {
                                !project_scope && module_key(module_ref) == module_key(&reference)
                            }
                        })
                        .collect();
                    if matches.len() == 1 {
                        input.active_job_id = Some(matches[0].id.clone());
                    } else if matches!(kind, DeviceCyclePreview) {
                        return Err(unsupported(
                            "Actual device cycle needs one exact active Job; static device_detail remains available",
                        ));
                    } else {
                        input.issues.push(render_issue("frozen_static_device",if input.issues.iter().any(|i|i.code=="render.missing_job_history") {
                            "Actual stage and active Job are unknown; showing the Run's frozen static declaration, not animated world pose"
                        } else {"No unique active Job: showing the Run's frozen static declaration, not animated world pose"}));
                    }
                }
                if let Some(preview) = &params.preview {
                    validate_declared_preview(input, &reference, preview)?;
                }
            }
            ModulePreview => {
                let target = target.ok_or_else(|| {
                    invalid_render(
                        "Module preview needs an explicit exact target",
                        "/params/object_id",
                    )
                })?;
                let reference = if matches!(target.kind, ObjectKind::DefinitionDraft) {
                    let ApiScope::DefinitionDraft(id) = scope else {
                        return Err(invalid_render(
                            "Opaque Draft target needs that Draft scope",
                            "/params/object_id",
                        ));
                    };
                    if target.id.as_str() != id.as_str() {
                        return Err(invalid_render(
                            "Target is a different opaque Draft",
                            "/params/object_id",
                        ));
                    }
                    let d = input.definition.as_ref().unwrap();
                    let r = ModuleRef {
                        id: d.id.clone(),
                        version: d.version.clone(),
                    };
                    if params
                        .module_ref
                        .as_ref()
                        .is_some_and(|actual| module_key(actual) != module_key(&r))
                    {
                        return Err(invalid_render(
                            "Selector does not name the actual Draft definition version",
                            "/params/module_ref",
                        ));
                    }
                    r
                } else {
                    if !matches!(target.kind, ObjectKind::Module) {
                        return Err(invalid_render(
                            "Module preview needs Module or same Draft target",
                            "/params/object_id",
                        ));
                    }
                    if let Some(reference) = &params.module_ref {
                        if reference.id.as_str() != target.id.as_str() {
                            return Err(invalid_render(
                                "Module selector and target ID disagree",
                                "/params/module_ref",
                            ));
                        }
                        reference.clone()
                    } else {
                        let candidates = self.render_module_candidates(scope, target, input);
                        if candidates.len() != 1 {
                            return Err(invalid_render(
                                "Module ID does not resolve to one authorized exact version; supply module_ref",
                                "/params/module_ref",
                            ));
                        }
                        candidates[0].clone()
                    }
                };
                self.render_module_closure(scope, &reference, input)?;
                // Resolved exact identity is retained in the normalized actual view.
                params.module_ref = Some(reference.clone());
                let package = input
                    .modules
                    .iter()
                    .find(|p| {
                        module_key(&ModuleRef {
                            id: p.definition.id.clone(),
                            version: p.definition.version.clone(),
                        }) == module_key(&reference)
                    })
                    .unwrap();
                if let ModulePayload::Templates(template) = &package.definition.payload {
                    if input.template_subset.is_none() {
                        let reference = package
                            .files
                            .iter()
                            .find(|f| {
                                f.path == template.project
                                    && matches!(f.kind, DocumentKind::Template)
                            })
                            .cloned()
                            .ok_or_else(|| {
                                error(
                                    ErrorCode::MissingReference,
                                    "Actual template subset is not listed by this exact package",
                                    Some("/params/module_ref"),
                                )
                            })?;
                        let bytes = self.render_package_file(scope, package, &reference, input)?;
                        input.template_subset = Some(
                            serde_json::from_slice::<Versioned<TemplateSubset>>(&bytes)
                                .map_err(|_| {
                                    invalid_render(
                                        "Actual template subset is not a supported typed document",
                                        "/params/module_ref",
                                    )
                                })?
                                .data,
                        );
                        input.files.push(FrozenRenderFile {
                            file: reference,
                            bytes,
                        });
                    }
                } else if !matches!(
                    package.definition.payload,
                    ModulePayload::Media(_)
                        | ModulePayload::Devices(_)
                        | ModulePayload::Items(_)
                        | ModulePayload::Resources(_)
                        | ModulePayload::Transport(_)
                ) {
                    input.issues.push(render_issue("nonvisual_definition","This category has no declared 3D presentation; only its actual definition and limitations can be shown"));
                }
                if let Some(preview) = &params.preview {
                    validate_declared_preview(input, &reference, preview)?;
                }
            }
            PackagingWorkbench | PackagingManufacturing => {
                let target = target
                    .filter(|target| matches!(target.kind, ObjectKind::Artifact))
                    .ok_or_else(|| {
                        invalid_render(
                            "Packaging views require one explicit stored Artifact",
                            "/params/object_id",
                        )
                    })?;
                let artifact = ArtifactId::try_from(target.id.as_str().to_owned()).unwrap();
                let documents = match scope {
                    ApiScope::Project => &self.professional,
                    ApiScope::DefinitionDraft(id) => &self.definition_drafts[id].professional,
                    _ => unreachable!(),
                };
                let (artifact, result, job) =
                    self.stored_packaging_triple(documents, scope, &artifact)?;
                let sample = if let Some(preview) = &params.preview {
                    if preview.pose_name.is_some()
                        || preview.phase_id.is_none()
                        || preview.progress.is_none()
                    {
                        return Err(invalid_render(
                            "Packaging preview requires exact step and saved progress, no pose_name",
                            "/params/preview",
                        ));
                    }
                    let samples: Vec<_> = result
                        .process_poses
                        .iter()
                        .filter(|sample| {
                            sample.step_id.as_str() == preview.phase_id.as_ref().unwrap().as_str()
                                && exact_data(&sample.progress, preview.progress.as_ref().unwrap())
                        })
                        .collect();
                    if samples.len() != 1
                        || !job.steps.iter().any(|step| step.id == samples[0].step_id)
                    {
                        return Err(invalid_render(
                            "Preview must match one exact saved process sample, without interpolation",
                            "/params/preview",
                        ));
                    }
                    Some(samples[0].clone())
                } else {
                    None
                };
                if matches!(kind, PackagingManufacturing) {
                    if result.manufacturing_files.is_empty() {
                        return Err(error(
                            ErrorCode::MissingReference,
                            "This exact result has no manufacturing files",
                            None,
                        ));
                    }
                    for sheet in &result.manufacturing_files {
                        let bytes = self.render_professional_file(documents, scope, &sheet.file)?;
                        input.files.push(FrozenRenderFile {
                            file: sheet.file.clone(),
                            bytes,
                        });
                    }
                }
                input.packaging = Some(RenderPackagingInput {
                    artifact,
                    result,
                    job,
                    sample,
                });
            }
            TimelineSnapshot => {
                if input.run.is_none() {
                    if !project_scope {
                        return Err(invalid_render(
                            "Draft timeline requires its authorized actual sample Run",
                            "/params/view/run_id",
                        ));
                    }
                    let target = target
                        .filter(|t| matches!(t.kind, ObjectKind::Scenario))
                        .ok_or_else(|| {
                            invalid_render(
                                "Input timeline needs an explicit saved Scenario",
                                "/params/object_id",
                            )
                        })?;
                    let scenario = self
                        .scenarios
                        .values()
                        .find(|s| s.id.as_str() == target.id.as_str())
                        .cloned()
                        .ok_or_else(|| missing(target, "Saved Scenario is absent"))?;
                    check_revision(
                        &scenario.project_revision,
                        &input.project.revision,
                        "/params/object_id",
                    )?;
                    input.scenarios = vec![scenario];
                } else if let Some(target) = target {
                    let valid = matches!(target.kind, ObjectKind::Run)
                        && target.id.as_str() == input.run.as_ref().unwrap().id.as_str()
                        || matches!(target.kind, ObjectKind::Scenario)
                            && input
                                .scenarios
                                .iter()
                                .any(|s| s.id.as_str() == target.id.as_str());
                    if !valid {
                        return Err(invalid_render(
                            "Timeline target does not belong to the selected frozen Run/Scenario",
                            "/params/object_id",
                        ));
                    }
                }
            }
            Comparison => {
                if target.is_some_and(|t| {
                    !matches!(t.kind, ObjectKind::Project)
                        || t.id.as_str() != input.project.id.as_str()
                }) {
                    return Err(invalid_render(
                        "Comparison target must be this Project",
                        "/params/object_id",
                    ));
                }
                let comparison = params.comparison.clone().ok_or_else(|| {
                    invalid_render(
                        "Comparison requires explicit original CompareRunsParams",
                        "/params/comparison",
                    )
                })?;
                input.comparison = Some(self.compare_runs(caller, scope, comparison.clone())?);
                for id in comparison.run_ids {
                    let RunView::Full(run) = self.get_run(
                        caller,
                        scope,
                        GetRunParams {
                            run_id: id,
                            query: RunQuery::FullResult,
                        },
                    )?
                    else {
                        unreachable!()
                    };
                    let summary = run_summary(&run);
                    input.comparison_runs.push(RenderComparisonInput {
                        project: self.frozen_run_project(&summary)?,
                        scenario: self.frozen_run_scenario(&summary)?,
                        run: *run,
                    });
                }
            }
        }
        if matches!(kind, TimelineSnapshot | Comparison | PackagingManufacturing) {
            input.issues.push(render_issue(
                "camera_not_applicable",
                "This is a 2D view; the 3D camera is not applied",
            ));
        }
        if matches!(
            kind,
            LineCanvas | MultiHallCanvas | DeviceDetail | DeviceCyclePreview | ModulePreview
        ) {
            self.freeze_render_assets(scope, params, input)?;
        }
        Ok(())
    }
}

fn validate_declared_preview(
    input: &FrozenRenderInput,
    reference: &ModuleRef,
    preview: &RenderPreviewState,
) -> Result<(), ApiError> {
    if preview
        .progress
        .as_ref()
        .is_some_and(|value| value.value() < 0.0 || value.value() > 1.0)
    {
        return Err(invalid_render(
            "Declared progress must be in [0,1]",
            "/params/preview/progress",
        ));
    }
    let definition = input
        .modules
        .iter()
        .find(|p| p.definition.id == reference.id && p.definition.version == reference.version)
        .map(|p| &p.definition)
        .unwrap();
    let presentation = match &definition.payload {
        ModulePayload::Media(p) => Some(p),
        ModulePayload::Devices(d) => d.presentation.as_ref(),
        ModulePayload::Items(i) => i.presentation_or_media_ref.as_ref(),
        ModulePayload::Resources(r) => r.presentation.as_ref(),
        ModulePayload::Transport(t) => t.presentation.as_ref(),
        _ => None,
    };
    if let Some(phase) = &preview.phase_id {
        let operation = match &definition.payload {
            ModulePayload::Devices(d) => match d.behavior.selected {
                Some(BehaviorSourceKind::Inline) => d.behavior.inline.as_ref(),
                Some(BehaviorSourceKind::OperationRef) => {
                    d.behavior
                        .operation_ref
                        .as_ref()
                        .and_then(|r| {
                            input.modules.iter().find(|p| {
                                p.definition.id == r.id && p.definition.version == r.version
                            })
                        })
                        .and_then(|p| {
                            if let ModulePayload::Operations(o) = &p.definition.payload {
                                Some(o)
                            } else {
                                None
                            }
                        })
                }
                None => None,
            },
            ModulePayload::Operations(operation) => Some(operation),
            _ => None,
        };
        if !operation.is_some_and(|o| o.phases.iter().any(|p| &p.id == phase))
            && !presentation.is_some_and(|p| {
                p.timeline
                    .iter()
                    .any(|clip| clip.phase_ref.as_ref() == Some(phase))
            })
        {
            return Err(invalid_render(
                "Preview phase is not declared by this exact definition",
                "/params/preview/phase_id",
            ));
        }
    }
    if let Some(name) = &preview.pose_name {
        if !presentation.is_some_and(|p| p.poses.iter().any(|pose| pose.id.as_str() == name)) {
            return Err(invalid_render(
                "Named pose is not declared by this exact presentation",
                "/params/preview/pose_name",
            ));
        }
    }
    Ok(())
}

impl ProjectHost {
    fn render_module_candidates(
        &self,
        scope: &ApiScope,
        target: &ObjectRef,
        input: &FrozenRenderInput,
    ) -> Vec<ModuleRef> {
        let mut refs: BTreeMap<(String, String), ModuleRef> = input
            .modules
            .iter()
            .map(|p| {
                let r = ModuleRef {
                    id: p.definition.id.clone(),
                    version: p.definition.version.clone(),
                };
                (module_key(&r), r)
            })
            .collect();
        if input.run.is_none() {
            for package in self
                .modules
                .values()
                .chain(self.bundled_modules.values().map(|b| &b.package))
            {
                let reference = ModuleRef {
                    id: package.definition.id.clone(),
                    version: package.definition.version.clone(),
                };
                if self.scoped_media_package(scope, &reference).is_ok() {
                    refs.insert(module_key(&reference), reference);
                }
            }
            if let Some(definition) = &input.definition {
                let r = ModuleRef {
                    id: definition.id.clone(),
                    version: definition.version.clone(),
                };
                refs.insert(module_key(&r), r);
            }
        }
        refs.into_values()
            .filter(|r| r.id.as_str() == target.id.as_str())
            .collect()
    }

    fn render_module_closure(
        &self,
        scope: &ApiScope,
        reference: &ModuleRef,
        input: &mut FrozenRenderInput,
    ) -> Result<(), ApiError> {
        let mut pending = vec![reference.clone()];
        let mut seen = BTreeSet::new();
        while let Some(reference) = pending.pop() {
            if !seen.insert(module_key(&reference)) {
                continue;
            }
            if let Some(package) = input.modules.iter().find(|p| {
                p.definition.id == reference.id && p.definition.version == reference.version
            }) {
                pending.extend(package.dependencies.clone());
                continue;
            }
            if input.run.is_some() {
                return Err(error(
                    ErrorCode::MissingReference,
                    "Exact module is not in this Run's actual frozen source closure",
                    Some("/params/module_ref"),
                ));
            }
            let (package, _) = self.scoped_media_package(scope, &reference)?;
            pending.extend(package.dependencies.clone());
            input.modules.push(package);
        }
        Ok(())
    }

    fn render_project_modules(
        &self,
        scope: &ApiScope,
        input: &mut FrozenRenderInput,
    ) -> Result<(), ApiError> {
        let refs: Vec<_> = input
            .project
            .module_refs
            .iter()
            .cloned()
            .chain(input.project.devices.iter().flat_map(|d| {
                std::iter::once(d.module_ref.clone())
                    .chain(d.route_operation_refs.clone())
                    .chain(d.operation_binding.clone())
            }))
            .collect();
        for reference in refs {
            self.render_module_closure(scope, &reference, input)?;
        }
        Ok(())
    }

    fn render_professional_file(
        &self,
        documents: &ProfessionalDocuments,
        scope: &ApiScope,
        reference: &FileRef,
    ) -> Result<Vec<u8>, ApiError> {
        documents
            .files
            .get(reference.path.as_str())
            .cloned()
            .or_else(|| {
                if matches!(scope, ApiScope::Project) {
                    self.render_archive_file(reference)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Exact stored Artifact manufacturing file bytes are absent",
                    Some("/params/object_id"),
                )
            })
    }

    fn render_archive_file(&self, reference: &FileRef) -> Option<Vec<u8>> {
        self.additions
            .iter()
            .find(|f| exact_data(&f.entry.file_ref, reference))
            .map(|f| f.bytes.clone())
            .or_else(|| {
                let archive = self.backing.as_ref()?;
                archive
                    .manifest
                    .documents
                    .iter()
                    .find(|e| exact_data(&e.file_ref, reference))?;
                archive.files.get(reference.path.as_str()).cloned()
            })
    }

    fn render_package_file(
        &self,
        scope: &ApiScope,
        package: &ModulePackage,
        reference: &FileRef,
        input: &FrozenRenderInput,
    ) -> Result<Vec<u8>, ApiError> {
        if !package.files.iter().any(|f| exact_data(f, reference)) {
            return Err(error(
                ErrorCode::MissingReference,
                "Exact file is not listed by its package",
                Some("/file"),
            ));
        }
        let key = ModuleRef {
            id: package.definition.id.clone(),
            version: package.definition.version.clone(),
        };
        let bytes = if input.run.is_some() {
            self.render_archive_file(reference)
        } else {
            if let ApiScope::DefinitionDraft(id) = scope {
                if self.definition_drafts[id].definition.id == key.id
                    && self.definition_drafts[id].definition.version == key.version
                {
                    if let Some(composition) = &self.definition_drafts[id].composition {
                        if let Some(file) = composition
                            .files
                            .iter()
                            .find(|f| exact_data(&f.entry.file_ref, reference))
                        {
                            return Ok(file.bytes.clone());
                        }
                    }
                }
            }
            self.module_source_files(&key)
                .get(reference.path.as_str())
                .cloned()
        };
        bytes.ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Actual exact package file bytes are absent",
                Some("/file"),
            )
        })
    }

    fn freeze_render_assets(
        &self,
        scope: &ApiScope,
        params: &RenderViewParams,
        input: &mut FrozenRenderInput,
    ) -> Result<(), ApiError> {
        for package in &input.modules {
            let reference = ModuleRef {
                id: package.definition.id.clone(),
                version: package.definition.version.clone(),
            };
            let source = if let (Some(run_id), Some(at_ms)) =
                (&params.view.run_id, &params.view.view_time_ms)
            {
                ObjectSource::RunModuleVersion {
                    run_id: run_id.clone(),
                    at_ms: at_ms.clone(),
                    module_ref: reference.clone(),
                }
            } else {
                ObjectSource::ModuleVersion {
                    module_ref: reference.clone(),
                }
            };
            for asset in &package.assets {
                let declared = package
                    .files
                    .iter()
                    .any(|f| f.path == asset.path && matches!(f.kind, DocumentKind::Asset))
                    || self.archive_media_file(&asset.path).is_some();
                let bytes = if !declared {
                    None
                } else if let Some(run_id) = &params.view.run_id {
                    if matches!(scope, ApiScope::DefinitionDraft(_)) {
                        self.frozen_cycle_media_bytes(run_id, &reference, asset)
                    } else {
                        self.archive_media_file(&asset.path).map(|(_, b)| b)
                    }
                } else if let ApiScope::DefinitionDraft(id) = scope {
                    let draft = &self.definition_drafts[id];
                    if draft.definition.id == reference.id
                        && draft.definition.version == reference.version
                    {
                        draft
                            .composition
                            .as_ref()
                            .and_then(|c| {
                                c.files
                                    .iter()
                                    .find(|f| {
                                        f.entry.file_ref.path == asset.path
                                            && matches!(f.entry.file_ref.kind, DocumentKind::Asset)
                                    })
                                    .map(|f| f.bytes.clone())
                            })
                            .or_else(|| draft.professional.files.get(asset.path.as_str()).cloned())
                            .or_else(|| self.archive_media_file(&asset.path).map(|(_, b)| b))
                    } else {
                        self.media_file_bytes(&reference, asset)
                    }
                } else {
                    self.media_file_bytes(&reference, asset)
                };
                if let Some(bytes) = bytes {
                    input.assets.push(FrozenRenderAsset {
                        source: source.clone(),
                        asset: asset.clone(),
                        bytes,
                    });
                } else {
                    input.issues.push(render_issue("missing_asset",&format!("Actual exact listed bytes absent for {}@{:?} / {}; no replacement image is provided",reference.id.as_str(),reference.version,asset.id.as_str())));
                }
            }
        }
        Ok(())
    }

    pub(super) fn finish_render(&mut self, task_id: TaskId, capture: RenderedCapture) {
        let Some(task) = self.tasks.get(&task_id) else {
            return;
        };
        if terminal(&task.state.task.state) {
            return;
        }
        let result = (|| {
            if task.cancelled.load(Ordering::Acquire) {
                return Err(render_cancelled());
            }
            let entry = self.render_state.entries.get(&task_id).ok_or_else(|| {
                invalid_render("Rendering task has no admitted frozen source", "/task_id")
            })?;
            if task.epoch != entry.job.frame.epoch {
                return Err(invalid_render(
                    "Actual frame does not match its exact Task/epoch/ScopeR/ProjectR/source/view/camera",
                    "/frame",
                ));
            }
            validate_capture(&entry.job, &capture)?;
            let file = AssetRef {
                id: capture.image_id.clone(),
                path: RelativeFilePath::try_from(format!(
                    "rendered/task-{}.png",
                    task_id.as_str().rsplit(':').next().unwrap()
                ))
                .unwrap(),
                kind: AssetKind::Png,
            };
            let image = RenderedImage {
                asset: file.clone(),
                mime_type: capture.mime_type,
                bytes: Arc::from(capture.bytes),
            };
            let mut issues = entry.job.input.issues.clone();
            issues.extend(capture.issues);
            let output = RenderOutput {
                scene: RenderedSceneRef {
                    evidence_origin: EvidenceOrigin::ActualSoftware,
                    id: capture.scene_id,
                    file,
                    view: capture.frame.request.view,
                    scene_revision: capture.frame.scope_revision.clone(),
                },
                requested_scope: capture.frame.scope,
                requested_scope_revision: capture.frame.scope_revision,
                visible_objects: capture.visible_objects,
                issues,
            };
            Ok((output, image))
        })();
        let task = self.tasks.get_mut(&task_id).unwrap();
        task.state.allowed_controls.clear();
        match result {
            Ok((output, image)) => {
                task.state.task.state = ComputationStatus::Completed;
                task.state.task.result_ref = Some(FileRef {
                    path: output.scene.file.path.clone(),
                    kind: DocumentKind::Asset,
                });
                task.state.result = Some(TaskOutput::Render(output));
                self.render_state.entries.get_mut(&task_id).unwrap().image = Some(image);
            }
            Err(failure) => {
                task.state.task.state = if matches!(failure.code, ErrorCode::Cancelled) {
                    ComputationStatus::Cancelled
                } else {
                    ComputationStatus::Failed
                };
                task.state.task.failure = Some(render_issue("failed", &failure.message));
                task.state.failure = Some(failure);
            }
        }
    }
}

fn validate_capture(job: &RenderJob, capture: &RenderedCapture) -> Result<(), ApiError> {
    if !exact_data(&capture.frame, &job.frame) {
        return Err(invalid_render(
            "Actual frame does not match its exact Task/epoch/ScopeR/ProjectR/source/view/camera",
            "/frame",
        ));
    }
    if capture.mime_type != "image/png" || !capture.bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(invalid_render("Backend must return actual PNG bytes with matching MIME", "/image"));
    }
    for object in &capture.visible_objects {
        if !render_visible_member(job, object) {
            let mut failure = invalid_render("Reported visible object is outside the actual frozen source", "/visible_objects");
            failure.object_refs.push(object.clone());
            return Err(failure);
        }
    }
    Ok(())
}

fn render_visible_member(job: &RenderJob, object: &ObjectRef) -> bool {
    let input = &job.input;
    let scenarios = input
        .scenarios
        .iter()
        .map(|s| (s.id.clone(), s.clone()))
        .collect();
    // A Draft's Project context is not its displayed Project object set.
    // In particular an existing same-ID Project Device is not the Draft target.
    if matches!(job.frame.scope, EditScope::Project(_))
        && lookup(&input.project, &scenarios, object).is_ok()
    {
        return true;
    }
    // A timeline draws the frozen schedule and computed history, not only the
    // physical objects present at its cursor time. Never read current Scenario
    // state here, or extend this history membership to a 3D capture.
    if matches!(job.frame.request.view.kind, RenderViewKind::TimelineSnapshot) {
        let declared = input.scenarios.iter().any(|scenario| match object.kind {
            ObjectKind::Order => scenario.orders.iter().any(|v| v.id == object.id),
            ObjectKind::Task => scenario.production_tasks.iter().any(|v| v.id == object.id),
            ObjectKind::Supply => scenario.supplies.iter().any(|v| v.id == object.id),
            _ => false,
        });
        let historical = input.run.as_ref().is_some_and(|run| match object.kind {
            ObjectKind::Job => run.jobs.iter().any(|v| v.id == object.id),
            ObjectKind::Transport => run.transport.iter().any(|v| v.id == object.id),
            ObjectKind::Item => run.events.iter().any(|event|
                matches!(event.subject.kind, ObjectKind::Item) && event.subject.id == object.id),
            _ => false,
        });
        if declared || historical {
            return true;
        }
    }
    match object.kind {
        ObjectKind::Module => input
            .modules
            .iter()
            .any(|p| p.definition.id.as_str() == object.id.as_str()),
        ObjectKind::DefinitionDraft => {
            matches!(&job.frame.scope,EditScope::DefinitionDraft(d)if d.id.as_str()==object.id.as_str())
        }
        ObjectKind::Run => {
            input
                .run
                .as_ref()
                .is_some_and(|r| r.id.as_str() == object.id.as_str())
                || input.comparison.as_ref().is_some_and(|_| {
                    job.frame
                        .request
                        .comparison
                        .as_ref()
                        .unwrap()
                        .run_ids
                        .iter()
                        .any(|id| id.as_str() == object.id.as_str())
                })
        }
        ObjectKind::Item => input
            .objects_at
            .iter()
            .any(|o| matches!(o,DomainObject::Item(i)if i.id==object.id)),
        ObjectKind::Job => input
            .objects_at
            .iter()
            .any(|o| matches!(o,DomainObject::Job(j)if j.id==object.id)),
        ObjectKind::Artifact => input
            .packaging
            .as_ref()
            .is_some_and(|p| p.artifact.id.as_str() == object.id.as_str()),
        ObjectKind::PackagingJob => input
            .packaging
            .as_ref()
            .is_some_and(|p| p.job.id == object.id),
        // A manufacturing part_id is not an AssetId. The renderer may report
        // its actual Artifact/PackagingJob, never invent an Asset identity.
        ObjectKind::Asset => input
            .assets
            .iter()
            .any(|a| a.asset.id.as_str() == object.id.as_str()),
        _ => false,
    }
}
