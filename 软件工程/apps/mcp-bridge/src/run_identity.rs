//! Compare only actually observed frozen Run identities and computed frontiers.
use serde_json::{Value, json};
use std::{collections::BTreeMap, io};
#[derive(Default)]
pub struct RunObservations(BTreeMap<String, Value>);
impl RunObservations {
    pub fn observe(&mut self, request: &Value, response: &Value) -> io::Result<()> {
        if request["operation"] != "get_run" || response["outcome"]["status"] != "ok" {
            return Ok(());
        }
        let view = &response["outcome"]["data"];
        if view["kind"] != "summary" && view["kind"] != "full" {
            return Ok(());
        }
        let run = &view["data"];
        let input = if view["kind"] == "summary" {
            &run["input"]
        } else {
            &run["input_ref"]
        };
        let observation = json!({"input":input,"window":run["window"],"frontier":run["computed_until_ms"],"status":run["computation_status"]});
        let id = request["params"]["run_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("Missing requested RunId"))?;
        if let Some(previous) = self.0.get(id) {
            let previous_frontier = previous["frontier"].as_u64();
            let next_frontier = observation["frontier"].as_u64();
            if previous["input"] != observation["input"]
                || previous["window"] != observation["window"]
                || previous_frontier.is_some_and(|p| next_frontier.is_none_or(|n| n < p))
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Run input/window changed or computed frontier went backwards",
                ));
            }
        }
        self.0.insert(id.to_owned(), observation);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_observations_accept_progress_and_reject_new_inputs_or_backwards_frontier() {
        let mut runs = RunObservations::default();
        let request = json!({"operation":"get_run","params":{"run_id":"run"}});
        let response = |kind: &str, input: u64, frontier: u64| json!({"outcome":{"status":"ok","data":{"kind":kind,"data":{"input":{"project_revision":input},"input_ref":{"project_revision":input},"window":{"start_ms":0,"end_ms":100},"computed_until_ms":frontier,"computation_status":"running"}}}});
        runs.observe(&request, &response("summary", 1, 10)).unwrap();
        runs.observe(&request, &response("full", 1, 20)).unwrap();
        assert!(runs.observe(&request, &response("full", 2, 30)).is_err());
        assert!(runs.observe(&request, &response("summary", 1, 19)).is_err());
    }
}
