// Deterministic local L03 proof. No DOM, Three.js, wall clock or animation state.
export class LineSimulation {
  constructor(config, variant = 'baseline') {
    this.config = structuredClone(config);
    this.variant = variant;
    this.now = 0;
    this.order = { ...config.order, accepted: 0 };
    this.stock = { ...config.supplies };
    this.tapeMm = Math.round(this.stock.tape_meter * 1000);
    delete this.stock.tape_meter;
    this.tapeReservedMm = 0;
    this.arrivals = structuredClone(config.arrivals);
    this.quality = structuredClone(config.quality_inputs);
    this.events = [];
    this.items = {};
    this.resources = Object.fromEntries(config.resource_instances.map(instance => [instance.role, null]));
    this.queues = Object.fromEntries(['asm', 'robot', 'inspect', 'passed', 'rework', 'reinspect', 'pack', 'seal', 'quarantine'].map(id => [id, []]));
    this.reservations = {};
    this.pallet = [];
    this.startedAssemblies = 0;
    this.devices = Object.fromEntries(config.stations.map(s => [s.id, { ...s, job: null, state: 'idle', reason: '', workMs: 0 }]));
    this.outputOpensAt = 0;
    if (variant === 'missing_pcb') this.arrivals = [];
    if (variant === 'missing_quality') this.quality['A-002'] = [null];
    if (variant === 'reject') this.quality['A-005'] = ['fail', 'fail'];
    if (variant === 'output_blocked') this.outputOpensAt = 180000;
    this.externalEvents = this.arrivals.map(a => ({ ...a, kind: 'arrival' }));
    if (this.outputOpensAt) this.externalEvents.push({ id: 'OPEN-PAL01', kind: 'open_output', at_ms: this.outputOpensAt });
    this.externalEvents.sort((a, b) => a.at_ms - b.at_ms || a.id.localeCompare(b.id));
    this.log('run_created', null, null, `订单${this.order.quantity}套；${variant}`);
    this.settle();
  }

  log(type, deviceId, itemId, detail) {
    this.events.push({ at_ms: this.now, type, device_id: deviceId, item_id: itemId, detail });
  }

  hasSpace(queue, owner = null) {
    const reservations = Object.entries(this.reservations).filter(([id, q]) => q === queue && id !== owner).length;
    return this.queues[queue].length + reservations < (queue === 'quarantine' ? this.order.quantity : this.config.queue_capacity);
  }

  take(queue, deviceId) {
    const id = this.queues[queue].shift();
    this.items[id].holder = deviceId;
    delete this.items[id].reserved_by;
    return id;
  }

  put(queue, id, deviceId) {
    this.items[id].holder = 'Q-' + queue;
    this.queues[queue].push(id);
    this.log('handoff', deviceId, id, '移交至Q-' + queue);
  }

  begin(device, queue, extra = {}) {
    const blockedRole = device.resources.find(role => this.resources[role]);
    if (blockedRole) { device.state = 'waiting_resource'; device.reason = `${blockedRole}由${this.resources[blockedRole]}占用`; return false; }
    const id = queue ? this.queues[queue][0] : null;
    for (const role of device.resources) this.resources[role] = device.id;
    const phases = device.behavior.stages;
    const durations = phases ? phases.map(s => Math.round(s.ratio * device.duration_ms)) : [device.duration_ms];
    if (device.operation === 'robot') this.items[id].reserved_by = device.id;
    else if (queue) this.take(queue, device.id);
    device.job = { itemId: id, startedAt: this.now, phase: 0, phaseStartedAt: this.now, durations,
      phaseIds: phases ? phases.map(s => s.id) : [device.operation], nextAt: this.now + durations[0],
      completedWork: 0, blocked: false, resultCreated: false, ...extra };
    device.state = 'working'; device.reason = '';
    this.log('job_started', device.id, id, device.operation);
    return true;
  }

  release(device) {
    for (const role of device.resources) this.resources[role] = null;
    device.workMs += device.job.completedWork;
    this.log('job_completed', device.id, device.job.itemId, device.operation);
    device.job = null; device.state = 'idle'; device.reason = '';
  }

