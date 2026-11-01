//! Private UI projection of actual immutable professional files in this Host.
use super::*;
use serde::de::DeserializeOwned;

impl ProjectHost {
    /// List only stored packaging records in the admitted scope; no file IO or decoding.
    pub fn list_stored_packaging_artifacts(
        &self,
        session_id: &ObjectId,
        project_id: &ProjectId,
        scope: &ApiScope,
        scope_revision: &Revision,
    ) -> Result<Vec<ArtifactId>, ApiError> {
        let documents =
            self.stored_packaging_scope(session_id, project_id, scope, scope_revision)?;
        Ok(documents
            .artifacts
            .values()
            .filter(|artifact| {
                matches!(artifact.input_ref.kind, DocumentKind::PackagingJob)
                    && matches!(artifact.result_ref.kind, DocumentKind::PackagingResult)
            })
            .map(|artifact| artifact.id.clone())
            .collect())
    }

    /// Tuple order is Artifact metadata, decoded exact result, decoded exact input.
    /// Read authority is independent of producer/acknowledgment/adoption authority.
    pub fn read_stored_packaging_artifact(
        &self,
        session_id: &ObjectId,
        project_id: &ProjectId,
        scope: &ApiScope,
        scope_revision: &Revision,
        artifact_id: &ArtifactId,
    ) -> Result<(ArtifactRecord, PackagingResult, PackageStructureJob), ApiError> {
        let documents =
            self.stored_packaging_scope(session_id, project_id, scope, scope_revision)?;
        self.stored_packaging_triple(documents, scope, artifact_id)
    }

    /// Actual UI-only preview of one file from this exact saved result. This
    /// does not authorize export, regenerate geometry, or accept an arbitrary path.
    pub fn read_stored_manufacturing_file(
        &self,
        session_id: &ObjectId,
        project_id: &ProjectId,
        scope: &ApiScope,
        scope_revision: &Revision,
        artifact_id: &ArtifactId,
        file: &FileRef,
    ) -> Result<Vec<u8>, ApiError> {
        let documents =
            self.stored_packaging_scope(session_id, project_id, scope, scope_revision)?;
        let (_, result, _) = self.stored_packaging_triple(documents, scope, artifact_id)?;
        if !result
            .manufacturing_files
            .iter()
            .any(|sheet| exact_data(&sheet.file, file))
        {
            return Err(error(
                ErrorCode::MissingReference,
                "FileRef is not listed by this exact stored manufacturing result",
                Some("/file"),
            ));
        }
        self.stored_packaging_bytes(documents, scope, file, "/file")
            .map(<[u8]>::to_vec)
    }

