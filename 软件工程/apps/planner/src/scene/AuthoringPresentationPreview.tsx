import { useMemo, useRef } from 'react';
import { MediaPresentationView } from '../features/media';
import { createPresentationHost } from '../services/presentationHost';
import type { AuthoringPresentationPreviewValue } from '../services/authoringPresentation';
import { ModulePresentationPreview } from './ModulePresentationPreview';

/** Authoring uses the ordinary byte reader and renderer without an actual Run or clock. */
export function AuthoringPresentationPreview({ value }: { value: AuthoringPresentationPreviewValue }) {
  const context = value.source?.context ?? null;
  const latest = useRef(context); latest.current = context;
  const host = useMemo(() => createPresentationHost(() => latest.current), []);
  const key = JSON.stringify(context);
  const reader = useMemo(() => value.source ? host.media(value.source.context, value.source.binding) : null, [host, key]);
  if (!value.source || !reader) return <><p role="status">可预览当前部件。要读取素材，请先保存定义草稿。</p><ModulePresentationPreview presentation={value.presentation} /></>;
  const reference = value.source.binding.request.source.data.module_ref;
  return <section aria-label="定义创作 · 实际素材预览">
    <h3>定义创作 · 实际素材预览</h3>
    <p>字节来源：{reference.id} @ {reference.version} · {context!.scope.kind === 'definition_draft' ? `定义草稿 ${context!.scope.data}` : '工程'} r{context!.scope_revision}。静态检查当前表单；未保存的新素材引用须保存后读取。不产生库存、产量或生产时钟。</p>
    <MediaPresentationView {...reader} presentation={value.presentation} binding={value.source.binding} rights={value.source.rights} lod="detailed" mode="asset_inspection" inspectionTitle="创作静态声明" inspectionLabel="静态预览" />
  </section>;
}