  wait(device, reason, state = 'waiting_output') {
    device.job.blocked = true; device.job.nextAt = null;
    device.state = state; device.reason = reason;
  }

  finishToQueue(device, queue) {
    if (!this.hasSpace(queue)) { this.wait(device, `Q-${queue}容量满`); return false; }
    this.put(queue, device.job.itemId, device.id);
    this.release(device);
    return true;
  }

  advancePhase(device) {
    const job = device.job;
    job.phase += 1; job.phaseStartedAt = this.now;
    job.nextAt = this.now + job.durations[job.phase]; job.blocked = false;
    device.state = 'working'; device.reason = '';
    this.log('phase_started', device.id, job.itemId, job.phaseIds[job.phase]);
  }

  complete(device) {
    const job = device.job;
    if (!job.blocked) job.completedWork += job.durations[job.phase];
    switch (device.operation) {
      case 'assemble': {
        if (!job.resultCreated) {
          const id = `A-${String(job.sequence).padStart(3, '0')}`;
          this.items[id] = { id, type: 'assembly', definition_ref: this.config.item_bindings.assembly, unit: 'piece', holder: device.id, completed_reworks: 0, quality: 'unchecked',
            components: job.components, order_id: this.order.id };
          job.itemId = id; job.resultCreated = true;
          this.log('assembly_created', device.id, id, '壳体1＋PCB1＋螺钉4；原料仅领用一次');
        }
        return this.finishToQueue(device, 'asm');
      }
      case 'robot': {
        if (job.phase === 0) { this.advancePhase(device); return true; }
        if (job.phase === 1) {
          this.take('asm', device.id);
          this.log('cargo_picked', device.id, job.itemId, '同一物品从来源转到夹爪持有');
          this.advancePhase(device); return true;
        }
        if (job.phase === 2) {
          if (!this.hasSpace('robot', device.id)) { this.wait(device, '下游输送入口满，保持闭爪与货物'); return false; }
          this.reservations[device.id] = 'robot';
          this.advancePhase(device); return true;
        }
        if (job.phase === 3) {
          delete this.reservations[device.id];
          this.put('robot', job.itemId, device.id);
          this.advancePhase(device); return true;
        }
        this.release(device); return true;
      }
      case 'conveyor': return this.finishToQueue(device, device.id === 'CV01' ? 'inspect' : 'pack');
      case 'inspect': {
        const item = this.items[job.itemId];
        if (!job.resultCreated) {
          const decisions = this.quality[item.id];
          const quality = decisions ? decisions[item.completed_reworks] : this.config.default_quality;
          if (quality == null) { this.wait(device, '缺少本物品本次检验结果，保留检查人员', 'waiting_quality'); return false; }
          item.quality = quality; job.resultCreated = true;
          job.route = quality === 'pass' ? 'passed' : item.completed_reworks < this.config.max_reworks ? 'rework' : 'quarantine';
          this.log('quality_decided', device.id, item.id, `${quality}，去向${job.route}，已返工${item.completed_reworks}次`);
        }
        return this.finishToQueue(device, job.route);
      }
      case 'rework': {
        if (!job.resultCreated) {
          const item = this.items[job.itemId];
          item.completed_reworks += 1; item.quality = 'needs_reinspection'; job.resultCreated = true;
          this.log('rework_completed', device.id, item.id, '身份不变；必须重新检验');
        }
        return this.finishToQueue(device, 'reinspect');
      }
      case 'pack': {
        if (!this.hasSpace('seal')) { this.wait(device, '装箱完成待接收，保留包装人员和原物品'); return false; }
        const content = this.items[job.itemId];
        const id = 'PK-' + content.id.slice(2);
        this.items[id] = { id, type: 'package', definition_ref: this.config.item_bindings.package, unit: 'piece', holder: device.id, closure: 'open', contents: [content.id],
          carton_id: job.cartonId, order_id: this.order.id };
        content.holder = id;
        this.log('package_created', device.id, id, '新包件包含原物品' + content.id);
        job.itemId = id;
        return this.finishToQueue(device, 'seal');
      }
      case 'seal': {
        if (job.phase === 0) { this.advancePhase(device); return true; }
        if (job.phase === 1) {
          this.tapeMm -= job.tapeMm; this.tapeReservedMm -= job.tapeMm;
          this.items[job.itemId].closure = 'sealed';
          this.log('package_sealed', device.id, job.itemId, `扣胶带${job.tapeMm / 1000}m一次，包件及内容物ID不变`);
          this.advancePhase(device); return true;
        }
        if (this.now < this.outputOpensAt || this.pallet.length >= this.config.pallet.capacity) {
          this.wait(device, this.now < this.outputOpensAt ? 'PAL01接收暂停' : 'PAL01十二个槽位已满'); return false;
        }
        this.items[job.itemId].holder = 'PAL01';
        this.pallet.push(job.itemId); this.order.accepted += 1;
        this.log('order_accepted', device.id, job.itemId, `PAL01接受${this.order.accepted}/${this.order.quantity}套`);
        this.release(device); return true;
      }
    }
    throw new Error('Unsupported local proof operation: ' + device.operation);
  }

