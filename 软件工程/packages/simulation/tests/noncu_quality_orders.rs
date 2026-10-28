mod noncu_quality_support;
use noncu_quality_support::*;
use original::*;
use serde_json::{Value, json};
use sf_domain::*;
fn t(n: u64) -> SimTimeMs {
    SimTimeMs(n.try_into().unwrap())
}
fn at(e: &sf_simulation::Engine, n: u64, id: &str) -> Value {
    json!(
        e.objects_at(t(n))
            .unwrap()
            .into_iter()
            .find(|i| i.id.as_str() == id)
            .unwrap()
    )
}

#[test]
fn explicit_three_piece_order_keeps_original_quality_identity_and_one_outstanding() {
    let v = profile("orders");
    let before = v.clone();
    let mut e = engine(&v);
    e.run_to_end();
    let r = snapshot(&e);
    assert_eq!(v, before);
    invariant(&r, 2, 1);
    assert_eq!(accepted(&r), 2);
    assert_eq!(r["orders"].as_array().unwrap().len(), 1);
    assert_eq!(r["orders"][0]["accepted"]["value"]["coefficient"], 2);
    assert_eq!(r["orders"][0]["outstanding"]["value"]["coefficient"], 1);
    assert!(r["orders"][0]["completion_ms"].is_null());
    assert!(
        r["acceptance_events"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["order_id"] == "NONCU-QUALITY-ORDER")
    );
    let pending = at(&e, 80000, &assembly(2));
    let checking = at(&e, 80000, &assembly(1));
    assert_eq!(pending["quality"], "pending");
    assert_eq!(
        pending["holder"],
        json!({"kind":"location","id":"pending-store"})
    );
    assert_eq!(checking["lifecycle"], "in_process");
    assert_eq!(checking["holder"]["kind"], "job");
    assert_eq!(named(&r, &assembly(1))["lifecycle"], "scrapped");
    assert_eq!(named(&r, &assembly(1))["completed_reworks"], 1);
    evidence(
        "explicit-order.json",
        &json!({"passed":true,"input":v,"run":r,"at80":[pending,checking]}),
    );
}

#[test]
fn invalid_quality_decisions_do_not_fill_real_order_or_cache_future_release() {
    let mut v = profile("orders");
    invalid_decisions(&mut v);
    let mut e = engine(&v);
    e.run_to_end();
    let r = snapshot(&e);
    invariant(&r, 1, 1);
    assert_eq!(accepted(&r), 1);
    assert_eq!(r["orders"][0]["outstanding"]["value"]["coefficient"], 2);
    assert_eq!(named(&r, &assembly(2))["quality"], "pending");
    assert_eq!(named(&r, &assembly(2))["holder"]["id"], "pending-store");
    assert_eq!(
        r["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["code"] == "simulation.quality_decision_unapplied")
            .count(),
        4
    );
    let mut end = profile("orders");
    end["scenario"]["initial_conditions"]["declared_quality_decisions"][0]["effective_at_ms"] =
        json!(120000);
    let mut at_end = engine(&end);
    at_end.run_to_end();
    let boundary = snapshot(&at_end);
    assert_eq!(accepted(&boundary), 1);
    assert_eq!(
        boundary["orders"][0]["outstanding"]["value"]["coefficient"],
        2
    );
    assert_eq!(at(&at_end, 119999, &assembly(2))["quality"], "pending");
    assert_eq!(at(&at_end, 120000, &assembly(2))["quality"], "released");
    assert!(at_end.objects_at(t(120001)).is_none());
    assert_eq!(
        at(&at_end, 120000, &assembly(2))["holder"],
        json!({"kind":"location","id":"passed-in"})
    );
    assert!(
        boundary["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|j| j["operation_ref"]["id"] != "w5.op.pack" || j["started_at_ms"] != 120000),
        "release at the closed ALL calendar boundary cannot start packing"
    );
    evidence(
        "decision-boundaries.json",
        &json!({"passed":true,"invalid_input":v,"invalid_run":r,"end_input":end,"end_run":boundary}),
    );
}

#[test]
fn failed_explicit_reinspection_cannot_evict_scrap_from_finite_isolation() {
    let input = profile("isolation");
    let modified = edited("isolation", &input);
    let mut results = vec![];
    for (capacity, v) in [(1, input), (2, modified)] {
        let mut e = engine(&v);
        e.run_to_end();
        let r = snapshot(&e);
        invariant(&r, 1, 1);
        let item = named(&r, &assembly(2));
        assert_eq!(item["quality"], "rejected");
        assert_eq!(item["lifecycle"], "available");
        assert_eq!(item["completed_reworks"], 1);
        assert_eq!(named(&r, &assembly(1))["lifecycle"], "scrapped");
        assert_eq!(named(&r, &assembly(1))["holder"]["id"], "quarantine");
        if capacity == 1 {
            assert_eq!(item["holder"]["kind"], "job");
        } else {
            assert_eq!(item["holder"], json!({"kind":"location","id":"quarantine"}));
        }
        assert!(
            intervals(&r, "W-Q").contains(&(115000, if capacity == 1 { 140000 } else { 125000 }))
        );
        results.push(
            json!({"capacity":capacity,"input":v,"run":r,"at125":at(&e,125000,&assembly(2))}),
        );
    }
    evidence(
        "isolation-capacity.json",
        &json!({"passed":true,"cases":results}),
    );
}
