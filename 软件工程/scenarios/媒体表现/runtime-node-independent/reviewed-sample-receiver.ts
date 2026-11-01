  async function receiveProcessSample(value: RunResult) {
    const current = deviceProcessRef.current, draft = current?.draft;
    if (!current || !draft || value.input_ref.project_id !== workspaceRef.current?.project?.id) return;
    if (!authoringIO || value.computed_until_ms == null) throw new Error('实际样例未提供原Host读取入口或计算边界。');
    const sessionId = workspaceRef.current.context.host.session_id;
    const scope: ProcessScope['scope'] = { kind: 'definition_draft', data: draft.summary.draft_id };
    function assertSampleOrigin() {
      const active = deviceProcessRef.current, workspace = workspaceRef.current;
      if (!active || active.id !== current!.id || workspace?.context.host.session_id !== sessionId || workspace.project?.id !== value.input_ref.project_id
        || active.draft?.summary.draft_id !== draft!.summary.draft_id || active.draft.summary.revision !== draft!.summary.revision
        || JSON.stringify(active.scope) !== JSON.stringify(current!.scope) || JSON.stringify(active.selection) !== JSON.stringify(current!.selection)) {
        throw new Error(`实际样例 ${value.id} 已产生，但过程来源已切换；请在原草稿读取该Run，不打开旧响应。`);
      }
    }
    async function checkSampleDraft() {
      const actual = await authoringIO!.dispatch<ProjectContext>(authoringRequest(value.input_ref.project_id, scope, 'get_context', { selection: [] }));
      assertSampleOrigin();
      if (actual.host.session_id !== sessionId || actual.project?.id !== value.input_ref.project_id
        || actual.definition?.draft_id !== draft!.summary.draft_id || actual.definition.revision !== draft!.summary.revision) {
        throw new Error(`实际样例 ${value.id} 已产生，但Host中的原定义草稿修订已变化；请重读来源，原输入保留。`);
      }
    }
    await checkSampleDraft();
    const frozen = await authoringIO.dispatch<DomainObject>(authoringRequest(value.input_ref.project_id, scope, 'get_object', {
      object_id: { kind: 'project', id: value.input_ref.project_id },
      source: { kind: 'run_snapshot', data: { run_id: value.id, at_ms: value.computed_until_ms } }, node_index: null,
    }));
    assertSampleOrigin();
    await checkSampleDraft();
    if (frozen.kind !== 'project' || frozen.data.id !== value.input_ref.project_id || frozen.data.revision !== value.input_ref.project_revision) {
      throw new Error('Host未返回此实际样例的冻结工程，不能按当前实例猜测作业归属。');
    }
    const matches = (reference: ModuleRef) => reference.id === draft.definition.id && reference.version === draft.definition.version;
    const jobs = value.jobs.filter(item => item.execution_ref.kind === 'definition_cycle' ? matches(item.execution_ref.data.module_ref)
      : frozen.data.devices.some(device => item.execution_ref.kind === 'project_device' && device.id === item.execution_ref.data.device_id && matches(device.module_ref)));
    if (jobs.length !== 1) throw new Error(`实际样例 ${value.id} 中匹配此精确草稿的作业有${jobs.length}项；需要唯一明确映射，不任选首个作业。`);
    const job = jobs[0];
    setDeviceProcess({ ...current, scope: { kind: 'definition_draft', data: draft.summary.draft_id }, selection: { kind: 'actual_run', run_id: value.id, job_id: job.id, at_ms: value.computed_until_ms } });
  }