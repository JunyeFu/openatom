import * as THREE from './vendor/three.module.js';
import { robotPose, assemblyPose, conveyorPose } from './equipment-motion.js';

const stateNames = { idle: '空闲', working: '工作', waiting_inputs: '缺料', waiting_resource: '等人员', waiting_output: '接收阻塞', waiting_quality: '等待检验结果' };
const blend = (a, b, t) => a.map((x, i) => THREE.MathUtils.lerp(x, b[i], THREE.MathUtils.smoothstep(t, 0, 1)));

export class LineView {
  constructor({ world, camera, container, config, addDevice, makeProduct, materials, selectedId = null }) {
    this.world = world; this.camera = camera; this.container = container; this.config = config;
    this.makeProduct = makeProduct; this.materials = materials; this.selectedId = selectedId;
    this.roots = new Map(); this.labels = new Map(); this.goods = new Map(); this.ownedGeometry = [];
    this.positions = Object.fromEntries(config.stations.map(s => [s.id, [s.placement[0], 0, s.placement[1]]]));
    this.positions.PAL01 = [config.pallet.placement[0], 0, config.pallet.placement[1]];
    for (const station of config.stations) {
      if (selectedId && station.id !== selectedId) continue;
      const [x, , z] = this.positions[station.id];
      const root = addDevice(station.kind, selectedId ? 0 : x, selectedId ? 0 : z);
      root.userData.instance_id = station.id;
      root.traverse(part => { if (part.userData.motion === 'sample_geometry') part.visible = false; });
      this.roots.set(station.id, root);
      if (station.kind === 'assembly') {
        const adapter = new THREE.Mesh(new THREE.BoxGeometry(.42, .021875, .32), materials.steel);
        this.ownedGeometry.push(adapter.geometry);
        adapter.name = 'adopted-item-fixture-adapter'; adapter.position.set(0, .8829625, .07); root.add(adapter);
      }
      this.addLabel(station.id, station.title, station.kind === 'conveyor' ? 1.25 : 2.38);
    }
    if (!selectedId || selectedId === 'PAL01') {
      const root = addDevice('pallet', selectedId ? 0 : config.pallet.placement[0], 0);
      root.traverse(part => { if (part.userData.motion === 'sample_geometry') part.visible = false; });
      this.roots.set('PAL01', root); this.addLabel('PAL01', '托盘资源', 1.65);
    }
    if (!selectedId) this.addFlow();
  }

  addLabel(id, title, height) {
    const label = document.createElement('div'); label.className = 'equipment-label'; label.dataset.instance = id;
    const name = document.createElement('strong'); name.textContent = id + ' ' + title;
    const status = document.createElement('span'); label.append(name, status); this.container.append(label);
    this.labels.set(id, { label, status, height });
  }

  addFlow() {
    const routes = [
      [[-.8, .035, -.75], [14.8, .035, -.75], 0x278fb3],
      [[5.6, .035, .75], [5.6, .035, 1.75], 0xd1932b],
      [[4.3, .035, 2.6], [4.3, .035, .15], 0xd1932b]
    ];
    for (const [from, to, color] of routes) {
      const start = new THREE.Vector3(...from), end = new THREE.Vector3(...to), direction = end.clone().sub(start);
      const arrow = new THREE.ArrowHelper(direction.clone().normalize(), start, direction.length(), color, .20, .14);
      this.world.add(arrow);
      this.ownedGeometry.push(arrow.line.geometry, arrow.cone.geometry);
    }
    // Port/buffer supports, not additional production devices or inventories.
    for (const queue of ['rework', 'reinspect', 'quarantine']) {
      const [x, y, z] = this.queuePosition(queue), top = y + .0015;
      const group = new THREE.Group(); group.name = 'Q-' + queue + '-support';
      const platform = new THREE.Mesh(new THREE.BoxGeometry(queue === 'quarantine' ? 1.3 : .50, .015, .48), this.materials.steel);
      platform.position.set(x, top - .0075, z); group.add(platform); this.ownedGeometry.push(platform.geometry);
      if (queue !== 'quarantine') for (const dx of [-.20, .20]) {
        const leg = new THREE.Mesh(new THREE.BoxGeometry(.04, top - .015, .04), this.materials.dark);
        leg.position.set(x + dx, (top - .015) / 2, z); group.add(leg); this.ownedGeometry.push(leg.geometry);
      }
      this.world.add(group);
    }
  }

  center(id, y = .8924) { const p = this.positions[id]; return [p[0], y, p[2] + .07]; }

  queuePosition(queue, slot = 0) {
    const positions = { asm: [.9, .8605, .03], robot: [2.26, .8055, 0], inspect: [4.56, .8055, 0],
      passed: [6.86, .8055, 0], pack: [8.94, .8055, 0], seal: [11.66, .807, 0],
      rework: [5.6, .86, 1.3], reinspect: [4.5, .8055, .42], quarantine: [4.65 + (slot % 3) * .38, .015, -1.25 - Math.floor(slot / 3) * .32] };
    return positions[queue];
  }

