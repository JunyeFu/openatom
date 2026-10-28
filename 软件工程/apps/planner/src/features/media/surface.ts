import type {AnchorRef} from '../../../../../packages/commands/generated/typescript/AnchorRef';
import type {VisualProgress,ClipPoint} from '../../../../../packages/presentation/src';
export interface Surface {
  draw:(point:VisualProgress|null,clip:ClipPoint|null)=>Promise<void>|void;
  dispose:()=>void;
  anchor:(ref:AnchorRef)=>{x:number;y:number}|null;
  view?:(kind:'iso'|'front'|'back'|'left'|'right'|'top'|'bottom'|'fit')=>void;
}
