//! Session-only definition rehearsals. The production Engine remains the sole
//! owner of simulated stock/time; none of these frozen inputs are Project files.
use super::host_definitions::validate_definition;
use super::*;

pub(super) struct DefinitionCycleInput {
    pub(super) input: sf_simulation::EngineInput,
    pub(super) identity: DefinitionScopeIdentity,
    pub(super) definition: ModuleDefinition,
    pub(super) packages: BTreeMap<(String, String), ModulePackage>,
    pub(super) media_files: BTreeMap<((String, String), String), Vec<u8>>,
}

fn source_issue(identity: &DefinitionScopeIdentity) -> Issue {
    Issue {
        code: "definition_cycle.frozen_draft_source".into(),
        object_ref: Some(ObjectRef {
            kind: ObjectKind::DefinitionDraft,
            id: ObjectId::try_from(identity.id.as_str().to_owned()).unwrap(),
        }),
        field_path: Some("/scope/data/revision".into()),
        message: format!(
            "隔离定义样例；DefinitionDraft {} revision {}。该草稿快照不是已发布的同版本定义，样例结果不属于正式工程运行、库存或人工认可。",
            identity.id.as_str(),
            identity.revision.0.value()
        ),
        blocks: vec![CapabilityPurpose::CompleteConclusion],
    }
}

fn unsupported_category(payload: &ModulePayload) -> Option<&'static str> {
    match payload {
        ModulePayload::Items(_) => Some(
            "物品定义只检查规格、单位与跟踪方式；工程物品类型没有精确 ModuleRef 映射，不能把未消费的定义冒充生产样例。",
        ),
        ModulePayload::Resources(_) => Some(
            "资源定义可检查容量、资格与日历声明；Engine 消费明确采用后的资源实例，本入口不会隐式覆盖实例容量或创建人员。",
        ),
        ModulePayload::Templates(_) => Some(
            "模板使用真实模板准备与显式实例采用入口；DefinitionCycle 不隐式复制布局、供应或订单。",
        ),
        ModulePayload::Professional(_) => Some(
            "专业定义不提供生产周期；纸包装使用实际 compute_packaging 及其输入，其他计划提供者不能冒充可调用。",
        ),
        ModulePayload::Media(_) => {
            Some("表现定义使用部件、锚点与动作预览；外观检查不创建生产运行或物品。")
        }
        ModulePayload::Tasks(task)
            if matches!(
                task.kind,
                TaskKind::DeferredMaintenance | TaskKind::DeferredConstruction
            ) =>
        {
            Some(
                "维修和工程施工任务是后置能力声明，当前不提供可调用周期；可继续保存和检查定义，不创建虚构的任务执行结果。",
            )
        }
        _ => None,
    }
}

impl ProjectHost {
    pub(super) fn definition_cycle_scope_available(
        &self,
        caller: &HostCallContext,
        id: &ModuleId,
    ) -> bool {
        self.project.is_some()
            && self.authorize_definition_cycle_project_read(caller).is_ok()
            && (matches!(caller.origin, CallerOrigin::InteractiveUi)
                || caller.allowed_definition_drafts.contains(id))
            && self
                .definition_drafts
                .get(id)
                .is_some_and(|draft| unsupported_category(&draft.definition.payload).is_none())
    }

