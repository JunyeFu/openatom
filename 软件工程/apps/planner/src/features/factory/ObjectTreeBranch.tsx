import type { MouseEventHandler, ReactNode } from 'react';

export function ObjectTreeBranch({ kind, title, selected, onSelect, children }: {
  kind: 'hall' | 'line'; title: string; selected: boolean;
  onSelect: MouseEventHandler<HTMLButtonElement>; children: ReactNode;
}) {
  return <div className={`tree-branch${kind === 'line' ? ' tree-line' : ''}`}>
    <button className={`tree-branch-title${selected ? ' selected' : ''}`} onClick={onSelect}>{kind === 'hall' ? '▱' : '▤'} {title}</button>
    <details open><summary aria-label={`展开或收起${kind === 'hall' ? '厂房' : '线组'} ${title}`} />{children}</details>
  </div>;
}
