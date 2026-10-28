import type { Issue } from '../../../../../packages/commands/generated/typescript/Issue';
export function IssueList({ issues }: { issues: readonly Issue[] }) {
  return <ul className="delivery-issues">{issues.map((i, n) => <li key={`${i.code}:${n}`}><code>{i.code}</code> {i.message}{i.field_path && <small> · {i.field_path}</small>}{i.object_ref && <small> · {i.object_ref.kind}:{i.object_ref.id}</small>}</li>)}</ul>;
}
export function JsonDetails({ label, value }: { label: string; value: unknown }) {
  return <details className="delivery-json"><summary>{label}</summary><pre>{JSON.stringify(value, null, 2)}</pre></details>;
}