    fn authorize_definition_cycle_project_read(
        &self,
        caller: &HostCallContext,
    ) -> Result<(), ApiError> {
        let allowed = matches!(caller.origin, CallerOrigin::InteractiveUi)
            || (self
                .project
                .as_ref()
                .is_some_and(|p| caller.granted_project.as_ref() == Some(&p.id))
                && caller
                    .allowed_operations
                    .iter()
                    .any(|op| matches!(op, OperationName::GetObject)));
        if !allowed {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Definition samples require current authority to read their mapped Project",
                None,
            ));
        }
        Ok(())
    }

    pub(super) fn run_definition_cycle(
        &mut self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: RunScenarioParams,
    ) -> Result<TaskHandle, ApiError> {
        self.authorize_definition_cycle_project_read(caller)?;
        let ApiScope::DefinitionDraft(id) = scope else {
            return Err(error(
                ErrorCode::InvalidInput,
                "DefinitionCycle requires DefinitionDraft scope",
                Some("/scope"),
            ));
        };
        let draft = self
            .definition_drafts
            .get(id)
            .expect("admitted definition draft");
        let RunIntent::DefinitionCycle {
            definition_revision,
            test_input,
        } = params.intent
        else {
            return Err(error(
                ErrorCode::InvalidInput,
                "DefinitionDraft requires explicit DefinitionCycle intent",
                Some("/params/intent"),
            ));
        };
        check_revision(
            &params.input_revision,
            &draft.revision,
            "/params/input_revision",
        )?;
        check_revision(
            &definition_revision,
            &draft.revision,
            "/params/intent/data/definition_revision",
        )?;
        if params.scenario_id != test_input.id
            || !exact_data(&params.observation_window, &test_input.window)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Definition sample Scenario ID and observation window must match explicit test_input",
                Some("/params/intent/data/test_input"),
            ));
        }
        let current = self.project.as_ref().expect("admitted active Project");
        check_revision(
            &test_input.project_revision,
            &current.revision,
            "/params/intent/data/test_input/project_revision",
        )?;
        if let Some(reason) = unsupported_category(&draft.definition.payload) {
            return Err(unsupported(reason));
        }
        validate_definition(&draft.definition)?;
        let reference = ModuleRef {
            id: draft.definition.id.clone(),
            version: draft.definition.version.clone(),
        };
        let selected = |device: &DeviceInstance| {
            test_input.scope.is_empty()
                || test_input
                    .scope
                    .iter()
                    .any(|s| matches!(s.kind, ObjectKind::Device) && s.id == device.id)
        };
        let actual_steps: Vec<_> = test_input
            .production_tasks
            .iter()
            .flat_map(|task| {
                current
                    .routes
                    .iter()
                    .filter(move |route| {
                        route.id == task.routing_ref.id
                            && route.revision == task.routing_ref.revision
                    })
                    .flat_map(|route| &route.steps)
            })
            .filter(|step| {
                current
                    .devices
                    .iter()
                    .any(|d| d.id == step.device_id && selected(d))
            })
            .collect();
        let consumed = match &draft.definition.payload {
            ModulePayload::Devices(_) => actual_steps.iter().any(|step| {
                current.devices.iter().any(|device| {
                    device.id == step.device_id
                        && module_key(&device.module_ref) == module_key(&reference)
                })
            }),
            ModulePayload::Operations(_) | ModulePayload::Tasks(_) => actual_steps
                .iter()
                .any(|step| module_key(&step.operation_ref) == module_key(&reference)),
            ModulePayload::Transport(_) => {
                test_input.scope.is_empty()
                    && test_input
                        .supplies
                        .iter()
                        .any(|s| module_key(&s.transport_ref) == module_key(&reference))
            }
            _ => false,
        };
        if !consumed {
            return Err(error(
                ErrorCode::MissingReference,
                "Explicit sample input must select a real mapped consumer of this exact definition id/version; no device, resource, port or stock is created implicitly",
                Some("/params/intent/data/test_input"),
            ));
        }

        let identity = DefinitionScopeIdentity {
            id: id.clone(),
            revision: draft.revision.clone(),
        };
        let definition = draft.definition.clone();
        let mut references = current.module_refs.clone();
        references.extend(
            current
                .devices
                .iter()
                .flat_map(|device| device.route_operation_refs.iter().cloned()),
        );
        references.extend(
            current
                .devices
                .iter()
                .filter_map(|device| device.operation_binding.clone()),
        );
        references.extend(
            test_input
                .supplies
                .iter()
                .map(|supply| supply.transport_ref.clone()),
        );
        let mut packages = BTreeMap::new();
        for reference in references {
            let key = module_key(&reference);
            if packages.contains_key(&key) {
                continue;
            }
            let source = self.modules.get(&key).ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Exact mapped sample module version is absent",
                    Some("/project/module_refs"),
                )
            })?;
            packages.insert(key, source.clone());
        }
        let package = packages.get_mut(&module_key(&reference)).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Definition sample requires an explicitly mapped exact source package",
                Some("/project/module_refs"),
            )
        })?;
        *package = self.draft_media_package(id)?;
        package.issues.push(source_issue(&identity));
        let mut pending: Vec<_> = packages
            .values()
            .flat_map(|package| package.dependencies.clone())
            .collect();
        while let Some(reference) = pending.pop() {
            let key = module_key(&reference);
            if packages.contains_key(&key) {
                continue;
            }
            if let Some(package) = self.adoption_source(&reference) {
                pending.extend(package.dependencies.clone());
                packages.insert(key, package.clone());
            }
        }
        let mut media_files = BTreeMap::new();
        for (key, package) in &mut packages {
            let reference = ModuleRef {
                id: package.definition.id.clone(),
                version: package.definition.version.clone(),
            };
            for asset in &package.assets {
                if let Some(bytes) = self.media_file_bytes(&reference, asset) {
                    let file = FileRef {
                        path: asset.path.clone(),
                        kind: DocumentKind::Asset,
                    };
                    if !package
                        .files
                        .iter()
                        .any(|declared| exact_data(declared, &file))
                        && self.archive_media_file(&asset.path).is_some()
                    {
                        package.files.push(file);
                    }
                    if package.files.iter().any(|file| {
                        file.path == asset.path && matches!(file.kind, DocumentKind::Asset)
                    }) {
                        media_files.insert((key.clone(), asset.path.as_str().to_owned()), bytes);
                    }
                }
            }
        }
        let definitions = packages
            .values()
            .map(|package| package.definition.clone())
            .collect::<Vec<_>>();
        // Recompute selected diagnostics with the explicit sample, never the
        // saved Scenario with the same display name/ID or its hidden seeds.
        let mut execution = self.current_draft();
        execution.scenarios = BTreeMap::from([(test_input.id.clone(), *test_input.clone())]);
        execution.project.scenario_refs = vec![test_input.id.clone()];
        execution.project.issues.retain(|issue| {
            !issue
                .object_ref
                .as_ref()
                .is_some_and(|object| matches!(object.kind, ObjectKind::Scenario))
        });
        refresh_business_issues_with_definitions(&mut execution, &definitions);
        let blocking = execution
            .project
            .issues
            .iter()
            .chain(test_input.issues.iter())
            .filter(|issue| {
                issue
                    .blocks
                    .iter()
                    .any(|purpose| matches!(purpose, CapabilityPurpose::Execution))
            })
            .cloned()
            .collect::<Vec<_>>();
        if !blocking.is_empty() {
            return Err(engine_failure(blocking));
        }
        execution.project.issues.push(source_issue(&identity));
        // Nominal professional bindings require the existing trusted producer
        // path; a sample does not gain that authority by cloning a Project.
        if current_consumer_bindings(&execution.project)
            .into_iter()
            .any(|binding| matches!(binding.mode, ProfessionalMode::CalculatedNominal))
        {
            return Err(unsupported(
                "Definition samples with calculated nominal bindings require their existing trusted professional execution consumer",
            ));
        }
        let run_id = RunId::try_from(format!(
            "{}:definition-cycle:{}:{}",
            self.host_instance_id.as_str(),
            self.epoch,
            self.next_task
        ))
        .unwrap();
        let directory = format!("session/definition-cycle/{:08}", self.next_task);
        let input = sf_simulation::EngineInput {
            run_id: run_id.clone(),
            input_ref: RunInputRef {
                project_id: execution.project.id.clone(),
                project_revision: execution.project.revision.clone(),
                scenario_id: test_input.id.clone(),
                scenario_revision: test_input.revision.clone(),
                frozen_project_file: file_ref(
                    &format!("{directory}/project.json"),
                    DocumentKind::Project,
                ),
                frozen_scenario_file: file_ref(
                    &format!("{directory}/scenario.json"),
                    DocumentKind::Scenario,
                ),
            },
            project: execution.project,
            scenario: *test_input,
            definitions,
            scope: execution.scenarios.values().next().unwrap().scope.clone(),
        };
        sf_simulation::check_inputs(&input).map_err(engine_failure)?;
        self.definition_cycle_inputs.insert(
            run_id,
            DefinitionCycleInput {
                input: input.clone(),
                identity: identity.clone(),
                definition,
                packages,
                media_files,
            },
        );
        Ok(self.enqueue_io(
            caller,
            TaskScope::Edit(EditScope::DefinitionDraft(identity.clone())),
            OperationName::RunScenario,
            identity.revision,
            None,
            IoInput::Run {
                input,
                supporting_scenarios: vec![],
            },
        ))
    }

    /// Called before finish_io borrows its task. This branch intentionally has
    /// no archive additions, dirty flag, content generation or history write.
    pub(super) fn finish_definition_cycle(
        &mut self,
        task_id: TaskId,
        input: sf_simulation::EngineInput,
        engine: sf_simulation::Engine,
    ) {
        let Some(task) = self.tasks.get_mut(&task_id) else {
            return;
        };
        if terminal(&task.state.task.state) {
            return;
        }
        let TaskScope::Edit(EditScope::DefinitionDraft(identity)) = &task.state.task.scope else {
            unreachable!("only definition sample completion routed here")
        };
        let mut result = engine.snapshot();
        let source = source_issue(identity);
        if !result.limitations.iter().any(|issue| exact_data(issue, &source)) {
            result.limitations.push(source);
        }
        task.state.task.state = result.computation_status.clone();
        task.state.task.result_ref = Some(run_result_file(&input));
        task.state.result = Some(TaskOutput::Run(run_summary(&result)));
        task.state.allowed_controls.clear();
        if task.epoch == self.epoch
            && self
                .project
                .as_ref()
                .is_some_and(|p| p.id == input.project.id)
            && self.definition_drafts.contains_key(&identity.id)
        {
            let scope = EditScope::DefinitionDraft(identity.clone());
            let history = StoredRunHistory {
                run_id: result.id.clone(),
                input_ref: result.input_ref.clone(),
                scope: scope.clone(),
                records: engine.item_history(),
                job_records: Some(engine.job_history()),
                port_outputs: Some(engine.port_output_history()),
            };
            self.runs.insert(
                result.id.clone(),
                HostRun {
                    result,
                    engine: Some(engine),
                    history: Some(history),
                    scope,
                    observation_cache: Mutex::new(Default::default()),
                },
            );
        }
    }

    /// Some(...) handles only actual DefinitionDraft frozen reads. Project
    /// reads continue down the original archive-backed exact-version path.
    pub(super) fn frozen_cycle_media_bytes(
        &self,
        run_id: &RunId,
        reference: &ModuleRef,
        asset: &AssetRef,
    ) -> Option<Vec<u8>> {
        self.definition_cycle_inputs
            .get(run_id)?
            .media_files
            .get(&(module_key(reference), asset.path.as_str().to_owned()))
            .cloned()
    }

    pub(super) fn frozen_definition_cycle_object(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: &GetObjectParams,
    ) -> Result<Option<DomainObject>, ApiError> {
        let ApiScope::DefinitionDraft(id) = scope else {
            return Ok(None);
        };
        let (run_id, at) = match &params.source {
            ObjectSource::RunSnapshot { run_id, at_ms }
            | ObjectSource::RunModuleVersion { run_id, at_ms, .. } => (run_id, at_ms),
            _ => return Ok(None),
        };
        // Only private sample Runs belong to this resolver. Preserve the
        // original frozen Project read admission (and error order) for formal
        // or unknown Runs; another real draft sample still uses get_run below.
        if !self
            .runs
            .get(run_id)
            .is_some_and(|run| matches!(run.scope, EditScope::DefinitionDraft(_)))
        {
            return Ok(None);
        }
        self.authorize_definition_cycle_project_read(caller)?;
        if !caller
            .allowed_operations
            .iter()
            .any(|op| matches!(op, OperationName::GetRun))
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Frozen definition sample reads require current GetRun authority",
                None,
            ));
        }
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
        if at.0.value() < summary.window.start_ms.0.value()
            || summary
                .computed_until_ms
                .as_ref()
                .is_none_or(|frontier| at.0.value() > frontier.0.value())
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Requested time is outside the computed definition sample frontier",
                Some("/params/source/data/at_ms"),
            ));
        }
        let frozen = self.definition_cycle_inputs.get(run_id).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Actual session definition sample inputs are absent",
                Some("/params/source"),
            )
        })?;
        if &frozen.identity.id != id {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Definition sample belongs to a different draft",
                None,
            ));
        }
        if let ObjectSource::RunModuleVersion { module_ref, .. } = &params.source {
            if !matches!(params.object_id.kind, ObjectKind::Module)
                || params.object_id.id.as_str() != module_ref.id.as_str()
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Frozen module source requires a matching exact Module identity",
                    Some("/params/source"),
                ));
            }
            return frozen
                .packages
                .get(&module_key(module_ref))
                .cloned()
                .map(|p| Some(DomainObject::Module(Box::new(p))))
                .ok_or_else(|| {
                    missing(
                        &params.object_id,
                        "Module is not in this sample's exact frozen definition set",
                    )
                });
        }
        match &params.object_id.kind {
            ObjectKind::Project
                if params.object_id.id.as_str() == frozen.input.project.id.as_str() =>
            {
                Ok(Some(DomainObject::Project(Box::new(
                    frozen.input.project.clone(),
                ))))
            }
            ObjectKind::Scenario
                if params.object_id.id.as_str() == frozen.input.scenario.id.as_str() =>
            {
                Ok(Some(DomainObject::Scenario(Box::new(
                    frozen.input.scenario.clone(),
                ))))
            }
            ObjectKind::DefinitionDraft
                if params.object_id.id.as_str() == frozen.identity.id.as_str() =>
            {
                Ok(Some(DomainObject::Definition(Box::new(
                    frozen.definition.clone(),
                ))))
            }
            ObjectKind::Item => Ok(None), // original computed ObjectsAt route
            _ => Err(missing(
                &params.object_id,
                "Object is not this definition sample's frozen input",
            )),
        }
    }
}
