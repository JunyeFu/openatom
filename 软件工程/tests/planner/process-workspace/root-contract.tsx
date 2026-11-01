import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { DeviceProcessWorkspace, type DeviceProcessWorkspaceProps } from '../../../apps/planner/src/features/factory/DeviceProcessWorkspace';
export { createImageTimelineMedia } from '../../../apps/planner/src/features/presentation-editor';

/** Static render only: effects, UI, Native, Host and asset bytes are not invoked. */
export function renderProcessContract(props: DeviceProcessWorkspaceProps) {
  return renderToStaticMarkup(createElement(DeviceProcessWorkspace, props));
}
