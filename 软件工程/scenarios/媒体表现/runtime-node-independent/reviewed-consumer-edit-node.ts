  async function editNode(request: NodeEditRequest) {
    if (editing) throw new Error('原图片过程编辑器仍打开；请先通过其未保存确认关闭，当前输入与节点检查保留。');
    if (!currentNode.current || nodeSelectionKey(currentNode.current) !== nodeSelectionKey(request.selection)) throw new ProcessSourceChanged();
    if (request.target.kind === 'device_override' && !request.target.frozen) {
      if (!props.onApplyNode || !props.workspace.project) throw new Error('原Project节点PutObject入口尚未接通。');
      setInlineEdit({ inspection: request, base: props.workspace.project }); return;
    }
    if (!props.onEditNodeSource) throw new Error('原定义创作入口尚未接通。');
    const captured = nodeSelectionKey(request.selection), generation = nodeGeneration.current;
    const assertCurrent = () => { if (nodeGeneration.current !== generation || !currentNode.current || nodeSelectionKey(currentNode.current) !== captured) throw new ProcessSourceChanged(); };
    assertCurrent();
    await props.onEditNodeSource({ ...request, assertCurrent });
  }