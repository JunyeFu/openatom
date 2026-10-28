//! S06-01 std-only queue/API experiment, not the production simulation engine.
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EventKey {
    at_ms: u64,
    phase: u8,
    subject: &'static str,
    sequence: u64,
}

fn ordered_events() -> Vec<EventKey> {
    let input = [
        EventKey { at_ms: 10, phase: 3, subject: "start", sequence: 4 },
        EventKey { at_ms: 10, phase: 1, subject: "arrival", sequence: 2 },
        EventKey { at_ms: 10, phase: 0, subject: "done", sequence: 1 },
        EventKey { at_ms: 10, phase: 2, subject: "transfer", sequence: 3 },
        EventKey { at_ms: 11, phase: 0, subject: "later", sequence: 5 },
    ];
    let mut heap: BinaryHeap<Reverse<EventKey>> = input.into_iter().map(Reverse).collect();
    let mut ordered = Vec::new();
    while let Some(Reverse(event)) = heap.pop() {
        ordered.push(event);
    }
    ordered
}

fn main() {
    let events = ordered_events();
    assert_eq!(events.iter().map(|e| e.subject).collect::<Vec<_>>(),
               ["done", "arrival", "transfer", "start", "later"]);
    assert_eq!(events, ordered_events());
    let mut objects = BTreeMap::new();
    objects.insert("B", 2u64);
    objects.insert("A", 1u64);
    assert_eq!(objects.keys().copied().collect::<Vec<_>>(), ["A", "B"]);
    let clock_end = 40u64;
    let accepted = [(40u64, 1u64), (41, 1)].iter()
        .filter(|(t, _)| *t <= clock_end).map(|(_, q)| q).sum::<u64>();
    assert_eq!(accepted, 1);
    println!(r#"{{"status":"passed_actual_std_queue_subset","event_order":["done","arrival","transfer","start","later"],"stable_key_iteration":["A","B"],"inclusive_window_end_quantity":1,"repeat_order_equal":true,"actual_product_engine":false}}"#);
}
