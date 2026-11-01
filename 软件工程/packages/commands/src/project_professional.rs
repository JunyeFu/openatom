//! Typed professional candidates. Host owns session/UI admission, grants,
//! revisions, atomic commit, nonreversible history boundaries and real workers.
use crate::{AdoptArtifactParams, ApiError, ApiFieldError, ErrorCode, ProjectDraft};
use serde::{Serialize, de::DeserializeOwned};
use sf_domain::*;
use std::collections::{BTreeMap, BTreeSet};

const PURPOSES: [&str; 5] = [
    "forming",
    "opening",
    "linear_extraction",
    "net_space",
    "manufacturing",
];
const PAYLOAD: [&str; 6] = [
    "editable_job",
    "forming",
    "opening",
    "linear_extraction",
    "net_space",
    "manufacturing",
];

/// Immutable version catalog and actual associated bytes, not a provider/module
/// registry. Module definitions are supplied by the same authoritative Host.
#[derive(Debug, Clone, Default)]
pub struct ProfessionalDocuments {
    pub jobs: BTreeMap<(ObjectId, u64), PackageStructureJob>,
    pub artifacts: BTreeMap<ArtifactId, ArtifactRecord>,
    pub results: BTreeMap<ArtifactId, PackagingResult>,
    pub files: BTreeMap<String, Vec<u8>>,
}

/// Host-owned current-process facts. These are never serde/wire or archive data.
#[derive(Debug, Clone, Default)]
pub struct ProfessionalTrust {
    actual_producers: BTreeSet<ArtifactId>,
    human_acknowledgments: BTreeSet<ObjectId>,
}

impl ProfessionalTrust {
    /// Only the real, verified Host worker completion calls this; file metadata
    /// or an imported ActualSoftware label must never call it.
    pub fn record_actual_producer(&mut self, artifact_id: ArtifactId) {
        self.actual_producers.insert(artifact_id);
    }

    /// Only after an actual UI acknowledgment atomically commits in the Host.
    pub fn record_human_acknowledgment(&mut self, acknowledgment_id: ObjectId) {
        self.human_acknowledgments.insert(acknowledgment_id);
    }

    pub fn clear(&mut self) {
        self.actual_producers.clear();
        self.human_acknowledgments.clear();
    }
}

fn equal<T: Serialize>(left: &T, right: &T) -> bool {
    serde_json::to_value(left).expect("typed professional data")
        == serde_json::to_value(right).expect("typed professional data")
}

fn error(code: ErrorCode, pointer: &str, message: impl Into<String>) -> ApiError {
    let message = message.into();
    ApiError {
        code,
        message: message.clone(),
        object_refs: vec![],
        field_errors: vec![ApiFieldError {
            pointer: pointer.into(),
            reason: message,
        }],
        expected_revision: None,
        actual_revision: None,
    }
}

fn invalid(pointer: &str, message: impl Into<String>) -> ApiError {
    error(ErrorCode::InvalidInput, pointer, message)
}
fn missing(pointer: &str, message: impl Into<String>) -> ApiError {
    error(ErrorCode::MissingReference, pointer, message)
}
fn set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

pub fn professional_issues(draft: &ProjectDraft) -> Vec<Issue> {
    sf_domain::check_professional_inputs(
        &draft.project,
        &draft.professional.jobs,
        &draft.professional.artifacts,
        &draft.professional.results,
    )
}

/// Operates on an admitted candidate only. New conditions create a new exact
/// Job version; an old ID/revision is never silently overwritten.
pub fn put_packaging_job(
    draft: &mut ProjectDraft,
    job: PackageStructureJob,
) -> Result<(), ApiError> {
    let key = (job.id.clone(), job.revision.0.value());
    if let Some(old) = draft.professional.jobs.get(&key) {
        if !equal(old, &job) {
            return Err(invalid(
                "/commands",
                "已保存的Job精确版本不可改写；修改输入须生成新Job修订。",
            ));
        }
    } else {
        draft.professional.jobs.insert(key, job);
    }
    Ok(())
}

/// Freezes the explicit computation budget without changing the draft. A saved
/// budgetless input keeps its identity/content; its execution input is a new
/// revision of that same requested Job, never the content of a newer Job.
pub fn freeze_packaging_execution_input(
    draft: &ProjectDraft,
    requested: &PackageStructureJob,
    budget: &CalculationBudget,
) -> Result<PackageStructureJob, ApiError> {
    let key = (requested.id.clone(), requested.revision.0.value());
    let stored = draft.professional.jobs.get(&key);
    if stored.is_some_and(|old| !equal(old, requested)) {
        return Err(invalid(
            "/params/packaging_job",
            "请求输入与已保存的同ID/修订内容不同，不能覆盖旧版或改用latest。",
        ));
    }
    if let Some(frozen_budget) = &requested.budget {
        if !equal(frozen_budget, budget) {
            return Err(invalid(
                "/params/computation_budget",
                "请求预算必须与Job中已冻结的完整预算及依据一致。",
            ));
        }
        return Ok(requested.clone());
    }
    let mut execution_input = requested.clone();
    if stored.is_some() {
        let highest = draft
            .professional
            .jobs
            .keys()
            .filter(|(id, _)| id == &requested.id)
            .map(|(_, revision)| *revision)
            .fold(requested.revision.0.value(), u64::max);
        execution_input.revision = highest
            .checked_add(1)
            .and_then(|revision| SafeUint::try_from(revision).ok())
            .map(Revision)
            .ok_or_else(|| {
                invalid(
                    "/params/packaging_job/revision",
                    "Job修订已达既有安全整数上界，不能冻结新的执行输入。",
                )
            })?;
    }
    execution_input.budget = Some(budget.clone());
    Ok(execution_input)
}

pub fn prepare_acknowledgment(
    draft: &ProjectDraft,
    ack: ArtifactAcknowledgment,
) -> Result<ArtifactAcknowledgment, ApiError> {
    let artifact = draft
        .professional
        .artifacts
        .get(&ack.artifact_ref)
        .ok_or_else(|| missing("/artifact_ref", "成果记录不存在。"))?;
    if !matches!(ack.decision_source, DecisionSource::HumanStatement(_)) {
        return Err(invalid(
            "/decision_source",
            "人工用途认可不能由预设或提供者结果代替；实际UI来源仍由Host验证。",
        ));
    }
    if !equal(&ack.accepted_subject, &artifact.subject_ref) {
        return Err(invalid(
            "/accepted_subject",
            "认可必须对应成果的精确目标版本。",
        ));
    }
    if ack.accepted_scope.is_empty()
        || !set(&ack.accepted_scope).is_subset(&set(&artifact.supported_scope))
    {
        return Err(invalid(
            "/accepted_scope",
            "明确选择的认可用途必须在成果支持范围内，不能按前缀扩张。",
        ));
    }
    if let Some(old) = draft
        .project
        .acknowledgments
        .iter()
        .find(|old| old.id == ack.id)
    {
        if !equal(old, &ack) {
            return Err(invalid(
                "/id",
                "旧认可记录不可原地改成另一用途或成果，请记录新的明确决定。",
            ));
        }
    }
    // No outcome or process trust changes here, including for Fail/Undetermined.
    Ok(ack)
}

