import type { ImportDraft } from '../../../../../packages/commands/generated/typescript/ImportDraft';
import type { ModuleRef } from '../../../../../packages/commands/generated/typescript/ModuleRef';
import { refKey } from './model';

export function importAssetChoices(draft: ImportDraft, selected: ModuleRef[]) {
  const keys = new Set(selected.map(refKey));
  return draft.proposed_data.flatMap(object => object.kind === 'module' && keys.has(refKey(object.data.definition))
    ? object.data.assets.map(asset => ({ asset, module: object.data.definition, rights: object.data.rights.find(right => right.asset_ref === asset.id) })) : []);
}
