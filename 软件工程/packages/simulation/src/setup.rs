//! Declared sequence-dependent preparation and first-article gates, owned by
//! the same Engine as production. A setup Job never creates material.
use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Rule {
    from: Option<String>,
    to: String,
    work: u64,
    resources: BTreeMap<ObjectId, u64>,
    external: bool,
}
#[derive(Debug, Clone)]
pub(crate) struct Policy {
    device: ObjectId,
    capacity: ObjectId,
    initial: Option<String>,
    family: String,
    rules: Vec<Rule>,
    article: Option<FirstArticleRequirement>,
    route: SubjectRevisionRef,
}
#[derive(Debug, Clone)]
pub(crate) struct Change {
    external: bool,
    device: ObjectId,
    capacity: ObjectId,
    to: String,
    from: Option<String>,
}
impl Change {
    pub(crate) fn transition(&self) -> sf_domain::cloud_execution::SetupTransition {
        sf_domain::cloud_execution::SetupTransition { from_family: self.from.clone(), to_family: self.to.clone() }
    }
}

pub(crate) fn external_setup(spec: &Job, transition: &sf_domain::cloud_execution::SetupTransition) -> Option<Job> {
    let policy = spec.family.as_ref()?;
    let rule = policy.rules.iter().find(|r| r.from == transition.from_family && r.to == transition.to_family && r.to == policy.family)?;
    Some(setup_spec(spec, policy, rule))
}

pub(crate) fn required_transitions(spec: &Job) -> Vec<sf_domain::cloud_execution::SetupTransition> {
    spec.family.iter().flat_map(|p| p.rules.iter().filter(|r| r.to == p.family))
        .map(|r| sf_domain::cloud_execution::SetupTransition { from_family: r.from.clone(), to_family: r.to.clone() }).collect()
}

fn setup_spec(original: &Job, policy: &Policy, rule: &Rule) -> Job {
    let mut spec = original.clone();
    spec.inputs.clear();
    spec.outputs.clear();
    spec.preparation = None;
    spec.quality = None;
    spec.family = None;
    spec.output_holds.clear();
    spec.job_resources = rule.resources.clone();
    if !rule.external { spec.job_resources.entry(policy.capacity.clone()).and_modify(|n| *n = (*n).max(1)).or_insert(1); }
    spec.phases = vec![Phase { id: "setup".to_owned().try_into().unwrap(), work: rule.work, resources: BTreeMap::new(), pause_resume: false }];
    spec.predecessors.clear();
    spec.setup = Some(Change { external: rule.external, device: policy.device.clone(), capacity: policy.capacity.clone(), to: policy.family.clone(), from: rule.from.clone() });
    spec
}
pub(crate) struct Family {
    current: Option<String>,
    epoch: u64,
    pending: Option<usize>,
}
pub(crate) struct ArticleGate {
    candidate: Option<usize>,
    released: bool,
    inspection: ModuleRef,
}

pub(crate) fn compile(
    input: &EngineInput,
    device: &DeviceInstance,
    step: &RouteStep,
    operation: &OperationDefinition,
    route_ref: &SubjectRevisionRef,
    issues: &mut Vec<Issue>,
) -> Option<Policy> {
    let family = operation.family.as_ref()?;
    let owner = object(ObjectKind::Device, &device.id);
    let policies: Vec<_> = input
        .project
        .setup_policies
        .iter()
        .filter(|p| p.device_id == device.id)
        .collect();
    if policies.len() != 1 {
        issues.push(issue(
            owner,
            "/setup_policies",
            "setup_policy",
            "A family operation requires one explicit device setup policy.",
        ));
        return None;
    }
    let policy = policies[0];
    let initial = input
        .scenario
        .initial_conditions
        .initial_setup_families
        .get(&device.id)
        .cloned()
        .or_else(|| policy.initial_family.clone());
    let mut seen = BTreeSet::new();
    let mut rules = vec![];
    for rule in &policy.changeovers {
        if !seen.insert((rule.from_family.clone(), rule.to_family.clone())) {
            issues.push(issue(
                owner.clone(),
                "/setup_policies/changeovers",
                "setup_matrix",
                "Changeovers require unique exact from/to families and canonical milliseconds.",
            ));
        }
        rules.push(Rule {
            from: rule.from_family.clone(),
            to: rule.to_family.clone(),
            work: rule.duration.value.0.value(),
            resources: resource_requirements(input, device, step, &rule.resources, &owner, issues),
            external: matches!(rule.preparation_mode, PreparationMode::External),
        });
    }
    let mut starts: BTreeSet<_> = policy
        .changeovers
        .iter()
        .map(|r| Some(r.to_family.clone()))
        .collect();
    starts.insert(initial.clone());
    for from in starts {
        if from.as_ref() != Some(family) && !rules.iter().any(|r| r.from == from && r.to == *family)
        {
            issues.push(issue(owner.clone(), "/setup_policies/changeovers", "setup_transition_missing", format!("No explicit changeover from {from:?} to {family}; unknown cold starts are not zero-time setup.")));
        }
    }
    if let Some(article) = &policy.first_article {
        let valid_route = input
            .project
            .routes
            .iter()
            .any(|r| r.id == article.route_ref.id && r.revision == article.route_ref.revision);
        let valid_inspector = module(input,&article.inspection_module).is_some_and(|d|matches!(&d.payload,ModulePayload::Operations(op) if matches!(op.builtin,RuleKind::InspectAndRoute)));
        if !valid_route || !valid_inspector {
            issues.push(issue(
                owner.clone(),
                "/setup_policies/first_article",
                "first_article_source",
                "First article requires its exact declared route and inspection operation version.",
            ));
        }
    }
    Some(Policy {
        device: device.id.clone(),
        capacity: device.role_bindings.resources
            [&RoleId::try_from("execution_capacity".to_owned()).unwrap()]
            .clone(),
        initial,
        family: family.clone(),
        rules,
        article: policy.first_article.clone(),
        route: route_ref.clone(),
    })
}