fn decode<T: DeserializeOwned>(
    files: &BTreeMap<String, Vec<u8>>,
    file: &FileRef,
) -> Result<T, ApiError> {
    let bytes = files.get(file.path.as_str()).ok_or_else(|| {
        missing(
            "/artifact_ref",
            format!("缺实际文件：{}", file.path.as_str()),
        )
    })?;
    serde_json::from_slice::<Versioned<T>>(bytes)
        .map(|value| value.data)
        .map_err(|e| {
            invalid(
                "/artifact_ref",
                format!("专业文件不是受支持的完整Versioned数据：{e}"),
            )
        })
}

/// Ingests immutable data, not authority. ActualSoftware is still only a claim
/// until Host records its actual worker completion in ProfessionalTrust.
pub fn ingest_professional_result(
    draft: &mut ProjectDraft,
    job: PackageStructureJob,
    artifact: ArtifactRecord,
    result: PackagingResult,
    files: BTreeMap<String, Vec<u8>>,
) -> Result<(), ApiError> {
    if artifact.id != result.artifact_id
        || result.job_ref.id != job.id
        || !equal(&result.job_ref.revision, &job.revision)
        || !equal(&artifact.subject_ref, &job.subject_ref)
        || !equal(&result.subject_ref, &job.subject_ref)
        || !equal(&artifact.actual_outcome, &result.outcome)
    {
        return Err(invalid(
            "/artifact_ref",
            "成果、结果、输入Job及目标版本必须彼此精确对应。",
        ));
    }
    if !matches!(artifact.input_ref.kind, DocumentKind::PackagingJob)
        || !matches!(artifact.result_ref.kind, DocumentKind::PackagingResult)
    {
        return Err(invalid(
            "/artifact_ref",
            "专业成果必须分别引用可编辑输入Job与实际结果文档。",
        ));
    }
    let encoded_job: PackageStructureJob = decode(&files, &artifact.input_ref)?;
    let encoded_result: PackagingResult = decode(&files, &artifact.result_ref)?;
    if !equal(&job, &encoded_job) || !equal(&result, &encoded_result) {
        return Err(invalid(
            "/artifact_ref",
            "专业文件实际内容与提交的不可变输入/结果不一致。",
        ));
    }
    if draft
        .professional
        .artifacts
        .get(&artifact.id)
        .is_some_and(|old| !equal(old, &artifact))
        || draft
            .professional
            .results
            .get(&artifact.id)
            .is_some_and(|old| !equal(old, &result))
    {
        return Err(invalid(
            "/artifact_ref",
            "旧成果或结果ID不可覆盖；新计算应生成新成果ID。",
        ));
    }
    for (path, bytes) in &files {
        if draft
            .professional
            .files
            .get(path)
            .is_some_and(|old| old != bytes)
        {
            return Err(invalid(
                "/artifact_ref",
                "已引用专业文件不可原位覆盖；请使用新的内部文件位置。",
            ));
        }
    }
    for file in &result.manufacturing_files {
        if !files.contains_key(file.file.path.as_str()) {
            return Err(missing("/artifact_ref", "实际结果声明的制造文件缺失。"));
        }
    }
    // Check the Job collision before any data mutation.
    put_packaging_job(draft, job)?;
    let id = artifact.id.clone();
    draft.professional.artifacts.insert(id.clone(), artifact);
    draft.professional.results.insert(id.clone(), result);
    draft.professional.files.extend(files);
    if !draft.project.artifact_refs.contains(&id) {
        draft.project.artifact_refs.push(id);
    }
    Ok(())
}

fn module<'a>(
    modules: &'a [ModulePackage],
    reference: &ModuleRef,
) -> Result<&'a ModulePackage, ApiError> {
    modules
        .iter()
        .find(|m| m.definition.id == reference.id && m.definition.version == reference.version)
        .ok_or_else(|| missing("/operation_ref", "同Host目录中缺少该精确模块版本。"))
}

fn selected_job<'a>(
    draft: &'a ProjectDraft,
    device: &DeviceInstance,
) -> Result<&'a PackageStructureJob, ApiError> {
    let (Some(TypedValue::String(id)), Some(TypedValue::Integer(revision))) = (
        device.configuration.get("packaging_job_id"),
        device.configuration.get("packaging_job_revision"),
    ) else {
        return Err(invalid(
            "/consumer_ref",
            "必须独立选择已声明的Job ID与修订，不从成果或latest反推。",
        ));
    };
    let id = ObjectId::try_from(id.clone()).map_err(|e| invalid("/consumer_ref", e))?;
    draft
        .professional
        .jobs
        .get(&(id, revision.value()))
        .ok_or_else(|| missing("/consumer_ref", "独立选择的Job精确版本不存在。"))
}

