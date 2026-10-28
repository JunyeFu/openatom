use super::*;
use serde_json::json;

type FrozenEvaluationInput = (
    ProjectState,
    ScenarioInput,
    Vec<ModuleDefinition>,
    RunResult,
);

/// Frame declared layout bounds for the report's A4 figure, without changing
/// saved views or the Run. Project placement uses the product's canonical m/Z-up
/// TRS hierarchy; source-media geometry is neither invented nor substituted.
fn report_layout_camera(project: &ProjectState) -> Result<CameraState, ApiError> {
    fn posed(mut v: [f64; 3], poses: &[&Pose]) -> [f64; 3] {
        fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
            [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
        }
        for pose in poses.iter().rev() {
            v = [v[0]*pose.scale.x.value(), v[1]*pose.scale.y.value(), v[2]*pose.scale.z.value()];
            let q = &pose.rotation;
            let xyz = [q.x.value(), q.y.value(), q.z.value()];
            let t = cross(xyz, v).map(|n| 2.*n);
            let u = cross(xyz, t);
            let translation = [pose.translation.x.value(), pose.translation.y.value(), pose.translation.z.value()];
            v = std::array::from_fn(|i| v[i] + q.w.value()*t[i] + u[i] + translation[i]);
        }
        v
    }
    let mut points = vec![];
    for hall in &project.halls {
        for x in [0., hall.width_m.value()] {
            for y in [0., hall.depth_m.value()] {
                for z in [0., hall.height_m.as_ref().map_or(0., |h| h.value())] {
                    points.push(posed([x,y,z], &[&hall.origin]));
                }
            }
        }
        for line in project.lines.iter().filter(|line| line.hall_id == hall.id) {
            for p in &line.region_polygon_m {
                points.push(posed([p.x.value(),p.y.value(),0.], &[&hall.origin,&line.transform]));
            }
        }
        for device in project.devices.iter().filter(|device| device.hall_id == hall.id) {
            let mut poses = vec![&hall.origin];
            if let Some(id) = &device.line_id {
                let Some(line) = project.lines.iter().find(|line| &line.id == id && line.hall_id == hall.id) else { continue; };
                poses.push(&line.transform);
            }
            poses.push(&device.pose);
            points.push(posed([0.;3], &poses));
            for p in &device.footprint_m {
                for z in [0.,device.height_m.as_ref().map_or(0., |h| h.value())] {
                    points.push(posed([p.x.value(),p.y.value(),z], &poses));
                }
            }
        }
        for location in project.locations.iter().filter(|location| location.hall_id.as_ref() == Some(&hall.id)) {
            points.push(posed([0.;3], &[&hall.origin,&location.position]));
        }
    }
    if points.is_empty() {
        return Ok(project.views.multi_hall_camera.clone());
    }
    let min: [f64;3] = std::array::from_fn(|i| points.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min));
    let max: [f64;3] = std::array::from_fn(|i| points.iter().map(|p| p[i]).fold(f64::NEG_INFINITY, f64::max));
    let center: [f64;3] = std::array::from_fn(|i| min[i] + (max[i]-min[i])/2.);
    let radius = (0..3).map(|i| ((max[i]-min[i])/2.).powi(2)).sum::<f64>().sqrt().max(0.5);
    // Same 42-degree vertical field as FactoryScene; the report viewport is
    // wider than tall. An enclosing sphere leaves all declared corners in view.
    let distance = radius / (21_f64.to_radians().sin()*0.9);
    let direction = [0.8,-1.,1.];
    let norm = 2.64_f64.sqrt();
    let vector = |values: [f64;3]| -> Result<Vec3,ApiError> {
        let number = |n| FiniteNumber::try_from(n).map_err(|_| error(ErrorCode::InvalidInput,"Report layout framing exceeds finite coordinates",Some("/project/halls")));
        Ok(Vec3{x:number(values[0])?,y:number(values[1])?,z:number(values[2])?})
    };
    Ok(CameraState {
        coordinate_frame:CoordinateFrame::RightHandedZUp, unit:LengthUnit::Meter,
        eye:vector(std::array::from_fn(|i| center[i]+direction[i]/norm*distance))?,
        target:vector(center)?, zoom:FiniteNumber::try_from(1.).unwrap(),
    })
}

