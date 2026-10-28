//! A declared acceptance target is a consumer of real outputs, never a source
//! of inventory, production tasks or synthetic OrderLines.
use crate::{EngineInput, compile::*};
use sf_domain::*;

pub(crate) fn select(
    input: &EngineInput,
    explicit: Option<&FactoryGenerationGoal>,
) -> Result<Option<FactoryGenerationGoal>, Vec<Issue>> {
    let project = object(ObjectKind::Project, &id(input.project.id.as_str()));
    let scenario = object(ObjectKind::Scenario, &id(input.scenario.id.as_str()));
    let sources = [
        (
            input.scenario.acceptance_goal.as_ref(),
            scenario.clone(),
            "/acceptance_goal",
        ),
        (
            input.project.generation_context.as_ref().map(|c| &c.goal),
            project.clone(),
            "/generation_context/goal",
        ),
        (explicit, project, "/generation_goal"),
    ];
    let mut selected: Option<(&FactoryGenerationGoal, ObjectRef, &str)> = None;
    let mut issues = vec![];
    for (goal, owner, path) in sources {
        let Some(goal) = goal else {
            continue;
        };
        if let Some((previous, _, _)) = &selected {
            // FactoryGenerationGoal has no PartialEq; its derived Debug covers
            // every typed field, including constraints and site metadata. Do
            // not compare only quantities or acceptance_scope and silently
            // select a different frozen source by precedence.
            if format!("{previous:?}") != format!("{goal:?}") {
                issues.push(issue(owner,path,"goal_source_conflict","Saved Scenario goal, real generation-context goal and explicit goal must agree in every field; no source overrides another."));
            }
        } else {
            selected = Some((goal, owner, path));
        }
    }
    if !input.scenario.orders.is_empty() {
        if input.scenario.acceptance_goal.is_some() || explicit.is_some() {
            issues.push(issue(scenario,if input.scenario.acceptance_goal.is_some(){"/acceptance_goal"}else{"/orders"},"goal_order_mode_conflict","A manual Scenario or explicit legacy goal cannot coexist with OrderLines; actual orders keep their own acceptance scopes."));
        }
        // A genuine generation context alone does not replace or change the
        // existing order-mode rules.
        return if issues.is_empty() {
            Ok(None)
        } else {
            Err(issues)
        };
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let Some((goal, owner, path)) = selected else {
        return Ok(None);
    };
    let scope = &goal.acceptance_scope;
    validate_scope(
        input,
        scope,
        &owner,
        &format!("{path}/acceptance_scope"),
        &mut issues,
    );
    for (index, additional) in goal.additional_acceptance_scopes.iter().enumerate() {
        let scope_path = format!("{path}/additional_acceptance_scopes/{index}");
        validate_scope(input, additional, &owner, &scope_path, &mut issues);
        if additional.unit != scope.unit {
            issues.push(issue(owner.clone(),format!("{scope_path}/unit"),"goal_unit","Additional acceptance scopes must use the primary goal unit; no mixed-unit aggregate is inferred."));
        }
    }
    if goal.observation_window.start_ms.0 != input.scenario.window.start_ms.0
        || goal.observation_window.end_ms.0 != input.scenario.window.end_ms.0
    {
        issues.push(issue(
            owner.clone(),
            format!("{path}/observation_window"),
            "goal_window",
            "The declared goal and actual Run must use the same observation window.",
        ));
    }
    if goal
        .accepted_quantity_per_day
        .as_ref()
        .is_some_and(|q| q.unit != scope.unit || amount(q) == 0)
    {
        issues.push(issue(owner.clone(),format!("{path}/accepted_quantity_per_day"),"goal_quantity","A specified goal quantity must be positive and use the explicit acceptance unit; it does not cap production or cumulative acceptance."));
    }
    if goal
        .production_calendar_ref
        .as_ref()
        .is_some_and(|id| !input.project.calendars.iter().any(|c| c.id == *id))
    {
        issues.push(issue(
            owner,
            format!("{path}/production_calendar_ref"),
            "goal_calendar",
            "A declared goal calendar must reference an actual project calendar.",
        ));
    }
    issues.extend(
        goal.constraints
            .iter()
            .filter(|i| {
                i.blocks
                    .iter()
                    .any(|p| matches!(p, CapabilityPurpose::Execution))
            })
            .cloned(),
    );
    if issues.is_empty() {
        Ok(Some(goal.clone()))
    } else {
        Err(issues)
    }
}

fn validate_scope(
    input: &EngineInput,
    scope: &AcceptanceScope,
    owner: &ObjectRef,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let product = input.project.products.iter().find(|p| {
        same_type(
            &SubjectRevisionRef {
                id: p.id.clone(),
                revision: p.revision.clone(),
            },
            &scope.product_ref,
        )
    });
    if product.is_none() {
        issues.push(issue(owner.clone(),format!("{path}/product_ref"),"goal_product_reference","Goal acceptance requires an exact declared Product revision, not an inferred output type."));
    }
    if product.is_some_and(|p| {
        !input.project.item_types.iter().any(|i| {
            same_type(
                &SubjectRevisionRef {
                    id: i.id.clone(),
                    revision: i.revision.clone(),
                },
                &p.item_type_ref,
            ) && i.unit == scope.unit
        })
    }) {
        issues.push(issue(owner.clone(),format!("{path}/unit"),"goal_unit","Goal acceptance unit must match the exact product's declared item type; no unit conversion is guessed."));
    }
    if !input
        .project
        .locations
        .iter()
        .any(|l| l.id == scope.destination)
    {
        issues.push(issue(
            owner.clone(),
            format!("{path}/destination"),
            "goal_destination",
            "Goal destination must be an explicit real location.",
        ));
    }
    if !matches!(scope.stage, AcceptanceStage::FactoryReleased)
        && !crate::engine::transport::supports_acceptance(
            input,
            &scope.product_ref,
            &scope.destination,
        )
    {
        issues.push(issue(owner.clone(),format!("{path}/stage"),"goal_acceptance_stage_scope","This production Engine proves factory_released only; received/unloaded acceptance requires the actual transport consumer."));
    }
}
