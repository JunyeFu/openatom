// Internal review selection; this does not grant asset acceptance.
export function capturePlan(entries, batch, devices = [], fullCloseups = false) {
  const available = entries.filter(entry => entry.batch === batch);
  for (const id of devices) {
    if (!available.some(entry => entry.id === id)) throw Error('Device outside current batch: ' + id);
  }
  const selected = devices.length ? available.filter(entry => devices.includes(entry.id)) : available;
  if (!selected.length) throw Error('No models selected for ' + batch);
  return {entries: selected, closeupProgress: fullCloseups ? [0, .5, 1] : [.5]};
}
