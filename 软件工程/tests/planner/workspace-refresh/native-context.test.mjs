import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

// Isolated transport-unit checks over actual service functions. No Native or cloud acceptance.
const source = readFileSync(new URL('../../../apps/planner/src/services/projectHost.ts', import.meta.url), 'utf8');
const dispatch = source.slice(source.indexOf('async function dispatchResult'), source.indexOf('export const moduleKey'));
const workspace = source.slice(source.indexOf('export async function readWorkspace'), source.indexOf('\nexport function applyCommands'));
const code = stripTypeScriptTypes((dispatch + workspace).replace(/^export /gm, ''), { mode: 'transform' });
const ui = { session_id: 'actual-ui-session', host_instance_id: 'actual-native-host', project_id: null, project_revision: null, scope: null, scope_revision: null };
const noProject = { host: { session_id: ui.session_id }, project: null };
function mounted(handler) {
  const invoke = async (name, { request }) => {
    assert.equal(name, 'project_api');
    return handler(request);
  };
  return new Function('invoke', 'nativeHostAvailable', 'nfr', 'HostFailure', `${code}\nreturn readWorkspace;`)(
    invoke, () => true, { mark() {}, reply() {} }, Error);
}
const reply = (request, data, context = ui) => ({ api_version: '0.1', request_id: request.request_id, context, outcome: { status: 'ok', data } });

test('workspace retains the actual Host instance separately from the calling UI session', async () => {
  const read = mounted(request => reply(request, request.operation === 'get_context' ? noProject : {}));
  const result = await read();
  assert.equal(result.hostContext.host_instance_id, 'actual-native-host');
  assert.equal(result.hostContext.session_id, 'actual-ui-session');
  assert.equal(result.context.host.session_id, result.hostContext.session_id);
});

test('missing or mismatched response context cannot become a cloud authority identity', async () => {
  for (const mode of ['missing', 'request']) {
    const read = mounted(request => ({ ...reply(request, noProject, mode === 'missing' ? null : ui), request_id: mode === 'request' ? 'another-request' : request.request_id }));
    await assert.rejects(read(), /当前请求.*上下文/);
  }
});

test('changing Host instance or caller during a workspace read rejects the mixed result', async () => {
  for (const field of ['host_instance_id', 'session_id']) {
    let contexts = 0;
    const project = { id: 'project-a', revision: 1, module_refs: [], scenario_refs: [] };
    const context = { ...noProject, project: { id: project.id, revision: 1 } };
    const read = mounted(request => {
      if (request.operation === 'get_context') {
        contexts++;
        return reply(request, context, contexts === 1 ? ui : { ...ui, [field]: 'changed-native-identity' });
      }
      return reply(request, request.operation === 'get_object' ? { kind: 'project', data: project } : {});
    });
    await assert.rejects(read(), /Native会话已变化/);
  }
});
