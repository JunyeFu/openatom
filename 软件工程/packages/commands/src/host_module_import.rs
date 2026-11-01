//! Explicit exact-definition import into the active Project's local library.
use super::*;

fn module_ref(package: &ModulePackage) -> ModuleRef {
    ModuleRef {
        id: package.definition.id.clone(),
        version: package.definition.version.clone(),
    }
}
fn issue(package: &ModulePackage, code: &str, message: &str) -> Issue {
    Issue {
        code: code.into(),
        object_ref: Some(ObjectRef {
            kind: ObjectKind::Module,
            id: ObjectId::try_from(package.definition.id.as_str().to_owned()).unwrap(),
        }),
        field_path: None,
        message: message.into(),
        blocks: vec![],
    }
}
fn asset_gaps(archive: &ProjectArchive, package: &ModulePackage) -> Vec<Issue> {
    archive
        .manifest
        .missing
        .iter()
        .filter(|issue| {
            package.assets.iter().any(|asset| {
                !archive.files.contains_key(asset.path.as_str())
                    && issue.field_path.as_deref() == Some(asset.path.as_str())
                    && issue.object_ref.as_ref().is_some_and(|target| {
                        matches!(target.kind, ObjectKind::Asset)
                            && target.id.as_str() == asset.id.as_str()
                    })
            })
        })
        .cloned()
        .collect()
}
pub(super) fn proposal(
    archive: &ProjectArchive,
    current: &BTreeMap<(String, String), ModulePackage>,
    selection: SelectedFileRef,
) -> ImportDraft {
    let modules = archive
        .modules()
        .expect("archive reader validated module documents");
    let conflicts = modules
        .iter()
        .filter(|package| {
            current
                .get(&module_key(&module_ref(package)))
                .is_some_and(|existing| !exact_data(existing, *package))
        })
        .map(|package| {
            issue(
                package,
                "module_import.exact_version_conflict",
                "当前工程已包含内容不同的同一精确版本；请另存新版本，不能覆盖。",
            )
        })
        .collect();
    ImportDraft {
        input_selection: selection,
        proposed_definitions: modules.iter().map(module_ref).collect(),
        proposed_objects: vec![],
        missing: modules.iter().flat_map(|package| package.issues.iter().cloned().chain(asset_gaps(archive, package))).collect(),
        conflicts,
        required_human_choices: vec!["明确选择精确模块版本和需要的素材；只导入定义，不采用源工程实例、库存、运行结果或认可。".into()],
        proposed_data: modules.into_iter().map(|package| DomainObject::Module(Box::new(package))).collect(),
    }
}