#[cfg(test)]
mod report_camera_tests {
    use super::*;

    #[test]
    fn report_camera_frames_rotated_scaled_translated_hall_instead_of_saved_corner_view() {
        let document: Versioned<ProjectState> = serde_json::from_str(include_str!(
            "../tests/fixtures/evaluation/r12-publication/saved-project/data/g000001/project-state.json"
        )).unwrap();
        let mut project = document.data;
        project.lines.clear(); project.devices.clear(); project.locations.clear();
        project.halls.truncate(1);
        let number = |n| FiniteNumber::try_from(n).unwrap();
        let hall = &mut project.halls[0];
        hall.width_m=number(30.); hall.depth_m=number(20.); hall.height_m=Some(number(6.));
        hall.origin.translation=Vec3{x:number(100.),y:number(200.),z:number(10.)};
        hall.origin.scale=Vec3{x:number(2.),y:number(3.),z:number(1.)};
        hall.origin.rotation=Quaternion{x:number(0.),y:number(0.),z:number(0.5_f64.sqrt()),w:number(0.5_f64.sqrt())};
        let camera=report_layout_camera(&project).unwrap();
        let target=[camera.target.x.value(),camera.target.y.value(),camera.target.z.value()];
        for (actual,expected) in target.into_iter().zip([70.,230.,13.]) { assert!((actual-expected).abs()<1e-9); }
        let eye=[camera.eye.x.value(),camera.eye.y.value(),camera.eye.z.value()];
        let distance=(0..3).map(|i|(eye[i]-target[i]).powi(2)).sum::<f64>().sqrt();
        let toward: [f64;3]=std::array::from_fn(|i|(eye[i]-target[i])/distance);
        let right_norm=(toward[0].powi(2)+toward[1].powi(2)).sqrt();
        let right=[-toward[1]/right_norm,toward[0]/right_norm,0.];
        let up=[toward[1]*right[2]-toward[2]*right[1],toward[2]*right[0]-toward[0]*right[2],toward[0]*right[1]-toward[1]*right[0]];
        // Independently known world corners: 90-degree hall rotation maps its
        // scaled 30x20 floor to x40..100/y200..260, at elevations10..16.
        for x in [40.,100.] { for y in [200.,260.] { for z in [10.,16.] {
            let point=[x-target[0],y-target[1],z-target[2]];
            let dot=|axis:[f64;3]|(0..3).map(|i|point[i]*axis[i]).sum::<f64>();
            let depth=distance-dot(toward);
            assert!(depth>0.);
            assert!(dot(up).abs()<depth*21_f64.to_radians().tan()*0.95);
            assert!(dot(right).abs()<depth*21_f64.to_radians().tan()*(720./450.)*0.95);
        }}}
    }
}

pub(super) struct EvaluationWork {
    destination: PathBuf,
    directory: String,
    selection: SelectedFileRef,
    context: EvaluationReport,
    inputs: Vec<FrozenEvaluationInput>,
    scenarios: Vec<ScenarioRevisionRef>,
    metrics: MetricScope,
    formats: Vec<sf_reporting::EvaluationOutputFormat>,
    resources: Arc<sf_reporting::EvaluationResources>,
    draft: ProjectDraft,
    backing: Option<ProjectArchive>,
    additions: Vec<ArchiveAddition>,
    renderings: Vec<(String, host_render::RenderWork)>,
}

fn report_failure(issues: Vec<Issue>) -> ApiError {
    let code = if issues.iter().any(|issue| issue.code.contains("cancelled")) {
        ErrorCode::Cancelled
    } else if issues
        .iter()
        .any(|issue| issue.code.contains("missing_font") || issue.code.contains("missing_asset"))
    {
        ErrorCode::MissingReference
    } else {
        ErrorCode::InvalidInput
    };
    let mut failure = engine_failure(issues);
    failure.code = code;
    failure.message =
        "Evaluation report production failed; no report directory was committed".into();
    failure
}

