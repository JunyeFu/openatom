import { useState } from 'react';
import type { ManufacturingFileDeclaration } from '../../services/manufacturingExport';
import type { ManufacturingExportPort, PackagingEvidence } from './types';
import { emptyManufacturingDeclarations, manufacturingDeclarationsReady } from './model';

/** Parent keys this form by actual session/Scope/R/Artifact/File identity. */
export function ManufacturingExportForm({ evidence, producer, working, onExport }: {
  evidence: PackagingEvidence; producer: ManufacturingExportPort | undefined; working: boolean;
  onExport(declarations: ManufacturingFileDeclaration[], confirmed: boolean): Promise<void>;
}) {
  const [declarations, setDeclarations] = useState(() => emptyManufacturingDeclarations(evidence));
  const [confirmed, setConfirmed] = useState(false);
  const available = producer?.status === 'available';
  const ready = manufacturingDeclarationsReady(evidence, declarations, confirmed);
  const update = (index: number, field: 'source' | 'license', value: string) => {
    setDeclarations(rows => rows.map((row, i) => i === index ? { ...row, [field]: value } : row));
    setConfirmed(false);
  };
  return <section className="packaging-manufacturing-export" aria-label="本次制造文件声明与交付">
    <h4>本次制造文件声明与交付</h4><p>成果 {evidence.artifact.id} · Job {evidence.result.job_ref.id}@r{evidence.result.job_ref.revision}。逐文件核对实际输出，填写来源和允许本次交付的许可。</p>
    {declarations.map((row, index) => <fieldset key={JSON.stringify(row.file)}><legend>{row.file.path}</legend>
      <p>Artifact {row.artifact_id} · {row.file.kind}</p>
      <label>文件来源<input aria-label={`${row.file.path} 来源`} value={row.source} disabled={working} onChange={event => update(index, 'source', event.target.value)} /></label>
      <label>本次交付许可<input aria-label={`${row.file.path} 许可`} value={row.license} disabled={working} onChange={event => update(index, 'license', event.target.value)} /></label>
    </fieldset>)}
    {!declarations.length && <p>当前成果没有实际制造文件，不能建立制造文件交付声明。</p>}
    <p>声明仅用于本次导出候选并随该包保存，不修改源工程的素材许可。重新导入只恢复声明，仍须重新确认本次携带。专业用途认可和工位绑定不能替代此确认。</p>
    <label><input type="checkbox" checked={confirmed} disabled={working || !declarations.length || !available} onChange={event => setConfirmed(event.target.checked)} />我已逐文件核对来源和许可，明确允许本次导出携带上述制造文件</label>
    {!available && <p role="status">{producer?.reason ?? '制造文件声明导出尚未接入真实宿主；原导出仍按实际缺项回执处理。'}</p>}
    <button disabled={working || !available || !ready} onClick={() => {
      const consent = confirmed;
      setConfirmed(false); // One explicit action; cancellation/failure/retry requires another confirmation.
      void onExport(structuredClone(declarations), consent);
    }}>明确声明并选择 ZIP 导出本次制造成果</button>
  </section>;
}
