import * as THREE from './vendor/three.module.js';

// Continuous simulation time. Only camera and playback rate change between shots.
export const LINE_SHOTS = [
  [0, 0, 2, 'overview', 'E01 / H01 / L03 · 六套总装交付'],
  [0, 40, 10, 'assembly', 'ASM01 · 齐套领用、定位与四点紧固'],
  [40, 53, 13, 'robot', 'R01 / CV01 · 取件、夹持、搬运与输送'],
  [53, 67, 9, 'inspection', 'V01 / CV02 · 首件检验与合格分流'],
  [67, 82, 10, 'pack', 'PK01 · 同一控制盒装入新包件'],
  [82, 90, 8, 'output', 'CASE01 · 入箱、封口与出箱'],
  [90, 119, 6, 'overview', '等待 · PCB缺料与托盘暂停接收'],
  [119, 122, 3, 'assembly', '120秒 · PCB到料后恢复装配'],
  [122, 173, 8, 'overview', '并行生产 · 在制件继续向下游流转'],
  [173, 180, 5, 'output', 'CASE01 · 已封包件保持，等待接收'],
  [180, 188, 8, 'output', '180秒 · 托盘恢复接收，释放阻塞'],
  [188, 203, 10, 'rework', 'RW01 · A-003返工，保留原物品身份'],
  [203, 213, 8, 'inspection', 'V01 · A-003返工后必须复检'],
  [213, 240, 8, 'packing', '复检合格 · A-003继续输送、装箱与封箱'],
  [240, 290, 7, 'overview', '后续批次 · 八台设备继续共同运行'],
  [290, 307, 6, 'inspection', '末件 · A-006检验与输送'],
  [307, 322, 8, 'pack', 'PK01 · A-006装入PK-006'],
  [322, 330, 8, 'output', 'CASE01 / PAL01 · 最后一套封箱与接受'],
  [330, 330, 6, 'pallet', '交付完成 · 六套包件全部进入PAL01'],
].map(([from, to, seconds, view, title]) => ({ from_ms: from * 1000, to_ms: to * 1000, seconds, view, title }));

