import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { renderFixture as render } from '../../../scenarios/Agent共建/runtime-public-render-consumer/regression/old-packaging-ssr/view-fixture.mjs';
const json = async path => JSON.parse(await readFile(new URL(`../../../${path}`, import.meta.url), 'utf8'));
const fixture = { project: (await json('packages/commands/tests/fixtures/comparison/actual-frozen-inputs.json'))[0][0],
  docs: await json('packages/commands/tests/fixtures/production/professional-documents.json'),
  input: (await json('modules/professional/paper-packaging-ui/sliding-sleeve-explicit-candidate.job.json')).data };
const renderFixture = mode => render(mode, fixture);

test('actual desktop component renders three mechanisms and blank entry plus full structural/process fields', () => {
  const html = renderFixture('project');
  for (const value of ['受限折叠容器', '外盒与多件内衬', '罩 / 滑套', '空白受限结构', '板片 / 多边形', '折带 / 轴', '固定连接', '多件 / 构件关系', '材料声明', '允许姿态', '结构变量', '尺寸与变量条件', '运动过程', '显式容差', '显式计算预算']) assert.ok(html.includes(value), value);
  assert.ok(html.includes('保存专业输入')); assert.ok(html.includes('计算已保存输入')); assert.ok(html.includes('Versioned') === false);
});
test('actual three-state result, original evidence origin, exact Job and unavailable samples are visible', () => {
  const html = renderFixture('project');
  for (const value of ['Undetermined', 'type_fixture', 'JOB-U', 'ART-U', '无实际过程样本', '当前本地输入与此成果所用输入不同']) assert.ok(html.includes(value), value);
});
test('missing exact input keeps actual file/result identity and renders no invented process geometry', () => {
  const html = renderFixture('no_exact_input'); assert.ok(html.includes('禁止用 latest 替代')); assert.ok(html.includes('ART-U')); assert.ok(html.includes('JOB-U')); assert.ok(!html.includes('真实板片与实际样本三维回看'));
});
test('actual component exposes explicit acknowledgment separately from exact station binding and persisted-reader unavailability', () => {
  const html = renderFixture('project'); for (const value of ['我的认可说明', '记录用途认可', '单独提交此工位', 'base_revision=', 'no actual persisted-result producer', '选择真实 ZIP']) assert.ok(html.includes(value), value);
  assert.ok(html.includes('没有可解析的真实 pack 行为'));
});
test('Draft capability keeps factory product/ack/binding restrictions explicit', () => {
  const html = renderFixture('draft'); assert.ok(html.includes('DefinitionDraft PROTOCOL-DRAFT')); assert.ok(html.includes('工程人工认可与绑定须在实际 Project')); assert.ok(html.includes('disabled'));
});
test('manufacturing protocol content preserves part/layers/unit and exact material quantity/area', () => {
  const html = renderFixture('manufacturing'); for (const value of ['PROTOCOL-PART', 'protocol-fixture.svg', 'cut / fold / glue', 'millimeter', 'PROTOCOL-MATERIAL', '12.5']) assert.ok(html.includes(value), value);
});

test('stored Artifact selector never seeds from the Project summary while its actual Scope list is pending', () => {
  const scoped = structuredClone(fixture);
  scoped.project.artifact_refs = ['PROJECT-ONLY-F18'];
  const html = render('available_list', scoped);
  assert.ok(!html.includes('value="PROJECT-ONLY-F18"'));
  assert.ok(html.includes('正在读取当前 Scope 的保存成果列表'));
  assert.ok(html.includes('未保存的专业输入') === false);
});

test('manufacturing declaration form lists exact output with blank rights, explicit one-export consent and separate professional acknowledgment', () => {
  const html = renderFixture('manufacturing_available');
  for (const value of ['本次制造文件声明与交付', 'protocol-fixture.svg 来源', 'protocol-fixture.svg 许可', '明确允许本次导出携带', '不修改源工程的素材许可', '专业用途认可和工位绑定不能替代此确认', '明确声明并选择 ZIP']) assert.ok(html.includes(value), value);
  assert.match(html, /aria-label="protocol-fixture.svg 来源"[^>]*value=""/);
  assert.match(html, /aria-label="protocol-fixture.svg 许可"[^>]*value=""/);
  assert.ok(!html.includes('CC0'));
  assert.ok(!html.includes('当前以含真实 SVG、结构和成果的可重开工程包交付'));
  assert.ok(html.includes('实际包含文件和缺项以 Host 回执为准'));
});
test('unwired manufacturing export stays explicitly unavailable and the original missing-file export remains present', () => {
  const html = renderFixture('manufacturing');
  assert.ok(html.includes('制造文件声明导出尚未接入真实宿主'));
  assert.ok(html.includes('未声明文件列缺项'));
  assert.match(html, /<button disabled="">明确声明并选择 ZIP 导出本次制造成果/);
});
