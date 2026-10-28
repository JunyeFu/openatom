// Finite S04 L03 fixture-to-scene bindings; not a Project/Engine adapter.
// These mappings describe the current UI example explicitly. Other resources,
// adopted instance IDs and missing histories are not inferred by name or time.
const bindings = {
  Assembly01: { id: 'A01', model: 'assembly' },
  Inspection01: { id: 'I01', model: 'inspection' },
  Packing01: { id: 'P01', model: 'sealer' },
  Rework01: { id: 'R01', model: 'assembly' },
};

export function manualReplayPlacement(seconds, data, selectedUnit, devices) {
  const unknown = reason => ({ status: 'unknown', reason });
  if (!Number.isFinite(seconds) || !data?.units?.length || !selectedUnit)
    return unknown('no_selected_recorded_unit');
  const units = data.units.filter(u => u.id === selectedUnit || u.id === 'package' + selectedUnit);
  if (units.length !== 1) return unknown('selected_unit_not_in_sample');
  const unit = units[0], item = unit.id.replace(/^package/, ''), at = seconds * 1000;
  const jobs = (data.jobs || []).filter(j => j.item === item && j.start_ms <= at && at < j.end_ms);
  const located = jobs.flatMap(job => (job.resources || []).flatMap(resource => {
    const binding = bindings[resource];
    if (!binding || !devices.some(d => d.id === binding.id && d.model === binding.model)) return [];
    return [{ device_id: binding.id, source_resource_id: resource, job_kind: job.kind }];
  }));
  const ids = new Set(located.map(p => p.device_id));
  if (ids.size !== 1) return unknown(jobs.length ? 'recorded_holder_not_mapped_to_scene' : 'no_recorded_position_at_time');
  return { status: 'located', sample_unit_id: unit.id, source: 'manual_expected_job_interval', ...located[0] };
}