  ensureGoods(id, type) {
    if (this.goods.has(id)) return this.goods.get(id);
    const root = new THREE.Group(); root.name = id; root.userData.item_id = id;
    if (type === 'assembly') this.makeProduct(root, [0, 0, 0], true);
    else {
      const [w, h, d] = this.config.item_visual_spec.package_dimensions_meter;
      for (const [dims, pos] of [[[w, .015, d], [0, .0075, 0]], [[.012, h, d], [-w / 2 + .006, h / 2, 0]],
        [[.012, h, d], [w / 2 - .006, h / 2, 0]], [[w, h, .012], [0, h / 2, -d / 2 + .006]], [[w, h, .012], [0, h / 2, d / 2 - .006]]]) {
        const wall = new THREE.Mesh(new THREE.BoxGeometry(...dims), this.materials.cardboard); wall.geometry.userData.owned_package_geometry = true; wall.position.set(...pos); wall.castShadow = true; root.add(wall);
      }
      const lid = new THREE.Mesh(new THREE.BoxGeometry(w, .008, d), this.materials.cardboard); lid.geometry.userData.owned_package_geometry = true; lid.name = 'package-lid'; lid.position.y = h; root.add(lid);
      const tape = new THREE.Mesh(new THREE.BoxGeometry(w, .003, .047), this.materials.gold); tape.geometry.userData.owned_package_geometry = true; tape.name = 'package-tape'; tape.position.y = h + .006; root.add(tape);
      const content = new THREE.Group(); content.name = 'package-content'; content.position.set(0, .0135, 0); this.makeProduct(content, [0, 0, 0], true); root.add(content);
    }
    this.world.add(root); this.goods.set(id, root); return root;
  }

  show(id, type, position, item = null) {
    const root = this.ensureGoods(id, type); root.visible = true;
    if (this.selectedId) {
      const base = this.positions[this.selectedId]; position = [position[0] - base[0], position[1], position[2] - base[2]];
    }
    root.position.set(...position);
    if (type === 'package') {
      const sealed = item?.closure === 'sealed'; root.getObjectByName('package-lid').visible = sealed;
      root.getObjectByName('package-tape').visible = sealed; root.getObjectByName('package-content').visible = !sealed;
      root.getObjectByName('package-content').userData.item_id = item?.contents?.[0] ?? null;
    }
    return root;
  }

  robotPath(s) {
    const home = [2.59, .935, -.9], pickup = [.9, .9155, .03], drop = [2.26, .8605, 0];
    let goal = home, closed = 0;
    if (s.phase_id === 'approach') goal = blend(home, pickup, s.phase_progress);
    else if (s.phase_id === 'grip') { goal = pickup; closed = s.phase_progress; }
    else if (s.phase_id === 'loaded_move') {
      const p = s.phase_progress;
      goal = p < .25 ? blend(pickup, [pickup[0], 1.65, pickup[2]], p * 4) : p < .75 ?
        blend([pickup[0], 1.65, pickup[2]], [drop[0], 1.65, drop[2]], (p - .25) * 2) : blend([drop[0], 1.65, drop[2]], drop, (p - .75) * 4);
      closed = 1;
    } else if (s.phase_id === 'release') { goal = drop; closed = 1 - s.phase_progress; }
    else if (s.phase_id === 'return') goal = blend(drop, home, s.phase_progress);
    return { goal, closed };
  }

  itemPosition(item, snapshot) {
    const holder = item.holder;
    if (holder.startsWith('Q-')) return this.queuePosition(holder.slice(2), snapshot.queues[holder.slice(2)].indexOf(item.id));
    if (holder === 'PAL01') {
      const slot = snapshot.pallet.indexOf(item.id), [x, , z] = this.positions.PAL01;
      return [x + (slot % 2 ? .25 : -.25), .115 + Math.floor(slot / 4) * .34, z + (Math.floor(slot / 2) % 2 ? .22 : -.22)];
    }
    if (!snapshot.devices[holder]) return null; // Contents are represented inside their package only.
    const s = snapshot.devices[holder];
    if (holder === 'R01') {
      const root = this.roots.get('R01');
      if (!root) return this.robotPath(s).goal.map((x, i) => x - (i === 1 ? .055 : 0));
      const point = root.getObjectByName('joint-6').localToWorld(new THREE.Vector3(0, -.29, 0));
      if (this.selectedId) point.add(new THREE.Vector3(...this.positions.R01));
      return point.toArray();
    }
    if (holder === 'CV01' || holder === 'CV02') {
      const p = this.positions[holder]; return [p[0] - 1.04 + 2.08 * s.effective_progress, .8055, p[2]];
    }
    if (holder === 'V01') {
      const p = this.positions.V01;
      return [p[0] + THREE.MathUtils.lerp(-1.04, .1, Math.min(1, s.effective_progress / .2)), .8055, p[2]];
    }
    if (holder === 'CASE01') {
      const p = this.positions.CASE01;
      const x = s.phase_id === 'infeed' ? -1.04 * (1 - s.phase_progress) : s.phase_id === 'outfeed' ? 1.04 * s.phase_progress : 0;
      return [p[0] + x, .807, p[2]];
    }
    if (holder === 'PK01') return blend(this.queuePosition('pack'), this.center('PK01'), Math.min(1, s.effective_progress / .2));
    if (holder === 'RW01') return blend(this.queuePosition('rework'), this.center('RW01'), Math.min(1, s.effective_progress / .2));
    return this.center(holder);
  }

