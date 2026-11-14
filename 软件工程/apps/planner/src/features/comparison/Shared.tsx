import type { Issue } from '../../../../../packages/commands/generated/typescript/Issue';
import { useState } from 'react';
import { issueGroups } from './issueReading';
export function IssueList({ issues }: { issues: readonly Issue[] }) {
  if (!issues.length) return null;
  return <ul className="delivery-issues">{issueGroups(issues).map(group => <li key={group.message}>{group.message}{group.issues.length > 1 && <span>（{group.issues.length}条）</span>}<JsonDetails label="查看技术来源" value={group.issues.length === 1 ? group.issues[0] : group.issues} /></li>)}</ul>;
}
export function IssueSummary({ issues, label = '适用范围与提示' }: { issues: readonly Issue[]; label?: string }) {
  const [open, setOpen] = useState(false);
  if (!issues.length) return null;
  const groups = issueGroups(issues), blocked = issues.filter(issue => issue.blocks.length).length;
  return <aside className="delivery-issue-summary" aria-label={label}><p><strong>{label} · {issues.length}条</strong>{blocked > 0 && `，其中${blocked}条涉及操作限制`}</p>
    <ul>{groups.slice(0, 2).map(group => <li key={group.message}>{group.message}{group.issues.length > 1 && `（${group.issues.length}条）`}</li>)}</ul>
    <details onToggle={event => setOpen(event.currentTarget.open)}><summary>查看全部提示与技术来源{groups.length > 2 ? `（另有${groups.length - 2}类提示）` : ''}</summary>{open && <IssueList issues={issues} />}</details>
  </aside>;
}
export function JsonDetails({ label, value }: { label: string; value: unknown }) {
  const [open, setOpen] = useState(false);
  return <details className="delivery-json" onToggle={event => setOpen(event.currentTarget.open)}><summary>{label}</summary>{open && <pre>{JSON.stringify(value, null, 2)}</pre>}</details>;
}