pub fn prepare_artifact_binding(
    draft: &ProjectDraft,
    modules: &[ModulePackage],
    params: &AdoptArtifactParams,
    binding_id: ObjectId,
    trust: &ProfessionalTrust,
    providers: &[ProviderCapability],
) -> Result<ConsumerBinding, ApiError> {
    if !matches!(params.consumer_ref.kind, ObjectKind::Device) {
        return Err(invalid(
            "/consumer_ref",
            "完整纸包装profile的消费者必须是明确设备。",
        ));
    }
    let device = draft
        .project
        .devices
        .iter()
        .find(|d| d.id == params.consumer_ref.id)
        .ok_or_else(|| missing("/consumer_ref", "实际消费者不存在。"))?;
    let definition = &module(modules, &device.module_ref)?.definition;
    let ModulePayload::Devices(device_definition) = &definition.payload else {
        return Err(invalid("/consumer_ref", "实际消费者模块不是设备定义。"));
    };
    for (name, binding, integer) in [
        (
            "packaging_job_id",
            "/payload/data/configuration/packaging_job_id/value",
            false,
        ),
        (
            "packaging_job_revision",
            "/payload/data/configuration/packaging_job_revision/value",
            true,
        ),
    ] {
        if !definition.parameters.iter().any(|p| {
            p.id.as_str() == name
                && p.binding == binding
                && if integer {
                    matches!(p.value_type, ValueKind::Integer) && p.integer
                } else {
                    matches!(p.value_type, ValueKind::String)
                }
        }) {
            return Err(invalid(
                "/consumer_ref",
                "消费者缺少Job选择的真实参数声明；同名configuration键不自动授予声明。",
            ));
        }
    }
    let independent = device.operation_binding.as_ref().or_else(|| {
        if matches!(
            device_definition.behavior.selected,
            Some(BehaviorSourceKind::OperationRef)
        ) {
            device_definition.behavior.operation_ref.as_ref()
        } else {
            None
        }
    });
    let operation = if let Some(selected) = independent {
        if !equal(selected, &params.operation_ref) {
            return Err(invalid(
                "/operation_ref",
                "请求工艺不是该消费者当前显式选择的精确工艺。",
            ));
        }
        let ModulePayload::Operations(operation) = &module(modules, selected)?.definition.payload
        else {
            return Err(invalid(
                "/operation_ref",
                "所选独立工艺不是Operations定义。",
            ));
        };
        operation
    } else {
        if !equal(&device.module_ref, &params.operation_ref)
            || !matches!(
                device_definition.behavior.selected,
                Some(BehaviorSourceKind::Inline)
            )
        {
            return Err(invalid(
                "/operation_ref",
                "未选择另一行为分支；inline工艺必须精确引用当前设备模块。",
            ));
        }
        device_definition
            .behavior
            .inline
            .as_ref()
            .ok_or_else(|| missing("/operation_ref", "选定inline行为没有实际声明。"))?
    };
    if !matches!(operation.builtin, RuleKind::Pack) {
        return Err(invalid(
            "/operation_ref",
            "当前选定工艺不是pack，不能用未选分支替代。",
        ));
    }
    let job = selected_job(draft, device)?;
    let product = draft
        .project
        .products
        .iter()
        .find(|p| p.id == job.subject_ref.id && p.revision == job.subject_ref.revision)
        .ok_or_else(|| missing("/consumer_ref", "独立Job的精确产品版本不是当前所选目标。"))?;
    let candidate = ConsumerBinding {
        id: binding_id,
        consumer_ref: params.consumer_ref.clone(),
        operation_ref: params.operation_ref.clone(),
        artifact_ref: params.artifact_id.clone(),
        expected_subject_revision: product.revision.clone(),
        required_payload: PAYLOAD.iter().map(|s| (*s).into()).collect(),
        requested_scope: params.requested_scope.clone(),
        acceptance_ref: params.acceptance_ref.clone(),
        mode: ProfessionalMode::CalculatedNominal,
    };
    let issues = sf_domain::binding_input_issues(
        &draft.project,
        &draft.professional.jobs,
        &draft.professional.artifacts,
        &draft.professional.results,
        &candidate,
    );
    if let Some(issue) = issues.first() {
        return Err(invalid(
            issue.field_path.as_deref().unwrap_or("/artifact_id"),
            issue.message.clone(),
        ));
    }
    if !trust.actual_producers.contains(&params.artifact_id)
        || !trust.human_acknowledgments.contains(&params.acceptance_ref)
    {
        return Err(error(
            ErrorCode::PermissionDenied,
            "/artifact_id",
            "缺少本Host真实producer或明确UI认可记录；序列化来源字段不是当前权限。",
        ));
    }
    let required: BTreeSet<_> = PURPOSES.into_iter().collect();
    if !providers.iter().any(|p| {
        matches!(p.producer_status, ProviderStatus::Available)
            && p.available_modes
                .iter()
                .any(|m| matches!(m, ProfessionalMode::CalculatedNominal))
            && required.is_subset(&set(&p.supported_scope))
    }) {
        return Err(error(
            ErrorCode::UnsupportedCapability,
            "/artifact_id",
            "实际纸包装名义计算提供者尚不可用，不能从intended_modes授能力或静默降级。",
        ));
    }
    let artifact = &draft.professional.artifacts[&params.artifact_id];
    let result = &draft.professional.results[&params.artifact_id];
    if !matches!(artifact.input_ref.kind, DocumentKind::PackagingJob)
        || !matches!(artifact.result_ref.kind, DocumentKind::PackagingResult)
        || !equal(
            job,
            &decode::<PackageStructureJob>(&draft.professional.files, &artifact.input_ref)?,
        )
        || !equal(
            result,
            &decode::<PackagingResult>(&draft.professional.files, &artifact.result_ref)?,
        )
    {
        return Err(invalid(
            "/artifact_id",
            "成果文件与独立所选Job/实际结果不一致。",
        ));
    }
    validate_manufacturing(job, result, &draft.professional.files)?;
    Ok(candidate)
}

fn validate_manufacturing(
    job: &PackageStructureJob,
    result: &PackagingResult,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<(), ApiError> {
    for part in &job.parts {
        let panels: BTreeSet<_> = part.panel_ids.iter().map(ObjectId::as_str).collect();
        let folds: BTreeSet<_> = job
            .fold_bands
            .iter()
            .filter(|f| panels.contains(f.panel_a.as_str()) && panels.contains(f.panel_b.as_str()))
            .map(|f| f.id.as_str())
            .collect();
        let glues: BTreeSet<_> = job
            .connections
            .iter()
            .filter(|c| {
                matches!(c.kind, ConnectionMaterialKind::Glue)
                    && panels.contains(c.panel_a.as_str())
            })
            .map(|c| c.id.as_str())
            .collect();
        let mut cut = false;
        let mut found_folds = BTreeSet::new();
        let mut found_glues = BTreeSet::new();
        for manufacturing in result
            .manufacturing_files
            .iter()
            .filter(|f| f.part_id == part.id)
        {
            if !matches!(manufacturing.unit, LengthUnit::Millimeter)
                || !matches!(manufacturing.file.kind, DocumentKind::Asset)
            {
                return Err(invalid(
                    "/artifact_id",
                    "制造文件必须保持声明的mm资产单位和文件角色。",
                ));
            }
            let bytes = files
                .get(manufacturing.file.path.as_str())
                .ok_or_else(|| missing("/artifact_id", "实际制造文件缺失。"))?;
            let text = std::str::from_utf8(bytes)
                .map_err(|e| invalid("/artifact_id", format!("制造SVG不是UTF-8：{e}")))?;
            let document = roxmltree::Document::parse(text)
                .map_err(|e| invalid("/artifact_id", format!("制造SVG不是完整XML：{e}")))?;
            let root = document.root_element();
            if root.tag_name().name() != "svg"
                || root.tag_name().namespace() != Some("http://www.w3.org/2000/svg")
            {
                return Err(invalid("/artifact_id", "制造文件缺少实际SVG根。"));
            }
            for name in ["width", "height"] {
                if !root
                    .attribute(name)
                    .and_then(|s| s.strip_suffix("mm"))
                    .and_then(|s| s.parse::<f64>().ok())
                    .is_some_and(|v| v.is_finite() && v > 0.0)
                {
                    return Err(invalid("/artifact_id", "制造SVG必须声明正的mm物理尺寸。"));
                }
            }
            for path in document
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "path")
            {
                if path
                    .ancestors()
                    .find_map(|node| node.attribute("data-part-ref"))
                    != Some(part.id.as_str())
                    || path.attribute("d").is_none_or(|d| d.trim().is_empty())
                {
                    return Err(invalid(
                        "/artifact_id",
                        "制造路径缺少真实part归属或路径内容。",
                    ));
                }
                let layer = path
                    .ancestors()
                    .find(|n| {
                        n.is_element()
                            && n.tag_name().name() == "g"
                            && matches!(n.attribute("id"), Some("cut" | "fold" | "glue"))
                    })
                    .and_then(|n| n.attribute("id"));
                match layer {
                    Some("cut") => {
                        let panel_ids: Vec<_> = path
                            .attribute("data-panel-refs")
                            .unwrap_or("")
                            .split_whitespace()
                            .collect();
                        let band_ids: Vec<_> = path
                            .attribute("data-fold-band-refs")
                            .unwrap_or("")
                            .split_whitespace()
                            .collect();
                        if panel_ids.is_empty() && band_ids.is_empty()
                            || panel_ids.iter().any(|id| !panels.contains(id))
                            || band_ids.iter().any(|id| !folds.contains(id))
                        {
                            return Err(invalid(
                                "/artifact_id",
                                "cut来源不对应当前part的实际板片/折带。",
                            ));
                        }
                        cut = true;
                    }
                    Some("fold") => {
                        let id = path
                            .attribute("data-fold-band-ref")
                            .filter(|id| folds.contains(id))
                            .ok_or_else(|| {
                                invalid("/artifact_id", "fold没有当前part的实际折带来源。")
                            })?;
                        found_folds.insert(id.to_owned());
                    }
                    Some("glue") => {
                        let id = path
                            .attribute("data-connection-ref")
                            .filter(|id| glues.contains(id))
                            .ok_or_else(|| {
                                invalid("/artifact_id", "glue没有当前part的真实胶连接来源。")
                            })?;
                        let connection = job
                            .connections
                            .iter()
                            .find(|c| c.id.as_str() == id)
                            .expect("declared glue");
                        let allowance_ids: Vec<_> = path
                            .attribute("data-allowance-ref")
                            .unwrap_or("")
                            .split_whitespace()
                            .collect();
                        let allowances: Vec<_> = allowance_ids
                            .iter()
                            .filter_map(|id| {
                                job.conditions.allowed_contacts.iter().find(|a| {
                                    a.id.as_str() == *id
                                        && connection.contact_allowance_refs.contains(&a.id)
                                })
                            })
                            .collect();
                        if allowances.is_empty() || allowances.len() != allowance_ids.len() {
                            return Err(invalid(
                                "/artifact_id",
                                "胶层缺少该连接实际局部接触区域。",
                            ));
                        }
                        let active = path.attribute("data-active-step-ref");
                        let matches_activation = match &connection.engage_after_step_id {
                            Some(step) => {
                                active == Some(step.as_str())
                                    && job.steps.iter().any(|s| s.id == *step)
                                    && allowances.iter().all(|a| a.active_step_ids.contains(step))
                            }
                            None => active == Some("initial"),
                        };
                        if !matches_activation {
                            return Err(invalid("/artifact_id", "胶层没有已声明生效过程来源。"));
                        }
                        found_glues.insert(id.to_owned());
                    }
                    _ => {
                        return Err(invalid(
                            "/artifact_id",
                            "制造路径没有cut/fold/glue实际层归属。",
                        ));
                    }
                }
            }
        }
        if !cut
            || !folds.iter().all(|id| found_folds.contains(*id))
            || !glues.iter().all(|id| found_glues.contains(*id))
        {
            return Err(invalid(
                "/artifact_id",
                "必需part的cut/fold/glue实际内容覆盖不完整。",
            ));
        }
    }
    Ok(())
}

