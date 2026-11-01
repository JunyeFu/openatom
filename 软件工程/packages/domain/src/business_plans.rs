//! Demand cancellation for a new plan restarted from its original initial input.
//! This does not edit a running Engine or turn calculation cancellation into rollback.

use std::collections::BTreeSet;

use crate::{
    AllocationMode, CapabilityPurpose, Issue, ObjectId, ObjectKind, ObjectRef, Quantity, Revision,
    SafeUint, ScenarioId, ScenarioInput,
};

fn problem(
    issues: &mut Vec<Issue>,
    source: &ScenarioInput,
    path: String,
    code: &str,
    message: &str,
) {
    issues.push(Issue {
        code: format!("business_plans.{code}"),
        object_ref: Some(ObjectRef {
            kind: ObjectKind::Scenario,
            id: ObjectId::try_from(source.id.as_str().to_owned()).expect("typed Scenario ID"),
        }),
        field_path: Some(path),
        message: message.to_owned(),
        blocks: vec![
            CapabilityPurpose::Execution,
            CapabilityPurpose::CompleteConclusion,
        ],
    });
}

/// Build an independent Scenario candidate, for submission through existing Put commands.
/// The caller supplies a fresh Scenario ID and the current Host Project revision.
/// A shared task needs explicit replanning when only some of its orders are cancelled:
/// finished-product quantities do not define a proportional material reservation release.
pub fn fork_without_unstarted_demands(
    source: &ScenarioInput,
    new_id: ScenarioId,
    project_revision: Revision,
    cancel_order_ids: &[ObjectId],
    cancel_task_ids: &[ObjectId],
) -> Result<ScenarioInput, Vec<Issue>> {
    let mut issues = vec![];
    if source.id == new_id {
        problem(
            &mut issues,
            source,
            "/id".into(),
            "new_scenario_required",
            "取消输入生成独立新方案；不能用源方案身份覆盖旧版本或当前运行。",
        );
    }
    let orders: BTreeSet<_> = cancel_order_ids.iter().collect();
    let mut tasks: BTreeSet<_> = cancel_task_ids.iter().cloned().collect();
    for id in &orders {
        if source
            .orders
            .iter()
            .filter(|order| order.id == **id)
            .count()
            != 1
        {
            problem(
                &mut issues,
                source,
                "/orders".into(),
                "order_not_unique",
                "被取消订单必须在源方案中唯一存在；不选择同名或缺失订单替代。",
            );
        }
    }
    for id in &tasks {
        if source
            .production_tasks
            .iter()
            .filter(|task| task.id == *id)
            .count()
            != 1
        {
            problem(
                &mut issues,
                source,
                "/production_tasks".into(),
                "task_not_unique",
                "被取消生产任务必须在源方案中唯一存在。",
            );
        }
    }
    for (i, task) in source.production_tasks.iter().enumerate() {
        if tasks.contains(&task.id) {
            continue;
        }
        let removed = task
            .order_allocations
            .iter()
            .filter(|a| orders.contains(&a.order_id))
            .count();
        if removed == 0 {
            continue;
        }
        if removed == task.order_allocations.len() {
            let scaled = |q: &Quantity| {
                u128::from(q.value.coefficient.value())
                    * 10_u128.pow(9 - u32::from(q.value.decimal_places.value()))
            };
            let same_units = task
                .order_allocations
                .iter()
                .all(|a| a.quantity.unit == task.planned_quantity.unit);
            let allocated = task
                .order_allocations
                .iter()
                .map(|a| scaled(&a.quantity))
                .sum::<u128>();
            if same_units && allocated == scaled(&task.planned_quantity) {
                tasks.insert(task.id.clone());
            } else {
                problem(
                    &mut issues,
                    source,
                    format!("/production_tasks/{i}/planned_quantity"),
                    "unassigned_task_requires_replan",
                    "该任务还含未分配计划量或不能按同单位确认完全归属，取消订单不能连带删除独立备货量；请显式重排或明确取消整个任务。",
                );
            }
        } else {
            problem(
                &mut issues,
                source,
                format!("/production_tasks/{i}/order_allocations"),
                "shared_task_requires_replan",
                "共享生产任务仍服务其他订单。请通过新方案明确剩余任务与物料分配；不能按成品数量比例猜测原料释放。",
            );
        }
    }
    for (i, allocation) in source
        .initial_conditions
        .material_allocations
        .iter()
        .enumerate()
    {
        let affected = tasks.contains(&allocation.demand_ref)
            || (matches!(
                allocation.mode,
                AllocationMode::SoftPlan | AllocationMode::Committed
            ) && orders.contains(&allocation.demand_ref));
        if !affected {
            continue;
        }
        if matches!(allocation.mode, AllocationMode::Committed) {
            problem(
                &mut issues,
                source,
                format!("/initial_conditions/material_allocations/{i}/mode"),
                "committed_not_cancellable",
                "已投入分配不能由新方案输入取消函数释放；保留原运行及在制，按声明处置或从原初始输入另建方案。",
            );
        }
        if matches!(allocation.mode, AllocationMode::SoftPlan)
            && source
                .orders
                .iter()
                .any(|order| order.id == allocation.demand_ref)
            && source
                .production_tasks
                .iter()
                .any(|task| task.id == allocation.demand_ref)
        {
            problem(
                &mut issues,
                source,
                format!("/initial_conditions/material_allocations/{i}/demand_ref"),
                "ambiguous_soft_demand",
                "软计划引用同时命中订单和生产任务，不能猜测应取消哪一方。",
            );
        }
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut candidate = source.clone();
    candidate.id = new_id;
    candidate.revision = Revision(SafeUint::try_from(1).expect("first Scenario revision"));
    candidate.project_revision = project_revision;
    candidate.orders.retain(|order| !orders.contains(&order.id));
    candidate
        .production_tasks
        .retain(|task| !tasks.contains(&task.id));
    candidate
        .initial_conditions
        .material_allocations
        .retain(|allocation| {
            !tasks.contains(&allocation.demand_ref)
                && !(matches!(allocation.mode, AllocationMode::SoftPlan)
                    && orders.contains(&allocation.demand_ref))
        });
    Ok(candidate)
}
