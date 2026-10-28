//! One desktop read of the existing actual Item/Job observation.
use super::*;
use sf_domain::{RunId, SimTimeMs};

#[derive(Default)]
pub(super) struct ObservationFrameCache {
    serial: u64,
    previous: Option<(String, u64, Vec<String>)>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ObservationFrame {
    Full {
        frame: u64,
        page: sf_commands::RunObjectsPage,
    },
    Delta {
        frame: u64,
        base: u64,
        run_id: RunId,
        at_ms: SimTimeMs,
        objects_len: usize,
        updates: Vec<(usize, sf_commands::DomainObject)>,
    },
}

impl ObservationFrameCache {
    fn encode(
        &mut self,
        key: String,
        base: Option<u64>,
        page: sf_commands::RunObjectsPage,
    ) -> Result<ObservationFrame, ApiError> {
        // Compare exact canonical serialization; no digest can equate distinct objects.
        let rows = page
            .objects
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| host_error(ErrorCode::ComputationFailed, &e.to_string()))?;
        self.serial += 1;
        let frame = self.serial;
        let result = match &self.previous {
            Some((previous_key, previous_frame, previous))
                if previous_key == &key && base == Some(*previous_frame) =>
            {
                let objects_len = page.objects.len();
                let updates = page
                    .objects
                    .into_iter()
                    .enumerate()
                    .filter(|(i, _)| previous.get(*i) != rows.get(*i))
                    .collect();
                ObservationFrame::Delta {
                    frame,
                    base: *previous_frame,
                    run_id: page.run_id,
                    at_ms: page.at_ms,
                    objects_len,
                    updates,
                }
            }
            _ => ObservationFrame::Full { frame, page },
        };
        // Only one actual admitted frame is retained; changing scope replaces it.
        self.previous = Some((key, frame, rows));
        Ok(result)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunObservationReadRequest {
    project_id: ProjectId,
    scope: ApiScope,
    scope_revision: Revision,
    run_id: RunId,
    at_ms: SimTimeMs,
}

impl ProjectApiState {
    fn read_run_observation_delta_ui(
        &self,
        window_label: &str,
        request: &RunObservationReadRequest,
        base: Option<u64>,
    ) -> Result<ObservationFrame, ApiError> {
        // Every call still obtains the actual Host observation through all existing admission gates.
        let page = self.read_run_observation_ui(window_label, request)?;
        let key = serde_json::to_string(&(
            &request.project_id,
            &request.scope,
            &request.scope_revision,
            &request.run_id,
        ))
        .map_err(|e| host_error(ErrorCode::ComputationFailed, &e.to_string()))?;
        self.observation_frames
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Scene observation transport state is poisoned",
                )
            })?
            .encode(key, base, page)
    }

    fn read_run_observation_ui(
        &self,
        window_label: &str,
        request: &RunObservationReadRequest,
    ) -> Result<sf_commands::RunObjectsPage, ApiError> {
        if window_label != "main" {
            return Err(host_error(
                ErrorCode::PermissionDenied,
                "Only the main desktop window may read scene observations",
            ));
        }
        self.host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Managed Project Host state is poisoned",
                )
            })?
            .read_run_observation_ui(
                &self.ui_session_id,
                &request.project_id,
                &request.scope,
                &request.scope_revision,
                &request.run_id,
                &request.at_ms,
            )
    }
}

#[tauri::command]
pub fn project_read_run_observation(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    request: RunObservationReadRequest,
) -> Result<sf_commands::RunObjectsPage, ApiError> {
    state.read_run_observation_ui(window.label(), &request)
}