fn missing_report(code: &str, message: &str) -> Issue {
    Issue {
        code: format!("evaluation.{code}"),
        object_ref: None,
        field_path: None,
        message: message.into(),
        blocks: vec![],
    }
}

impl EvaluationWork {
    pub(super) fn run(
        mut self,
        cancelled: &AtomicBool,
        on_progress: &mut dyn FnMut(TaskProgress),
    ) -> Result<ExportEvaluation, ApiError> {
        let total = self.renderings.len() + 4;
        let mut completed = 0;
        let mut progress = |phase: &str, completed: usize| -> Result<(), ApiError> {
            if cancelled.load(Ordering::Acquire) {
                return Err(error(ErrorCode::Cancelled, "Evaluation export cancelled", None));
            }
            on_progress(TaskProgress {
                phase: phase.into(),
                processed: SafeUint::try_from(completed as u64).unwrap(),
                total: Some(SafeUint::try_from(total as u64).unwrap()),
                message: Some("计数为已完成实际阶段，不代表耗时百分比。".into()),
            });
            Ok(())
        };
        let mut resources = sf_reporting::EvaluationResources {
            cjk_font: self.resources.cjk_font.clone(),
            font_license: self.resources.font_license.clone(),
            files: self.resources.files.clone(),
        };
        for (index, (title, work)) in self.renderings.into_iter().enumerate() {
            progress("生成实际画面", completed)?;
            let capture = work.capture_for_report(cancelled)?;
            let file = AssetRef {
                id: capture.image_id,
                path: RelativeFilePath::try_from(format!("images/view-{:04}.png", index + 1)).unwrap(),
                kind: AssetKind::Png,
            };
            resources.files.insert(file.path.as_str().to_owned(), capture.bytes);
            // This grant belongs to the generated report image, not to the
            // original media files. Their separate rights remain unchanged.
            self.context.asset_rights.push(AssetRights {
                asset_ref: file.id.clone(),
                source: Some(format!("Actual software render: {title}")),
                license: Some("Generated report visualization; original source asset license declarations remain separate".into()),
                redistribution_confirmed: true,
                missing_reason: None,
            });
            self.context.missing_conditions.extend(capture.issues);
            self.context.layout_and_schedule.push(ReportImage {
                title,
                evidence_kind: if capture.frame.request.view.run_id.is_some() {
                    EvidenceKind::ActualRunView
                } else { EvidenceKind::ActualPresentation },
                scene_ref: RenderedSceneRef {
                    evidence_origin: EvidenceOrigin::ActualSoftware,
                    id: capture.scene_id,
                    file,
                    scene_revision: capture.frame.scope_revision.clone(),
                    view: capture.frame.request.view,
                },
            });
            completed += 1;
        }
        progress("整理报告数据", completed)?;
        let mut report = sf_reporting::build_evaluation(
            self.context,
            &self.inputs,
            &self.metrics.keys,
            &self.metrics.objects,
            &self.metrics.acceptance,
        )
        .map_err(report_failure)?;
        completed += 1;
        // The producer derives Run-bound references. Keep explicitly selected
        // current/historical design inputs too; their metrics are never invented.
        for selected in self.scenarios {
            if !report
                .data
                .scenario_refs
                .iter()
                .any(|actual| exact_data(actual, &selected))
            {
                report.data.scenario_refs.push(selected);
            }
        }
        progress("生成所选报告格式", completed)?;
        let rendered =
            sf_reporting::render_evaluation(&report, &self.formats, &resources, cancelled)
                .map_err(report_failure)?;
        completed += 1;
        progress("整理可编辑工程", completed)?;
        let archive = crate::project_store::snapshot_project_archive(
            &self.draft,
            self.backing.as_ref(),
            &self.additions,
        )?;
        completed += 1;
        let files: Vec<_> = rendered
            .into_iter()
            .map(|file| {
                Ok((
                    RelativeFilePath::try_from(file.relative_path)
                        .map_err(|message| error(ErrorCode::InvalidInput, &message, None))?,
                    file.bytes,
                ))
            })
            .collect::<Result<_, ApiError>>()?;
        progress("提交交付文件", completed)?;
        crate::project_store::write_evaluation_directory(
            &self.destination,
            &self.directory,
            &archive,
            &files,
            cancelled,
        )?;
        // Publication already committed. A later cancellation cannot undo it.
        on_progress(TaskProgress {
            phase: "交付文件已提交".into(),
            processed: SafeUint::try_from(total as u64).unwrap(),
            total: Some(SafeUint::try_from(total as u64).unwrap()),
            message: Some("等待宿主接收真实任务终态。".into()),
        });
        let mut output_files: Vec<_> = files
            .iter()
            .map(|(path, _)| {
                file_ref(
                    &format!("{}/{}", self.directory, path.as_str()),
                    if path.as_str() == "report.json" {
                        DocumentKind::Evaluation
                    } else {
                        DocumentKind::Asset
                    },
                )
            })
            .collect();
        output_files.push(file_ref(
            &format!("{}/project/project.json", self.directory),
            DocumentKind::Project,
        ));
        output_files.extend(archive.manifest.documents.iter().map(|entry| {
            FileRef {
                path: RelativeFilePath::try_from(format!(
                    "{}/project/{}",
                    self.directory,
                    entry.file_ref.path.as_str()
                ))
                .unwrap(),
                kind: entry.file_ref.kind.clone(),
            }
        }));
        let limitations = report
            .data
            .missing_conditions
            .iter()
            .chain(&report.data.limitations)
            .cloned()
            .collect();
        Ok(ExportEvaluation {
            destination: self.selection,
            context: report.data,
            files: output_files,
            limitations,
        })
    }
}

