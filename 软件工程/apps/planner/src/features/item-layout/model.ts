import type { ProjectState } from '../../../../../packages/commands/generated/typescript/ProjectState';
import type { EditableProjectObject } from '../../../../../packages/commands/generated/typescript/EditableProjectObject';
import type { ItemSlotLayout } from '../../../../../packages/commands/generated/typescript/ItemSlotLayout';
import type { ItemDisplaySlot } from '../../../../../packages/commands/generated/typescript/ItemDisplaySlot';
import type { Pose } from '../../../../../packages/commands/generated/typescript/Pose';

export type ItemLayoutOwner = { kind: 'device' | 'location' | 'item_type'; id: string };
export type LayoutObject = Extract<EditableProjectObject, { kind: 'device' | 'location' | 'item_type' }>;
type TextVector = { x: string; y: string; z: string };
export interface SlotInput {
  item: string; anchor: string; unit: '' | Pose['unit']; coordinate_frame: '' | Pose['coordinate_frame'];
  translation: TextVector; rotation: TextVector & { w: string }; scale: TextVector;
}
export const ownerKey = (owner: ItemLayoutOwner) => JSON.stringify([owner.kind, owner.id]);
export const typeKey = (ref: { id: string; revision: string }) => JSON.stringify(ref);
export function layoutObject(project: ProjectState, owner: ItemLayoutOwner): LayoutObject | null {
  if (owner.kind === 'device') { const data = project.devices.find(v => v.id === owner.id); return data ? { kind: 'device', data } : null; }
  if (owner.kind === 'location') { const data = project.locations.find(v => v.id === owner.id); return data ? { kind: 'location', data } : null; }
  const data = project.item_types.find(v => v.id === owner.id); return data ? { kind: 'item_type', data } : null;
}
export const objectLayout = (value: LayoutObject) => value.kind === 'item_type' ? value.data.contents_layout : value.data.item_layout;
export const withLayout = (value: LayoutObject, layout: ItemSlotLayout | null): LayoutObject => value.kind === 'item_type'
  ? { ...value, data: { ...value.data, contents_layout: layout } } : value.kind === 'device'
    ? { ...value, data: { ...value.data, item_layout: layout } } : { ...value, data: { ...value.data, item_layout: layout } };
export const blankSlot = (): SlotInput => ({ item: '', anchor: '', unit: '', coordinate_frame: '', translation: { x: '', y: '', z: '' }, rotation: { x: '', y: '', z: '', w: '' }, scale: { x: '', y: '', z: '' } });
export function slotInput(slot: ItemDisplaySlot): SlotInput {
  const vector = (v: { x: number; y: number; z: number }) => ({ x: String(v.x), y: String(v.y), z: String(v.z) });
  return { item: typeKey(slot.item_type_ref), anchor: slot.anchor_role ?? '', unit: slot.offset.unit, coordinate_frame: slot.offset.coordinate_frame,
    translation: vector(slot.offset.translation), rotation: { ...vector(slot.offset.rotation), w: String(slot.offset.rotation.w) }, scale: vector(slot.offset.scale) };
}
export interface LayoutInput { declared: boolean; assignment: '' | 'item_id_order'; slots: SlotInput[] }
export const layoutInput = (layout: ItemSlotLayout | null | undefined): LayoutInput => ({ declared: layout != null, assignment: layout?.assignment ?? '', slots: layout?.slots.map(slotInput) ?? [] });
export function parseLayout(input: LayoutInput, owner: ItemLayoutOwner): ItemSlotLayout | null {
  if (!input.declared) return null;
  if (input.assignment !== 'item_id_order') throw new Error('请明确选择item_id_order时点展示分配。');
  const number = (s: string, field: string) => { if (!s.trim() || !Number.isFinite(Number(s))) throw new Error(`${field}须明确填写有限数值。`); return Number(s); };
  const slots: ItemDisplaySlot[] = input.slots.map((slot, i) => {
    if (!slot.item) throw new Error(`槽${i + 1}须选择精确子类型@revision。`);
    const item_type_ref = JSON.parse(slot.item) as ItemDisplaySlot['item_type_ref'];
    if (!slot.unit || !slot.coordinate_frame) throw new Error(`槽${i + 1}须明确选择单位与坐标系。`);
    const vector = (v: TextVector, field: string) => ({ x: number(v.x, field + ' x'), y: number(v.y, field + ' y'), z: number(v.z, field + ' z') });
    const offset: Pose = { unit: slot.unit, coordinate_frame: slot.coordinate_frame, translation: vector(slot.translation, '平移'),
      rotation: { ...vector(slot.rotation, '四元数'), w: number(slot.rotation.w, '四元数 w') }, scale: vector(slot.scale, '尺度') };
    if (Object.values(offset.rotation).every(v => v === 0) || Object.values(offset.scale).some(v => v === 0)) throw new Error(`槽${i + 1}的旋转或尺度退化，请明确修正。`);
    return { item_type_ref, anchor_role: owner.kind === 'location' ? null : slot.anchor || null, offset };
  });
  return { assignment: input.assignment, slots };
}