const poses = {
  overview: [[7.1, .8, .45], [8.4, 9, 16]],
  assembly: [[0, 1.05, .02], [2.5, 2.2, 3.4]],
  robot: [[1.6, 1, -.1], [3.1, 2.6, 4.8]],
  inspection: [[6.05, 1, .2], [3.5, 2.8, 5.6]],
  pack: [[10.3, 1, .07], [2.5, 2.3, 3.9]],
  output: [[13.5, .8, .05], [2.8, 2.9, 4.5]],
  rework: [[5.4, 1.05, 2.0], [2.2, 2.4, 3.6]],
  packing: [[10.4, .8, .1], [3.8, 3.2, 6.8]],
  pallet: [[14.9, .5, 0], [1.4, 1.9, 2.4]],
};
const stateNames = { idle: '空闲', working: '作业中', waiting_inputs: '等待物料', waiting_resource: '等待人员', waiting_output: '等待接收', waiting_quality: '等待检验结果' };
const phaseNames = { assemble: '定位 / 紧固', approach: '靠近取件', grip: '夹紧', loaded_move: '带件搬运', release: '放件', return: '空载回位', conveyor: '输送', inspect: '定位检验', rework: '返工', pack: '装箱', infeed: '入箱', seal: '封口', outfeed: '出箱' };
const color = state => state === 'working' ? '#64dfba' : state.startsWith('waiting') ? '#ffc766' : '#a5b5c4';
const font = (ctx, size, bold = false) => { ctx.font = `${bold ? 'bold ' : ''}${size}px "Microsoft YaHei",sans-serif`; };
function text(ctx, value, x, y, size = 20, fill = '#dbe7ef', bold = false, maxWidth = Infinity) {
  font(ctx, size, bold); ctx.fillStyle = fill;
  while (ctx.measureText(value).width > maxWidth) value = value.slice(0, -2) + '…';
  ctx.fillText(value, x, y);
}
function narrative(s) {
  const t = s.now_ms / 1000;
  if (s.order.accepted === 6) return '330秒：六套密封包件已接受；未交0套；所有人员释放，物料余量归零。';
  if (t < 40) return '一套开工领用：壳体1 / PCB 1 / 螺钉4；物品数量与设备动作来自同一仿真。';
  if (t < 80) return '同一A-001依次移交；装箱才建立包含A-001的新包件PK-001。';
  if (t < 90) return 'ASM01已用完初始2片PCB，等待到料；CASE01按实际阶段封口，扣胶带一次。';
  if (t < 120) return '等待：ASM01缺PCB；CASE01保持已封PK-001，PAL01暂停接收，胶带不重复扣。';
  if (t < 180) return '120秒PCB到料4片，装配恢复；CASE01仍保持PK-001，等待PAL01恢复接收。';
  if (t < 183) return '180秒PAL01恢复接收：PK-001进入托盘，CASE01开始处理PK-002。';
  if (t < 203) return '183秒A-003初检不合格，转入RW01返工；原A-003身份保持，不能直接包装。';
  if (t < 213) return '203秒A-003返工完成，回V01复检；同一物品累计返工1次。';
  if (t < 240) return '213秒A-003复检合格，转CV02后装箱；PK-003包含原A-003。';
  return `持续生产：PAL01已接受${s.order.accepted}/6套；剩余包件按检验、装箱、封箱路线完成。`;
}
function drawOverlay(ctx, s, shot, videoSeconds, frame, config, run, camera, view) {
  ctx.fillStyle = '#122332'; ctx.fillRect(0, 0, 1920, 94); ctx.fillRect(1550, 106, 350, 954); ctx.fillRect(20, 850, 1510, 210);
  text(ctx, shot.title, 28, 44, 30, '#ffffff', true, 1410);
  text(ctx, '真实GLB逐帧渲染  ·  8设备实例 + PAL01托盘资源  ·  同一业务运行', 30, 77, 20, '#a9bdcc');
  text(ctx, `业务 ${ (s.now_ms / 1000).toFixed(1) } / 330.0 s`, 1515, 42, 25, '#ffffff', true);
  const rate = (shot.to_ms - shot.from_ms) / (shot.seconds * 1000);
  text(ctx, `${rate ? rate.toFixed(2) + '倍回放' : '业务时间定格'}  ·  视频 ${videoSeconds.toFixed(1)} / 143 s`, 1515, 75, 17, '#a9bdcc');

  // Item labels use the same live object positions as the rendered geometry.
  const labels = [];
  for (const [id, root] of view.goods) {
    if (!root.visible || id.startsWith('WIP')) continue;
    const point = root.position.clone(); point.y += id.startsWith('PK') ? .39 : .22; point.project(camera);
    let x = 20 + (point.x + 1) * 755, y = 106 + (1 - point.y) * 362;
    if (point.z < -1 || point.z > 1 || x < 60 || x > 1490 || y < 130 || y > 740) continue;
    for (const other of labels) if (Math.abs(other.x - x) < 90 && Math.abs(other.y - y) < 30) y = other.y - 32;
    ctx.fillStyle = 'rgba(18,35,50,.86)'; ctx.fillRect(x - 44, y - 24, 88, 28);
    text(ctx, id, x - 34, y - 4, 17, '#ffffff', true); labels.push({ x, y });
  }
  ctx.fillStyle = 'rgba(18,35,50,.9)'; ctx.fillRect(40, 765, 1470, 58);
  text(ctx, narrative(s), 58, 803, 23, s.devices.CASE01.state === 'waiting_output' ? '#ffd58c' : '#ffffff', false, 1432);

  text(ctx, '八台设备 · 实时业务状态', 1570, 138, 21, '#ffffff', true);
  config.stations.forEach((station, i) => {
    const d = s.devices[station.id], y = 172 + i * 66;
    ctx.fillStyle = '#223748'; ctx.fillRect(1566, y - 21, 318, 59);
    text(ctx, station.id + '  ' + station.title, 1578, y, 17, '#ffffff', true, 294);
    text(ctx, `${stateNames[d.state]} · ${d.item_id ?? (d.phase_id ? '在制件' : '—')} · ${Math.round(d.effective_progress * 100)}%`, 1578, y + 26, 16, color(d.state), false, 294);
  });
  text(ctx, `PAL01接受  ${s.order.accepted} / 6`, 1570, 740, 30, s.order.accepted === 6 ? '#64dfba' : '#ffffff', true);
  text(ctx, `未交 ${s.order.unmet}  ·  隔离 ${s.quarantined.length}`, 1570, 771, 18);
  text(ctx, `壳体 ${s.stock.shell}  PCB ${s.stock.pcb}  螺钉 ${s.stock.fastener}`, 1570, 815, 18);
  text(ctx, `纸箱 ${s.stock.carton}  胶带 ${s.stock.tape_meter.toFixed(1)}m`, 1570, 843, 18);
  const roles = { assembler: '装配人员', inspector: '检查人员', repairer: '返工人员', packer: '包装人员' };
  Object.entries(s.resources).forEach(([role, owner], i) => text(ctx, `${roles[role]}：${owner ?? '空闲'}`, 1570, 883 + i * 25, 16, owner ? '#64dfba' : '#a5b5c4'));
  text(ctx, '托盘包件ID', 1570, 998, 17, '#ffffff', true);
  text(ctx, s.pallet.slice(0, 3).join('  ') || '—', 1570, 1022, 16);
  text(ctx, s.pallet.slice(3).join('  '), 1570, 1046, 16);

  text(ctx, '物流路线', 40, 879, 17, '#a9bdcc');
  text(ctx, 'ASM01 → R01 → CV01 → V01 → CV02 → PK01 → CASE01 → PAL01', 140, 879, 20, '#ffffff', true);
  text(ctx, '返工支路：V01 → RW01 → V01复检', 42, 909, 18, '#ffc766');
  const events = run.events.filter(e => !['phase_started', 'job_completed', 'handoff'].includes(e.type)).slice(-3);
  events.forEach((e, i) => {
    const detail = e.type === 'job_started' ? '开始' + (phaseNames[e.detail] ?? e.detail) : e.detail;
    text(ctx, `${(e.at_ms / 1000).toFixed(1)} s · ${e.device_id ?? '工程'} ${e.item_id ?? ''} · ${detail}`, 42, 940 + i * 26, 18, '#dbe7ef', false, 1442);
  });
  text(ctx, 'L03本地业务仿真  |  工时 / 质量 / 供料为声明输入  |  1080p · 30fps', 42, 1037, 16, '#a9bdcc');
  ctx.fillStyle = '#344c5e'; ctx.fillRect(0, 1070, 1920, 10); ctx.fillStyle = '#64dfba'; ctx.fillRect(0, 1070, 1920 * (frame + 1) / 4290, 10);
}