    /// Shared exact-file resolution after the caller's own scope admission.
    pub(super) fn stored_packaging_triple(
        &self,
        documents: &ProfessionalDocuments,
        scope: &ApiScope,
        artifact_id: &ArtifactId,
    ) -> Result<(ArtifactRecord, PackagingResult, PackageStructureJob), ApiError> {
        let artifact = documents.artifacts.get(artifact_id).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Artifact is not stored in this actual scope",
                Some("/artifact_id"),
            )
        })?;
        if !matches!(artifact.input_ref.kind, DocumentKind::PackagingJob)
            || !matches!(artifact.result_ref.kind, DocumentKind::PackagingResult)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Artifact must reference a PackagingJob and PackagingResult",
                Some("/artifact_id"),
            ));
        }
        let input: PackageStructureJob = self.stored_packaging_document(
            documents,
            scope,
            &artifact.input_ref,
            "/artifact/input_ref",
        )?;
        let result: PackagingResult = self.stored_packaging_document(
            documents,
            scope,
            &artifact.result_ref,
            "/artifact/result_ref",
        )?;
        if artifact.id != result.artifact_id
            || result.job_ref.id != input.id
            || !exact_data(&result.job_ref.revision, &input.revision)
            || !exact_data(&artifact.subject_ref, &input.subject_ref)
            || !exact_data(&result.subject_ref, &input.subject_ref)
            || !exact_data(&artifact.actual_outcome, &result.outcome)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Stored Artifact, exact input, result and subject versions disagree",
                Some("/artifact_id"),
            ));
        }
        Ok((artifact.clone(), result, input))
    }

    fn stored_packaging_scope(
        &self,
        session_id: &ObjectId,
        project_id: &ProjectId,
        scope: &ApiScope,
        scope_revision: &Revision,
    ) -> Result<&ProfessionalDocuments, ApiError> {
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
                "Stored professional files require the actual UI session",
                None,
            ));
        }
        self.admit(
            caller,
            &Some(project_id.clone()),
            scope,
            &OperationName::GetObject,
        )?;
        let (actual_revision, documents) = match scope {
            ApiScope::Project => (&self.project.as_ref().unwrap().revision, &self.professional),
            ApiScope::DefinitionDraft(id) => {
                self.admit(
                    caller,
                    &Some(project_id.clone()),
                    &ApiScope::Project,
                    &OperationName::GetObject,
                )?;
                let draft = &self.definition_drafts[id];
                (&draft.revision, &draft.professional)
            }
            ApiScope::Workspace => {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Stored artifacts require Project or DefinitionDraft scope",
                    Some("/scope"),
                ));
            }
        };
        check_revision(scope_revision, actual_revision, "/scope_revision")?;
        Ok(documents)
    }

    pub(super) fn stored_packaging_document<T: DeserializeOwned>(
        &self,
        documents: &ProfessionalDocuments,
        scope: &ApiScope,
        reference: &FileRef,
        pointer: &str,
    ) -> Result<T, ApiError> {
        let bytes = self.stored_packaging_bytes(documents, scope, reference, pointer)?;
        serde_json::from_slice::<Versioned<T>>(bytes).map(|document| document.data).map_err(|failure| error(
            ErrorCode::InvalidInput,
            &format!("Stored professional file is not supported complete Versioned data: {failure}"),
            Some(pointer),
        ))
    }

    /// Same exact indexed/file-map closure for JSON and actual SVG bytes.
    pub(super) fn stored_packaging_bytes<'a>(
        &'a self,
        documents: &'a ProfessionalDocuments,
        scope: &ApiScope,
        reference: &FileRef,
        pointer: &str,
    ) -> Result<&'a [u8], ApiError> {
        let bytes = documents
            .files
            .get(reference.path.as_str())
            .map(Vec::as_slice)
            .or_else(|| {
                // Draft professional files are session-owned. A Project archive or
                // another draft cannot supply missing bytes for that scope.
                if !matches!(scope, ApiScope::Project) {
                    return None;
                }
                self.additions
                    .iter()
                    .find(|file| exact_data(&file.entry.file_ref, reference))
                    .map(|file| file.bytes.as_slice())
                    .or_else(|| {
                        let archive = self.backing.as_ref()?;
                        archive
                            .manifest
                            .documents
                            .iter()
                            .find(|entry| exact_data(&entry.file_ref, reference))?;
                        archive
                            .files
                            .get(reference.path.as_str())
                            .map(Vec::as_slice)
                    })
            })
            .ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Actual declared professional file bytes are absent",
                    Some(pointer),
                )
            })?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ui() -> ObjectId {
        "stored-unit-ui".to_owned().try_into().unwrap()
    }
    fn call(host: &mut ProjectHost, operation: &str, params: Value) -> Value {
        let project = host.project.as_ref().map(|project| project.id.clone());
        let request = serde_json::from_value(json!({"api_version":"0.1","request_id":"stored-unit","project_id":project,"scope":{"kind":if project.is_some(){"project"}else{"workspace"}},"operation":operation,"params":params})).unwrap();
        let response = host.dispatch(&ui(), request);
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        response["outcome"]["data"].clone()
    }
    fn actual() -> (ProjectHost, ArtifactId) {
        let mut host = ProjectHost::new("stored-unit-host".to_owned().try_into().unwrap(), ui());
        call(
            &mut host,
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"stored actual","time_reference":{"label":"explicit"},"departure":"stay"}}]}),
        );
        let cases: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/production/packaging-acceptance-cases.json"
        ))
        .unwrap();
        let job = cases["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == "HOLD01")
            .unwrap()["job"]["data"]
            .clone();
        let revision = host.project.as_ref().unwrap().revision.clone();
        let task:TaskHandle=serde_json::from_value(call(&mut host,"compute_packaging",json!({"packaging_job":job,"input_revision":revision,"computation_budget":job["budget"]}))).unwrap();
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        host.finish_io(work.pop().unwrap().run());
        let state: TaskState =
            serde_json::from_value(call(&mut host, "get_task", json!({"task_id":task.id})))
                .unwrap();
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let Some(TaskOutput::Packaging(result)) = state.result else {
            panic!("actual packaging")
        };
        assert!(matches!(result.outcome, CheckOutcome::Pass));
        (host, result.artifact_id)
    }
    fn read(
        host: &ProjectHost,
        artifact: &ArtifactId,
    ) -> Result<(ArtifactRecord, PackagingResult, PackageStructureJob), ApiError> {
        let project = host.project.as_ref().unwrap();
        host.read_stored_packaging_artifact(
            &ui(),
            &project.id,
            &ApiScope::Project,
            &project.revision,
            artifact,
        )
    }

    #[test]
    fn missing_corrupt_or_cross_referenced_files_are_errors_without_using_cached_job_or_result() {
        let (mut host, id) = actual();
        let (artifact, result, input) = read(&host, &id).unwrap();
        let input_path = artifact.input_ref.path.as_str().to_owned();
        let result_path = artifact.result_ref.path.as_str().to_owned();
        let bytes = host.professional.files.remove(&input_path).unwrap();
        assert!(
            host.professional
                .jobs
                .contains_key(&(input.id.clone(), input.revision.0.value()))
        );
        assert!(matches!(
            read(&host, &id).unwrap_err().code,
            ErrorCode::MissingReference
        ));
        host.professional
            .files
            .insert(input_path.clone(), b"{}".to_vec());
        assert!(matches!(
            read(&host, &id).unwrap_err().code,
            ErrorCode::InvalidInput
        ));
        host.professional
            .files
            .insert(input_path.clone(), bytes.clone());
        let mut wrong = input.clone();
        wrong.revision = Revision((input.revision.0.value() + 1).try_into().unwrap());
        host.professional
            .files
            .insert(input_path.clone(), encode_document(&wrong));
        assert!(matches!(
            read(&host, &id).unwrap_err().code,
            ErrorCode::InvalidInput
        ));
        host.professional.files.insert(input_path, bytes);
        let original_result = host.professional.files[&result_path].clone();
        let mut wrong = result.clone();
        wrong.artifact_id = "other-artifact".to_owned().try_into().unwrap();
        host.professional
            .files
            .insert(result_path.clone(), encode_document(&wrong));
        assert!(matches!(
            read(&host, &id).unwrap_err().code,
            ErrorCode::InvalidInput
        ));
        host.professional.files.insert(result_path, original_result);
        host.professional
            .artifacts
            .get_mut(&id)
            .unwrap()
            .input_ref
            .kind = DocumentKind::Asset;
        assert!(matches!(
            read(&host, &id).unwrap_err().code,
            ErrorCode::InvalidInput
        ));
    }

    #[test]
    fn actual_ui_still_requires_get_object_before_any_artifact_or_revision_disclosure() {
        let (mut host, id) = actual();
        host.sessions.get_mut(&ui()).unwrap().allowed_operations = vec![OperationName::GetContext];
        for artifact in [&id, &"missing".to_owned().try_into().unwrap()] {
            let failure = read(&host, artifact).unwrap_err();
            assert!(matches!(failure.code, ErrorCode::PermissionDenied));
            assert!(failure.actual_revision.is_none());
        }
        host.sessions.get_mut(&ui()).unwrap().allowed_operations = vec![OperationName::GetObject];
        assert!(read(&host, &id).is_ok());
    }

    #[test]
    fn actual_saved_backing_and_its_declared_additions_work_but_unlisted_bytes_do_not() {
        let (mut host, id) = actual();
        let expected = serde_json::to_value(read(&host, &id).unwrap()).unwrap();
        let parent = std::env::temp_dir().canonicalize().unwrap();
        let directory = parent.join(format!("sf-stored-unit-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let selected = host
            .select_directory(&ui(), FilePurpose::SaveProject, directory.clone())
            .unwrap();
        let revision = host.project.as_ref().unwrap().revision.clone();
        let task: TaskHandle = serde_json::from_value(call(
            &mut host,
            "save_project",
            json!({"revision":revision,"selected_directory_ref":selected}),
        ))
        .unwrap();
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        host.finish_io(work.pop().unwrap().run());
        let state: TaskState =
            serde_json::from_value(call(&mut host, "get_task", json!({"task_id":task.id})))
                .unwrap();
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let artifact = host.professional.artifacts[&id].clone();
        let archive = read_project_archive(&directory).unwrap();
        host.backing = Some(archive.clone());
        host.professional.files.clear();
        assert_eq!(
            serde_json::to_value(read(&host, &id).unwrap()).unwrap(),
            expected
        );
        host.backing
            .as_mut()
            .unwrap()
            .manifest
            .documents
            .retain(|entry| !exact_data(&entry.file_ref, &artifact.input_ref));
        assert!(matches!(
            read(&host, &id).unwrap_err().code,
            ErrorCode::MissingReference
        ));
        host.backing = None;
        for reference in [&artifact.input_ref, &artifact.result_ref] {
            let entry = archive
                .manifest
                .documents
                .iter()
                .find(|entry| exact_data(&entry.file_ref, reference))
                .unwrap()
                .clone();
            host.additions.push(ArchiveAddition {
                bytes: archive.files[reference.path.as_str()].clone(),
                entry,
            });
        }
        assert_eq!(
            serde_json::to_value(read(&host, &id).unwrap()).unwrap(),
            expected
        );
        let resolved = directory.canonicalize().unwrap();
        assert_eq!(resolved.parent(), Some(parent.as_path()));
        std::fs::remove_dir_all(resolved).unwrap();
    }

    #[test]
    fn draft_cannot_fill_missing_private_files_from_an_actual_project_artifact() {
        let (mut host, id) = actual();
        let artifact = host.professional.artifacts[&id].clone();
        let mut definition: Value = serde_json::from_str(include_str!(
            "../../../modules/devices/template-support/professional/module.json"
        ))
        .unwrap();
        definition["data"]["definition"]["id"] = json!("stored.private.draft");
        let revision = host.project.as_ref().unwrap().revision.clone();
        let commit:EditCommit=serde_json::from_value(call(&mut host,"apply_commands",json!({"base_revision":revision,"commands":[{"kind":"create_definition_draft","data":definition["data"]["definition"]}]}))).unwrap();
        let EditScope::DefinitionDraft(identity) = commit.scope else {
            panic!("actual draft")
        };
        // Damage injection tests the source gate; this is not a producer/import.
        host.definition_drafts
            .get_mut(&identity.id)
            .unwrap()
            .professional
            .artifacts
            .insert(id.clone(), artifact);
        let project = host.project.as_ref().unwrap();
        let error = host
            .read_stored_packaging_artifact(
                &ui(),
                &project.id,
                &ApiScope::DefinitionDraft(identity.id),
                &identity.revision,
                &id,
            )
            .unwrap_err();
        assert!(matches!(error.code, ErrorCode::MissingReference));
    }

    #[test]
    fn list_filters_both_reference_kinds_without_reading_files_and_still_requires_get_object() {
        let (mut host, id) = actual();
        let mut wrong_input = host.professional.artifacts[&id].clone();
        wrong_input.id = "list-non-packaging-input".to_owned().try_into().unwrap();
        wrong_input.input_ref.kind = DocumentKind::Asset;
        let mut wrong_result = host.professional.artifacts[&id].clone();
        wrong_result.id = "list-non-packaging-result".to_owned().try_into().unwrap();
        wrong_result.result_ref.kind = DocumentKind::Asset;
        host.professional
            .artifacts
            .insert(wrong_input.id.clone(), wrong_input);
        host.professional
            .artifacts
            .insert(wrong_result.id.clone(), wrong_result);
        // Explicit damage injection: a list must not decode files or depend on Project's summary.
        host.professional.files.clear();
        host.project.as_mut().unwrap().artifact_refs.clear();
        let project = host.project.as_ref().unwrap();
        let project_id = project.id.clone();
        let revision = project.revision.clone();
        assert_eq!(
            host.list_stored_packaging_artifacts(&ui(), &project_id, &ApiScope::Project, &revision)
                .unwrap(),
            vec![id.clone()]
        );
        assert!(matches!(
            read(&host, &id).unwrap_err().code,
            ErrorCode::MissingReference
        ));
        assert!(host.drain_io_work().is_empty());
        host.sessions.get_mut(&ui()).unwrap().allowed_operations = vec![OperationName::GetContext];
        let wrong_revision = Revision(999_u64.try_into().unwrap());
        let failure = host
            .list_stored_packaging_artifacts(
                &ui(),
                &project_id,
                &ApiScope::Project,
                &wrong_revision,
            )
            .unwrap_err();
        assert!(matches!(failure.code, ErrorCode::PermissionDenied));
        assert!(failure.actual_revision.is_none());
    }
}