#[tauri::command(async)]
pub fn project_read_run_observation_delta(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    request: RunObservationReadRequest,
    base_frame: Option<u64>,
) -> Result<ObservationFrame, ApiError> {
    state.read_run_observation_delta_ui(window.label(), &request, base_frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page(at: u64, titles: &[&str]) -> sf_commands::RunObjectsPage {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../packages/commands/tests/fixtures/comparison/actual-frozen-inputs.json"
        )))
        .unwrap();
        let objects = titles
            .iter()
            .map(|title| {
                let mut hall = fixture[0][0]["halls"][0].clone();
                hall["title"] = json!(title);
                serde_json::from_value(json!({"kind":"hall","data":hall})).unwrap()
            })
            .collect();
        sf_commands::RunObjectsPage {
            run_id: serde_json::from_value(json!("R1")).unwrap(),
            at_ms: serde_json::from_value(json!(at)).unwrap(),
            objects,
            next_index: None,
        }
    }

    #[test]
    fn exact_unchanged_objects_do_not_repeat_on_the_wire() {
        let mut cache = ObservationFrameCache::default();
        let first = cache
            .encode("P1:r1:R1".into(), None, page(0, &["A", "B"]))
            .unwrap();
        assert!(matches!(first, ObservationFrame::Full { frame: 1, .. }));
        match cache
            .encode("P1:r1:R1".into(), Some(1), page(1000, &["A", "B"]))
            .unwrap()
        {
            ObservationFrame::Delta {
                frame: 2,
                base: 1,
                at_ms,
                objects_len: 2,
                updates,
                ..
            } => {
                assert_eq!(at_ms.0.value(), 1000);
                assert!(updates.is_empty());
            }
            _ => panic!("Expected actual frame delta"),
        }
    }

    #[test]
    fn deltas_preserve_changes_order_additions_and_truncation() {
        let mut cache = ObservationFrameCache::default();
        cache
            .encode("P1:r1:R1".into(), None, page(0, &["A", "B"]))
            .unwrap();
        match cache
            .encode("P1:r1:R1".into(), Some(1), page(6000, &["B", "A", "C"]))
            .unwrap()
        {
            ObservationFrame::Delta {
                objects_len: 3,
                updates,
                ..
            } => {
                assert_eq!(
                    updates.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
                    vec![0, 1, 2]
                );
                assert_eq!(
                    serde_json::to_value(&updates[0].1).unwrap()["data"]["title"],
                    "B"
                );
            }
            _ => panic!("Expected reordered actual objects"),
        }
        match cache
            .encode("P1:r1:R1".into(), Some(2), page(0, &["B"]))
            .unwrap()
        {
            ObservationFrame::Delta {
                objects_len: 1,
                updates,
                at_ms,
                ..
            } => {
                assert!(updates.is_empty());
                assert_eq!(at_ms.0.value(), 0);
            }
            _ => panic!("Expected actual reverse seek and truncation"),
        }
    }

    #[test]
    fn unknown_or_interleaved_base_and_changed_scope_receive_complete_actual_frames() {
        let mut cache = ObservationFrameCache::default();
        cache
            .encode("P1:r1:R1".into(), None, page(0, &["A"]))
            .unwrap();
        assert!(matches!(
            cache
                .encode("P1:r1:R1".into(), Some(99), page(1, &["B"]))
                .unwrap(),
            ObservationFrame::Full { frame: 2, .. }
        ));
        assert!(matches!(
            cache
                .encode("P1:r2:R1".into(), Some(2), page(2, &["C"]))
                .unwrap(),
            ObservationFrame::Full { frame: 3, .. }
        ));
        assert!(matches!(
            cache
                .encode("P1:r2:R1".into(), Some(2), page(3, &["D"]))
                .unwrap(),
            ObservationFrame::Full { frame: 4, .. }
        ));
    }

    #[test]
    fn delta_transport_never_serves_cached_objects_before_actual_host_admission() {
        let state = ProjectApiState::new();
        state
            .observation_frames
            .lock()
            .unwrap()
            .encode("P1:r1:R1".into(), None, page(0, &["A"]))
            .unwrap();
        let request = serde_json::from_value(request()).unwrap();
        let failure = state
            .read_run_observation_delta_ui("agent", &request, Some(1))
            .unwrap_err();
        assert!(matches!(failure.code, ErrorCode::PermissionDenied));
        assert!(state
            .read_run_observation_delta_ui("main", &request, Some(1))
            .is_err());
        assert_eq!(state.observation_frames.lock().unwrap().serial, 1);
    }
    fn request() -> Value {
        json!({"project_id":"P1","scope":{"kind":"project"},"scope_revision":1,"run_id":"R1","at_ms":600000})
    }
    #[test]
    fn scene_observation_has_no_caller_or_result_override() {
        serde_json::from_value::<RunObservationReadRequest>(request()).unwrap();
        for key in ["caller", "session_id", "path", "objects", "jobs", "items"] {
            let mut value = request();
            value[key] = json!("not permitted");
            assert!(serde_json::from_value::<RunObservationReadRequest>(value).is_err());
        }
    }
    #[test]
    fn scene_observation_requires_main_before_source_lookup() {
        let state = ProjectApiState::new();
        let failure = state
            .read_run_observation_ui("agent", &serde_json::from_value(request()).unwrap())
            .unwrap_err();
        assert!(matches!(failure.code, ErrorCode::PermissionDenied));
    }
}
