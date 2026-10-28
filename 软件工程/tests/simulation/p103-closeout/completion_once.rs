#[path = "support.rs"]
mod cases;
use cases::{combined, original::*};
use serde_json::json;
use sf_domain::{HolderRef, SimTimeMs, SimulationEventPayload};

fn time(value: u64) -> SimTimeMs {
    SimTimeMs(value.try_into().unwrap())
}

#[test]
fn resumed_work_and_repeated_history_queries_commit_one_physical_output_and_transfer() {
    let mut evidence = vec![];
    for policy in ["pause_resume", "non_preemptive"] {
        for capacity in [0, 1] {
            let value = combined(policy, capacity);
            let mut e = engine(&value);
            while e
                .snapshot()
                .computed_until_ms
                .as_ref()
                .is_none_or(|t| t.0.value() < 20_000)
            {
                e.step();
            }
            e.pause();
            let prefix = snapshot(&e);
            assert!(e.objects_at(time(20_001)).is_none());
            let held = e.objects_at(time(20_000)).unwrap();
            let raw = held
                .iter()
                .find(|i| i.id.as_str() == "noncu-future-raw")
                .unwrap();
            assert!(matches!(raw.holder, HolderRef::Job(_)));
            e.resume();
            e.run_to_end();
            let finished = snapshot(&e);
            let parts = physical(&finished, "part");
            assert_eq!(parts.len(), 1);
            let part = &parts[0];
            let transfers: Vec<_> = e
                .snapshot()
                .events
                .into_iter()
                .filter_map(|event| match event.payload {
                    SimulationEventPayload::ItemTransferred(transfer)
                        if transfer.item_ref.as_str() == part["id"].as_str().unwrap() =>
                    {
                        Some(transfer)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(transfers.len(), usize::from(capacity == 1));
            assert_eq!(
                part["holder"]["kind"],
                if capacity == 1 { "location" } else { "job" }
            );
            if capacity == 1 {
                assert!(
                    matches!(&transfers[0].to_holder,HolderRef::Location(id)if id.as_str()=="finished")
                );
                assert_eq!(finished["jobs"][0]["finished_at_ms"], 85000);
            } else {
                assert_eq!(finished["jobs"][0]["pending_outputs"], json!([part["id"]]));
                assert!(finished["jobs"][0]["finished_at_ms"].is_null());
            }
            for at in [
                19_999, 20_000, 40_000, 79_999, 80_000, 84_999, 85_000, 100_000, 20_000,
            ] {
                let _ = e.objects_at(time(at)).unwrap();
                let _ = e.jobs_at(time(at)).unwrap();
            }
            e.run_to_end();
            assert_eq!(snapshot(&e), finished);
            evidence.push(json!({"policy":policy,"capacity":capacity,"paused":prefix,
                "output_transfer_count":transfers.len(),"run":finished}));
        }
    }
    cases::evidence(
        "completion-once.json",
        &json!({"passed":true,"cases":evidence}),
    );
}
