  function navigate(run: () => void) {
    if (nodeDirty) { setLeaveNode({ run }); return; }
    currentNode.current = null; nodeGeneration.current++;
    setInlineEdit(null); setNodeSelection(null); run();
  }
