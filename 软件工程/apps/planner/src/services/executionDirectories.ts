import { HostFailure, selectDirectory } from './projectHost';
import { CloudProductionError, type CloudExportDirectories } from './cloudProduction';
import type { ExecutionReportDestination } from '../../../../packages/commands/generated/typescript/ExecutionReportDestination';
import type { CloudDataExportDestination } from '../../../../packages/commands/generated/typescript/CloudDataExportDestination';

type Purpose = ExecutionReportDestination['purpose'] | CloudDataExportDestination['purpose'];
async function choose<P extends Purpose>(purpose: P) {
  try {
    const selected = await selectDirectory(purpose);
    if (selected.purpose !== purpose) throw new CloudProductionError('directory_purpose', '桌面返回的目录用途与本次导出不一致，尚未提交导出。');
    return { selection_id: selected.selection_id, purpose };
  } catch (error) {
    if (error instanceof HostFailure && error.detail.code === 'Cancelled') return null;
    throw error;
  }
}

/** Both purposes are registered by the Native binary built with this frontend. */
export function createExecutionExportDirectories(): CloudExportDirectories {
  return { report: () => choose('export_execution_report'), data: () => choose('export_cloud_data') };
}
