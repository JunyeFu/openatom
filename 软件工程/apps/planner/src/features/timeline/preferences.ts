import { useState } from 'react';
import type { Props, TimelineViewPreferencesV1 } from './types';
import { displayGroups } from './projection';

export function preferenceKey(project: string, run: string | null) { return `superfactory.timeline.view.v1:${project}:${run ? `run:${run}` : 'draft'}`; }
export function initialPreferences(): TimelineViewPreferencesV1 {
  return { version: 1, group_overrides: [], independent_device_ids: [], row_order: [], collapsed_group_ids: [], pinned_resource_ids: [], axis_windows: {}, row_scroll: 0, snap_enabled: true };
}
export function readPreferences(key: string, storage: Pick<Storage, 'getItem'>) {
  const raw = storage.getItem(key); if (!raw) return initialPreferences();
  const value = JSON.parse(raw);
  const ids = (v: unknown) => Array.isArray(v) && v.every(id => typeof id === 'string');
  if (value?.version !== 1 || !Array.isArray(value.group_overrides) || !value.group_overrides.every((g: TimelineViewPreferencesV1['group_overrides'][number]) => g && typeof g.id === 'string' && typeof g.hall_id === 'string' && typeof g.title === 'string' && ids(g.members)) || !ids(value.independent_device_ids) || !ids(value.row_order) || !ids(value.collapsed_group_ids) || !ids(value.pinned_resource_ids) || !value.axis_windows || typeof value.axis_windows !== 'object' || !Object.values(value.axis_windows).every(w => !!w && typeof w === 'object' && 'start_ms' in w && 'end_ms' in w && Number.isSafeInteger(w.start_ms) && Number.isSafeInteger(w.end_ms) && (w.end_ms as number) > (w.start_ms as number)) || !Number.isFinite(value.row_scroll) || value.row_scroll < 0 || typeof value.snap_enabled !== 'boolean') throw new Error('视图偏好不适用');
  return value as TimelineViewPreferencesV1;
}
export function useViewPreferences(props: Props, key: string) {
  const [stored] = useState(() => {
    const defaults = initialPreferences(), project = props.run ? props.project : props.draftProject ?? props.project;
    if (project) defaults.collapsed_group_ids = displayGroups(project, defaults).map(g => g.id);
    if(props.readonly)return {value:defaults,error:''};
    try { return { value: localStorage.getItem(key) ? readPreferences(key, localStorage) : defaults, error: '' }; }
    catch { return { value: defaults, error: '视图偏好未恢复' }; }
  });
  const [local, setLocal] = useState(stored.value), [storageError, setStorageError] = useState(stored.error);
  const value = props.preferences ?? local;
  function update(next: TimelineViewPreferencesV1) {
    if(props.readonly)return;
    setLocal(next); props.onPreferencesChange?.(next);
    if (!props.onPreferencesChange && !storageError) {
      try { localStorage.setItem(key, JSON.stringify(next)); } catch { setStorageError('视图偏好未保存'); }
    }
  }
  return { value, update, storageError };
}
