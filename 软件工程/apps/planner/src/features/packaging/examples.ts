import folded from '../../../../../modules/professional/paper-packaging-ui/restricted-folded-container.job.json?raw';
import multi from '../../../../../modules/professional/paper-packaging-ui/outer-box-inner-liner.job.json?raw';
import sleeve from '../../../../../modules/professional/paper-packaging-ui/sliding-sleeve-explicit-candidate.job.json?raw';
import blank from '../../../../../modules/professional/paper-packaging-ui/blank.job.json?raw';
import type { PackageStructureJob } from '../../../../../packages/commands/generated/typescript/PackageStructureJob';
import { shapeIssues } from '../modules/model';
import { packagingSchemas } from './schemas';

/** These are original self-built declaration inputs, never computed results or material approval. */
export const packagingExamples = [
  { title: '受限折叠容器', raw: folded }, { title: '外盒与多件内衬', raw: multi },
  { title: '罩 / 滑套明确候选', raw: sleeve }, { title: '空白受限结构', raw: blank },
].map(({ title, raw }) => {
  const document: { schema_version: string; data: PackageStructureJob } = JSON.parse(raw);
  const issues = shapeIssues(packagingSchemas.PackageStructureJob, document.data);
  if (issues.length) throw new Error(`声明输入 ${title} 与 canonical 类型不符：${issues[0].path}`);
  return { title, job: document.data };
});
