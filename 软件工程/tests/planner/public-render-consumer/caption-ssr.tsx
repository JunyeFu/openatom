import {renderToStaticMarkup} from 'react-dom/server';
import {ProcessViewport} from '../../../apps/planner/src/features/packaging/ProcessViewport';
import type {PackageStructureJob} from '../../../packages/commands/generated/typescript/PackageStructureJob';
export const renderCaption=(job:PackageStructureJob,capture:boolean)=>renderToStaticMarkup(<ProcessViewport job={job} sample={null} capture={capture?{key:'explicit-SSR-only-caption',camera:{coordinate_frame:'right_handed_y_up',unit:'meter',eye:{x:5,y:5,z:5},target:{x:0,y:0,z:0},zoom:1},object:{kind:'artifact',id:'SSR-caption'},onFrame(){},onFailure(){}}:undefined}/>);
