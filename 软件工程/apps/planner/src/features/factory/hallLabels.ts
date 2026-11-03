import type { HallDesign } from '../../../../../packages/commands/generated/typescript/HallDesign';

export function hallOptionLabel(halls: readonly HallDesign[], hall: HallDesign): string {
  const title = hall.title.trim() || '未命名厂房';
  const matches = halls.filter(candidate => (candidate.title.trim() || '未命名厂房') === title);
  if (matches.length < 2) return title;
  const position = matches.findIndex(candidate => candidate.id === hall.id) + 1;
  return `${title}（同名 ${position}/${matches.length}）`;
}