/// Used by session history: professional facts are outside ordinary undo.
pub(crate) fn professional_changed(before: &ProjectDraft, after: &ProjectDraft) -> bool {
    !equal(
        &before.project.acknowledgments,
        &after.project.acknowledgments,
    ) || !equal(&before.project.bindings, &after.project.bindings)
        || !equal(&before.project.artifact_refs, &after.project.artifact_refs)
        || !equal(
            &before.professional.jobs.values().collect::<Vec<_>>(),
            &after.professional.jobs.values().collect::<Vec<_>>(),
        )
        || !equal(
            &before.professional.artifacts,
            &after.professional.artifacts,
        )
        || !equal(&before.professional.results, &after.professional.results)
        || before.professional.files != after.professional.files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

    fn fixture<T: DeserializeOwned>(name: &str) -> T {
        serde_json::from_slice::<Versioned<T>>(
            &fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../tools/type-contract-proof/fixtures")
                    .join(format!("{name}.json")),
            )
            .unwrap(),
        )
        .unwrap()
        .data
    }

    fn bytes<T: Serialize + Clone>(data: &T) -> Vec<u8> {
        serde_json::to_vec(&Versioned {
            schema_version: SchemaVersion::V1,
            data: data.clone(),
        })
        .unwrap()
    }

    fn draft() -> ProjectDraft {
        ProjectDraft {
            project: fixture("project"),
            scenarios: BTreeMap::new(),
            professional: ProfessionalDocuments::default(),
        }
    }

    fn data(
        outcome: CheckOutcome,
    ) -> (
        PackageStructureJob,
        ArtifactRecord,
        PackagingResult,
        BTreeMap<String, Vec<u8>>,
    ) {
        let job: PackageStructureJob = fixture("packaging-job");
        let mut artifact: ArtifactRecord = fixture("artifact");
        let mut result: PackagingResult = fixture("packaging-result");
        artifact.actual_outcome = outcome.clone();
        artifact.supported_scope = vec!["design_reference".into()];
        result.outcome = outcome;
        let files = BTreeMap::from([
            (artifact.input_ref.path.as_str().into(), bytes(&job)),
            (artifact.result_ref.path.as_str().into(), bytes(&result)),
        ]);
        (job, artifact, result, files)
    }

    fn requested_budget() -> CalculationBudget {
        CalculationBudget {
            candidate_limit: SafeUint::try_from(3).unwrap(),
            interval_query_limit: SafeUint::try_from(100).unwrap(),
            subdivision_limit: SafeUint::try_from(20).unwrap(),
            basis: "explicit finite request".into(),
        }
    }

    #[test]
    fn freezing_an_unstored_input_keeps_its_requested_revision_and_content() {
        let mut draft = draft();
        let requested: PackageStructureJob = fixture("packaging-job");
        let mut newer = requested.clone();
        newer.revision = Revision(SafeUint::try_from(7).unwrap());
        newer.materials[0].thickness_mm = FiniteNumber::try_from(7.0).unwrap();
        put_packaging_job(&mut draft, newer).unwrap();
        let before = draft.clone();
        let budget = requested_budget();
        let actual = freeze_packaging_execution_input(&draft, &requested, &budget).unwrap();
        let mut expected = requested.clone();
        expected.budget = Some(budget);
        assert!(equal(&actual, &expected));
        assert!(requested.budget.is_none());
        assert!(!professional_changed(&before, &draft));
    }

    #[test]
    fn freezing_a_saved_budgetless_input_derives_a_new_revision_without_retargeting() {
        let mut draft = draft();
        let requested: PackageStructureJob = fixture("packaging-job");
        put_packaging_job(&mut draft, requested.clone()).unwrap();
        let mut newer = requested.clone();
        newer.revision = Revision(SafeUint::try_from(7).unwrap());
        newer.materials[0].thickness_mm = FiniteNumber::try_from(7.0).unwrap();
        put_packaging_job(&mut draft, newer).unwrap();
        let mut unrelated = requested.clone();
        unrelated.id = ObjectId::try_from("OTHER-JOB".to_owned()).unwrap();
        unrelated.revision = Revision(SafeUint::try_from(MAX_SAFE_UINT).unwrap());
        put_packaging_job(&mut draft, unrelated).unwrap();
        draft.project.devices[0].configuration.insert(
            "packaging_job_revision".into(),
            TypedValue::Integer(requested.revision.0.clone()),
        );
        let before = draft.clone();
        let budget = requested_budget();
        let actual = freeze_packaging_execution_input(&draft, &requested, &budget).unwrap();
        let mut expected = requested.clone();
        expected.revision = Revision(SafeUint::try_from(8).unwrap());
        expected.budget = Some(budget);
        assert!(equal(&actual, &expected));
        assert!(equal(&draft.project, &before.project));
        assert!(!professional_changed(&before, &draft));
        assert!(equal(
            &draft.professional.jobs[&(requested.id.clone(), 1)],
            &requested
        ));
        assert!(!draft.professional.jobs.contains_key(&(requested.id, 8)));
    }

    #[test]
    fn an_existing_frozen_budget_requires_all_limits_and_its_basis_to_match() {
        let mut draft = draft();
        let mut requested: PackageStructureJob = fixture("packaging-job");
        let budget = requested_budget();
        requested.budget = Some(budget.clone());
        for saved in [false, true] {
            if saved {
                put_packaging_job(&mut draft, requested.clone()).unwrap();
            }
            let before = draft.clone();
            let actual = freeze_packaging_execution_input(&draft, &requested, &budget).unwrap();
            assert!(equal(&actual, &requested));
            for field in 0..4 {
                let mut mismatch = budget.clone();
                match field {
                    0 => mismatch.candidate_limit = SafeUint::try_from(4).unwrap(),
                    1 => mismatch.interval_query_limit = SafeUint::try_from(101).unwrap(),
                    2 => mismatch.subdivision_limit = SafeUint::try_from(21).unwrap(),
                    _ => mismatch.basis = "another stated basis".into(),
                }
                let failure =
                    freeze_packaging_execution_input(&draft, &requested, &mismatch).unwrap_err();
                assert!(matches!(failure.code, ErrorCode::InvalidInput));
                assert_eq!(
                    failure.field_errors[0].pointer,
                    "/params/computation_budget"
                );
            }
            assert!(!professional_changed(&before, &draft));
        }
    }

    #[test]
    fn a_conflicting_saved_input_is_rejected_before_budget_freezing_or_latest_selection() {
        let mut draft = draft();
        let requested: PackageStructureJob = fixture("packaging-job");
        put_packaging_job(&mut draft, requested.clone()).unwrap();
        let mut newer = requested.clone();
        newer.revision = Revision(SafeUint::try_from(9).unwrap());
        put_packaging_job(&mut draft, newer).unwrap();
        let before = draft.clone();
        for frozen in [false, true] {
            let mut changed = requested.clone();
            changed.materials[0].thickness_mm = FiniteNumber::try_from(2.0).unwrap();
            if frozen {
                changed.budget = Some(requested_budget());
            }
            let failure = freeze_packaging_execution_input(&draft, &changed, &requested_budget())
                .unwrap_err();
            assert!(matches!(failure.code, ErrorCode::InvalidInput));
            assert_eq!(failure.field_errors[0].pointer, "/params/packaging_job");
        }
        assert!(!professional_changed(&before, &draft));
    }

    #[test]
    fn deriving_an_execution_input_honors_the_existing_revision_integer_limit() {
        let mut draft = draft();
        let mut requested: PackageStructureJob = fixture("packaging-job");
        requested.revision = Revision(SafeUint::try_from(MAX_SAFE_UINT).unwrap());
        put_packaging_job(&mut draft, requested.clone()).unwrap();
        let before = draft.clone();
        let failure =
            freeze_packaging_execution_input(&draft, &requested, &requested_budget()).unwrap_err();
        assert!(matches!(failure.code, ErrorCode::InvalidInput));
        assert_eq!(
            failure.field_errors[0].pointer,
            "/params/packaging_job/revision"
        );
        assert!(!professional_changed(&before, &draft));
        assert!(equal(
            draft.professional.jobs.values().next().unwrap(),
            &requested
        ));
    }

    #[test]
    fn real_kernel_unknown_result_uses_the_frozen_revision_without_overwriting_r1() {
        let mut draft = draft();
        consumer(&mut draft);
        let original: PackageStructureJob = fixture("packaging-job");
        assert!(original.tolerance.is_none());
        put_packaging_job(&mut draft, original.clone()).unwrap();
        let selection = draft.project.devices[0].configuration.clone();
        let budget = requested_budget();
        let execution = freeze_packaging_execution_input(&draft, &original, &budget).unwrap();
        let context = sf_packaging::PackagingArtifactContext {
            artifact_id: ArtifactId::try_from("actual-kernel-unknown".to_owned()).unwrap(),
            input_ref: FileRef {
                path: RelativeFilePath::try_from("professional/job-r2.json".to_owned()).unwrap(),
                kind: DocumentKind::PackagingJob,
            },
            result_ref: FileRef {
                path: RelativeFilePath::try_from("professional/actual-unknown.json".to_owned())
                    .unwrap(),
                kind: DocumentKind::PackagingResult,
            },
            manufacturing_directory: RelativeFilePath::try_from(
                "professional/actual-unknown".to_owned(),
            )
            .unwrap(),
        };
        let product = sf_packaging::compute_packaging(
            &execution,
            &budget,
            context,
            &std::sync::atomic::AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        assert!(matches!(product.result.outcome, CheckOutcome::Undetermined));
        assert!(matches!(
            product.result.evidence_origin,
            EvidenceOrigin::ActualSoftware
        ));
        let provider_scope = set(&product.provider.supported_scope);
        assert!(
            product.artifact.unit.as_str() == "millimeter"
                && PURPOSES
                    .iter()
                    .all(|purpose| provider_scope.contains(purpose)),
            "Actual producer must preserve the frozen unit and business-purpose capability contract: artifact.unit={}, provider.supported_scope={:?}",
            product.artifact.unit.as_str(),
            product.provider.supported_scope
        );
        assert!(product.result.checks.iter().any(|check| {
            check
                .unresolved
                .iter()
                .any(|issue| issue.code == "packaging.missing_tolerance")
        }));
        assert!(equal(&product.selected_job, &execution));
        assert_eq!(product.result.job_ref.revision.0.value(), 2);
        let mut files = BTreeMap::from([
            (
                product.artifact.input_ref.path.as_str().to_owned(),
                bytes(&product.selected_job),
            ),
            (
                product.artifact.result_ref.path.as_str().to_owned(),
                bytes(&product.result),
            ),
        ]);
        files.extend(
            product
                .manufacturing_bytes
                .into_iter()
                .map(|(reference, data)| (reference.path.as_str().to_owned(), data)),
        );
        ingest_professional_result(
            &mut draft,
            product.selected_job,
            product.artifact,
            product.result,
            files,
        )
        .unwrap();
        assert_eq!(draft.professional.jobs.len(), 2);
        assert!(equal(
            &draft.professional.jobs[&(original.id.clone(), 1)],
            &original
        ));
        assert!(equal(
            &draft.professional.jobs[&(original.id, 2)],
            &execution
        ));
        assert!(equal(&draft.project.devices[0].configuration, &selection));
        assert!(draft.project.acknowledgments.is_empty() && draft.project.bindings.is_empty());
    }

    #[test]
    fn real_kernel_failure_and_manufacturing_outputs_are_consumed_without_inventing_adoption() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/production/packaging-acceptance-cases.json"
        )))
        .unwrap();
        for name in ["HOLD03", "MAN01", "MAN04", "LOCAL_APPROACH"] {
            let case = cases["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|case| case["id"] == name)
                .unwrap();
            let requested =
                serde_json::from_value::<Versioned<PackageStructureJob>>(case["job"].clone())
                    .unwrap()
                    .data;
            let mut draft = draft();
            let budget = requested.budget.as_ref().unwrap();
            let execution = freeze_packaging_execution_input(&draft, &requested, budget).unwrap();
            let product = sf_packaging::compute_packaging(
                &execution,
                budget,
                sf_packaging::PackagingArtifactContext {
                    artifact_id: ArtifactId::try_from(format!("actual-{name}")).unwrap(),
                    input_ref: FileRef {
                        path: RelativeFilePath::try_from(format!("professional/{name}/job.json"))
                            .unwrap(),
                        kind: DocumentKind::PackagingJob,
                    },
                    result_ref: FileRef {
                        path: RelativeFilePath::try_from(format!(
                            "professional/{name}/result.json"
                        ))
                        .unwrap(),
                        kind: DocumentKind::PackagingResult,
                    },
                    manufacturing_directory: RelativeFilePath::try_from(format!(
                        "professional/{name}/manufacturing"
                    ))
                    .unwrap(),
                },
                &std::sync::atomic::AtomicBool::new(false),
                &mut |_| {},
            )
            .unwrap_or_else(|issues| panic!("{name}: actual computation rejected: {issues:?}"));
            let mut files = BTreeMap::from([
                (
                    product.artifact.input_ref.path.as_str().to_owned(),
                    bytes(&product.selected_job),
                ),
                (
                    product.artifact.result_ref.path.as_str().to_owned(),
                    bytes(&product.result),
                ),
            ]);
            files.extend(
                product
                    .manufacturing_bytes
                    .into_iter()
                    .map(|(reference, data)| (reference.path.as_str().to_owned(), data)),
            );
            if name == "HOLD03" {
                assert!(matches!(product.result.outcome, CheckOutcome::Fail));
            } else {
                assert!(!product.result.manufacturing_files.is_empty(), "{name}");
                validate_manufacturing(&product.selected_job, &product.result, &files)
                    .unwrap_or_else(|failure| panic!("{name}: {failure:?}"));
            }
            let id = product.artifact.id.clone();
            let exact_result = product.result.clone();
            ingest_professional_result(
                &mut draft,
                product.selected_job,
                product.artifact,
                product.result,
                files,
            )
            .unwrap();
            assert!(equal(&draft.professional.results[&id], &exact_result));
            assert!(draft.project.acknowledgments.is_empty() && draft.project.bindings.is_empty());
        }
    }

    #[test]
    fn actual_products_require_explicit_current_adoption_and_explicit_historical_reselection() {
        let mut draft = draft();
        let module = consumer(&mut draft);
        draft.project.routes[0].steps[0].operation_ref =
            draft.project.devices[0].module_ref.clone();
        let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/production/packaging-acceptance-cases.json"
        )))
        .unwrap();
        let mut job = serde_json::from_value::<Versioned<PackageStructureJob>>(
            cases["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|case| case["id"] == "COVERAGE-restricted_folded_container")
                .unwrap()["job"]
                .clone(),
        )
        .unwrap()
        .data;
        job.id = ObjectId::try_from("PACKJOB1".to_owned()).unwrap();
        job.revision = Revision(SafeUint::try_from(1).unwrap());
        job.subject_ref = SubjectRevisionRef {
            id: draft.project.products[0].id.clone(),
            revision: draft.project.products[0].revision.clone(),
        };
        job.conditions.product_envelope.as_mut().unwrap().subject = job.subject_ref.clone();
        let original_job = job.clone();
        let mut trust = ProfessionalTrust::default();
        let mut providers = vec![];
        let mut first_request = None;
        let mut old_binding = None;
        let mut old_artifact = None;
        for revision in [1, 2] {
            if revision == 2 {
                draft.project.products[0].revision =
                    SubjectRevisionId::try_from("r2".to_owned()).unwrap();
                draft.project.devices[0].configuration.insert(
                    "packaging_job_revision".into(),
                    TypedValue::Integer(SafeUint::try_from(2).unwrap()),
                );
                job.revision = Revision(SafeUint::try_from(2).unwrap());
                job.subject_ref.revision = draft.project.products[0].revision.clone();
                job.conditions.product_envelope.as_mut().unwrap().subject = job.subject_ref.clone();
            }
            let name = format!("actual-selection-r{revision}");
            let product = sf_packaging::compute_packaging(
                &job,
                job.budget.as_ref().unwrap(),
                sf_packaging::PackagingArtifactContext {
                    artifact_id: ArtifactId::try_from(name.clone()).unwrap(),
                    input_ref: FileRef {
                        path: RelativeFilePath::try_from(format!("professional/{name}/job.json"))
                            .unwrap(),
                        kind: DocumentKind::PackagingJob,
                    },
                    result_ref: FileRef {
                        path: RelativeFilePath::try_from(format!(
                            "professional/{name}/result.json"
                        ))
                        .unwrap(),
                        kind: DocumentKind::PackagingResult,
                    },
                    manufacturing_directory: RelativeFilePath::try_from(format!(
                        "professional/{name}/manufacturing"
                    ))
                    .unwrap(),
                },
                &std::sync::atomic::AtomicBool::new(false),
                &mut |_| {},
            )
            .unwrap();
            assert!(matches!(product.result.outcome, CheckOutcome::Pass));
            let id = product.artifact.id.clone();
            providers.push(product.provider);
            let mut files = BTreeMap::from([
                (
                    product.artifact.input_ref.path.as_str().to_owned(),
                    bytes(&product.selected_job),
                ),
                (
                    product.artifact.result_ref.path.as_str().to_owned(),
                    bytes(&product.result),
                ),
            ]);
            files.extend(
                product
                    .manufacturing_bytes
                    .into_iter()
                    .map(|(file, data)| (file.path.as_str().to_owned(), data)),
            );
            ingest_professional_result(
                &mut draft,
                product.selected_job,
                product.artifact,
                product.result,
                files,
            )
            .unwrap();
            if revision == 2 {
                assert!(
                    !professional_issues(&draft).is_empty(),
                    "new calculation/selection alone is not a new adoption"
                );
            }
            let ack = prepare_acknowledgment(&draft, ArtifactAcknowledgment {
                id: ObjectId::try_from(format!("purpose-r{revision}")).unwrap(),
                artifact_ref: id.clone(),
                accepted_subject: job.subject_ref.clone(),
                accepted_scope: PURPOSES.iter().map(|s| (*s).into()).collect(),
                decision_source: DecisionSource::HumanStatement("pure candidate test decision; actual UI admission is tested by Host consumers".into()),
                conditions: vec![],
                recorded_at_iso8601: None,
            }).unwrap();
            // These callbacks model already-committed Host events in this unit
            // test. The producer/result are real; UI/grant/Run evidence remains
            // the responsibility of the independent actual Host tests.
            trust.record_actual_producer(id.clone());
            trust.record_human_acknowledgment(ack.id.clone());
            draft.project.acknowledgments.push(ack.clone());
            let mut request = params(&draft);
            request.artifact_id = id.clone();
            request.acceptance_ref = ack.id;
            let binding = prepare_artifact_binding(
                &draft,
                std::slice::from_ref(&module),
                &request,
                ObjectId::try_from(format!("explicit-r{revision}")).unwrap(),
                &trust,
                &providers,
            )
            .unwrap();
            if revision == 1 {
                first_request = Some(request);
                old_binding = Some(binding.clone());
                old_artifact = Some(draft.professional.artifacts[&id].clone());
            }
            draft.project.bindings.push(binding);
            assert!(professional_issues(&draft).is_empty());
        }
        let old_binding = old_binding.unwrap();
        let old_artifact = old_artifact.unwrap();
        assert!(equal(&draft.project.bindings[0], &old_binding));
        assert!(equal(
            &draft.professional.artifacts[&old_artifact.id],
            &old_artifact
        ));
        assert!(equal(
            &draft.professional.jobs[&(original_job.id.clone(), 1)],
            &original_job
        ));
        draft.project.products[0].revision = original_job.subject_ref.revision;
        draft.project.devices[0].configuration.insert(
            "packaging_job_revision".into(),
            TypedValue::Integer(SafeUint::try_from(1).unwrap()),
        );
        assert!(
            !professional_issues(&draft).is_empty(),
            "returning to r1 does not reactivate an older adoption"
        );
        let reselected = prepare_artifact_binding(
            &draft,
            &[module],
            &first_request.unwrap(),
            ObjectId::try_from("explicit-r1-again".to_owned()).unwrap(),
            &trust,
            &providers,
        )
        .unwrap();
        assert_ne!(reselected.id, old_binding.id);
        draft.project.bindings.push(reselected);
        assert!(professional_issues(&draft).is_empty());
        assert_eq!(draft.project.bindings.len(), 3);
        assert!(equal(&draft.project.bindings[0], &old_binding));
        assert!(equal(
            &draft.professional.artifacts[&old_artifact.id],
            &old_artifact
        ));
    }

    #[test]
    fn job_versions_are_immutable_and_new_material_input_preserves_the_old_job() {
        let mut draft = draft();
        let job: PackageStructureJob = fixture("packaging-job");
        put_packaging_job(&mut draft, job.clone()).unwrap();
        let mut changed = job.clone();
        changed.materials[0].thickness_mm = FiniteNumber::try_from(2.0).unwrap();
        let before = draft.clone();
        assert!(put_packaging_job(&mut draft, changed.clone()).is_err());
        assert!(!professional_changed(&before, &draft));
        changed.revision = Revision(SafeUint::try_from(2).unwrap());
        put_packaging_job(&mut draft, changed).unwrap();
        assert_eq!(draft.professional.jobs.len(), 2);
        assert!(equal(&draft.professional.jobs[&(job.id.clone(), 1)], &job));
        assert!(draft.professional.results.is_empty());
    }

    #[test]
    fn unknown_and_fail_ingest_as_data_without_fabricating_process_trust_or_binding() {
        for outcome in [CheckOutcome::Undetermined, CheckOutcome::Fail] {
            let mut draft = draft();
            let (job, artifact, result, files) = data(outcome.clone());
            ingest_professional_result(&mut draft, job, artifact.clone(), result, files).unwrap();
            assert!(equal(
                &draft.professional.artifacts[&artifact.id].actual_outcome,
                &outcome
            ));
            assert!(equal(
                &draft.professional.results[&artifact.id].outcome,
                &outcome
            ));
            assert!(draft.project.bindings.is_empty());
            assert!(draft.project.acknowledgments.is_empty());
            let trust = ProfessionalTrust::default();
            assert!(trust.actual_producers.is_empty() && trust.human_acknowledgments.is_empty());
        }
    }

    #[test]
    fn inconsistent_input_or_outcome_leaves_the_entire_candidate_unchanged() {
        for mismatch in ["file", "outcome"] {
            let mut draft = draft();
            let before = draft.clone();
            let (job, mut artifact, result, mut files) = data(CheckOutcome::Undetermined);
            if mismatch == "file" {
                files.insert(artifact.input_ref.path.as_str().into(), b"{}".to_vec());
            } else {
                artifact.actual_outcome = CheckOutcome::Fail;
            }
            assert!(ingest_professional_result(&mut draft, job, artifact, result, files).is_err());
            assert!(!professional_changed(&before, &draft));
            assert!(equal(&before.project, &draft.project));
        }
    }

    #[test]
    fn explicit_design_reference_acknowledgment_never_changes_unknown_or_failed_results() {
        for outcome in [CheckOutcome::Undetermined, CheckOutcome::Fail] {
            let mut draft = draft();
            let (job, artifact, result, files) = data(outcome.clone());
            ingest_professional_result(&mut draft, job, artifact.clone(), result, files).unwrap();
            let ack = ArtifactAcknowledgment {
                id: ObjectId::try_from("explicit-review".to_owned()).unwrap(),
                artifact_ref: artifact.id.clone(),
                accepted_subject: artifact.subject_ref.clone(),
                accepted_scope: vec!["design_reference".into()],
                decision_source: DecisionSource::HumanStatement("design reference only".into()),
                conditions: vec![],
                recorded_at_iso8601: None,
            };
            let prepared = prepare_acknowledgment(&draft, ack.clone()).unwrap();
            assert!(equal(&prepared, &ack));
            assert!(equal(
                &draft.professional.results[&artifact.id].outcome,
                &outcome
            ));
            assert!(draft.project.acknowledgments.is_empty());
            let mut wrong = ack;
            wrong.accepted_scope = vec!["forming".into()];
            assert!(prepare_acknowledgment(&draft, wrong).is_err());
        }
    }

    #[test]
    fn serialized_human_or_software_metadata_never_populates_current_host_authority() {
        let mut trust = ProfessionalTrust::default();
        let id = ArtifactId::try_from("A1".to_owned()).unwrap();
        let ack = ObjectId::try_from("actual-ui-event".to_owned()).unwrap();
        assert!(!trust.actual_producers.contains(&id));
        assert!(!trust.human_acknowledgments.contains(&ack));
        // These callbacks model Host-owned events only; no positive nominal
        // computation/binding is produced or claimed by this unit test.
        trust.record_actual_producer(id.clone());
        trust.record_human_acknowledgment(ack.clone());
        assert!(trust.actual_producers.contains(&id) && trust.human_acknowledgments.contains(&ack));
        trust.clear();
        assert!(trust.actual_producers.is_empty() && trust.human_acknowledgments.is_empty());
    }

    fn consumer(draft: &mut ProjectDraft) -> ModulePackage {
        let module = serde_json::from_str::<Versioned<ModulePackage>>(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../packaging/consumer-device.design.json"
        )))
        .unwrap()
        .data;
        let device = &mut draft.project.devices[0];
        device.module_ref = ModuleRef {
            id: module.definition.id.clone(),
            version: module.definition.version.clone(),
        };
        device.operation_binding = None;
        device.configuration.insert(
            "packaging_job_id".into(),
            TypedValue::String("PACKJOB1".into()),
        );
        device.configuration.insert(
            "packaging_job_revision".into(),
            TypedValue::Integer(SafeUint::try_from(1).unwrap()),
        );
        module
    }

    fn params(draft: &ProjectDraft) -> AdoptArtifactParams {
        AdoptArtifactParams {
            artifact_id: ArtifactId::try_from("A1".to_owned()).unwrap(),
            acceptance_ref: ObjectId::try_from("review".to_owned()).unwrap(),
            consumer_ref: ObjectRef {
                kind: ObjectKind::Device,
                id: draft.project.devices[0].id.clone(),
            },
            operation_ref: draft.project.devices[0].module_ref.clone(),
            requested_scope: PURPOSES.iter().map(|s| (*s).into()).collect(),
            base_revision: draft.project.revision.clone(),
        }
    }

    #[test]
    fn same_named_instance_keys_do_not_replace_actual_parameter_declarations() {
        let mut draft = draft();
        let mut module = consumer(&mut draft);
        module
            .definition
            .parameters
            .retain(|p| p.id.as_str() != "packaging_job_revision");
        let error = prepare_artifact_binding(
            &draft,
            &[module],
            &params(&draft),
            ObjectId::try_from("candidate".to_owned()).unwrap(),
            &ProfessionalTrust::default(),
            &[],
        )
        .unwrap_err();
        assert!(matches!(error.code, ErrorCode::InvalidInput));
        assert!(draft.project.bindings.is_empty());
    }

    #[test]
    fn missing_independent_job_cannot_be_inferred_from_an_existing_artifact() {
        let mut draft = draft();
        let (job, artifact, result, files) = data(CheckOutcome::Undetermined);
        ingest_professional_result(&mut draft, job, artifact, result, files).unwrap();
        let module = consumer(&mut draft);
        draft.project.devices[0].configuration.insert(
            "packaging_job_id".into(),
            TypedValue::String("not-selected-artifact-input".into()),
        );
        let error = prepare_artifact_binding(
            &draft,
            &[module],
            &params(&draft),
            ObjectId::try_from("candidate".to_owned()).unwrap(),
            &ProfessionalTrust::default(),
            &[],
        )
        .unwrap_err();
        assert!(matches!(error.code, ErrorCode::MissingReference));
        assert!(draft.project.bindings.is_empty());
    }

    #[test]
    fn manufacturing_checks_xml_bytes_and_part_sources_not_only_layer_strings() {
        let mut job: PackageStructureJob = fixture("packaging-job");
        let mut panel = job.panels[0].clone();
        panel.id = ObjectId::try_from("PANEL2".to_owned()).unwrap();
        panel.flat_pose.translation.x = FiniteNumber::try_from(100.0).unwrap();
        job.parts[0].panel_ids.push(panel.id.clone());
        job.panels.push(panel);
        let mut result: PackagingResult = fixture("packaging-result");
        let path = RelativeFilePath::try_from("manufacturing/part.svg".to_owned()).unwrap();
        result.manufacturing_files = vec![ManufacturingFile {
            part_id: job.parts[0].id.clone(),
            file: FileRef {
                path: path.clone(),
                kind: DocumentKind::Asset,
            },
            layers: vec![SvgLayer::Cut],
            unit: LengthUnit::Millimeter,
        }];
        let xml = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200mm\" height=\"60mm\" data-part-ref=\"PART1\"><g id=\"cut\"><path d=\"M0 0 L200 0 L200 60 L0 60 Z\" data-panel-refs=\"PANEL1 PANEL2\"/></g></svg>".to_vec();
        let files = BTreeMap::from([(path.as_str().to_owned(), xml.clone())]);
        validate_manufacturing(&job, &result, &files).unwrap();
        for body in [
            b"<svg/>".to_vec(),
            Vec::new(),
            String::from_utf8(xml.clone())
                .unwrap()
                .replace("PART1", "wrong-part")
                .into_bytes(),
        ] {
            assert!(
                validate_manufacturing(
                    &job,
                    &result,
                    &BTreeMap::from([(path.as_str().to_owned(), body)])
                )
                .is_err()
            );
        }
        assert!(matches!(result.outcome, CheckOutcome::Undetermined));
    }

    #[test]
    fn manufacturing_glue_sources_belong_once_to_the_application_part() {
        let mut job: PackageStructureJob = fixture("packaging-job");
        let mut panel = job.panels[0].clone();
        panel.id = ObjectId::try_from("PANEL2".to_owned()).unwrap();
        panel.part_id = ObjectId::try_from("PART2".to_owned()).unwrap();
        let mut part = job.parts[0].clone();
        part.id = panel.part_id.clone();
        part.root_panel = panel.id.clone();
        part.panel_ids = vec![panel.id.clone()];
        job.parts.push(part);
        job.panels.push(panel);
        for name in ["ALLOW1", "ALLOW2"] {
            job.conditions.allowed_contacts.push(LocalContactAllowance {
                id: ObjectId::try_from(name.to_owned()).unwrap(),
                panel_a: job.panels[0].id.clone(),
                panel_b: job.panels[1].id.clone(),
                face_a: PanelFaceSide::Front,
                face_b: PanelFaceSide::Back,
                polygon_a_mm: job.panels[0].polygon_mm.clone(),
                polygon_b_mm: job.panels[1].polygon_mm.clone(),
                normal_overlap_limit_mm: FiniteNumber::try_from(0.1).unwrap(),
                active_step_ids: vec![],
                basis: "explicit initial connection; parser fixture only".into(),
            });
        }
        job.connections.push(FixedConnection {
            id: ObjectId::try_from("GLUE1".to_owned()).unwrap(),
            panel_a: job.panels[0].id.clone(),
            panel_b: job.panels[1].id.clone(),
            frame_a: job.panels[0].flat_pose.clone(),
            frame_b: job.panels[1].flat_pose.clone(),
            kind: ConnectionMaterialKind::Glue,
            allow_contact: true,
            contact_allowance_refs: job
                .conditions
                .allowed_contacts
                .iter()
                .map(|a| a.id.clone())
                .collect(),
            engage_after_step_id: None,
        });
        let mut result: PackagingResult = fixture("packaging-result");
        result.manufacturing_files = job
            .parts
            .iter()
            .map(|p| ManufacturingFile {
                part_id: p.id.clone(),
                file: FileRef {
                    path: RelativeFilePath::try_from(format!(
                        "manufacturing/{}.svg",
                        p.id.as_str()
                    ))
                    .unwrap(),
                    kind: DocumentKind::Asset,
                },
                layers: vec![SvgLayer::Cut, SvgLayer::Fold, SvgLayer::Glue],
                unit: LengthUnit::Millimeter,
            })
            .collect();
        let mut files = BTreeMap::new();
        for (index, file) in result.manufacturing_files.iter().enumerate() {
            let glue = if index == 0 {
                "<path data-connection-ref=\"GLUE1\" data-allowance-ref=\"ALLOW1 ALLOW2\" data-active-step-ref=\"initial\" d=\"M0 0 L10 0 L10 10 L0 10 Z\"/>"
            } else {
                ""
            };
            files.insert(
                file.file.path.as_str().to_owned(),
                format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100mm\" height=\"60mm\" data-part-ref=\"{}\"><g id=\"cut\"><path data-panel-refs=\"{}\" d=\"M0 0 L100 0 L100 60 L0 60 Z\"/></g><g id=\"glue\">{glue}</g></svg>", job.parts[index].id.as_str(), job.panels[index].id.as_str()).into_bytes(),
            );
        }
        // Only XML/source consumption is checked; no software calculation or
        // positive nominal result is fabricated by this parser fixture.
        validate_manufacturing(&job, &result, &files).unwrap();
        let path = result.manufacturing_files[0].file.path.as_str();
        for (old, new) in [
            ("ALLOW2", "unrelated-allowance"),
            ("initial", "missing-step"),
        ] {
            let mut invalid_files = files.clone();
            invalid_files.insert(
                path.to_owned(),
                String::from_utf8(files[path].clone())
                    .unwrap()
                    .replace(old, new)
                    .into_bytes(),
            );
            assert!(validate_manufacturing(&job, &result, &invalid_files).is_err());
        }
        assert!(matches!(result.outcome, CheckOutcome::Undetermined));
    }
}
