import type { ApiError } from '../../../../../packages/commands/generated/typescript/ApiError';
import type { ObjectRef } from '../../../../../packages/commands/generated/typescript/ObjectRef';

export function HostFailureDetails({ detail, onLocate }: { detail: ApiError; onLocate?: (object: ObjectRef) => void }) {
  if (!detail.field_errors.length && !detail.object_refs.length) return null;
  return <details className="host-failure-details"><summary>查看具体原因与相关对象</summary>
    <ul>{detail.field_errors.map((field, index) => <li key={index}><strong>{field.pointer || '输入检查'}</strong>：{field.reason}</li>)}</ul>
    <div>{detail.object_refs.map((object, index) => <button key={index} disabled={!onLocate} onClick={() => onLocate?.(object)}>定位 {object.kind} · {object.id}</button>)}</div>
  </details>;
}