impl ProjectHost {
    fn evaluation_renderings(
        &self,
        caller: &HostCallContext,
        inputs: &[FrozenEvaluationInput],
        scenarios: &[ScenarioInput],
        missing: &mut Vec<Issue>,
    ) -> Result<Vec<(String, host_render::RenderWork)>, ApiError> {
        if !self.rendering_available() {
            missing.push(missing_report("actual_render_missing",
                "本Host尚无实际渲染器图像；报告不使用AI预览或虚构布局/生产画面。"));
            return Ok(vec![]);
        }
        let project = self.project.as_ref().expect("admitted Project");
        let id = TaskId::try_from(format!("{}:task:{}",self.host_instance_id.as_str(),self.next_task)).unwrap();
        let mut renderings = vec![];
        let mut add = |title: String, frozen: &ProjectState, kind: RenderViewKind,
            run: Option<&RunResult>, scenario: Option<&ScenarioInput>| -> Result<(), ApiError> {
            let params = RenderViewParams {
                revision: project.revision.clone(),
                view: RenderViewSpec {
                    kind, canvas: CanvasKind::MultiHallCanvas, hall_id: None,
                    object_ref: scenario.map(|s|ObjectRef {kind:ObjectKind::Scenario,id:ObjectId::try_from(s.id.as_str().to_owned()).unwrap()}),
                    project_revision: frozen.revision.clone(),
                    run_id: run.map(|r|r.id.clone()),
                    view_time_ms: run.and_then(|r|r.computed_until_ms.clone()),
                    lod: frozen.views.lod.clone(),
                    camera: report_layout_camera(frozen)?,
                },
                object_id: None, preview: None, delivery: RenderDelivery::CaptureOnly,
                module_ref: None, comparison: None,
            };
            let viewport = if matches!(params.view.kind, RenderViewKind::TimelineSnapshot) {(720,320)} else {(720,450)};
            let work = self.prepare_render_work(caller,&ApiScope::Project,params,id.clone())?.with_report_viewport(viewport);
            renderings.push((title,work));
            Ok(())
        };
        add(format!("导出工程布局 · {} r{}",project.id.as_str(),project.revision.0.value()),project,RenderViewKind::MultiHallCanvas,None,None)?;
        for (frozen, _, _, run) in inputs {
            let Some(at) = &run.computed_until_ms else {
                missing.push(missing_report("run_render_unavailable",
                    &format!("Run {} 尚无已计算时点；不生成实际画布或时间轴。",run.id.as_str())));
                continue;
            };
            for (kind,label) in [(RenderViewKind::MultiHallCanvas,"冻结布局"),(RenderViewKind::TimelineSnapshot,"实际时间轴")] {
                add(format!("Run {} · {label} · 工程 r{} · {} ms",run.id.as_str(),frozen.revision.0.value(),at.0.value()),frozen,kind,Some(run),None)?;
            }
        }
        if inputs.is_empty() {
            for scenario in scenarios {
                if !exact_data(&scenario.project_revision,&project.revision) {
                    missing.push(missing_report("historical_input_timeline_unavailable",
                        &format!("方案 {} r{} 没有匹配当前导出工程修订的输入时间轴；不与当前布局拼接。",scenario.id.as_str(),scenario.revision.0.value())));
                    continue;
                }
                add(format!("输入时间轴 · {} r{}",scenario.id.as_str(),scenario.revision.0.value()),project,RenderViewKind::TimelineSnapshot,None,Some(scenario))?;
            }
        }
        Ok(renderings)
    }

