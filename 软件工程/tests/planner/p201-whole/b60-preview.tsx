import { createRoot } from 'react-dom/client';
import { ModulePresentationPreview } from '../../../apps/planner/src/scene/ModulePresentationPreview';
import type { Presentation } from '../../../packages/commands/generated/typescript/Presentation';
import station from './fixtures/b60/p201.b60.station.module.json';
import '../../../apps/planner/src/styles/workbench.css';
createRoot(document.getElementById('root')!).render(<main style={{ height: '100vh', padding: 12 }}><ModulePresentationPreview presentation={station.data.definition.payload.data.presentation as Presentation} title="B60固定站 · 声明几何检查（非运行结果）" /></main>);
