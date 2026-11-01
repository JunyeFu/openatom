//! Pure checks for the four distinct professional objects. No calculation,
//! approval, binding mutation, stock, or current-process trust is created here.
use crate::*;
use serde::Serialize;
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

fn equal<T: Serialize>(left: &T, right: &T) -> bool {
    serde_json::to_value(left).expect("typed professional input")
        == serde_json::to_value(right).expect("typed professional input")
}

fn problem(
    issues: &mut Vec<Issue>,
    binding: &ConsumerBinding,
    field: &str,
    code: &str,
    message: &str,
) {
    issues.push(Issue {
        code: format!("professional_inputs.{code}"),
        object_ref: Some(binding.consumer_ref.clone()),
        field_path: Some(field.into()),
        message: message.into(),
        blocks: vec![
            CapabilityPurpose::Execution,
            CapabilityPurpose::CompleteConclusion,
        ],
    });
}

fn tokens(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

pub fn check_professional_inputs(
    project: &ProjectState,
    jobs: &BTreeMap<(ObjectId, u64), PackageStructureJob>,
    artifacts: &BTreeMap<ArtifactId, ArtifactRecord>,
    results: &BTreeMap<ArtifactId, PackagingResult>,
) -> Vec<Issue> {
    let mut issues: Vec<_> = project
        .bindings
        .iter()
        .flat_map(|binding| binding_reference_issues(project, jobs, artifacts, results, binding))
        .collect();
    for binding in current_consumer_bindings(project) {
        issues.extend(binding_current_use_issues(
            project, jobs, artifacts, results, binding,
        ));
    }
    issues
}

/// The last successful explicit adoption for each exact consumer is current.
/// References are returned in adoption order; stored history is never changed.
pub fn current_consumer_bindings(project: &ProjectState) -> Vec<&ConsumerBinding> {
    let mut current: Vec<&ConsumerBinding> = vec![];
    for binding in &project.bindings {
        if let Some(index) = current
            .iter()
            .position(|old| equal(&old.consumer_ref, &binding.consumer_ref))
        {
            current.remove(index);
        }
        current.push(binding);
    }
    current
}

/// A proposed explicit adoption always receives the full gate, including when
/// the same artifact has appeared in historical bindings.
pub fn binding_input_issues(
    project: &ProjectState,
    jobs: &BTreeMap<(ObjectId, u64), PackageStructureJob>,
    artifacts: &BTreeMap<ArtifactId, ArtifactRecord>,
    results: &BTreeMap<ArtifactId, PackagingResult>,
    binding: &ConsumerBinding,
) -> Vec<Issue> {
    let mut issues = binding_reference_issues(project, jobs, artifacts, results, binding);
    issues.extend(binding_current_use_issues(
        project, jobs, artifacts, results, binding,
    ));
    issues
}

fn binding_reference_issues(
    project: &ProjectState,
    jobs: &BTreeMap<(ObjectId, u64), PackageStructureJob>,
    artifacts: &BTreeMap<ArtifactId, ArtifactRecord>,
    results: &BTreeMap<ArtifactId, PackagingResult>,
    binding: &ConsumerBinding,
) -> Vec<Issue> {
    let mut issues = vec![];
    if matches!(binding.consumer_ref.kind, ObjectKind::Device)
        && !project
            .devices
            .iter()
            .any(|d| d.id == binding.consumer_ref.id)
    {
        problem(
            &mut issues,
            binding,
            "/consumer_ref",
            "missing_consumer",
            "绑定所引用的实际设备不存在。",
        );
    }
    let Some(artifact) = artifacts.get(&binding.artifact_ref) else {
        problem(
            &mut issues,
            binding,
            "/artifact_ref",
            "missing_artifact",
            "绑定引用的成果记录不存在。",
        );
        return issues;
    };
    if artifact.id != binding.artifact_ref
        || artifact.subject_ref.revision != binding.expected_subject_revision
    {
        problem(
            &mut issues,
            binding,
            "/expected_subject_revision",
            "artifact_subject",
            "绑定保存的目标版本与所选成果版本不一致。",
        );
    }
    match project
        .acknowledgments
        .iter()
        .find(|a| a.id == binding.acceptance_ref)
    {
        None => problem(
            &mut issues,
            binding,
            "/acceptance_ref",
            "missing_acknowledgment",
            "成果、用途认可和消费者绑定分别存在；不能从成果生成认可。",
        ),
        Some(ack) => {
            if ack.artifact_ref != artifact.id
                || !equal(&ack.accepted_subject, &artifact.subject_ref)
            {
                problem(
                    &mut issues,
                    binding,
                    "/acceptance_ref",
                    "acknowledgment_subject",
                    "认可对应的成果或精确目标版本不匹配。",
                );
            }
            if !matches!(ack.decision_source, DecisionSource::HumanStatement(_)) {
                problem(
                    &mut issues,
                    binding,
                    "/acceptance_ref",
                    "nonhuman_acknowledgment",
                    "预设或提供者结果不能替代指定用途的人工认可。",
                );
            }
            if !tokens(&binding.requested_scope).is_subset(&tokens(&ack.accepted_scope)) {
                problem(
                    &mut issues,
                    binding,
                    "/requested_scope",
                    "acknowledgment_scope",
                    "请求用途越过已明确认可的精确用途集合。",
                );
            }
        }
    }
    if !tokens(&binding.requested_scope).is_subset(&tokens(&artifact.supported_scope)) {
        problem(
            &mut issues,
            binding,
            "/requested_scope",
            "artifact_scope",
            "请求用途超出成果实际声明的支持范围，不按前缀匹配。",
        );
    }
    if matches!(binding.mode, ProfessionalMode::CalculatedNominal) {
        let Some(result) = results.get(&binding.artifact_ref) else {
            problem(
                &mut issues,
                binding,
                "/artifact_ref",
                "missing_actual_result",
                "实际专业结果尚未取得，不能从声明或认可制造Pass。",
            );
            return issues;
        };
        if result.artifact_id != artifact.id || !equal(&result.subject_ref, &artifact.subject_ref) {
            problem(
                &mut issues,
                binding,
                "/artifact_ref",
                "result_record_mismatch",
                "历史或当前结果必须归属于其精确成果和目标版本。",
            );
        }
        if !equal(&result.outcome, &artifact.actual_outcome) {
            problem(
                &mut issues,
                binding,
                "/artifact_ref",
                "outcome_mismatch",
                "成果记录与实际结果三态不一致。",
            );
        }
        match jobs.get(&(result.job_ref.id.clone(), result.job_ref.revision.0.value())) {
            None => problem(
                &mut issues,
                binding,
                "/artifact_ref",
                "missing_result_job",
                "结果引用的不可变Job精确版本不存在。",
            ),
            Some(job)
                if job.id != result.job_ref.id
                    || !equal(&job.revision, &result.job_ref.revision)
                    || !equal(&job.subject_ref, &artifact.subject_ref) =>
            {
                problem(
                    &mut issues,
                    binding,
                    "/artifact_ref",
                    "result_job_identity",
                    "结果所归属Job的身份、修订或目标不一致。",
                );
            }
            Some(_) => {}
        }
    }
    issues
}

fn binding_current_use_issues(
    project: &ProjectState,
    jobs: &BTreeMap<(ObjectId, u64), PackageStructureJob>,
    artifacts: &BTreeMap<ArtifactId, ArtifactRecord>,
    results: &BTreeMap<ArtifactId, PackagingResult>,
    binding: &ConsumerBinding,
) -> Vec<Issue> {
    let mut issues = vec![];
    if matches!(binding.mode, ProfessionalMode::SoftwareInLoop) {
        problem(
            &mut issues,
            binding,
            "/mode",
            "unsupported_mode",
            "首版未实现software_in_loop，不能自动降级为声明或名义计算。",
        );
        return issues;
    }
    if !matches!(binding.consumer_ref.kind, ObjectKind::Device) {
        problem(
            &mut issues,
            binding,
            "/consumer_ref",
            "consumer_kind",
            "当前专业消费者必须是明确的设备实例。",
        );
        return issues;
    }
    let Some(consumer) = project
        .devices
        .iter()
        .find(|d| d.id == binding.consumer_ref.id)
    else {
        return issues;
    };
    if consumer
        .operation_binding
        .as_ref()
        .is_some_and(|effective| !equal(effective, &binding.operation_ref))
    {
        problem(
            &mut issues,
            binding,
            "/operation_ref",
            "operation_changed",
            "该消费者当前显式工艺已改变，原绑定需复核；不改写原成果或认可。",
        );
    }
    let Some(artifact) = artifacts.get(&binding.artifact_ref) else {
        return issues;
    };
    if project
        .acknowledgments
        .iter()
        .find(|a| a.id == binding.acceptance_ref)
        .is_some_and(|ack| {
            ack.conditions.iter().any(|issue| {
                issue.blocks.iter().any(|purpose| {
                    matches!(
                        purpose,
                        CapabilityPurpose::Execution | CapabilityPurpose::CompleteConclusion
                    )
                })
            })
        })
    {
        problem(
            &mut issues,
            binding,
            "/acceptance_ref",
            "acknowledgment_conditions",
            "认可仍含阻断用途的条件，不能当作无条件可执行。",
        );
    }
    if matches!(binding.mode, ProfessionalMode::DeclaredProcess) {
        // A declaration has no professional calculation result. It is never
        // promoted into the complete nominal paper-packaging profile.
        if !project
            .products
            .iter()
            .any(|p| p.id == artifact.subject_ref.id && p.revision == artifact.subject_ref.revision)
        {
            problem(
                &mut issues,
                binding,
                "/expected_subject_revision",
                "declaration_subject_changed",
                "当前声明用途的目标版本已改变；旧版本成果及认可仍保留。",
            );
        }
        return issues;
    }
    let selection = match (
        consumer.configuration.get("packaging_job_id"),
        consumer.configuration.get("packaging_job_revision"),
    ) {
        (Some(TypedValue::String(id)), Some(TypedValue::Integer(revision))) => {
            ObjectId::try_from(id.clone())
                .ok()
                .map(|id| (id, revision.value()))
        }
        _ => None,
    };
    let Some(selection) = selection else {
        problem(
            &mut issues,
            binding,
            "/configuration",
            "missing_job_selection",
            "必须独立明确选择Job ID和修订，不能由成果或latest反推。",
        );
        return issues;
    };
    let Some(job) = jobs.get(&selection) else {
        problem(
            &mut issues,
            binding,
            "/configuration/packaging_job_revision",
            "missing_selected_job",
            "所选Job精确版本不存在，不能改用另一个版本。",
        );
        return issues;
    };
    if job.id != selection.0
        || job.revision.0.value() != selection.1
        || !equal(&job.subject_ref, &artifact.subject_ref)
    {
        problem(
            &mut issues,
            binding,
            "/configuration",
            "selected_job_subject",
            "独立所选Job身份/目标与该成果不一致。",
        );
    }
    let product = project
        .products
        .iter()
        .find(|p| p.id == job.subject_ref.id && p.revision == job.subject_ref.revision);
    match product {
        None => problem(
            &mut issues,
            binding,
            "/expected_subject_revision",
            "selected_subject_changed",
            "所选Job的精确产品版本不再是当前目标；仅该绑定需复核，旧成果和旧认可不变。",
        ),
        Some(product) => {
            if !project.routes.iter().any(|route| {
                route.id == product.routing_ref.id
                    && route.revision == product.routing_ref.revision
                    && route.steps.iter().any(|step| {
                        step.device_id == consumer.id
                            && equal(&step.operation_ref, &binding.operation_ref)
                    })
            }) {
                problem(
                    &mut issues,
                    binding,
                    "/consumer_ref",
                    "consumer_route",
                    "所选产品的精确路线不包含该消费者当前工艺。",
                );
            }
        }
    }
    if !(job.conditions.forming_required
        && job.conditions.opening_required
        && job.conditions.extraction_required
        && job.conditions.net_space_required)
        || job.conditions.product_envelope.is_none()
        || job.conditions.extraction.is_none()
        || job.parts.is_empty()
        || job.panels.is_empty()
        || job.materials.is_empty()
    {
        problem(
            &mut issues,
            binding,
            "/configuration",
            "incomplete_profile_input",
            "完整纸包装profile仍缺成型、开合、固定姿态取物或净空间输入；不能靠少勾用途降级。",
        );
    }
    if !matches!(job.unit, LengthUnit::Millimeter)
        || !matches!(job.coordinate_frame, CoordinateFrame::RightHandedZUp)
        || artifact.unit.as_str() != "millimeter"
        || !equal(&artifact.coordinate_frame, &job.coordinate_frame)
    {
        problem(
            &mut issues,
            binding,
            "/artifact_ref",
            "unit_or_frame",
            "专业输入和成果必须保持声明的mm/右手Z-up坐标，不用显示尺度替代。",
        );
    }
    let required: BTreeSet<_> = PURPOSES.into_iter().collect();
    if !required.is_subset(&tokens(&binding.requested_scope)) {
        problem(
            &mut issues,
            binding,
            "/requested_scope",
            "incomplete_profile_scope",
            "完整profile必须覆盖五项用途；空集或部分用途不是完整名义绑定。",
        );
    }
    if !PAYLOAD
        .into_iter()
        .collect::<BTreeSet<_>>()
        .is_subset(&tokens(&binding.required_payload))
    {
        problem(
            &mut issues,
            binding,
            "/required_payload",
            "incomplete_payload",
            "绑定的必需内容必须含可编辑Job及完整五用途，不能由请求人省略。",
        );
    }
    let Some(result) = results.get(&artifact.id) else {
        return issues;
    };
    if result.artifact_id != artifact.id
        || result.job_ref.id != job.id
        || !equal(&result.job_ref.revision, &job.revision)
        || !equal(&result.subject_ref, &job.subject_ref)
    {
        problem(
            &mut issues,
            binding,
            "/artifact_ref",
            "result_input_mismatch",
            "实际结果必须对应独立所选Job的精确输入；同产品不同材料Job也不能复用。",
        );
    }
    if !tokens(&binding.requested_scope).is_subset(&tokens(&result.supported_scope)) {
        problem(
            &mut issues,
            binding,
            "/requested_scope",
            "result_scope",
            "请求用途也必须被实际结果支持，不能仅扩大成果记录的scope。",
        );
    }
    if !matches!(result.outcome, CheckOutcome::Pass)
        || !matches!(artifact.actual_outcome, CheckOutcome::Pass)
    {
        problem(
            &mut issues,
            binding,
            "/artifact_ref",
            "result_not_pass",
            "Fail/Undetermined必须保留原态；人工认可不会把它改成Pass。",
        );
    }
    if !matches!(result.evidence_origin, EvidenceOrigin::ActualSoftware)
        || !matches!(artifact.evidence_origin, EvidenceOrigin::ActualSoftware)
    {
        problem(
            &mut issues,
            binding,
            "/artifact_ref",
            "untrusted_result_claim",
            "导入、设计、手算或type_fixture不是实际专业计算证明。",
        );
    }
    for purpose in [
        PackagingStepPurpose::Forming,
        PackagingStepPurpose::Opening,
        PackagingStepPurpose::LinearExtraction,
        PackagingStepPurpose::NetSpace,
        PackagingStepPurpose::Manufacturing,
    ] {
        let checks: Vec<_> = result
            .checks
            .iter()
            .filter(|check| equal(&check.purpose, &purpose))
            .collect();
        if checks.is_empty()
            || checks.iter().any(|check| {
                !matches!(check.outcome, CheckOutcome::Pass) || !check.unresolved.is_empty()
            })
        {
            problem(
                &mut issues,
                binding,
                "/artifact_ref",
                "required_checks",
                "必需目的必须有非空、完整且无未决项的实际Pass；空集合all()不授通过。",
            );
        }
    }
    for part in &job.parts {
        if !result.manufacturing_files.iter().any(|file| {
            file.part_id == part.id
                && matches!(file.unit, LengthUnit::Millimeter)
                && file
                    .layers
                    .iter()
                    .any(|layer| matches!(layer, SvgLayer::Cut))
        }) || !result.material_counts.iter().any(|count| {
            count.part_id == part.id
                && count.quantity.value() == 1
                && count
                    .area_mm2
                    .as_ref()
                    .is_some_and(|area| area.value() > 0.0)
                && job
                    .materials
                    .iter()
                    .any(|material| material.id == count.material_id)
                && job.panels.iter().any(|panel| {
                    part.panel_ids.contains(&panel.id) && panel.material_ref == count.material_id
                })
        }) {
            problem(
                &mut issues,
                binding,
                "/artifact_ref",
                "missing_part_manufacturing",
                "每个显式part必须有实际制造文件及材料数量，不能只靠全局token列表。",
            );
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> (
        ProjectState,
        BTreeMap<(ObjectId, u64), PackageStructureJob>,
        BTreeMap<ArtifactId, ArtifactRecord>,
        BTreeMap<ArtifactId, PackagingResult>,
    ) {
        let mut project = serde_json::from_str::<Versioned<ProjectState>>(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/type-contract-proof/fixtures/project.json"
        )))
        .unwrap()
        .data;
        let job = serde_json::from_str::<Versioned<PackageStructureJob>>(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/type-contract-proof/fixtures/packaging-job.json"
        )))
        .unwrap()
        .data;
        let mut artifact =
            serde_json::from_str::<Versioned<ArtifactRecord>>(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tools/type-contract-proof/fixtures/artifact.json"
            )))
            .unwrap()
            .data;
        let result = serde_json::from_str::<Versioned<PackagingResult>>(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/type-contract-proof/fixtures/packaging-result.json"
        )))
        .unwrap()
        .data;
        artifact.supported_scope = PURPOSES.iter().map(|s| (*s).into()).collect();
        let device = &mut project.devices[0];
        device.configuration.insert(
            "packaging_job_id".into(),
            TypedValue::String(job.id.as_str().into()),
        );
        device.configuration.insert(
            "packaging_job_revision".into(),
            TypedValue::Integer(job.revision.0.clone()),
        );
        project.routes[0].steps[0].operation_ref = device.module_ref.clone();
        let ack_id = ObjectId::try_from("review-r1".to_owned()).unwrap();
        project.acknowledgments.push(ArtifactAcknowledgment {
            id: ack_id.clone(),
            artifact_ref: artifact.id.clone(),
            accepted_subject: artifact.subject_ref.clone(),
            accepted_scope: artifact.supported_scope.clone(),
            decision_source: DecisionSource::HumanStatement("review of fixture only".into()),
            conditions: vec![],
            recorded_at_iso8601: None,
        });
        project.bindings.push(ConsumerBinding {
            id: ObjectId::try_from("binding-r1".to_owned()).unwrap(),
            consumer_ref: ObjectRef {
                kind: ObjectKind::Device,
                id: device.id.clone(),
            },
            operation_ref: device.module_ref.clone(),
            artifact_ref: artifact.id.clone(),
            expected_subject_revision: artifact.subject_ref.revision.clone(),
            required_payload: PAYLOAD.iter().map(|s| (*s).into()).collect(),
            requested_scope: artifact.supported_scope.clone(),
            acceptance_ref: ack_id,
            mode: ProfessionalMode::CalculatedNominal,
        });
        (
            project,
            BTreeMap::from([((job.id.clone(), job.revision.0.value()), job)]),
            BTreeMap::from([(artifact.id.clone(), artifact)]),
            BTreeMap::from([(result.artifact_id.clone(), result)]),
        )
    }

    fn has(issues: &[Issue], code: &str) -> bool {
        issues
            .iter()
            .any(|i| i.code == format!("professional_inputs.{code}"))
    }

    #[test]
    fn current_bindings_follow_explicit_adoption_order_for_each_exact_consumer() {
        let (mut project, _, _, _) = input();
        let first = project.bindings[0].clone();
        let mut other = first.clone();
        other.id = ObjectId::try_from("OTHER-CURRENT".to_owned()).unwrap();
        other.consumer_ref.id = ObjectId::try_from("OTHER-DEVICE".to_owned()).unwrap();
        project.bindings.push(other.clone());
        let mut next = first.clone();
        next.id = ObjectId::try_from("NEW-CURRENT".to_owned()).unwrap();
        next.operation_ref.id = ModuleId::try_from("another.operation".to_owned()).unwrap();
        project.bindings.push(next.clone());
        let original = project.bindings.clone();
        let selected = current_consumer_bindings(&project);
        assert_eq!(selected.len(), 2);
        assert!(equal(selected[0], &other));
        assert!(equal(selected[1], &next));
        assert!(equal(&project.bindings, &original));
        // Returning to the first artifact alone is not a selection. A new
        // explicit adoption fact for it becomes current without rewriting it.
        let mut reselected = first;
        reselected.id = ObjectId::try_from("EXPLICIT-RESELECT".to_owned()).unwrap();
        project.bindings.push(reselected.clone());
        assert!(equal(
            *current_consumer_bindings(&project).last().unwrap(),
            &reselected
        ));
        assert!(equal(&project.bindings[..3].to_vec(), &original));
    }

    #[test]
    fn superseded_use_gates_do_not_hide_historical_identity_and_ownership_errors() {
        let (mut project, jobs, artifacts, results) = input();
        let mut current_ack = project.acknowledgments[0].clone();
        current_ack.id = ObjectId::try_from("CURRENT-ACK".to_owned()).unwrap();
        let mut current = project.bindings[0].clone();
        current.id = ObjectId::try_from("CURRENT-DECLARATION".to_owned()).unwrap();
        current.acceptance_ref = current_ack.id.clone();
        current.mode = ProfessionalMode::DeclaredProcess;
        project.acknowledgments[0].conditions.push(Issue {
            code: "old.conditional_review".into(),
            object_ref: None,
            field_path: None,
            message: "an older conditional acknowledgment is retained".into(),
            blocks: vec![CapabilityPurpose::Execution],
        });
        project.acknowledgments.push(current_ack);
        project.bindings.push(current);
        // The current declaration is evaluated as a declaration. Historical
        // Unknown/TypeFixture data does not acquire a nominal Pass.
        assert!(check_professional_inputs(&project, &jobs, &artifacts, &results).is_empty());
        let mut broken = project.clone();
        broken.bindings[0].expected_subject_revision =
            SubjectRevisionId::try_from("wrong-history".to_owned()).unwrap();
        assert!(has(
            &check_professional_inputs(&broken, &jobs, &artifacts, &results),
            "artifact_subject"
        ));
        let mut broken = project.clone();
        broken.acknowledgments.remove(0);
        assert!(has(
            &check_professional_inputs(&broken, &jobs, &artifacts, &results),
            "missing_acknowledgment"
        ));
        let mut broken = results.clone();
        broken.values_mut().next().unwrap().job_ref.revision =
            Revision(SafeUint::try_from(99).unwrap());
        assert!(has(
            &check_professional_inputs(&project, &jobs, &artifacts, &broken),
            "missing_result_job"
        ));
        let mut broken = results.clone();
        broken.values_mut().next().unwrap().outcome = CheckOutcome::Fail;
        assert!(has(
            &check_professional_inputs(&project, &jobs, &artifacts, &broken),
            "outcome_mismatch"
        ));
        assert!(matches!(
            results.values().next().unwrap().outcome,
            CheckOutcome::Undetermined
        ));
    }

    #[test]
    fn unknown_failed_or_fixture_results_never_become_pass_from_human_acknowledgment() {
        let (project, jobs, mut artifacts, mut results) = input();
        for outcome in [CheckOutcome::Undetermined, CheckOutcome::Fail] {
            artifacts.values_mut().next().unwrap().actual_outcome = outcome.clone();
            results.values_mut().next().unwrap().outcome = outcome;
            let issues = check_professional_inputs(&project, &jobs, &artifacts, &results);
            assert!(has(&issues, "result_not_pass"));
            assert!(has(&issues, "untrusted_result_claim"));
        }
    }

    #[test]
    fn unbound_historical_failures_are_retained_without_globally_disabling_the_project() {
        let (mut project, jobs, artifacts, results) = input();
        project.bindings.clear();
        assert!(check_professional_inputs(&project, &jobs, &artifacts, &results).is_empty());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn current_product_revision_changes_only_its_binding_and_preserves_r1_facts() {
        let (mut project, mut jobs, mut artifacts, mut results) = input();
        // A second real consumer/subject is present: the r2 transition must not
        // globally stale every binding merely because one product changed.
        let mut other_product = project.products[0].clone();
        other_product.id = ObjectId::try_from("OTHER-PRODUCT".to_owned()).unwrap();
        let mut other_device = project.devices[0].clone();
        other_device.id = ObjectId::try_from("OTHER-DEVICE".to_owned()).unwrap();
        let mut other_job = jobs.values().next().unwrap().clone();
        other_job.id = ObjectId::try_from("OTHER-JOB".to_owned()).unwrap();
        other_job.subject_ref.id = other_product.id.clone();
        other_device.configuration.insert(
            "packaging_job_id".into(),
            TypedValue::String(other_job.id.as_str().into()),
        );
        let mut other_artifact = artifacts.values().next().unwrap().clone();
        other_artifact.id = ArtifactId::try_from("OTHER-ARTIFACT".to_owned()).unwrap();
        other_artifact.subject_ref = other_job.subject_ref.clone();
        let mut other_result = results.values().next().unwrap().clone();
        other_result.artifact_id = other_artifact.id.clone();
        other_result.subject_ref = other_job.subject_ref.clone();
        other_result.job_ref.id = other_job.id.clone();
        let mut other_ack = project.acknowledgments[0].clone();
        other_ack.id = ObjectId::try_from("OTHER-ACK".to_owned()).unwrap();
        other_ack.artifact_ref = other_artifact.id.clone();
        other_ack.accepted_subject = other_job.subject_ref.clone();
        let mut other_binding = project.bindings[0].clone();
        other_binding.id = ObjectId::try_from("OTHER-BINDING".to_owned()).unwrap();
        other_binding.consumer_ref.id = other_device.id.clone();
        other_binding.artifact_ref = other_artifact.id.clone();
        other_binding.acceptance_ref = other_ack.id.clone();
        let mut route_step = project.routes[0].steps[0].clone();
        route_step.id = ObjectId::try_from("OTHER-STEP".to_owned()).unwrap();
        route_step.device_id = other_device.id.clone();
        project.routes[0].steps.push(route_step);
        project.products.push(other_product);
        project.devices.push(other_device);
        project.acknowledgments.push(other_ack);
        project.bindings.push(other_binding);
        jobs.insert((other_job.id.clone(), 1), other_job);
        artifacts.insert(other_artifact.id.clone(), other_artifact);
        results.insert(other_result.artifact_id.clone(), other_result);
        let old_ack = project.acknowledgments.clone();
        let old_binding = project.bindings.clone();
        let before = check_professional_inputs(&project, &jobs, &artifacts, &results);
        assert!(!has(&before, "selected_subject_changed"));
        project.products[0].revision = SubjectRevisionId::try_from("r2".to_owned()).unwrap();
        let after = check_professional_inputs(&project, &jobs, &artifacts, &results);
        let changed: Vec<_> = after
            .iter()
            .filter(|i| i.code == "professional_inputs.selected_subject_changed")
            .collect();
        assert_eq!(changed.len(), 1);
        assert_eq!(
            changed[0].object_ref.as_ref().unwrap().id,
            project.devices[0].id
        );
        assert!(equal(&project.acknowledgments, &old_ack));
        assert!(equal(&project.bindings, &old_binding));
        assert_eq!(
            artifacts
                .values()
                .next()
                .unwrap()
                .subject_ref
                .revision
                .as_str(),
            "r1"
        );
    }

    #[test]
    fn same_subject_different_selected_job_revision_cannot_reuse_old_result() {
        let (mut project, mut jobs, artifacts, results) = input();
        let mut job = jobs.values().next().unwrap().clone();
        job.revision = Revision(SafeUint::try_from(2).unwrap());
        job.materials[0].thickness_mm = FiniteNumber::try_from(2.0).unwrap();
        jobs.insert((job.id.clone(), 2), job);
        project.devices[0].configuration.insert(
            "packaging_job_revision".into(),
            TypedValue::Integer(SafeUint::try_from(2).unwrap()),
        );
        assert!(has(
            &check_professional_inputs(&project, &jobs, &artifacts, &results),
            "result_input_mismatch"
        ));
        assert_eq!(jobs.len(), 2);
        assert_eq!(
            results.values().next().unwrap().job_ref.revision.0.value(),
            1
        );
    }

    #[test]
    fn another_route_step_on_the_same_inline_device_is_not_pack_consumption() {
        let (mut project, jobs, artifacts, results) = input();
        let before = check_professional_inputs(&project, &jobs, &artifacts, &results);
        assert!(!has(&before, "consumer_route"));
        project.routes[0].steps[0].operation_ref.id =
            ModuleId::try_from("other.preparation".to_owned()).unwrap();
        let after = check_professional_inputs(&project, &jobs, &artifacts, &results);
        assert!(has(&after, "consumer_route"));
        assert!(equal(
            &project.bindings[0].operation_ref,
            &project.devices[0].module_ref
        ));
    }

    #[test]
    fn title_and_lod_changes_do_not_create_new_professional_version_issues() {
        let (mut project, jobs, artifacts, results) = input();
        let before = check_professional_inputs(&project, &jobs, &artifacts, &results);
        project.devices[0].title = "new display name".into();
        project.views.lod = LodLevel::Simplified;
        let after = check_professional_inputs(&project, &jobs, &artifacts, &results);
        assert!(equal(&before, &after));
    }

    #[test]
    fn empty_checks_and_partial_scope_cannot_satisfy_the_complete_profile() {
        let (mut project, jobs, artifacts, mut results) = input();
        project.bindings[0].requested_scope = vec!["forming".into()];
        results.values_mut().next().unwrap().checks.clear();
        let issues = check_professional_inputs(&project, &jobs, &artifacts, &results);
        assert!(has(&issues, "incomplete_profile_scope"));
        assert_eq!(
            issues
                .iter()
                .filter(|i| i.code == "professional_inputs.required_checks")
                .count(),
            5
        );
    }

    #[test]
    fn declared_process_is_truthfully_separate_and_software_in_loop_stays_deferred() {
        let (mut project, jobs, artifacts, results) = input();
        project.bindings[0].mode = ProfessionalMode::DeclaredProcess;
        project.devices[0].configuration.clear();
        let declared = check_professional_inputs(&project, &jobs, &artifacts, &results);
        assert!(!has(&declared, "missing_job_selection"));
        assert!(!has(&declared, "result_not_pass"));
        project.bindings[0].mode = ProfessionalMode::SoftwareInLoop;
        assert!(has(
            &check_professional_inputs(&project, &jobs, &artifacts, &results),
            "unsupported_mode"
        ));
    }
}
