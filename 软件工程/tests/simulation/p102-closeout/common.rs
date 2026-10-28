#![allow(dead_code)]
// This is a consumer of the frozen public Host and its one production Engine.
// Existing fixture adapters stay read-only; no production behavior lives here.
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
pub mod host;
#[path = "../../../packages/commands/tests/fixtures/production/p1_engine_support.rs"]
pub mod p1;

use serde_json::{Value, json};
use sf_domain::*;
use std::{collections::BTreeSet, path::PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_owned()
}

pub fn expected(case: &str) -> Value {
    let source: Value = serde_json::from_str(include_str!(
        "../../../scenarios/并行汇合与返工/cases.design.json"
    ))
    .unwrap();
    let row = source["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == case)
        .unwrap();
    assert_eq!(p1::original_expectation(case), row["expected"]);
    row["expected"].clone()
}

fn at_device(job: &JobState, device: &str) -> bool {
    matches!(&job.execution_ref, JobExecutionRef::ProjectDevice { device_id, .. }
        if device_id.as_str() == device)
}

pub fn assert_fixed(case: &str, run: &RunResult) -> Value {
    assert!(matches!(
        run.evidence_origin,
        EvidenceOrigin::ActualSoftware
    ));
    assert!(matches!(
        run.computation_status,
        ComputationStatus::Completed
    ));
    assert!(run.whole_window_computed);
    assert_eq!(run.computed_until_ms.as_ref().unwrap().0.value(), 180_000);
    assert!(run.limitations.is_empty(), "{:?}", run.limitations);
    let decisions: Vec<_> = run
        .events
        .iter()
        .filter_map(|event| match &event.payload {
            SimulationEventPayload::QualityChanged(decision) => Some(decision),
            _ => None,
        })
        .collect();
    // The frozen independent Host fixture forbids partial order acceptance.
    // Package completion is measured at its packing job, as in the original
    // independent acceptance test; it is distinct from whole-order acceptance.
    let mut package_finish_seconds: Vec<_> = run
        .jobs
        .iter()
        .filter(|job| at_device(job, "PACK-01"))
        .filter_map(|job| job.finished_at_ms.as_ref().map(|t| t.0.value() / 1000))
        .collect();
    package_finish_seconds.sort();
    let summary = json!({
        "package_finish_seconds":package_finish_seconds,
        "packages":run.final_items.iter().filter(|i|i.item_type_ref.id.as_str()=="package").count(),
        "quarantined":run.final_items.iter().filter(|i|matches!(&i.holder,HolderRef::Location(id)if id.as_str()=="quarantine")).count(),
        "awaiting_result":run.final_items.iter().filter(|i|matches!(i.quality,QualityState::Pending)).count(),
        "rework_cycles":run.final_items.iter().map(|i|i.completed_reworks.value()).sum::<u64>(),
        "inspection_started_count":run.jobs.iter().filter(|j|at_device(j,"QC-01")&&j.started_at_ms.is_some()).count(),
        "quality_decisions":decisions.len(),
        "waiting_for_inspector":run.jobs.iter().filter(|j|at_device(j,"QC-01")&&j.started_at_ms.is_none()&&matches!(j.state,JobStatus::WaitingResource)).count()
    });
    assert_eq!(summary, expected(case), "original case {case}");
    let packages = summary["packages"].as_u64().unwrap();
    let accepted = if packages == 2 { 2 } else { 0 };
    assert_eq!(run.orders[0].accepted.value.coefficient.value(), accepted);
    assert_eq!(
        run.orders[0].outstanding.value.coefficient.value(),
        2 - accepted
    );
    assert_eq!(run.goal_met, Some(case == "A"));

    for resource in run
        .resource_claims
        .iter()
        .map(|c| c.resource_id.clone())
        .collect::<BTreeSet<_>>()
    {
        let mut spans: Vec<_> = run
            .resource_claims
            .iter()
            .filter(|c| c.resource_id == resource)
            .flat_map(|c| {
                assert_eq!(c.units.value(), 1);
                assert_eq!(c.intervals.len(), c.interval_end_causes.len());
                c.intervals
                    .iter()
                    .map(|i| (i.start_ms.0.value(), i.end_ms.0.value()))
            })
            .collect();
        spans.sort();
        assert!(
            spans.windows(2).all(|pair| pair[0].1 <= pair[1].0),
            "{resource:?}: {spans:?}"
        );
    }
    let assemblies: Vec<_> = run
        .final_items
        .iter()
        .filter(|i| i.item_type_ref.id.as_str() == "assembly")
        .collect();
    assert_eq!(assemblies.len(), 2);
    assert_eq!(
        run.final_items
            .iter()
            .map(|i| &i.id)
            .collect::<BTreeSet<_>>()
            .len(),
        run.final_items.len()
    );
    assert!(
        assemblies
            .iter()
            .all(|i| !matches!(i.lifecycle, Lifecycle::Scrapped | Lifecycle::Transformed))
    );
    assert_eq!(
        decisions
            .iter()
            .map(|d| &d.id)
            .collect::<BTreeSet<_>>()
            .len(),
        decisions.len()
    );
    for decision in decisions {
        assert!(matches!(decision.source_kind, QualitySourceKind::Preset));
        assert!(assemblies.iter().any(|i| i.id == decision.item_ref));
    }
    for package in run
        .final_items
        .iter()
        .filter(|i| i.item_type_ref.id.as_str() == "package")
    {
        assert_eq!(package.contents.len(), 1);
        assert!(assemblies.iter().any(|i| i.id == package.contents[0]
            && matches!(&i.holder,HolderRef::Container(id)if *id==package.id)
            && matches!(i.quality, QualityState::Released)));
    }
    let reworks: Vec<_> = run
        .jobs
        .iter()
        .filter(|j| at_device(j, "RW-01") && j.finished_at_ms.is_some())
        .collect();
    assert_eq!(reworks.len(), if case == "U" { 0 } else { 1 });
    for rework in reworks {
        assert_eq!(rework.input_refs.len(), 1);
        let item = assemblies
            .iter()
            .find(|i| i.id == rework.input_refs[0])
            .unwrap();
        assert_eq!(item.completed_reworks.value(), 1);
        assert!(run.jobs.iter().any(|j| {
            at_device(j, "QC-01")
                && j.input_refs == rework.input_refs
                && j.started_at_ms
                    .as_ref()
                    .is_some_and(|t| t.0.value() == 120_000)
        }));
    }
    if case == "Q" {
        let item = assemblies
            .iter()
            .find(|i| matches!(&i.holder,HolderRef::Location(id)if id.as_str()=="quarantine"))
            .unwrap();
        assert!(matches!(item.quality, QualityState::Rejected));
    }
    if case == "U" {
        let held = run
            .jobs
            .iter()
            .find(|j| at_device(j, "QC-01") && j.started_at_ms.is_some())
            .unwrap();
        for resource in ["W-Q", "CAP-QC-01"] {
            let result = serde_json::to_value(run).unwrap();
            assert_eq!(p1::intervals(&result, resource), vec![(90_000, 180_000)]);
        }
        let work: u64 = run
            .phase_work_intervals
            .iter()
            .filter(|w| w.job_id == held.id)
            .map(|w| w.interval.end_ms.0.value() - w.interval.start_ms.0.value())
            .sum();
        assert_eq!(work, 10_000);
    }
    summary
}

pub fn business(mut value: Value) -> Value {
    // Run identity and its exact frozen-file refs necessarily change on a new run.
    // Every actual event, item ID/holder, job, resource interval and outcome stays.
    value.as_object_mut().unwrap().remove("id");
    value.as_object_mut().unwrap().remove("input_ref");
    value
}
