import { useEffect, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import type { DesktopCloseState } from '../generated/DesktopCloseState';

export type WindowCloseDecision = 'stay' | 'save' | 'discard';

export interface WindowCloseState {
  hasUnappliedInput: boolean;
  workInProgress: boolean;
  /** A confirmation for one Project/revision must not close a different one. */
  contextKey: string;
  confirmClose(): Promise<WindowCloseDecision>;
  save(): Promise<void>;
  onError(error: Error): void;
}

export function useWindowClose(state: WindowCloseState): void {
  const current = useRef(state);
  current.current = state;
  useEffect(() => {
    if (!('__TAURI_INTERNALS__' in window)) return;
    let disposed = false;
    let closing = false;
    let unlisten: (() => void) | undefined;
    const desktop = getCurrentWindow();
    desktop.onCloseRequested(async event => {
      event.preventDefault();
      if (closing) return;
      closing = true;
      const captured = current.current;
      const read = () => invoke<DesktopCloseState>('project_close_state');
      const identity = (context: DesktopCloseState) => JSON.stringify([context.project_id, context.project_revision, context.definition_revisions]);
      try {
        if (captured.workInProgress) throw new Error('当前操作尚未结束，请完成后再关闭。');
        let context = await read();
        if (context.execution_blockers.length) throw new Error('生产会话仍在运行、等待核对或结清设备退出。请在云端生产中停止新派工并完成结清后关闭。');
        if (context.active_tasks.length || current.current.workInProgress) throw new Error('当前仍有操作或实际任务未结束。请等待完成，或明确取消任务后再关闭。');
        if (captured.contextKey !== current.current.contextKey) throw new Error('当前编辑上下文已变化，请重新核对后关闭。');
        let discardConfirmed = false;
        if (current.current.hasUnappliedInput || context.unsaved) {
          const decision = await captured.confirmClose();
          if (decision === 'stay') return;
          const latest = await read();
          if (identity(latest) !== identity(context) || latest.active_tasks.length || latest.execution_blockers.length || current.current.contextKey !== captured.contextKey || current.current.workInProgress) {
            throw new Error('关闭确认期间工程修订或任务状态已变化。当前内容保留，请重新核对后关闭。');
          }
          if (decision === 'save') {
            if (current.current.hasUnappliedInput || !context.project_id) throw new Error('请先返回编辑，应用或丢弃本地输入，再保存工程。');
            await current.current.save();
            const saved = await read();
            if (identity(saved) !== identity(context) || saved.active_tasks.length || saved.execution_blockers.length || current.current.contextKey !== captured.contextKey || current.current.workInProgress) {
              throw new Error('保存期间工程修订或任务状态已变化。窗口保留，请重新核对后关闭。');
            }
            if (saved.unsaved || current.current.hasUnappliedInput) throw new Error('仍有未保存内容或未提交的定义草稿，请返回对应工作区处理后再关闭。');
            context = saved;
          } else {
            discardConfirmed = true;
          }
        }
        await invoke<void>('project_finish_close', { expected: context, discardConfirmed });
      }
      catch (error) { current.current.onError(error instanceof Error ? error : new Error(typeof error === 'string' ? error : JSON.stringify(error))); }
      finally { closing = false; }
    }).then(stop => {
      if (disposed) stop();
      else unlisten = stop;
    }, error => current.current.onError(error instanceof Error ? error : new Error(String(error))));
    return () => { disposed = true; unlisten?.(); };
  }, []);
}