export async function recordLine({ config, run, view, camera, renderer, scene, post, onProgress, isCancelled }) {
  const capture = document.createElement('canvas'); capture.width = 1920; capture.height = 1080;
  const ctx = capture.getContext('2d'), initialSize = renderer.getSize(new THREE.Vector2()), initialAspect = camera.aspect;
  let frame = 0, encoderStarted = false;
  const seenWorking = new Set(), seenWaiting = new Set();
  try {
    const start = await post('/record-start', { headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ profile: 'line' }) });
    if (start.frames_expected !== LINE_SHOTS.reduce((n, s) => n + s.seconds * 30, 0)) throw new Error('录制时长与编码配置不一致');
    encoderStarted = true;
    renderer.setSize(1510, 724, false); camera.aspect = 1510 / 724; camera.updateProjectionMatrix();
    for (const shot of LINE_SHOTS) {
      const [target, offset] = poses[shot.view], aim = new THREE.Vector3(...target), base = new THREE.Vector3(...offset);
      for (let local = 0; local < shot.seconds * 30; local++, frame++) {
        if (isCancelled()) throw new Error('录制已取消');
        const t = local / (shot.seconds * 30);
        run.advanceTo(shot.from_ms + (shot.to_ms - shot.from_ms) * t, true);
        const snapshot = run.snapshot();
        for (const d of Object.values(snapshot.devices)) {
          if (d.state === 'working') seenWorking.add(d.id);
          if (d.state.startsWith('waiting')) seenWaiting.add(d.id + ':' + d.state);
        }
        const orbit = base.clone().applyAxisAngle(new THREE.Vector3(0, 1, 0), .10 * Math.sin((t - .5) * Math.PI));
        camera.position.copy(aim).add(orbit); camera.lookAt(aim); view.update(snapshot);
        renderer.info.reset(); renderer.render(scene, camera);
        ctx.fillStyle = '#e4e9ee'; ctx.fillRect(0, 0, 1920, 1080); ctx.drawImage(renderer.domElement, 20, 106);
        drawOverlay(ctx, snapshot, shot, frame / 30, frame, config, run, camera, view);
        const png = await new Promise(resolve => capture.toBlob(resolve, 'image/png'));
        await post('/record-frame', { headers: { 'Content-Type': 'image/png', 'X-Frame-Index': String(frame) }, body: png });
        if (frame % 30 === 0) onProgress(frame + 1, start.frames_expected, snapshot, shot.title);
      }
    }
    const business = run.result(), final = business.snapshot;
    if (seenWorking.size !== 8 || !seenWaiting.has('ASM01:waiting_inputs') || !seenWaiting.has('CASE01:waiting_output') ||
        final.pallet.length !== 6 || final.order.accepted !== 6 || final.items['A-003'].completed_reworks !== 1 || final.items['A-003'].quality !== 'pass') {
      throw new Error('本次录制未覆盖八设备、等待恢复、返工复检和六套接受');
    }
    const completion = await post('/record-finish', { headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({
      shots: LINE_SHOTS, business_result: business,
      coverage: { working_device_ids: [...seenWorking], recorded_waiting_states: [...seenWaiting], accepted_packages: final.pallet, completed_at_ms: final.now_ms },
    }) });
    encoderStarted = false;
    return completion;
  } finally {
    if (encoderStarted) await post('/record-cancel');
    renderer.setSize(initialSize.x, initialSize.y, false); camera.aspect = initialAspect; camera.updateProjectionMatrix();
  }
}