  tryStart(device) {
    device.state = 'idle'; device.reason = '';
    const operation = device.operation;
    if (operation === 'assemble') {
      if (this.startedAssemblies >= this.order.quantity) { device.reason = '订单装配任务已释放完'; return false; }
      const recipe = device.behavior.inputs.map(line => ({ item: device.ports.find(p => p.id === line.port).item, quantity: line.quantity }));
      const missing = recipe.filter(line => this.stock[line.item] < line.quantity);
      if (missing.length) { device.state = 'waiting_inputs'; device.reason = '缺料：' + missing.map(x => x.item).join('、'); return false; }
      const sequence = this.startedAssemblies + 1;
      if (!this.begin(device, null, { sequence, components: [`SHELL-${sequence}`, `PCB-${sequence}`, ...Array.from({ length: 4 }, (_, i) => `BOLT-${sequence}-${i + 1}`)] })) return false;
      for (const line of recipe) this.stock[line.item] -= line.quantity;
      this.startedAssemblies += 1;
      this.log('material_committed', device.id, null, `第${sequence}套齐套领用；原料库存已扣`);
      return true;
    }
    let queue = operation === 'robot' ? 'asm' : operation === 'inspect' ? (this.queues.reinspect.length ? 'reinspect' : 'inspect') :
      operation === 'rework' ? 'rework' : operation === 'pack' ? 'pack' : operation === 'seal' ? 'seal' : device.id === 'CV01' ? 'robot' : 'passed';
    if (!this.queues[queue].length) { device.reason = '无待处理物品'; return false; }
    if (operation === 'pack' && this.stock.carton < 1) { device.state = 'waiting_inputs'; device.reason = '缺纸箱'; return false; }
    const tapeMm = operation === 'seal' ? Math.round(device.behavior.inputs.find(i => i.port === 'tape').quantity * 1000) : 0;
    if (operation === 'seal' && this.tapeMm - this.tapeReservedMm < tapeMm) { device.state = 'waiting_inputs'; device.reason = '缺胶带'; return false; }
    const extra = operation === 'pack' ? { cartonId: `CARTON-${this.order.quantity - this.stock.carton + 1}` } : operation === 'seal' ? { tapeMm } : {};
    if (!this.begin(device, queue, extra)) return false;
    if (operation === 'pack') this.stock.carton -= 1;
    if (operation === 'seal') this.tapeReservedMm += tapeMm;
    return true;
  }

