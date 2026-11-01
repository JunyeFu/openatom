import type {MediaPresentationProps,MediaPresentationCommonProps} from '../../../apps/planner/src/features/media/types';
import type {ActualMediaFrame} from '../../../packages/presentation/src';
declare const common:MediaPresentationCommonProps;
declare const frame:ActualMediaFrame;
const actual:MediaPresentationProps={...common,mode:'actual_run',frame};
const preview:MediaPresentationProps={...common,mode:'declared_preview',preview:{phase_ref:'auto',view_u:.525},onPreviewSeek:value=>{value.view_u;}};
const inspection:MediaPresentationProps={...common,mode:'asset_inspection'};
// @ts-expect-error Declarations cannot carry an actual or synthetic Run frame.
const mixed:MediaPresentationProps={...common,mode:'declared_preview',preview:{phase_ref:'auto',view_u:.5},frame};
// @ts-expect-error Actual mode requires its actual frame.
const missing:MediaPresentationProps={...common,mode:'actual_run'};
// @ts-expect-error A declaration seeks unitless preview progress, not Run milliseconds.
const wrongSeek:MediaPresentationProps={...common,mode:'declared_preview',preview:{phase_ref:'auto',view_u:.5},onSeek:()=>{}};
// @ts-expect-error Explicit mode is required; null is not an actual Run.
const ambiguous:MediaPresentationProps={...common,frame:null};
void[actual,preview,inspection,mixed,missing,wrongSeek,ambiguous];