    /// Trusted in-process font/license/attachment injection, frozen per task.
    /// Missing resources remain explicit producer errors, never silent fallback.
    pub fn set_evaluation_resources(&mut self, resources: sf_reporting::EvaluationResources) {
        self.evaluation_resources = Arc::new(resources);
    }

    fn evaluation_inputs(&self, run_ids: &[RunId]) -> Result<Vec<FrozenEvaluationInput>, ApiError> {
        if run_ids.iter().collect::<BTreeSet<_>>().len() != run_ids.len() {
            return Err(error(
                ErrorCode::InvalidInput,
                "Evaluation Run IDs must be distinct",
                Some("/params/run_refs"),
            ));
        }
        let current = self.project.as_ref().expect("admitted Project");
        for run in run_ids.iter().filter_map(|id| self.runs.get(id)) {
            if !matches!(&run.scope,EditScope::Project(id) if id==&current.id && run.result.input_ref.project_id==current.id)
            {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Selected evaluation Run is outside the current Project scope",
                    None,
                ));
            }
        }
        run_ids
            .iter()
            .map(|id| {
                let run = self.runs.get(id).ok_or_else(|| {
                    error(
                        ErrorCode::MissingReference,
                        "Evaluation Run is absent",
                        Some("/params/run_refs"),
                    )
                })?;
                let summary = run_summary(&run.result);
                let project = self.frozen_run_project(&summary)?;
                let scenario = self.frozen_run_scenario(&summary)?;
                let mut seen = BTreeSet::new();
                let definitions = project
                    .module_refs
                    .iter()
                    .chain(
                        project
                            .devices
                            .iter()
                            .flat_map(|device| &device.route_operation_refs),
                    )
                    .filter(|reference| seen.insert(module_key(reference)))
                    .map(|reference| {
                        self.frozen_run_module(&summary, reference)
                            .map(|package| package.definition)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((project, scenario, definitions, run.result.clone()))
            })
            .collect()
    }

    fn evaluation_scenario(
        &self,
        reference: &ScenarioReference,
    ) -> Result<ScenarioInput, ApiError> {
        if let Some(scenario) = self
            .scenarios
            .get(&reference.scenario_id)
            .filter(|scenario| exact_data(&scenario.revision, &reference.revision))
        {
            return Ok(scenario.clone());
        }
        // Prefer actual Project Run inputs, then exact declared Scenario files
        // in this Project archive. DefinitionDraft Run files remain excluded.
        for run in self.runs.values().filter(|run|matches!(&run.scope,EditScope::Project(id) if self.project.as_ref().is_some_and(|project|id==&project.id))) {
            if run.result.input_ref.scenario_id==reference.scenario_id && exact_data(&run.result.input_ref.scenario_revision,&reference.revision) {
                return self.frozen_run_scenario(&run_summary(&run.result));
            }
        }
        if let Some(addition) = self.evaluation_historical_scenario_file(reference) {
            let document: Versioned<ScenarioInput> = serde_json::from_slice(&addition.bytes)
                .map_err(|failure| {
                    error(
                        ErrorCode::InvalidInput,
                        &failure.to_string(),
                        Some("/params/scenario_refs"),
                    )
                })?;
            if document.data.id == reference.scenario_id
                && exact_data(&document.data.revision, &reference.revision)
            {
                return Ok(document.data);
            }
            return Err(error(
                ErrorCode::InvalidInput,
                "Historical Scenario bytes do not match their actual declared identity",
                Some("/params/scenario_refs"),
            ));
        }
        Err(error(
            ErrorCode::MissingReference,
            "Exact current or authorized historical Scenario revision is absent",
            Some("/params/scenario_refs"),
        ))
    }

    fn evaluation_historical_scenario_file(
        &self,
        reference: &ScenarioReference,
    ) -> Option<ArchiveAddition> {
        let excluded: BTreeSet<_> = self
            .runs
            .values()
            .filter(|run| !matches!(run.scope, EditScope::Project(_)))
            .map(|run| run.result.input_ref.frozen_scenario_file.path.as_str())
            .collect();
        let selected = |entry: &DocumentEntry| {
            matches!(entry.file_ref.kind, DocumentKind::Scenario)
                && matches!(entry.object_ref.kind, ObjectKind::Scenario)
                && entry.object_ref.id.as_str() == reference.scenario_id.as_str()
                && entry
                    .revision
                    .as_ref()
                    .is_some_and(|revision| exact_data(revision, &reference.revision))
                && !excluded.contains(entry.file_ref.path.as_str())
        };
        let found = self
            .additions
            .iter()
            .find(|addition| selected(&addition.entry))
            .cloned()
            .or_else(|| {
                self.backing
                    .as_ref()?
                    .manifest
                    .documents
                    .iter()
                    .find(|entry| selected(entry))
                    .and_then(|entry| {
                        self.backing
                            .as_ref()?
                            .files
                            .get(entry.file_ref.path.as_str())
                            .map(|bytes| ArchiveAddition {
                                entry: entry.clone(),
                                bytes: bytes.clone(),
                            })
                    })
            });
        found.map(|mut addition| {
            if matches!(addition.entry.role, DocumentRole::ScenarioInput) {
                addition.entry.role = DocumentRole::SupportingFile;
            }
            addition
        })
    }

    pub(super) fn export_evaluation(
        &mut self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: ExportEvaluationParams,
    ) -> Result<TaskHandle, ApiError> {
        if !matches!(scope, ApiScope::Project) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Evaluation export requires Project scope",
                Some("/scope"),
            ));
        }
        let draft = self.current_draft();
        let project = &draft.project;
        check_revision(&params.revision, &project.revision, "/params/revision")?;
        if params.formats.is_empty() {
            return Err(error(
                ErrorCode::InvalidInput,
                "Select at least one evaluation output format",
                Some("/params/formats"),
            ));
        }
        if matches!(params.report_mode, ReportMode::DesignDescription)
            && !params.run_refs.is_empty()
            || matches!(params.report_mode, ReportMode::SimulationEvaluation)
                && params.run_refs.is_empty()
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Report mode and explicit selected Run references disagree",
                Some("/params/report_mode"),
            ));
        }
        let inputs = self.evaluation_inputs(&params.run_refs)?;
        let mut selected = vec![];
        let mut keys = BTreeSet::new();
        for reference in &params.scenario_refs {
            if !keys.insert((reference.scenario_id.clone(), reference.revision.0.value())) {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Evaluation Scenario references must be distinct",
                    Some("/params/scenario_refs"),
                ));
            }
            selected.push(self.evaluation_scenario(reference)?);
        }
        let destination = self.selected_directory(
            caller,
            &params.selected_destination_ref,
            &FilePurpose::ExportEvaluation,
        )?;
        let mut conclusions=vec!["导出修订记录当前草稿；每个实际Run仍使用自己的冻结输入、观察窗和精确模块，不读取当前费率替换历史费率。".into()];
        conclusions.push(format!("导出工程的来源/模板/场地/班次记录（不是本次重新检查的结论）：{}",json!({"title":project.title,"sources":project.sources,"template_adoptions":project.template_adoptions,"halls":project.halls,"calendars":project.calendars,"setup_policies":project.setup_policies})));
        // Reading reports need the professional decision and its exact source,
        // not every panel transform. The complete immutable results and pose
        // history remain in the accompanying editable project at result_ref.
        let professional_results: Vec<_> = draft.professional.results.values().map(|result| {
            json!({
                "evidence_origin": result.evidence_origin,
                "artifact_id": result.artifact_id,
                "job_ref": result.job_ref,
                "subject_ref": result.subject_ref,
                "outcome": result.outcome,
                "checks": result.checks,
                "manufacturing_files": result.manufacturing_files,
                "material_counts": result.material_counts,
                "budget_used": result.budget_used,
                "supported_scope": result.supported_scope,
                "limitations": result.limitations,
                "process_pose_sample_count": result.process_poses.len(),
                "result_ref": draft.professional.artifacts.get(&result.artifact_id).map(|artifact| &artifact.result_ref)
            })
        }).collect();
        conclusions.push(format!("保存的专业成果/人工认可/当前采用事实（不据来源标签新增运行授权）：{}",json!({"artifacts":draft.professional.artifacts.values().collect::<Vec<_>>(),"acknowledgments":project.acknowledgments,"current_bindings":current_consumer_bindings(project),"results":professional_results})));
        if !draft.professional.results.is_empty() {
            conclusions.push("专业过程姿态按样本数摘要；完整结果、逐面板姿态和制造文件按上述精确引用保存在随附可编辑工程的 project/ 目录中。报告摘要不改变计算结论、认可状态或当前绑定。".into());
        }
        let mut assumptions = project.parameter_sources.clone();
        for scenario in &selected {
            conclusions.push(format!(
                "明确选择的方案输入快照：{}",
                serde_json::to_string(scenario).unwrap()
            ));
            assumptions.extend(scenario.parameter_overrides.iter().map(|parameter| {
                ParameterSource {
                    target: parameter.target.clone(),
                    field_path: parameter.path.clone(),
                    value: Some(parameter.value.clone()),
                    basis: parameter.basis.clone(),
                    source_ref: parameter.source_ref.clone(),
                }
            }));
        }
        for (project, _, _, run) in &inputs {
            conclusions.push(format!("Run {} 的冻结来源/模板/场地/班次/专业采用记录：{}",run.id.as_str(),json!({"revision":project.revision,"sources":project.sources,"template_adoptions":project.template_adoptions,"halls":project.halls,"calendars":project.calendars,"generation_context":project.generation_context,"acknowledgments":project.acknowledgments,"current_bindings":current_consumer_bindings(project)})));
        }
        let mut missing = project.issues.clone();
        missing.extend(
            selected
                .iter()
                .flat_map(|scenario| scenario.issues.iter())
                .cloned(),
        );
        let renderings = self.evaluation_renderings(caller, &inputs, &selected, &mut missing)?;
        if project
            .generation_context
            .as_ref()
            .and_then(|context| context.draft_check.as_ref())
            .is_none()
        {
            missing.push(missing_report(
                "layout_check_missing",
                "导出工程未保存实际静态场地检查结果，不能据显示尺寸宣称布局通过。",
            ));
        }
        let rights: BTreeMap<_, _> = self
            .modules
            .values()
            .flat_map(|package| &package.rights)
            .map(|right| (right.asset_ref.clone(), right.clone()))
            .chain(
                self.backing
                    .iter()
                    .flat_map(|archive| archive.manifest.assets.iter())
                    .map(|right| (right.asset_ref.clone(), right.clone())),
            )
            .collect();
        let references: Vec<_> = selected
            .iter()
            .map(|scenario| ScenarioRevisionRef {
                id: scenario.id.clone(),
                revision: scenario.revision.clone(),
            })
            .collect();
        let context = EvaluationReport {
            id: ObjectId::try_from(format!(
                "{}:evaluation:{}",
                self.host_instance_id.as_str(),
                self.next_task
            ))
            .unwrap(),
            mode: params.report_mode,
            project_id: project.id.clone(),
            project_revision: project.revision.clone(),
            scenario_refs: references.clone(),
            run_refs: params.run_refs,
            time_reference: Some(project.time_reference.clone()),
            observation_window: selected
                .first()
                .filter(|first| {
                    selected
                        .iter()
                        .all(|scenario| exact_data(&scenario.window, &first.window))
                })
                .map(|scenario| scenario.window.clone()),
            acceptance_scope: params.metric_scope.acceptance.clone(),
            question_and_scope: format!(
                "工程「{}」：按明确选择的方案/实际Run评估目的接受量、时间、资源和条件成本；未选择Run时仅说明设计。",
                project.title
            ),
            input_assumptions: assumptions,
            module_versions: project.module_refs.clone(),
            layout_and_schedule: vec![],
            scenario_comparison: None,
            conclusions,
            missing_conditions: missing,
            supported_scope: vec![
                "实际冻结模型输入和所选Run的模拟评估；不将模拟表现当现场观测或软件性能实测。"
                    .into(),
            ],
            limitations: vec![],
            editable_project_ref: Some(file_ref("project/project.json", DocumentKind::Project)),
            asset_rights: rights.into_values().collect(),
            generation_context: project.generation_context.clone(),
            run_evaluations: vec![],
        };
        let formats = params
            .formats
            .into_iter()
            .map(|format| match format {
                EvaluationFormat::Pdf => sf_reporting::EvaluationOutputFormat::Pdf,
                EvaluationFormat::Html => sf_reporting::EvaluationOutputFormat::Html,
                EvaluationFormat::Csv => sf_reporting::EvaluationOutputFormat::Csv,
                EvaluationFormat::Json => sf_reporting::EvaluationOutputFormat::Json,
            })
            .collect();
        let mut additions = self.additions.clone();
        for scenario in &selected {
            if !self
                .scenarios
                .get(&scenario.id)
                .is_some_and(|current| exact_data(&current.revision, &scenario.revision))
            {
                let reference = ScenarioReference {
                    scenario_id: scenario.id.clone(),
                    revision: scenario.revision.clone(),
                };
                if let Some(addition) = self.evaluation_historical_scenario_file(&reference) {
                    if !additions.iter().any(|existing| {
                        existing.entry.file_ref.path == addition.entry.file_ref.path
                    }) {
                        additions.push(addition);
                    }
                }
            }
        }
        let work = EvaluationWork {
            destination,
            directory: format!("evaluation-{:08}", self.next_task),
            selection: params.selected_destination_ref.clone(),
            context,
            inputs,
            scenarios: references,
            metrics: params.metric_scope,
            formats,
            resources: self.evaluation_resources.clone(),
            draft,
            backing: self.backing.clone(),
            additions,
            renderings,
        };
        Ok(self.enqueue_io(
            caller,
            TaskScope::Edit(EditScope::Project(work.draft.project.id.clone())),
            OperationName::ExportEvaluation,
            params.revision,
            Some(params.selected_destination_ref),
            IoInput::Evaluation(Box::new(work)),
        ))
    }
}