  update(snapshot) {
    for (const object of this.goods.values()) object.visible = false;
    for (const [id, root] of this.roots) {
      if (id === 'PAL01') continue;
      const s = snapshot.devices[id];
      if (id === 'R01') {
        const { goal, closed } = this.robotPath(s), base = this.positions.R01;
        robotPose(root, [goal[0] - base[0], goal[1], goal[2] - base[2]], closed);
      } else if (s.operation === 'assemble' || s.operation === 'rework' || s.operation === 'pack') {
        assemblyPose(root, s.effective_progress, !!s.phase_id, s.operation);
      } else conveyorPose(root, s.work_ms, s.state === 'working');
    }
    for (const item of Object.values(snapshot.items)) {
      const position = this.itemPosition(item, snapshot);
      if (!position) continue;
      if (this.selectedId && item.holder !== this.selectedId && !(this.selectedId === 'PAL01' && item.holder === 'PAL01')) continue;
      this.show(item.id, item.type, position, item);
    }
    const asm = snapshot.devices.ASM01;
    if (asm.phase_id && !asm.item_id && (!this.selectedId || this.selectedId === 'ASM01')) {
      const object = this.show('WIP-ASM01', 'assembly', this.center('ASM01'));
      object.traverse(part => {
        if (['pcb-board', 'pcb-chip', 'chip-pin', 'pcb-capacitor'].includes(part.name)) {
          if (part.userData.base_y == null) part.userData.base_y = part.position.y;
          part.position.y = part.userData.base_y + .18 * (1 - Math.min(1, asm.effective_progress / .2));
        }
      });
    }
    const pack = snapshot.devices.PK01;
    if (pack.phase_id && (!this.selectedId || this.selectedId === 'PK01')) {
      const cartonPosition = pack.effective_progress < .8 ? this.center('PK01', .8939) : blend(this.center('PK01', .8939), this.queuePosition('seal'), (pack.effective_progress - .8) / .2);
      const carton = this.show('WIP-CARTON-PK01', 'package', cartonPosition, { closure: 'open' });
      carton.getObjectByName('package-content').visible = false;
      const item = this.goods.get(pack.item_id);
      if (item?.visible && pack.effective_progress > .2) {
        item.position.copy(carton.position); item.position.y += .0135 + .25 * Math.sin(Math.PI * Math.min(1, (pack.effective_progress - .2) / .6));
      }
    }
    const placed = [];
    for (const [id, entry] of this.labels) {
      const root = this.roots.get(id), s = snapshot.devices[id];
      const position = root.position.clone().add(new THREE.Vector3(0, entry.height, 0)); position.project(this.camera);
      entry.label.hidden = position.z > 1 || position.z < -1;
      entry.label.dataset.state = s?.state ?? 'idle';
      entry.status.textContent = s ? `${stateNames[s.state]} ${s.phase_id ? Math.round(s.effective_progress * 100) + '%' : ''}${s.item_id ? ' · ' + s.item_id : ''}` : `${snapshot.pallet.length}/${this.config.pallet.capacity}箱`;
      const width = entry.label.offsetWidth, height = entry.label.offsetHeight;
      const left = (position.x + 1) * this.container.clientWidth / 2;
      let top = (1 - position.y) * this.container.clientHeight / 2;
      for (const other of placed) if (Math.abs(left - other.left) < (width + other.width) / 2 + 4 && top - height < other.top + 4 && top > other.top - other.height - 4) top = other.top - other.height - 5;
      entry.label.style.left = left + 'px'; entry.label.style.top = top + 'px';
      placed.push({ left, top, width, height });
    }
  }

  dispose() {
    for (const { label } of this.labels.values()) label.remove();
    for (const root of this.roots.values()) root.traverse(part => { if (part.userData.live_geometry) part.geometry.dispose(); });
    for (const root of this.goods.values()) root.traverse(part => { if (part.isMesh && part.geometry.userData.owned_package_geometry) part.geometry.dispose(); });
    for (const geometry of this.ownedGeometry) geometry.dispose();
    for (const root of this.roots.values()) root.traverse(part => { if (part.userData.own_material) part.material.dispose(); });
  }
}