impl ProjectHost {
    pub(super) fn adopt_imported_modules(
        &mut self,
        caller: &HostCallContext,
        task_id: &TaskId,
        choices: &ImportAcceptanceChoices,
    ) -> Result<EditCommit, ApiError> {
        if !matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "类别包采用需要实际界面明确确认。",
                None,
            ));
        }
        if !matches!(choices.chosen_policy, ImportPolicy::IntoProject)
            || !matches!(choices.departure, DepartureDecision::Stay)
            || !choices.objects_to_create.is_empty()
            || !choices.reused_bindings.item_types.is_empty()
            || !choices.reused_bindings.resources.is_empty()
            || !choices.reused_bindings.targets.is_empty()
            || !choices.reused_bindings.ports.is_empty()
        {
            return Err(unsupported(
                "此入口只向当前工程导入所选精确定义；实例采用和角色映射须另行明确操作。",
            ));
        }
        let before = self.current_draft();
        let task = self.owned_task(caller, &ApiScope::Project, task_id)?;
        if task.epoch != self.epoch
            || !matches!(
                task.state.originating_operation,
                OperationName::ImportPackage
            )
            || !matches!(task.state.task.state, ComputationStatus::Completed)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "请先在当前工程完成类别包读取。",
                None,
            ));
        }
        check_revision(
            &task.state.task.input_revision,
            &before.project.revision,
            "/params/commands/0/data/task_id",
        )?;
        let archive = task
            .staged
            .as_ref()
            .ok_or_else(|| error(ErrorCode::InvalidInput, "导入任务没有暂存包。", None))?;
        let source: BTreeMap<_, _> = archive
            .modules()?
            .into_iter()
            .map(|package| (module_key(&module_ref(&package)), package))
            .collect();
        let selected: BTreeSet<_> = choices.module_versions.iter().map(module_key).collect();
        if selected.is_empty()
            || selected.len() != choices.module_versions.len()
            || selected.iter().any(|key| !source.contains_key(key))
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "请选择暂存包中不重复的精确模块版本。",
                Some("/params/commands/0/data/choices/module_versions"),
            ));
        }
        let selected_assets: BTreeSet<_> = choices
            .selected_assets
            .iter()
            .map(|id| id.as_str())
            .collect();
        if selected_assets.len() != choices.selected_assets.len()
            || selected_assets.iter().any(|id| {
                !selected.iter().any(|key| {
                    source[key]
                        .assets
                        .iter()
                        .any(|asset| asset.id.as_str() == *id)
                })
            })
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "所选素材必须属于本次选中的实际模块。",
                Some("/params/commands/0/data/choices/selected_assets"),
            ));
        }
        let mut imports = LibraryImports::default();
        let mut missing_assets = vec![];
        for key in &selected {
            let original = &source[key];
            host_definitions::validate_definition(&original.definition)?;
            if self
                .modules
                .get(key)
                .is_some_and(|existing| !exact_data(existing, original))
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "同一精确模块版本已存在不同内容，不能覆盖。",
                    Some("/params/commands/0/data/choices/module_versions"),
                ));
            }
            let package = original.clone();
            for dependency in &package.dependencies {
                let dependency_key = module_key(dependency);
                if !selected.contains(&dependency_key)
                    && !self.modules.contains_key(&dependency_key)
                {
                    return Err(error(
                        ErrorCode::MissingReference,
                        "请同时选择所需的精确依赖版本，或先将其导入当前工程。",
                        Some("/params/commands/0/data/choices/module_versions"),
                    ));
                }
            }
            if package
                .assets
                .iter()
                .any(|asset| !selected_assets.contains(asset.id.as_str()))
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "请完整选择所选精确定义的随附素材；当前入口不支持部分素材采用，也不会暗中复制未选素材。",
                    Some("/params/commands/0/data/choices/selected_assets"),
                ));
            }
            let gaps = asset_gaps(archive, &package);
            if (!package.issues.is_empty() || !gaps.is_empty()) && !choices.accept_missing_as_draft
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "所选定义仍有检查项；须明确接受缺项草稿。",
                    Some("/params/commands/0/data/choices/accept_missing_as_draft"),
                ));
            }
            for asset in &package.assets {
                if !archive.manifest.documents.iter().any(|entry| {
                    entry.file_ref.path == asset.path
                        && matches!(entry.file_ref.kind, DocumentKind::Asset)
                        && matches!(entry.role, DocumentRole::Asset)
                        && entry.object_ref.id.as_str() == asset.id.as_str()
                }) {
                    return Err(error(
                        ErrorCode::MissingReference,
                        "素材须有实际随附 Asset 文件。",
                        Some(asset.path.as_str()),
                    ));
                }
                let right = package
                    .rights
                    .iter()
                    .find(|right| right.asset_ref == asset.id);
                if self
                    .modules
                    .values()
                    .chain(imports.modules.values())
                    .any(|other| {
                        other
                            .assets
                            .iter()
                            .any(|old| old.id == asset.id && !exact_data(old, asset))
                            || other
                                .rights
                                .iter()
                                .find(|old| old.asset_ref == asset.id)
                                .is_some_and(|old| {
                                    right.is_none_or(|right| !exact_data(old, right))
                                })
                    })
                {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "素材身份或许可与当前工程已存内容冲突，未导入任何版本。",
                        Some(asset.path.as_str()),
                    ));
                }
            }
            for gap in gaps {
                if !missing_assets.iter().any(|old| exact_data(old, &gap)) {
                    missing_assets.push(gap);
                }
            }
            let target = ObjectRef {
                kind: ObjectKind::Module,
                id: ObjectId::try_from(package.definition.id.as_str().to_owned()).unwrap(),
            };
            let mut additions = vec![];
            let mut companion_files = package.files.clone();
            for asset in &package.assets {
                if !companion_files.iter().any(|file| file.path == asset.path) {
                    companion_files.push(FileRef {
                        path: asset.path.clone(),
                        kind: DocumentKind::Asset,
                    });
                }
            }
            for file in &companion_files {
                if !archive.files.contains_key(file.path.as_str())
                    && missing_assets
                        .iter()
                        .any(|issue| issue.field_path.as_deref() == Some(file.path.as_str()))
                {
                    // Only explicitly accepted, archive-validated optional assets
                    // stay absent. Other companion files still require real bytes.
                    continue;
                }
                let bytes = archive.files.get(file.path.as_str()).ok_or_else(|| {
                    error(
                        ErrorCode::MissingReference,
                        "所选模块的伴随文件没有实际包内容。",
                        Some(file.path.as_str()),
                    )
                })?;
                let asset = package.assets.iter().find(|asset| asset.path == file.path);
                let mut entry = archive
                    .manifest
                    .documents
                    .iter()
                    .find(|entry| exact_data(&entry.file_ref, file))
                    .cloned()
                    .ok_or_else(|| {
                        error(
                            ErrorCode::MissingReference,
                            "所选模块的伴随文件没有对应的清单项。",
                            Some(file.path.as_str()),
                        )
                    })?;
                // Keep typed snapshot identity/revision; importing a definition
                // never turns its source Scenario into an editable input.
                entry.role = if asset.is_some() {
                    DocumentRole::Asset
                } else {
                    DocumentRole::SupportingFile
                };
                additions.push(ArchiveAddition {
                    entry,
                    bytes: bytes.clone(),
                });
            }
            if !self.modules.contains_key(key) {
                let hex = |value: &str| {
                    value
                        .as_bytes()
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>()
                };
                let path = RelativeFilePath::try_from(format!(
                    "documents/imported/{}/{}/module.json",
                    hex(&key.0),
                    hex(&key.1)
                ))
                .unwrap();
                additions.push(ArchiveAddition {
                    entry: DocumentEntry {
                        role: DocumentRole::ModuleDefinition,
                        module_ref: Some(module_ref(&package)),
                        object_ref: target,
                        revision: None,
                        file_ref: FileRef {
                            path,
                            kind: DocumentKind::Module,
                        },
                        required: true,
                    },
                    bytes: encode_document(&package),
                });
                imports.modules.insert(key.clone(), package);
            }
            for addition in additions {
                let path = addition.entry.file_ref.path.as_str();
                let existing = self
                    .additions
                    .iter()
                    .chain(imports.additions.values())
                    .find(|other| {
                        other
                            .entry
                            .file_ref
                            .path
                            .as_str()
                            .eq_ignore_ascii_case(path)
                    });
                let archived = self.backing.as_ref().and_then(|backing| {
                    backing
                        .manifest
                        .documents
                        .iter()
                        .find(|other| other.file_ref.path.as_str().eq_ignore_ascii_case(path))
                        .map(|entry| (entry, backing.files.get(entry.file_ref.path.as_str())))
                });
                if existing.is_some_and(|other| {
                    other.bytes != addition.bytes || !exact_data(&other.entry, &addition.entry)
                }) || archived.is_some_and(|(entry, bytes)| {
                    bytes != Some(&addition.bytes) || !exact_data(entry, &addition.entry)
                }) {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "所选伴随文件与当前工程不可变文件冲突，未导入任何版本。",
                        Some(path),
                    ));
                }
                if existing.is_none() && archived.is_none() {
                    imports.additions.insert(path.to_owned(), addition);
                }
            }
        }
        // Keep source package instances, current stock, Runs and recognition out.
        let mut candidate = before.clone();
        for issue in missing_assets {
            if !candidate
                .project
                .issues
                .iter()
                .any(|old| exact_data(old, &issue))
            {
                candidate.project.issues.push(issue);
            }
        }
        let highwater = finalize_draft(&mut candidate, &self.scenario_highwater)?;
        let mut definitions = self.modules.clone();
        definitions.extend(imports.modules.clone());
        refresh_business_issues_with_definitions(
            &mut candidate,
            &definitions
                .values()
                .map(|package| package.definition.clone())
                .collect::<Vec<_>>(),
        );
        validate_candidate(&candidate)?;
        self.history.module_import_boundary(&candidate);
        let mut changed_objects: Vec<_> = choices
            .module_versions
            .iter()
            .map(|reference| ObjectRef {
                kind: ObjectKind::Module,
                id: ObjectId::try_from(reference.id.as_str().to_owned()).unwrap(),
            })
            .collect();
        changed_objects.push(ObjectRef {
            kind: ObjectKind::Project,
            id: ObjectId::try_from(candidate.project.id.as_str().to_owned()).unwrap(),
        });
        changed_objects.extend(candidate.scenarios.values().map(|scenario| ObjectRef {
            kind: ObjectKind::Scenario,
            id: ObjectId::try_from(scenario.id.as_str().to_owned()).unwrap(),
        }));
        let result = EditCommit {
            project_id: candidate.project.id.clone(),
            scope: EditScope::Project(candidate.project.id.clone()),
            previous_revision: Some(before.project.revision),
            new_revision: candidate.project.revision.clone(),
            changed_objects,
            issues: candidate.project.issues.clone(),
            history: self.history.availability(),
        };
        self.project = Some(candidate.project);
        self.scenarios = candidate.scenarios;
        self.scenario_highwater = highwater;
        self.modules.extend(imports.modules);
        self.additions.extend(imports.additions.into_values());
        self.content_generation += 1;
        self.dirty = true;
        Ok(result)
    }
}
