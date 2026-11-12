import type { Issue } from '../../../../../packages/commands/generated/typescript/Issue';
import { useState } from 'react';
export function IssueList({ issues, readable = false }: { issues: readonly Issue[]; readable?: boolean }) {
  return <ul className="delivery-issues">{issues.map((i, n) => <li key={`${i.code}:${n}`}>{readable ? <>{i.message}<JsonDetails label="查看技术来源" value={i} /></> : <><code>{i.code}</code> {i.message}{i.field_path && <small> · {i.field_path}</small>}{i.object_ref && <small> · {i.object_ref.kind}:{i.object_ref.id}</small>}</>}</li>)}</ul>;
}
export function JsonDetails({ label, value }: { label: string; value: unknown }) {
  const [open, setOpen] = useState(false);
  return <details className="delivery-json" onToggle={event => setOpen(event.currentTarget.open)}><summary>{label}</summary>{open && <pre>{JSON.stringify(value, null, 2)}</pre>}</details>;
}