impl Engine {
    // None means ready; Some(true) created an actual setup Job, Some(false) waits.
    pub(super) fn prepare_family(&mut self, index: usize) -> Option<bool> {
        if self.jobs[index].state.started_at_ms.is_some() || self.jobs[index].spec.setup.is_some() {
            return None;
        }
        let policy = self.jobs[index].spec.family.clone()?;
        self.families
            .entry(policy.device.clone())
            .or_insert_with(|| Family {
                current: policy.initial.clone(),
                epoch: 0,
                pending: None,
            });
        let state = &self.families[&policy.device];
        if let Some(pending) = state.pending {
            let change = self.jobs[pending].spec.setup.as_ref().unwrap();
            if state.current.as_ref() != Some(&policy.family) {
                if change.external
                    && change.to == policy.family
                    && self.jobs[pending].produced
                    && self.occupied(&policy.capacity) == 0
                {
                    self.activate_setup(pending);
                    return Some(true);
                }
                self.family_wait(
                    index,
                    &policy.device,
                    "Declared changeover has not committed",
                );
                return Some(false);
            }
        }
        if state.current.as_ref() != Some(&policy.family) {
            let rule = policy
                .rules
                .iter()
                .find(|r| r.from == state.current && r.to == policy.family)
                .cloned();
            let Some(rule) = rule else {
                self.limitations.push(issue(
                    object(ObjectKind::Device, &policy.device),
                    "/setup_policies/changeovers",
                    "setup_transition_missing",
                    "Actual current family has no declared transition.",
                ));
                self.status = ComputationStatus::Failed;
                return Some(false);
            };
            if !rule.external && self.occupied(&policy.capacity) > 0 {
                self.family_wait(
                    index,
                    &policy.device,
                    "Internal preparation requires free equipment",
                );
                return Some(false);
            }
            let mut spec = setup_spec(&self.jobs[index].spec, &policy, &rule);
            spec.id = stable(
                "setup",
                &[
                    policy.device.as_str(),
                    &state.epoch.to_string(),
                    spec.id.as_str(),
                ],
            );
            spec.release = self.now;
            let next = self.jobs.len();
            let mut running = RunningJob::new(spec);
            running.dormant = false;
            self.jobs.push(running);
            self.families.get_mut(&policy.device).unwrap().pending = Some(next);
            self.family_wait(
                index,
                &policy.device,
                "Await actual sequence-dependent preparation",
            );
            return Some(true);
        }
        let article = policy
            .article
            .as_ref()
            .filter(|a| a.family == policy.family && same_type(&a.route_ref, &policy.route))?;
        let key = stable(
            "first-article",
            &[
                policy.device.as_str(),
                self.jobs[index].spec.operation_ref.id.as_str(),
                &String::from(self.jobs[index].spec.operation_ref.version.clone()),
                policy.route.id.as_str(),
                policy.route.revision.as_str(),
                &state.epoch.to_string(),
                article.gate_id.as_str(),
            ],
        );
        let gate = self
            .article_gates
            .entry(key.clone())
            .or_insert_with(|| ArticleGate {
                candidate: None,
                released: false,
                inspection: article.inspection_module.clone(),
            });
        if !gate.released && gate.candidate.is_some_and(|candidate| candidate != index) {
            self.family_wait(
                index,
                &policy.device,
                "Matching first article of this setup epoch needs an actual release decision",
            );
            return Some(false);
        }
        self.jobs[index].article_key = Some(key);
        None
    }
    fn family_wait(&mut self, index: usize, device: &ObjectId, required: &str) {
        self.jobs[index].state.state = JobStatus::WaitingResource;
        self.set_wait(
            index,
            vec![WaitReason {
                kind: WaitReasonKind::Preparation,
                blocking_object: Some(object(ObjectKind::Device, device)),
                required: required.into(),
                next_known_boundary_ms: None,
            }],
            false,
        );
    }
    pub(super) fn start_article(&mut self, index: usize) {
        if let Some(key) = &self.jobs[index].article_key {
            self.article_gates
                .get_mut(key)
                .unwrap()
                .candidate
                .get_or_insert(index);
        }
    }
    pub(super) fn release_article(&mut self, decision: &QualityDecision) {
        if !matches!(decision.disposition, QualityDisposition::Release) {
            return;
        }
        for ancestor in self.lineage(&decision.item_ref) {
            if let Some(key) = self.article_items.get(&ancestor) {
                let gate = self.article_gates.get_mut(key).unwrap();
                if same_module(&gate.inspection, &decision.inspection_ref) {
                    gate.released = true;
                }
            }
        }
    }
    pub(super) fn commit_setup_one(&mut self) -> bool {
        for i in 0..self.jobs.len() {
            let Some(change) = self.jobs[i].spec.setup.clone() else {
                continue;
            };
            if !self.jobs[i].produced || matches!(self.jobs[i].state.state, JobStatus::Completed) {
                continue;
            }
            if change.external {
                self.set_wait(i,vec![WaitReason{kind:WaitReasonKind::Preparation,blocking_object:Some(object(ObjectKind::Device,&change.device)),required:"External preparation complete; activation follows actual recipe dispatch on free equipment".into(),next_known_boundary_ms:None}],true);
                continue;
            }
            // Its own internal fixture claim is allowed. External preparation can
            // finish while an older family still works, but cannot switch it.
            if self.occupied(&change.capacity) > self.held(i, &change.capacity) {
                self.family_wait_completed_setup(i, &change.device);
                continue;
            }
            self.activate_setup(i);
            return true;
        }
        false
    }
    fn activate_setup(&mut self, index: usize) {
        let change = self.jobs[index].spec.setup.as_ref().unwrap();
        let state = self.families.get_mut(&change.device).unwrap();
        state.current = Some(change.to.clone());
        state.epoch += 1;
        state.pending = None;
        self.finish(index);
    }
    fn family_wait_completed_setup(&mut self, index: usize, device: &ObjectId) {
        self.set_wait(
            index,
            vec![WaitReason {
                kind: WaitReasonKind::Preparation,
                blocking_object: Some(object(ObjectKind::Device, device)),
                required: "Prepared family waits for equipment before activation".into(),
                next_known_boundary_ms: None,
            }],
            true,
        );
    }
}

