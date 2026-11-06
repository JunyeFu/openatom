-- Cover raw-prefix filters and minute counts without fetching typed payloads.
-- Device precedes time so scoped arrays use index probes, not a per-row filter.
CREATE INDEX telemetry_query_cover
ON device_events(dataset_id,device_ref,cloud_received_at,ingress_id)
WHERE event_kind IN ('telemetry','generated_load');
