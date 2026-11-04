import { useId } from 'react';

type Props = {
  hasProject: boolean;
  hasRecords: boolean;
  hasSelection: boolean;
  hasFrozenInput: boolean;
  busy: boolean;
  editing: boolean;
  active: boolean;
  onReplay: () => void;
};

export function RunReplayControl(props: Props) {
  const reasonId = useId();
  const reason = !props.hasProject ? '请先打开或新建工程。'
    : props.busy ? '正在处理，请完成后回看。'
    : props.editing ? '请先返回画布并处理未应用输入，再回看运行记录。'
    : !props.hasRecords ? '暂无运行记录。先配置生产方案并完成一次运行，再回看冻结输入和结果。'
    : !props.hasSelection ? '请从“历史结果”选择一条运行记录，再回看。'
    : !props.hasFrozenInput ? '尚未读到所选记录的冻结输入，请重新选择记录或刷新工程。'
    : null;
  return <>
    <button disabled={reason !== null} aria-describedby={reason ? reasonId : undefined}
      className={props.active ? 'active' : ''} onClick={props.onReplay}>运行回看</button>
    {reason && <small id={reasonId} className="run-replay-reason" role="status">{reason}</small>}
  </>;
}