  settle() {
    let changed;
    do {
      changed = false;
      // Complete work first; arrival/output changes then unblock transfers and new jobs.
      for (const device of Object.values(this.devices)) {
        if (device.job && device.job.nextAt != null && device.job.nextAt <= this.now) {
          changed = this.complete(device) || changed;
        }
      }
      while (this.externalEvents.length && this.externalEvents[0].at_ms <= this.now) {
        const event = this.externalEvents.shift();
        if (event.kind === 'arrival') { this.stock[event.item] += event.quantity; this.log('arrival', null, null, `${event.item}到料${event.quantity}`); }
        else this.log('receiver_opened', null, null, 'PAL01恢复接收');
        changed = true;
      }
      for (const device of Object.values(this.devices)) if (device.job?.blocked) changed = this.complete(device) || changed;
      // Inspection prefers returned rework. Stable instance order resolves equal-time jobs.
      for (const device of Object.values(this.devices)) if (!device.job) changed = this.tryStart(device) || changed;
    } while (changed);
  }

  advanceTo(targetMs, stopAtOrderAcceptance = false) {
    if (targetMs < this.now) throw new Error('Cannot rewind live production state; reset a new run.');
    if (stopAtOrderAcceptance && this.order.accepted === this.order.quantity) return this.snapshot();
    while (this.now < targetMs) {
      let next = targetMs;
      if (this.externalEvents.length) next = Math.min(next, this.externalEvents[0].at_ms);
      for (const device of Object.values(this.devices)) if (device.job?.nextAt != null) next = Math.min(next, device.job.nextAt);
      this.now = next; this.settle();
      if (stopAtOrderAcceptance && this.order.accepted === this.order.quantity) break;
    }
    return this.snapshot();
  }

  supply(item, quantity) {
    if (item === 'tape_meter') this.tapeMm += Math.round(quantity * 1000);
    else this.stock[item] += quantity;
    this.log('manual_arrival', null, null, `${item}补料${quantity}`); this.settle();
  }

  setQuality(id, result) {
    const count = this.items[id].completed_reworks;
    if (!this.quality[id]) this.quality[id] = [];
    this.quality[id][count] = result;
    this.log('quality_input', 'V01', id, `补充第${count + 1}次检验结果${result}`); this.settle();
  }

  openReceiver() {
    this.outputOpensAt = this.now; this.log('receiver_opened', null, null, '人工恢复PAL01接收'); this.settle();
  }

  stationState(device) {
    const job = device.job;
    if (!job) return { id: device.id, state: device.state, waiting_reason: device.reason, phase_id: null,
      effective_progress: 0, phase_progress: 0, item_id: null, work_ms: device.workMs, operation: device.operation };
    const active = job.blocked ? 0 : Math.max(0, Math.min(this.now - job.phaseStartedAt, job.durations[job.phase]));
    return { id: device.id, state: device.state, waiting_reason: device.reason, phase_id: job.phaseIds[job.phase],
      effective_progress: Math.min(1, (job.completedWork + active) / device.duration_ms),
      phase_progress: job.blocked ? 1 : active / job.durations[job.phase],
      item_id: job.itemId, work_ms: device.workMs + job.completedWork + active, operation: device.operation,
      completed_work_ms: job.completedWork, phase_index: job.phase, started_at_ms: job.startedAt };
  }

  snapshot() {
    return { now_ms: this.now, variant: this.variant, order: { ...this.order, unmet: this.order.quantity - this.order.accepted },
      stock: { ...this.stock, tape_meter: this.tapeMm / 1000, tape_reserved_meter: this.tapeReservedMm / 1000 },
      devices: Object.fromEntries(Object.values(this.devices).map(d => [d.id, this.stationState(d)])),
      resources: { ...this.resources }, queues: structuredClone(this.queues), items: structuredClone(this.items),
      pallet: [...this.pallet], quarantined: [...this.queues.quarantine], pending_arrivals: structuredClone(this.externalEvents) };
  }

  result() {
    return { scope: 'L03_executed_local_business_simulation_not_Rust_product_acceptance', input: structuredClone(this.config), input_variant: this.variant,
      snapshot: this.snapshot(), events: structuredClone(this.events), last_acceptance_ms: this.events.filter(e => e.type === 'order_accepted').at(-1)?.at_ms ?? null };
  }
}