pub(super) struct ExternalFamilyPlan {
    pub work: u64,
    pub resources: BTreeMap<ObjectId, u64>,
    pub pending: Option<usize>,
}
impl Engine {
    pub(super) fn external_family_plan(&self, spec: &Job) -> Result<Option<ExternalFamilyPlan>, String> {
        let Some(policy) = &spec.family else { return Ok(None) };
        let state = self.families.get(&policy.device);
        let current = state.map_or(&policy.initial, |s| &s.current);
        if current.as_ref() == Some(&policy.family) { return Ok(None); }
        if let Some(index) = state.and_then(|s| s.pending) {
            let job = &self.jobs[index];
            if job.spec.setup.as_ref().unwrap().to != policy.family {
                return Err("external.other_family_preparation_pending: current preparation must settle before estimating another family".into());
            }
            if current.as_ref() != Some(&policy.family) {
                return Ok(Some(ExternalFamilyPlan { work: job.spec.phases[0].work,
                    resources: job.spec.job_resources.clone(), pending: Some(index) }));
            }
        }
        let rule = policy.rules.iter().find(|r| &r.from == current && r.to == policy.family)
            .ok_or_else(|| "external.setup_transition_missing: actual family has no declared transition".to_owned())?;
        let mut resources = rule.resources.clone();
        if !rule.external { resources.entry(policy.capacity.clone()).and_modify(|n| *n = (*n).max(1)).or_insert(1); }
        Ok(Some(ExternalFamilyPlan { work: rule.work, resources, pending: None }))
    }
}
