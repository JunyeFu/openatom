import type { ItemSlotLayout as Canonical } from '../../../tools/type-contract-proof/output/typescript/ItemSlotLayout';
import type { ItemSlotLayout as Runtime } from '../../../packages/commands/generated/typescript/ItemSlotLayout';
import type { ItemType } from '../../../packages/commands/generated/typescript/ItemType';
import type { StockLocation } from '../../../packages/commands/generated/typescript/StockLocation';
import type { DeviceInstance } from '../../../packages/commands/generated/typescript/DeviceInstance';
export const runtime: (source: Canonical) => Runtime = value => value;
export const canonical: (source: Runtime) => Canonical = value => value;
export function owners(item:ItemType,location:StockLocation,device:DeviceInstance) {
 const values: (Runtime | null | undefined)[] = [item.contents_layout,location.item_layout,device.item_layout];
 return values.map(value => value?.assignment === 'item_id_order' ? value.slots.map(slot=>[slot.item_type_ref.id,slot.item_type_ref.revision,slot.anchor_role,slot.offset.unit]) : null);
}
