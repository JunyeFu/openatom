  async function editProcessNodeSource(request: NodeEditRequest) {
    const origin = deviceProcessRef.current, context = request.selection.context;
    const io = authoringIO, base = workspaceRef.current?.project;
    if (!origin || !io || !base) throw new Error('请在实际设备过程来源中重新选择节点。');
    function assertOrigin() {
      const active = deviceProcessRef.current, current = workspaceRef.current;
      const revision = active?.scope.kind === 'definition_draft' ? active.draft?.summary.revision : current?.project?.revision;
      if (!active || active.id !== origin!.id || current?.context.host.session_id !== context.session_id
        || current.project?.id !== context.project_id || revision !== context.scope_revision
        || JSON.stringify(active.scope) !== JSON.stringify(context.scope)
        || JSON.stringify(active.selection) !== JSON.stringify(context.selection)) {
        throw new Error('节点所属工程、修订或观察来源已变化，请在当前来源重新选择；不打开旧响应。');
      }
    }
    async function checkHost() {
      const actual = await io!.dispatch<ProjectContext>(authoringRequest(context.project_id, context.scope, 'get_context', { selection: [] }));
      assertOrigin();
      const revision = context.scope.kind === 'definition_draft' ? actual.definition?.revision : actual.project?.revision;
      if (actual.host.session_id !== context.session_id || actual.project?.id !== context.project_id || revision !== context.scope_revision) {
        throw new Error('实际 Host 的节点来源修订已变化，请重读后再创作。');
      }
    }
    assertOrigin();
    actionCount.current++; setBusy(true); setFailure(null); setMessage('');
    try {
      await checkHost();
      let draft: AuthoringDraft;
      if (request.target.kind === 'definition_draft' && !request.target.frozen) {
        draft = await readDraft(io, context.project_id, request.target.draft_id);
        assertOrigin();
        if (context.scope.kind !== 'definition_draft' || context.scope.data !== draft.summary.draft_id || draft.summary.revision !== context.scope_revision) {
          throw new Error('节点不属于当前定义草稿修订，不能跳到其他草稿编辑。');
        }
      } else {
        const owner = request.selection.owner;
        const object = await io.dispatch<DomainObject>(authoringRequest(context.project_id, context.scope, 'get_object', {
          object_id: owner.object_id, source: owner.source, node_index: null,
        }));
        assertOrigin();
        let definition: ModuleDefinition;
        if (request.target.kind === 'module_version' && object.kind === 'module') {
          if (object.data.definition.id !== request.target.module_ref.id || object.data.definition.version !== request.target.module_ref.version) {
            throw new Error('Host 未返回所选节点的精确模块版本。');
          }
          definition = object.data.definition;
        } else if (request.target.kind === 'device_override' && request.target.frozen && object.kind === 'device'
          && object.data.id === request.target.device_id && owner.source.kind === 'run_snapshot' && object.data.presentation_override) {
          const module = await io.dispatch<DomainObject>(authoringRequest(context.project_id, context.scope, 'get_object', {
            object_id: { kind: 'module', id: object.data.module_ref.id },
            source: { kind: 'run_module_version', data: { ...owner.source.data, module_ref: object.data.module_ref } }, node_index: null,
          }));
          assertOrigin();
          if (module.kind !== 'module' || module.data.definition.payload.category !== 'devices'
            || module.data.definition.id !== object.data.module_ref.id || module.data.definition.version !== object.data.module_ref.version) {
            throw new Error('未读到同一冻结实例的精确设备定义；不借当前定义补齐。');
          }
          definition = { ...module.data.definition, payload: { category: 'devices', data: {
            ...module.data.definition.payload.data, presentation: object.data.presentation_override,
          } } };
        } else throw new Error('此节点来源没有可用的定义创作路径；当前实例自有节点请使用所选节点编辑。');
        await checkHost();
        const commit = await io.dispatch<EditCommit>(authoringRequest(context.project_id, { kind: 'project' }, 'apply_commands', {
          base_revision: base.revision, commands: [{ kind: 'create_definition_draft', data: definition }],
        }));
        assertOrigin();
        if (commit.scope.kind !== 'definition_draft') throw new Error('Host 未返回新定义草稿 Scope。');
        draft = await readDraft(io, context.project_id, commit.scope.data.id, commit.history, commit.new_revision);
        assertOrigin();
        rememberCommit(commit);
      }
      await refreshLatest.current();
      assertOrigin();
      setDeviceProcess(null); setDeliveryTab(null); setLibraryOpen(false); setSource('draft');
      setAuthoringDraft(draft); setAuthoringPresentation(null);
      setAuthoring({ id: crypto.randomUUID(), draft });
      setMessage(request.target.frozen ? '已从所选冻结来源创建可编辑草稿；采用须明确操作，旧运行保持原输入。' : '已打开所选节点的实际定义草稿。');
    } finally { actionCount.current--; setBusy(actionCount.current > 0); }
  }