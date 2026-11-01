import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { RoundedBoxGeometry } from 'three/addons/geometries/RoundedBoxGeometry.js';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import { GLTFExporter } from 'three/addons/exporters/GLTFExporter.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { rigRobot, rigAssembly, rigConveyor, markSamples } from './equipment-motion.js';
import { LineSimulation } from './line-simulation.mjs';
import { LineView } from './line-view.js';
import { recordLine } from './line-recording.js';

const started = performance.now();
const $ = id => document.getElementById(id);
const scene = new THREE.Scene();
scene.background = new THREE.Color('#e4e9ee');
const camera = new THREE.PerspectiveCamera(36, 16 / 9, 0.02, 350);
const renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'high-performance' });
renderer.setSize(1920, 1080, false);
renderer.setPixelRatio(1);
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 0.82;
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
renderer.info.autoReset = false;
$('canvas-wrap').appendChild(renderer.domElement);
const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;
controls.dampingFactor = 0.09;
controls.minDistance = 0.15;
controls.maxDistance = 180;
controls.maxPolarAngle = Math.PI;
const pmrem = new THREE.PMREMGenerator(renderer);
const environment = pmrem.fromScene(new RoomEnvironment(), 0.04);
scene.environment = environment.texture;
scene.environmentIntensity = 0.75;
const ambient = new THREE.HemisphereLight(0xf1f7ff, 0x7c8690, 1.15);
scene.add(ambient);
const sun = new THREE.DirectionalLight(0xffffff, 2.2);
sun.position.set(8, 16, 5);
sun.castShadow = true;
sun.shadow.mapSize.set(2048, 2048);
sun.shadow.normalBias = 0.005;
sun.shadow.bias = -0.00002;
scene.add(sun, sun.target);
const fill = new THREE.DirectionalLight(0xc5dcff, 0.8);
fill.position.set(-7, 5, -9);
scene.add(fill);

const mat = (color, metalness = 0.0, roughness = 0.5, more = {}) => new THREE.MeshStandardMaterial({ color, metalness, roughness, ...more });
const M = {
  aluminum: mat('#bcc3ca', 0.83, 0.30), steel: mat('#8e969f', 0.92, 0.25), dark: mat('#202a35', 0.55, 0.35),
  rubber: mat('#242a2f', 0.05, 0.8), white: mat('#d8dee4', 0.55, 0.33), blue: mat('#376e96', 0.55, 0.32),
  pcb: mat('#078458', 0.15, 0.46), gold: mat('#d7ba62', 0.74, 0.3), green: mat('#398165', 0.05, 0.7),
  red: mat('#c24a41', 0.13, 0.35), amber: mat('#dfae45', 0.15, 0.35), light: mat('#45b679', 0.12, 0.3, { emissive: '#126430', emissiveIntensity: 0.5 }),
  cardboard: mat('#b98a55', 0.0, 0.85), wood: mat('#ae895e', 0.0, 0.88), screen: mat('#182c40', 0.25, 0.2),
  floor: mat('#cbd2da', 0.06, 0.82), silver: mat('#d8dde0', 0.93, 0.23), glass: mat('#384e61', 0.25, 0.24), solder: mat('#d4d9d5', 0.8, 0.3),
};
// One shared 2K PBR set, produced locally; no photographed asset is embedded.
function brushedMetalMaps(size = 2048) {
  const canvases = Array.from({ length: 3 }, () => { const canvas = document.createElement('canvas'); canvas.width = size; canvas.height = size; return canvas; });
  const images = canvases.map(canvas => canvas.getContext('2d').createImageData(size, size));
  let seed = 271828;
  for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    const grain = (seed >>> 27) - 16, streak = Math.sin(y * 0.47) * 4;
    const offset = (y * size + x) * 4;
    const base = Math.round(224 + grain * 0.35 + streak);
    const rough = Math.round(184 + grain * 0.7 + streak);
    images[0].data.set([base, base + 1, base + 3, 255], offset);
    images[1].data.set([255, rough, 255, 255], offset); // Shared glTF ORM: roughness G, metalness B.
    images[2].data.set([128, Math.round(128 + grain * 0.22 + streak * 0.35), 254, 255], offset);
  }
  const maps = canvases.map((canvas, i) => {
    canvas.getContext('2d').putImageData(images[i], 0, 0);
    const texture = new THREE.CanvasTexture(canvas); texture.name = ['shared-brushed-basecolor', 'shared-brushed-roughness', 'shared-brushed-normal'][i];
    texture.anisotropy = Math.min(8, renderer.capabilities.getMaxAnisotropy());
    if (i === 0) texture.colorSpace = THREE.SRGBColorSpace;
    return texture;
  });
  return maps;
}
const metalMaps = brushedMetalMaps();
for (const material of [M.aluminum, M.steel]) {
  material.map = metalMaps[0]; material.roughnessMap = metalMaps[1]; material.metalnessMap = metalMaps[1]; material.normalMap = metalMaps[2];
  material.normalScale.set(0.18, 0.18);
}
const geometryCache = new Map();
const names = { assembly: '定位与紧固装配台', robot: '六轴搬运机器人', conveyor: '皮带输送机', inspection: '相机检验工位', sealer: '封箱工位', pallet: '成品托盘' };

function mesh(parent, geometry, material, pos, name, rot = null) {
  const object = new THREE.Mesh(geometry, material);
  object.name = name;
  object.position.set(...pos);
  if (rot) object.rotation.set(...rot);
  object.castShadow = true;
  object.receiveShadow = true;
  parent.add(object);
  return object;
}
function box(parent, dims, pos, material, name, bevel = 0.006) {
  const radius = Math.min(bevel, ...dims.map(x => x / 4));
  const key = ['box', ...dims, radius].join(':');
  if (!geometryCache.has(key)) geometryCache.set(key, radius ? new RoundedBoxGeometry(...dims, 2, radius) : new THREE.BoxGeometry(...dims));
  return mesh(parent, geometryCache.get(key), material, pos, name);
}
function cylinder(parent, radius, height, pos, material, name, axis = 'y', segments = 24) {
  const key = ['cyl', radius, height, segments].join(':');
  if (!geometryCache.has(key)) geometryCache.set(key, new THREE.CylinderGeometry(radius, radius, height, segments));
  const rotation = axis === 'x' ? [0, 0, Math.PI / 2] : axis === 'z' ? [Math.PI / 2, 0, 0] : null;
  return mesh(parent, geometryCache.get(key), material, pos, name, rotation);
}
function bolt(parent, pos, axis = 'y', size = 0.012, normalSign = 1) {
  cylinder(parent, size, 0.009, pos, M.steel, 'bolt-head', axis, 12);
  const p = [...pos]; p[{ x: 0, y: 1, z: 2 }[axis]] += normalSign * 0.005;
  cylinder(parent, size * 0.44, 0.002, p, M.dark, 'hex-socket', axis, 6);
}
function extrusion(parent, dims, pos, name) {
  box(parent, dims, pos, M.aluminum, name, 0.003);
  const long = dims.indexOf(Math.max(...dims));
  for (const side of [-1, 1]) {
    const p = [...pos], d = [...dims];
    const normal = long === 2 ? 0 : 2;
    p[normal] += side * (dims[normal] / 2 + 0.0004);
    const transverse = 3 - long - normal;
    d[normal] = 0.001; d[transverse] = Math.min(d[transverse], 0.009);
    box(parent, d, p, M.dark, name + '-groove', 0);
  }
}
function cable(parent, points, radius, name, segments = 24) {
  const curve = new THREE.CatmullRomCurve3(points.map(p => new THREE.Vector3(...p)));
  mesh(parent, new THREE.TubeGeometry(curve, segments, radius, 8, false), M.rubber, [0, 0, 0], name);
}
function frame(parent, width, depth, height, detailed = true) {
  for (const x of [-width / 2 + 0.05, width / 2 - 0.05]) for (const z of [-depth / 2 + 0.05, depth / 2 - 0.05]) {
    extrusion(parent, [0.07, height - 0.065, 0.07], [x, (height + 0.065) / 2, z], 'aluminum-leg');
    cylinder(parent, 0.055, 0.035, [x, 0.0175, z], M.rubber, 'leveling-foot');
    cylinder(parent, 0.014, 0.035, [x, 0.0525, z], M.steel, 'leveling-stem');
    if (detailed) {
      box(parent, [0.1, 0.075, 0.013], [x, height - 0.1, z + 0.043], M.steel, 'corner-bracket');
      bolt(parent, [x, height - 0.09, z + 0.055], 'z');
    }
  }
  for (const z of [-depth / 2 + 0.05, depth / 2 - 0.05]) for (const y of [0.2, height - 0.04]) extrusion(parent, [width - 0.06, 0.06, 0.06], [0, y, z], 'cross-beam');
  for (const x of [-width / 2 + 0.05, width / 2 - 0.05]) extrusion(parent, [0.06, 0.06, depth - 0.06], [x, height - 0.04, 0], 'side-beam');
}
function controlPanel(parent, pos, detailed = true, mountingHeight = 0.825) {
  const stemHeight = pos[1] - mountingHeight;
  box(parent, [0.10, 0.025, 0.23], [pos[0], mountingHeight + 0.0125, pos[2] - 0.085], M.steel, 'panel-mounting-foot');
  box(parent, [0.028, stemHeight, 0.028], [pos[0], mountingHeight + stemHeight / 2, pos[2] - 0.09], M.aluminum, 'panel-support-post');
  box(parent, [0.14, 0.07, 0.065], [pos[0], pos[1], pos[2] - 0.062], M.steel, 'panel-rear-bracket');
  const panel = new THREE.Group(); panel.position.set(...pos); panel.name = 'operator-panel'; parent.add(panel);
  box(panel, [0.22, 0.30, 0.10], [0, 0, 0], M.white, 'panel-housing', 0.012);
  box(panel, [0.15, 0.15, 0.004], [0, 0.045, 0.052], M.screen, 'lcd-panel');
  if (detailed) {
    for (let i = 0; i < 4; i++) box(panel, [0.09 - i * 0.013, 0.007, 0.003], [-0.008, 0.09 - i * 0.03, 0.0555], M.light, 'lcd-readout');
    cylinder(panel, 0.017, 0.014, [-0.055, -0.08, 0.057], M.red, 'emergency-stop', 'z');
    cylinder(panel, 0.014, 0.01, [0.012, -0.08, 0.055], M.light, 'start-button', 'z');
    cylinder(panel, 0.011, 0.01, [0.065, -0.08, 0.055], M.dark, 'selector', 'z');
  }
}
function signalTower(parent, pos) {
  cylinder(parent, 0.016, 0.18, pos, M.aluminum, 'tower-support');
  for (const [i, material] of [M.light, M.amber, M.red].entries()) cylinder(parent, 0.031, 0.05, [pos[0], pos[1] + 0.12 + i * 0.055, pos[2]], material, 'signal-segment');
  cylinder(parent, 0.034, 0.016, [pos[0], pos[1] + 0.25, pos[2]], M.dark, 'tower-cap');
}
function product(parent, pos, detail = true) {
  const p = new THREE.Group(); p.name = 'control-box-product'; p.position.set(...pos); parent.add(p);
  box(p, [0.34, 0.025, 0.25], [0, 0.014, 0], M.aluminum, 'housing-base', 0.007);
  for (const x of [-0.163, 0.163]) box(p, [0.018, 0.08, 0.25], [x, 0.06, 0], M.aluminum, 'housing-wall-x');
  for (const z of [-0.117, 0.117]) box(p, [0.34, 0.08, 0.017], [0, 0.06, z], M.aluminum, 'housing-wall-z');
  box(p, [0.28, 0.008, 0.19], [0, 0.047, 0], M.pcb, 'pcb-board', 0.002);
  if (detail) {
    for (let i = 0; i < 6; i++) {
      const x = -0.10 + (i % 3) * 0.095, z = -0.055 + Math.floor(i / 3) * 0.08;
      box(p, [0.035, 0.016, 0.034], [x, 0.06, z], M.dark, 'pcb-chip');
      for (let j = 0; j < 5; j++) for (const s of [-1, 1]) box(p, [0.004, 0.004, 0.009], [x - 0.012 + j * 0.006, 0.055, z + s * 0.022], M.solder, 'chip-pin', 0);
    }
    for (let i = 0; i < 8; i++) cylinder(p, 0.006, 0.016, [-0.12 + i * 0.034, 0.064, 0.068], M.gold, 'pcb-capacitor', 'y', 12);
    for (const x of [-0.12, 0.12]) for (const z of [-0.08, 0.08]) {
      cylinder(p, 0.008, 0.0165, [x, 0.03475, z], M.steel, 'pcb-standoff');
      cylinder(p, 0.010, 0.0025, [x, 0.05225, z], M.steel, 'pcb-washer');
      bolt(p, [x, 0.058, z], 'y', 0.008);
    }
    for (const x of [-0.09, 0.01]) {
      cylinder(p, 0.019, 0.044, [x, 0.054, 0.145], M.steel, 'cable-gland', 'z', 12);
      cylinder(p, 0.012, 0.05, [x, 0.054, 0.149], M.dark, 'gland-opening', 'z', 12);
    }
  }
  return p;
}
function assembly(high) {
  const root = new THREE.Group(); root.name = 'ASM01-positioning-fastening';
  frame(root, 1.65, 1.10, 0.78, high);
  box(root, [1.72, 0.045, 1.16], [0, 0.8025, 0], M.aluminum, 'machined-worktop', 0.014);
  box(root, [0.66, 0.007, 0.54], [0, 0.8285, 0.07], M.dark, 'fixture-mounting-pad', 0);
  box(root, [0.66, 0.028, 0.54], [0, 0.846, 0.07], M.steel, 'fixture-baseplate');
  product(root, [0, 0.87, 0.07], high).scale.setScalar(1.35);
  for (const x of [-0.1755, 0.1755]) for (const z of [-0.038, 0.178]) cylinder(root, 0.014, 0.012025, [x, 0.8660125, z], M.steel, 'product-mounting-pad');
  for (const x of [-0.30, 0.30]) for (const z of [-0.18, 0.32]) {
    box(root, [0.055, 0.12, 0.055], [x, 0.92, z], M.steel, 'locator-post');
    cylinder(root, 0.012, 0.055, [x, 0.985, z], M.silver, 'locator-pin');
    if (high) bolt(root, [x, 0.858, z + 0.055]);
  }
  for (const x of [-0.50, 0.50]) extrusion(root, [0.09, 1.04, 0.09], [x, 1.34, -0.40], 'gantry-upright');
  extrusion(root, [1.10, 0.09, 0.10], [0, 1.88, -0.40], 'gantry-crossbar');
  box(root, [0.35, 0.62, 0.055], [0, 1.54, -0.39], M.aluminum, 'z-axis-backplate');
  const driverX = 0.162, driverZ = 0.178; // The existing front-right PCB screw, clear of chips.
  for (const x of [driverX - 0.09, driverX + 0.09]) {
    cylinder(root, 0.012, 0.58, [x, 1.55, -0.34], M.silver, 'linear-guide');
    for (const y of [1.29, 1.81]) box(root, [0.035, 0.03, 0.055], [x, y, -0.3435], M.steel, 'linear-guide-support');
  }
  root.getObjectByName('z-axis-backplate').position.x = driverX;
  const moving = new THREE.Group(); moving.name = 'z-slide-and-driver'; moving.userData.motion = 'phase_z'; root.add(moving);
  moving.userData.travel = 0.083; // Bit stops at y=.957, above the target screw at .9564.
  for (const x of [driverX - 0.09, driverX + 0.09]) box(moving, [0.047, 0.11, 0.055], [x, 1.55, -0.32], M.steel, 'linear-bearing-block');
  box(moving, [0.24, 0.24, 0.055], [driverX, 1.53, -0.265], M.white, 'moving-carriage');
  box(moving, [0.11, 0.04, 0.46], [driverX, 1.53, -0.052], M.aluminum, 'driver-support-bracket');
  cylinder(moving, 0.034, 0.30, [driverX, 1.38, driverZ], M.dark, 'electric-driver');
  cylinder(moving, 0.036, 0.025, [driverX, 1.47, driverZ], M.light, 'driver-status-ring');
  cylinder(moving, 0.020, 0.10, [driverX, 1.19, driverZ], M.steel, 'driver-chuck');
  cylinder(moving, 0.004, 0.10, [driverX, 1.09, driverZ], M.silver, 'screwdriver-bit', 'y', 12);
  controlPanel(root, [0.70, 1.02, 0.37], high);
  box(root, [0.23, 0.03, 0.12], [0.605, 1.88, -0.40], M.steel, 'signal-tower-bracket');
  signalTower(root, [0.68, 1.985, -0.4]);
  box(root, [0.52, 0.45, 0.21], [0, 0.47, -0.36], M.white, 'electrical-enclosure');
  box(root, [0.29, 0.32, 0.006], [0.04, 0.47, -0.252], M.aluminum, 'cabinet-door');
  if (high) {
    for (const x of [-1.07, 1.07]) {
      box(root, [0.45, 0.10, 0.40], [x, 0.79, 0.03], M.aluminum, 'short-transfer-frame');
      box(root, [0.45, 0.022, 0.29], [x, 0.851, 0.03], M.green, 'short-transfer-belt');
      for (const z of [-0.16, 0.22]) extrusion(root, [0.45, 0.06, 0.025], [x, 0.87, z], 'transfer-guide');
      for (const xOffset of [-0.15, 0.15]) bolt(root, [x+xOffset, 0.902, -0.16], 'y', 0.008);
      box(root, [0.23, 0.12, 0.28], [Math.sign(x) * 0.85, 0.745, 0.03], M.steel, 'transfer-mounting-bracket');
      for (const z of [-0.10, 0.16]) {
        extrusion(root, [0.045, 0.705, 0.045], [Math.sign(x) * 1.16, 0.3875, z], 'transfer-leg');
        cylinder(root, 0.035, 0.035, [Math.sign(x) * 1.16, 0.0175, z], M.rubber, 'transfer-foot');
      }
    }
    for (const x of [-0.70, -0.35, 0.35, 0.70]) for (const z of [-0.44, 0.44]) bolt(root, [x, 0.831, z]);
    for (const x of [driverX - 0.09, driverX + 0.09]) for (const y of [1.3, 1.48, 1.67, 1.81]) bolt(root, [x, y, -0.328], 'z', 0.008);
    const chainPath = new THREE.CatmullRomCurve3([[0.31, 1.79, -0.32], [0.46, 1.88, -0.32], [0.59, 1.77, -0.32], [0.54, 1.55, -0.32], [0.34, 1.52, -0.32]].map(p => new THREE.Vector3(...p)));
    mesh(root, new THREE.TubeGeometry(chainPath, 40, 0.009, 8, false), M.rubber, [0, 0, 0], 'drag-chain-cable-core');
    const chainCount = Math.ceil(chainPath.getLength() / 0.031);
    for (let i = 0; i <= chainCount; i++) {
      const t = i / chainCount;
      const chainLink = box(root, [0.034, 0.029, 0.048], chainPath.getPointAt(t).toArray(), M.dark, 'drag-chain-link');
      chainLink.quaternion.setFromUnitVectors(new THREE.Vector3(1, 0, 0), chainPath.getTangentAt(t));
    }
    box(root, [0.06, 0.07, 0.075], [0.31, 1.79, -0.355], M.steel, 'drag-chain-fixed-anchor');
    box(root, [0.065, 0.06, 0.075], [0.32, 1.52, -0.355], M.steel, 'drag-chain-return-anchor');
    cable(root, [[0.34, 1.52, -0.32], [0.35, 1.42, -0.22], [0.25, 1.55, -0.24]], 0.009, 'slide-flexible-lead');
    cable(root, [[0.0, 1.7, -0.45], [-0.2, 1.92, -0.49], [-0.52, 1.83, -0.5], [-0.55, 1.03, -0.5], [-0.1, 0.62, -0.46]], 0.011, 'power-and-signal-cable');
    for (const x of [-0.73, 0.73]) {
      box(root, [0.04, 0.06, 0.024], [x, 0.88, 0.04], M.dark, 'photoelectric-sensor');
      cylinder(root, 0.006, 0.005, [x, 0.89, 0.055], M.red, 'sensor-lens', 'z', 12);
    }
    box(root, [0.29, 0.012, 0.24], [0.5, 0.849, 0.08], M.aluminum, 'housing-cover');
    for (const x of [0.39, 0.61]) for (const z of [-0.008, 0.168]) bolt(root, [x, 0.86, z], 'y', 0.008);
  }
  return rigAssembly(root);
}
function link(parent, a, b, width, depth, name) {
  const start = new THREE.Vector3(...a), end = new THREE.Vector3(...b), delta = end.clone().sub(start);
  const object = box(parent, [width, delta.length(), depth], start.clone().add(end).multiplyScalar(0.5).toArray(), M.white, name, 0.045);
  object.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), delta.normalize());
  return object;
}
function robot(high) {
  const root = new THREE.Group(); root.name = 'R01-six-axis-handling';
  box(root, [0.80, 0.09, 0.72], [0, 0.045, 0], M.dark, 'robot-baseplate', 0.025);
  cylinder(root, 0.24, 0.18, [0, 0.18, 0], M.steel, 'axis1-base', 'y', high ? 40 : 16);
  const a = [0, 0.65, 0], b = [-0.28, 1.40, 0], c = [0.52, 1.58, 0], d = [0.89, 1.33, 0];
  // One continuous shoulder housing replaces the intersecting base/shoulder cylinders.
  const shoulder = new THREE.Shape();
  shoulder.moveTo(-0.19, 0.27); shoulder.lineTo(0.19, 0.27); shoulder.lineTo(0.17, a[1]);
  shoulder.absarc(0, a[1], 0.17, 0, Math.PI, false); shoulder.lineTo(-0.19, 0.27); shoulder.closePath();
  const shoulderGeometry = new THREE.ExtrudeGeometry(shoulder, { depth: 0.26, bevelEnabled: true, bevelThickness: 0.01, bevelSize: 0.01, bevelSegments: 3, steps: 1, curveSegments: high ? 24 : 12 });
  shoulderGeometry.translate(0, 0, -0.13);
  mesh(root, shoulderGeometry, M.white, [0, 0, 0], 'axis1-shoulder-housing');
  const armStart = new THREE.Vector3(...a).lerp(new THREE.Vector3(...b), 0.14);
  const rollDirection = new THREE.Vector3(...c).sub(new THREE.Vector3(...b)).normalize();
  const forearmEnd = new THREE.Vector3(...c).addScaledVector(rollDirection, -0.08);
  link(root, armStart.toArray(), b, 0.20, 0.22, 'upper-arm'); link(root, b, forearmEnd.toArray(), 0.17, 0.19, 'forearm'); link(root, c, d, 0.13, 0.14, 'wrist-link');
  for (const [i, pos] of [a, b, d].entries()) {
    const radius = [0.17, 0.15, 0.10][i], thickness = [0.28, 0.245, 0.175][i], axis = [2, 3, 5][i];
    if (axis !== 2) cylinder(root, radius, thickness, pos, M.white, 'axis' + axis + '-housing', 'z', high ? 40 : 16);
    for (const sign of [-1, 1]) {
      const face = thickness / 2;
      cylinder(root, radius * 0.88, 0.006, [pos[0], pos[1], sign * (face + 0.003)], M.dark, 'axis' + axis + '-seal', 'z', high ? 40 : 16);
      cylinder(root, radius * 0.78, 0.018, [pos[0], pos[1], sign * (face + 0.015)], M.aluminum, 'axis' + axis + '-cap', 'z', high ? 32 : 16);
      if (high) for (let k = 0; k < 6; k++) {
        const theta = k * Math.PI / 3, boltRadius = radius * 0.60;
        bolt(root, [pos[0] + boltRadius * Math.cos(theta), pos[1] + boltRadius * Math.sin(theta), sign * (face + 0.0285)], 'z', 0.007, sign);
      }
    }
  }
  const roll = cylinder(root, 0.135, 0.19, c, M.white, 'axis4-roll-housing', 'y', high ? 40 : 16);
  roll.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), rollDirection);
  const rollSeal = cylinder(root, 0.138, 0.012, new THREE.Vector3(...c).addScaledVector(rollDirection, 0.055).toArray(), M.dark, 'axis4-roll-seal');
  rollSeal.quaternion.copy(roll.quaternion);
  cylinder(root, 0.064, 0.12, [d[0], 1.17, 0], M.dark, 'axis6-tool-mount');
  cylinder(root, 0.074, 0.020, [d[0], 1.115, 0], M.aluminum, 'tool-flange');
  box(root, [0.48, 0.08, 0.13], [d[0], 1.07, 0], M.steel, 'parallel-gripper');
  for (const sign of [-1, 1]) {
    const jaw = new THREE.Group(); jaw.name = sign < 0 ? 'gripper-left-slide' : 'gripper-right-slide';
    jaw.userData.motion = 'gripper'; jaw.userData.side = sign; root.add(jaw);
    box(jaw, [0.022, 0.12, 0.07], [d[0] + sign * 0.21, 0.97, 0], M.aluminum, 'gripper-jaw');
    box(jaw, [0.005, 0.045, 0.055], [d[0] + sign * 0.1965, 0.935, 0], M.rubber, 'jaw-contact-pad', 0.002);
  }
  if (high) {
    cable(root, [[0, 0.47, -0.14], [-0.10, 0.76, -0.17], [-0.24, 1.25, -0.17], [-0.34, 1.45, -0.18], [-0.20, 1.59, -0.16], [0.30, 1.60, -0.17], [0.57, 1.60, -0.17], [0.72, 1.47, -0.14], [0.98, 1.29, -0.15], [0.89, 1.09, -0.065]], 0.018, 'robot-dress-pack', 64);
    for (const [point, tangent, rearFace] of [
      [[-0.24, 1.25, -0.17], [-0.24, 0.69, -0.01], -0.11],
      [[0.30, 1.60, -0.17], [0.77, 0.01, -0.01], -0.095],
      [[0.72, 1.47, -0.14], [0.41, -0.31, 0.02], -0.07],
    ]) {
      const depth = rearFace - point[2] + 0.01;
      box(root, [0.048, 0.03, depth], [point[0], point[1], point[2] + depth / 2], M.dark, 'dress-pack-mount');
      const strap = mesh(root, new THREE.TorusGeometry(0.021, 0.003, 8, 24), M.steel, point, 'dress-pack-clamp');
      strap.quaternion.setFromUnitVectors(new THREE.Vector3(0, 0, 1), new THREE.Vector3(...tangent).normalize());
    }
    for (const x of [-0.3, 0.3]) for (const z of [-0.27, 0.27]) bolt(root, [x, 0.0945, z], 'y', 0.017);
  }
  return rigRobot(root);
}
function conveyor(high) {
  const root = new THREE.Group(); root.name = 'CV01-belt-conveyor';
  frame(root, 2.10, 0.60, 0.72, high);
  for (const z of [-0.31, 0.31]) extrusion(root, [2.25, 0.13, 0.045], [0, 0.76, z], 'conveyor-side-rail');
  box(root, [2.18, 0.024, 0.52], [0, 0.795, 0], M.green, 'belt-top', 0.003);
  for (const x of [-1.04, 1.04]) cylinder(root, 0.06, 0.56, [x, 0.752, 0], M.steel, 'end-roller', 'z', high ? 32 : 12);
  box(root, [0.13, 0.16, 0.12], [-1.04, 0.70, 0.35], M.steel, 'drive-gearbox');
  cylinder(root, 0.08, 0.20, [-1.04, 0.67, 0.49], M.blue, 'gear-motor', 'z');
  if (high) {
    for (let i = 0; i < 15; i++) cylinder(root, 0.025, 0.51, [-0.9 + i * 0.13, 0.74, 0], M.steel, 'support-roller', 'z', 16);
    for (let i = 0; i < 9; i++) for (const sign of [-1, 1]) bolt(root, [-0.95 + i * 0.24, 0.76, sign * 0.337], 'z', 0.006, sign);
    cable(root, [[-1.04, 0.64, 0.54], [-0.64, 0.43, 0.34], [-0.25, 0.21, 0.27]], 0.008, 'motor-cable');
  }
  return rigConveyor(root, M.dark);
}
function inspection(high) {
  const root = conveyor(high); root.name = 'V01-camera-inspection';
  for (const z of [-0.38, 0.38]) extrusion(root, [0.06, 0.90, 0.06], [0.1, 1.16, z], 'vision-frame');
  extrusion(root, [0.06, 0.06, 0.85], [0.1, 1.61, 0], 'vision-crossbar');
  box(root, [0.13, 0.16, 0.13], [0.1, 1.49, 0], M.blue, 'industrial-camera');
  cylinder(root, 0.042, 0.08, [0.1, 1.375, 0], M.dark, 'camera-lens');
  mesh(root, new THREE.TorusGeometry(0.075, 0.013, 8, 32), M.white, [0.1, 1.32, 0], 'ring-light', [Math.PI / 2, 0, 0]);
  for (const x of [0.05, 0.15]) cylinder(root, 0.004, 0.12, [x, 1.38, -0.05], M.steel, 'ring-light-mount');
  controlPanel(root, [0.2, 1.16, 0.46], high);
  if (high) cable(root, [[0.1, 1.55, -0.05], [0.1, 1.7, -0.36], [0.1, 1.08, -0.39], [0.4, 0.65, -0.39]], 0.008, 'camera-cable');
  return markSamples(root, []);
}
function sealer(high) {
  const root = conveyor(high); root.name = 'CASE01-carton-sealer';
  for (const z of [-0.39, 0.39]) extrusion(root, [0.07, 0.80, 0.07], [0, 1.11, z], 'sealer-upright');
  box(root, [0.38, 0.11, 0.92], [0, 1.49, 0], M.white, 'sealer-head');
  cylinder(root, 0.115, 0.055, [0, 1.60, 0.13], M.cardboard, 'tape-reel', 'z', high ? 40 : 16);
  cylinder(root, 0.045, 0.06, [0, 1.60, 0.132], M.dark, 'tape-reel-hub', 'z');
  box(root, [0.49, 0.34, 0.36], [0, 0.98, 0], M.cardboard, 'carton', 0.005);
  box(root, [0.50, 0.005, 0.047], [0, 1.153, 0], M.gold, 'carton-tape', 0);
  box(root, [0.30, 0.24, 0.10], [0, 1.3195, 0], M.dark, 'taping-cassette');
  for (const x of [-0.12, 0.12]) cylinder(root, 0.022, 0.07, [x, 1.1775, 0], M.rubber, 'tape-pressure-roller', 'z');
  for (const sign of [-1, 1]) {
    box(root, [0.80, 0.14, 0.055], [0, 0.98, sign * 0.211], M.blue, 'side-drive');
    box(root, [0.76, 0.12, 0.012], [0, 0.98, sign * 0.185], M.rubber, 'side-drive-contact-belt');
    box(root, [0.12, 0.06, 0.19], [0, 0.98, sign * 0.305], M.steel, 'side-drive-mount');
  }
  controlPanel(root, [-0.44, 1.15, 0.4], high);
  return markSamples(root, ['carton', 'carton-tape']);
}
function pallet(high) {
  const root = new THREE.Group(); root.name = 'PAL01-finished-cartons';
  for (const z of [-0.40, 0, 0.40]) box(root, [1.1, 0.07, 0.13], [0, 0.035, z], M.wood, 'pallet-runner');
  for (let i = 0; i < 6; i++) box(root, [0.13, 0.045, 0.98], [-0.47 + i * 0.19, 0.0925, 0], M.wood, 'pallet-deck');
  for (let y = 0; y < 3; y++) for (const x of [-0.25, 0.25]) for (const z of [-0.22, 0.22]) {
    box(root, [0.47, 0.32, 0.41], [x, 0.275 + y * 0.32, z], M.cardboard, 'finished-carton', 0.007);
    if (high) box(root, [0.47, 0.004, 0.045], [x, 0.437 + y * 0.32, z], M.gold, 'tape-strip', 0);
  }
  return markSamples(root, ['finished-carton', 'tape-strip']);
}
const builders = { assembly, robot, conveyor, inspection, sealer, pallet };
const originals = {}, optimized = {};
const assetStats = {};
const edgeMaterial = new THREE.LineBasicMaterial({ color: 0x455361, transparent: true, opacity: 0.35, depthWrite: false });
const simplifiedMaterials = new Map();
function simplifiedMaterial(material) {
  if (!simplifiedMaterials.has(material)) {
    const simplified = material.clone(); simplified.map = null; simplified.normalMap = null; simplified.roughnessMap = null; simplified.metalnessMap = null;
    simplifiedMaterials.set(material, simplified);
  }
  return simplifiedMaterials.get(material);
}
function optimize(original, withEdges = false) {
  const root = new THREE.Group(); root.name = original.name + '-render-batched';
  const byMaterial = new Map(), edgeParts = []; original.updateMatrixWorld(true);
  original.traverse(object => {
    if (!object.isMesh) return;
    let ancestor = object;
    while (ancestor && ancestor !== original) {
      if (ancestor.userData.motion) return;
      ancestor = ancestor.parent;
    }
    const geometry = object.geometry.index ? object.geometry.toNonIndexed() : object.geometry.clone();
    geometry.applyMatrix4(object.matrixWorld);
    if (withEdges) edgeParts.push(new THREE.EdgesGeometry(geometry, 25));
    if (!byMaterial.has(object.material)) byMaterial.set(object.material, []);
    byMaterial.get(object.material).push(geometry);
  });
  for (const [material, geometries] of byMaterial) {
    const merged = mergeGeometries(geometries, false);
    const object = mesh(root, merged, withEdges ? material : simplifiedMaterial(material), [0, 0, 0], 'static-' + material.uuid.slice(0, 6));
    object.userData.staticBatch = true;
    geometries.forEach(g => g.dispose());
  }
  if (edgeParts.length) {
    const lines = new THREE.LineSegments(mergeGeometries(edgeParts, false), edgeMaterial);
    lines.name = 'CAD-contour-edges'; root.add(lines); edgeParts.forEach(g => g.dispose());
  }
  for (const child of original.children) if (child.userData.motion) {
    const copy = child.clone(true);
    if (!withEdges) copy.traverse(o => { if (o.isMesh) o.material = simplifiedMaterial(o.material); });
    root.add(copy);
  }
  return root;
}
for (const [kind, builder] of Object.entries(builders)) {
  originals[kind] = builder(true);
  optimized[kind] = { high: optimize(originals[kind], true), low: optimize(builder(false)) };
  let tris = 0, meshes = 0;
  originals[kind].traverse(o => { if (o.isMesh) { meshes++; tris += (o.geometry.index?.count ?? o.geometry.attributes.position.count) / 3; } });
  assetStats[kind] = { triangles: tris, editable_mesh_nodes: meshes, render_batches: optimized[kind].high.children.length };
}
const world = new THREE.Group(); scene.add(world);
const ground = new THREE.Mesh(new THREE.PlaneGeometry(200, 200), M.floor);
ground.rotation.x = -Math.PI / 2; ground.position.y = 0; ground.receiveShadow = true; scene.add(ground);
const grid = new THREE.GridHelper(100, 100, 0xaab4bf, 0xb8c2cd); grid.position.y = 0.001; grid.material.transparent = true; grid.material.opacity = 0.40; scene.add(grid);
const lineConfig = await (await fetch('line-config.json')).json();
let lineRun = new LineSimulation(lineConfig), lineView = null, simulationRunning = false;
const defaultInstances = { assembly: 'ASM01', robot: 'R01', conveyor: 'CV01', inspection: 'V01', sealer: 'CASE01', pallet: 'PAL01' };
const stateNames = { idle: '空闲', working: '工作', waiting_inputs: '缺料', waiting_resource: '等人员', waiting_output: '接收阻塞', waiting_quality: '待检验结果' };
const state = { scene: 'line', kind: 'assembly', instanceId: 'ASM01', previewPose: false, quality: 'high', shadow: true, phase: 0, view: 'iso', results: [], startup_ms: null };
for (const record of [...lineConfig.stations, { id: 'PAL01', title: '托盘资源', kind: 'pallet' }]) {
  const button = document.createElement('button'); button.dataset.instanceId = record.id;
  button.textContent = record.id + ' ' + record.title;
  button.addEventListener('click', () => { state.kind = record.kind; state.instanceId = record.id; state.previewPose = false; state.scene = 'device'; buildScene(); updateBusinessUI(); });
  $('line-instances').append(button);
}
function updateBusinessUI() {
  const s = lineRun.snapshot();
  $('line-clock').textContent = (s.now_ms / 1000).toFixed(1) + ' s' + (simulationRunning ? ' · 运行' : ' · 暂停');
  $('line-count').textContent = `托盘接受 ${s.order.accepted} / ${s.order.quantity} · 未交 ${s.order.unmet} · 隔离 ${s.quarantined.length}`;
  const resources = Object.entries(s.resources).map(([role, owner]) => role + '：' + (owner ?? '空闲')).join('\n');
  const selected = s.devices[state.instanceId];
  $('line-business').textContent = `订单 ${s.order.id}\n壳体 ${s.stock.shell} · PCB ${s.stock.pcb}\n螺钉 ${s.stock.fastener} · 纸箱 ${s.stock.carton}\n胶带 ${s.stock.tape_meter.toFixed(1)} m（预留 ${s.stock.tape_reserved_meter.toFixed(1)}）\n\n${resources}\n\n` +
    (selected ? `${state.instanceId} ${stateNames[selected.state]}\n阶段 ${selected.phase_id ?? '—'} · 有效进度 ${Math.round(selected.effective_progress * 100)}%\n物品 ${selected.item_id ?? '—'}\n${selected.waiting_reason}` : 'PAL01：' + s.pallet.join('、'));
  $('line-timeline').textContent = lineRun.events.slice(-9).map(e => `${(e.at_ms / 1000).toFixed(1)} s · ${e.device_id ?? '工程'}${e.item_id ? ' / ' + e.item_id : ''} · ${e.detail}`).join('\n');
  $('line-timeline').scrollTop = $('line-timeline').scrollHeight;
  for (const button of document.querySelectorAll('[data-instance-id]')) {
    const id = button.dataset.instanceId, record = lineConfig.stations.find(d => d.id === id);
    button.textContent = record ? `${id} ${record.title} · ${stateNames[s.devices[id].state]}` : `PAL01 托盘资源 · ${s.pallet.length}/12箱`;
    button.setAttribute('aria-pressed', String(id === state.instanceId && state.scene === 'device' && !state.previewPose));
  }
  $('line-quality').disabled = s.devices.V01.state !== 'waiting_quality';
  $('phase').disabled = simulationRunning || state.scene !== 'device';
}
let loadedGeometryBytes = 0, textureEstimate = 0, sceneMeshes = 0;
function addDevice(kind, x, z, yaw = 0) {
  const object = optimized[kind][state.quality].clone(true); object.position.set(x, 0, z); object.rotation.y = yaw; world.add(object); return object;
}
function batchDevices(placements) {
  for (const kind of Object.keys(builders)) {
    const positions = placements.filter(p => p.kind === kind);
    if (!positions.length) continue;
    const model = optimized[kind][state.quality]; model.updateMatrixWorld(true);
    model.traverse(part => {
      if (part.isLineSegments) {
        const geometries = positions.map(p => {
          const placement = new THREE.Matrix4().makeRotationY(p.yaw ?? 0); placement.setPosition(p.x, 0, p.z);
          return part.geometry.clone().applyMatrix4(placement.multiply(part.matrixWorld));
        });
        const lineBatch = new THREE.LineSegments(mergeGeometries(geometries, false), part.material);
        lineBatch.userData.temporaryGeometry = true; world.add(lineBatch); geometries.forEach(g => g.dispose()); return;
      }
      if (!part.isMesh) return;
      const batch = new THREE.InstancedMesh(part.geometry, part.material, positions.length);
      batch.name = kind + ':' + part.name; batch.castShadow = true; batch.receiveShadow = true;
      positions.forEach((p, i) => {
        const placement = new THREE.Matrix4().makeRotationY(p.yaw ?? 0); placement.setPosition(p.x, 0, p.z);
        batch.setMatrixAt(i, placement.multiply(part.matrixWorld));
      });
      batch.computeBoundingSphere(); world.add(batch);
    });
  }
}
function bytesAndTextures() {
  const geometries = new Set(), textures = new Set(); let bytes = 0;
  world.traverse(o => {
    if (!o.isMesh && !o.isLineSegments) return;
    geometries.add(o.geometry);
    for (const v of Object.values(o.material)) if (v?.isTexture) textures.add(v);
    if (o.isInstancedMesh) bytes += o.instanceMatrix.array.byteLength;
  });
  for (const g of geometries) { for (const a of Object.values(g.attributes)) bytes += a.array.byteLength; if (g.index) bytes += g.index.array.byteLength; }
  let textureBytes = 0;
  for (const texture of textures) {
    const image = texture.image;
    if (image?.width && image?.height) textureBytes += image.width * image.height * 4 * (texture.generateMipmaps ? 4 / 3 : 1);
  }
  return { geometryBytes: bytes, textureBytes, geometryCount: geometries.size };
}
function setView(view = 'iso') {
  state.view = view;
  if (view === 'work' && state.scene === 'device') {
    controls.target.set(0, 1.15, -0.04); camera.position.set(1.3, 2.06, 1.75); camera.lookAt(controls.target); controls.update(); return;
  }
  if (view === 'work') view = 'iso';
  const extent = state.scene === 'device' ? 0.66 : state.scene === 'line' ? 3.0 : 7.5;
  const target = state.scene === 'device' ? new THREE.Vector3(0.05, 0.95, 0) : state.scene === 'line' ? new THREE.Vector3(7.1, 0.6, 0.5) : new THREE.Vector3(0, 0.7, 0);
  controls.target.copy(target);
  const vectors = { iso: [3.5, 2.7, 4.5], front: [0, 1.15, 5.7], back: [0, 1.15, -5.7], top: [0.02, 6.0, 0.01] };
  const vector = new THREE.Vector3(...vectors[view]).multiplyScalar(extent);
  if (state.scene === 'device') {
    world.updateMatrixWorld(true);
    const bounds = new THREE.Box3().setFromObject(world);
    controls.target.copy(bounds.getCenter(new THREE.Vector3()));
    const direction = vector.normalize();
    const right = new THREE.Vector3().crossVectors(camera.up, direction).normalize();
    const up = new THREE.Vector3().crossVectors(direction, right).normalize();
    const tanV = Math.tan(THREE.MathUtils.degToRad(camera.fov / 2)), tanH = tanV * camera.aspect;
    let distance = 0;
    for (const x of [bounds.min.x, bounds.max.x]) for (const y of [bounds.min.y, bounds.max.y]) for (const z of [bounds.min.z, bounds.max.z]) {
      const corner = new THREE.Vector3(x, y, z).sub(controls.target);
      distance = Math.max(distance, corner.dot(direction) + 1.10 * Math.max(Math.abs(corner.dot(up)) / tanV, Math.abs(corner.dot(right)) / tanH));
    }
    camera.position.copy(controls.target).addScaledVector(direction, distance);
    camera.lookAt(controls.target); controls.update(); return;
  }
  camera.position.copy(target).add(vector); camera.lookAt(target); controls.update();
}
function applyPose() {
  if (state.scene !== 'device') return;
  world.traverse(o => {
    if (o.userData.motion === 'phase_z') o.position.y = -state.phase * o.userData.travel;
    if (o.userData.motion === 'gripper') {
      if (!o.userData.rest_position) o.userData.rest_position = o.position.toArray();
      o.position.fromArray(o.userData.rest_position); o.position.x -= o.userData.side * state.phase * 0.022;
    }
  });
}
function buildScene() {
  if (lineView) { lineView.dispose(); lineView = null; }
  world.children.slice().forEach(o => { world.remove(o); if (o.isInstancedMesh) o.dispose(); if (o.userData.temporaryGeometry) o.geometry.dispose(); });
  if (state.scene === 'device' && state.previewPose) addDevice(state.kind, 0, 0);
  else if (state.scene === 'device' || state.scene === 'line') {
    lineView = new LineView({ world, camera, container: $('canvas-wrap'), config: lineConfig, addDevice, makeProduct: product, materials: M,
      selectedId: state.scene === 'device' ? state.instanceId : null });
    lineView.update(lineRun.snapshot());
  } else {
    const placements = [];
    const kinds = ['assembly', 'robot', 'conveyor', 'inspection', 'sealer'];
    for (let hall = 0; hall < 3; hall++) for (let i = 0; i < 20; i++) placements.push({ kind: kinds[i % kinds.length], x: (i % 10 - 4.5) * 2.7, z: (hall - 1) * 8 + Math.floor(i / 10) * 2.6 });
    batchDevices(placements);
    const goods = new THREE.InstancedMesh(new THREE.BoxGeometry(0.16, 0.08, 0.12), M.pcb, 3000);
    goods.name = '3000-visual-items'; goods.castShadow = false; goods.receiveShadow = false;
    const transform = new THREE.Matrix4();
    for (let i = 0; i < 3000; i++) { const column = i % 100, row = Math.floor(i / 100); transform.makeTranslation((column - 49.5) * 0.23, 0.09, (row - 14.5) * 0.28); goods.setMatrixAt(i, transform); }
    goods.computeBoundingSphere(); world.add(goods);
  }
  if (state.previewPose) applyPose();
  const bound = state.scene === 'device' ? 3 : state.scene === 'line' ? 16 : 32;
  Object.assign(sun.shadow.camera, { left: -bound, right: bound, top: bound, bottom: -bound, near: 0.1, far: 100 }); sun.shadow.camera.updateProjectionMatrix();
  sun.position.set(state.scene === 'line' ? 13 : 8, state.scene === 'load' ? 30 : 16, 7);
  sun.target.position.set(state.scene === 'line' ? 5 : 0, 0, 0); sun.target.updateMatrixWorld();
  setView();
  const memory = bytesAndTextures(); loadedGeometryBytes = memory.geometryBytes; textureEstimate = memory.textureBytes;
  sceneMeshes = 0; world.traverse(o => { if (o.isMesh) sceneMeshes++; });
  $('geometry-memory').textContent = (loadedGeometryBytes / 1048576).toFixed(2) + ' MiB（当前模型）';
  $('texture-memory').textContent = (textureEstimate / 1048576).toFixed(2) + ' MiB（材质贴图）';
  $('scene-name').textContent = state.scene === 'device' ? (state.previewPose ? names[state.kind] + ' · 隔离姿态' : state.instanceId + ' · ' + names[state.kind]) : state.scene === 'line' ? 'L03 总装检查与包装线 · 8设备实例' : '3厂房 / 60设备 / 3000物品表现';
  $('object-name').textContent = state.scene === 'device' ? names[state.kind] : '共同表现负载';
  document.querySelectorAll('[data-scene]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.scene === state.scene)));
  document.querySelectorAll('[data-device]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.device === state.kind && state.scene === 'device')));
  $('status').textContent = '就绪 · L03业务状态驱动设备与物品；鼠标可自由旋转'; updateBusinessUI();
}
const gl = renderer.getContext();
const debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
const gpuName = debugInfo ? gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER);
$('renderer').textContent = gpuName;
const timerExt = gl.getExtension('EXT_disjoint_timer_query_webgl2');
const gpuQueries = [];
let recordingVideo = false;
let measurement = null, lastFrame = performance.now(), lastUpdate = 0, rollingFrames = [], latestCalls = 0, latestTriangles = 0;
const percentile = (samples, p) => { if (!samples.length) return null; const sorted = samples.slice().sort((a, b) => a - b); return sorted[Math.min(sorted.length - 1, Math.floor(p * sorted.length))]; };
function metrics() {
  return { scope: 'L03_local_business_and_3D_proof_not_full_product', business: lineRun.snapshot(), date: new Date().toISOString(), three_revision: THREE.REVISION, render_buffer: [1920, 1080], dpr: 1, renderer: gpuName, gpu_timer_available: !!timerExt, material_profile: state.quality === 'high' ? 'shared_2048_basecolor_roughness_normal' : 'simplified_untextured', startup_ms: state.startup_ms, scene: state.scene, quality: state.quality, shadow: state.shadow, camera: { position: camera.position.toArray(), target: controls.target.toArray(), view: state.view }, model_geometry_bytes: loadedGeometryBytes, model_texture_estimate_bytes: textureEstimate, render_target_minimum_bytes: 1920 * 1080 * 8 + 2048 * 2048 * 4, environment_texture_excluded_from_model_bytes: true, scene_mesh_batches: sceneMeshes, latest_frame_calls_including_shadows: latestCalls, latest_frame_triangles_including_shadows: latestTriangles, assets: assetStats, runs: state.results };
}
function writeMetrics() { $('metrics-json').value = JSON.stringify(metrics(), null, 2); }
function finishMeasurement() {
  const test = measurement; measurement = null;
  const medianFrame = percentile(test.frame, 0.5), p95Frame = percentile(test.frame, 0.95);
  const result = { scene: test.scene, kind: test.kind, quality: test.quality, shadow: test.shadow, camera_at_start: test.camera, duration_ms: performance.now() - test.start, measured_frames: test.frame.length, median_frame_interval_ms: medianFrame, p95_frame_interval_ms: p95Frame, median_fps_from_intervals: medianFrame ? 1000 / medianFrame : null, median_cpu_submit_ms: percentile(test.cpu, 0.5), p95_cpu_submit_ms: percentile(test.cpu, 0.95), median_gpu_render_ms: percentile(test.gpu, 0.5), p95_gpu_render_ms: percentile(test.gpu, 0.95), gpu_samples: test.gpu.length, triangles: latestTriangles, calls: latestCalls, geometry_bytes: loadedGeometryBytes };
  state.results.push(result);
  $('summary').textContent = `${names[result.kind]} / ${result.scene} / ${result.quality}\n帧间隔 中位 ${medianFrame?.toFixed(2)} ms；p95 ${p95Frame?.toFixed(2)} ms\n提交 中位 ${result.median_cpu_submit_ms?.toFixed(2)} ms；p95 ${result.p95_cpu_submit_ms?.toFixed(2)} ms\nGPU 中位 ${result.median_gpu_render_ms?.toFixed(2) ?? '不可读'} ms；p95 ${result.p95_gpu_render_ms?.toFixed(2) ?? '不可读'} ms\n${result.measured_frames}帧 / ${result.gpu_samples}个GPU样本`;
  $('benchmark').disabled = false;
  controls.enabled = true;
  document.querySelectorAll('[data-scene],[data-device],[data-view],#quality,#shadow,#phase,#reopen,#save,#record').forEach(b => b.disabled = false);
  $('status').textContent = '10秒表现测量完成；帧率受屏幕刷新和浏览器调度影响'; writeMetrics();
}
function animate(now) {
  requestAnimationFrame(animate);
  if (recordingVideo) return;
  const interval = now - lastFrame; lastFrame = now; if (interval > 0) rollingFrames.push(interval); if (rollingFrames.length > 90) rollingFrames.shift();
  if (simulationRunning && !measurement) {
    lineRun.advanceTo(lineRun.now + Math.max(0, interval) * Number($('line-speed').value), true);
    if (lineRun.order.accepted === lineRun.order.quantity) {
      simulationRunning = false; $('status').textContent = `L03交付完成：${lineRun.order.accepted}套，最后接受${(lineRun.result().last_acceptance_ms / 1000).toFixed(1)}秒`;
    }
  }
  controls.update();
  if (lineView) lineView.update(lineRun.snapshot());
  while (gpuQueries.length && gl.getQueryParameter(gpuQueries[0].query, gl.QUERY_RESULT_AVAILABLE)) {
    const pending = gpuQueries.shift();
    if (!gl.getParameter(timerExt.GPU_DISJOINT_EXT)) pending.test.gpu.push(gl.getQueryParameter(pending.query, gl.QUERY_RESULT) / 1e6);
    gl.deleteQuery(pending.query);
  }
  const active = measurement && now - measurement.start > 1500;
  let query;
  if (active && timerExt && gpuQueries.length < 50) { query = gl.createQuery(); gl.beginQuery(timerExt.TIME_ELAPSED_EXT, query); }
  renderer.info.reset();
  const before = performance.now(); renderer.render(scene, camera); const cpu = performance.now() - before;
  latestCalls = renderer.info.render.calls; latestTriangles = renderer.info.render.triangles;
  if (query) { gl.endQuery(timerExt.TIME_ELAPSED_EXT); gpuQueries.push({ query, test: measurement }); }
  if (active) { measurement.frame.push(interval); measurement.cpu.push(cpu); }
  if (measurement && now - measurement.start >= 11500) finishMeasurement();
  if (now - lastUpdate > 500) {
    const median = percentile(rollingFrames, 0.5); $('fps').textContent = median ? (1000 / median).toFixed(1) + ' FPS（帧间隔）' : '—';
    $('triangles').textContent = latestTriangles.toLocaleString(); $('calls').textContent = latestCalls.toLocaleString();
    $('pose').textContent = `方位 ${(controls.getAzimuthalAngle() * 180 / Math.PI).toFixed(1)}° / 极角 ${(controls.getPolarAngle() * 180 / Math.PI).toFixed(1)}°`;
    updateBusinessUI(); lastUpdate = now;
  }
}
document.querySelectorAll('[data-scene]').forEach(b => b.addEventListener('click', () => { state.scene = b.dataset.scene; state.previewPose = false; buildScene(); writeMetrics(); }));
document.querySelectorAll('[data-device]').forEach(b => b.addEventListener('click', () => { state.kind = b.dataset.device; state.instanceId = defaultInstances[state.kind]; state.previewPose = true; state.scene = 'device'; buildScene(); writeMetrics(); }));
document.querySelectorAll('[data-view]').forEach(b => b.addEventListener('click', () => setView(b.dataset.view)));
$('quality').addEventListener('change', e => { state.quality = e.target.value; buildScene(); writeMetrics(); });
$('shadow').addEventListener('change', e => { state.shadow = e.target.checked; renderer.shadowMap.enabled = state.shadow; writeMetrics(); });
$('phase').addEventListener('input', e => { state.phase = Number(e.target.value); if (!state.previewPose) { state.previewPose = true; buildScene(); } applyPose(); $('phase-value').textContent = Math.round(state.phase * 100) + '%'; });
$('line-run').addEventListener('click', () => { state.previewPose = false; if (state.scene === 'device') buildScene(); simulationRunning = true; lastFrame = performance.now(); updateBusinessUI(); });
$('line-pause').addEventListener('click', () => { simulationRunning = false; updateBusinessUI(); });
$('line-step').addEventListener('click', () => { simulationRunning = false; state.previewPose = false; lineRun.advanceTo(lineRun.now + 1000); if (lineView) lineView.update(lineRun.snapshot()); updateBusinessUI(); writeMetrics(); });
function resetLine() { simulationRunning = false; lineRun = new LineSimulation(lineConfig, $('line-scenario').value); state.previewPose = false; buildScene(); writeMetrics(); }
$('line-reset').addEventListener('click', resetLine); $('line-scenario').addEventListener('change', resetLine);
$('line-supply').addEventListener('click', () => { lineRun.supply('pcb', 4); updateBusinessUI(); });
$('line-quality').addEventListener('click', () => { lineRun.setQuality(lineRun.devices.V01.job.itemId, 'pass'); updateBusinessUI(); });
$('line-open').addEventListener('click', () => { lineRun.openReceiver(); updateBusinessUI(); });
$('line-export').addEventListener('click', () => {
  const url = URL.createObjectURL(new Blob([JSON.stringify(lineRun.result(), null, 2)], { type: 'application/json' }));
  const link = document.createElement('a'); link.href = url; link.download = 'L03-business-result.json'; link.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
});
$('benchmark').addEventListener('click', () => {
  measurement = { start: performance.now(), scene: state.scene, kind: state.kind, quality: state.quality, shadow: state.shadow, camera: { position: camera.position.toArray(), target: controls.target.toArray() }, frame: [], cpu: [], gpu: [] };
  $('benchmark').disabled = true;
  controls.enabled = false;
  document.querySelectorAll('[data-scene],[data-device],[data-view],#quality,#shadow,#phase,#reopen,#save,#record').forEach(b => b.disabled = true);
  $('status').textContent = '固定场景与相机：预热1.5秒＋测量10秒；请保持窗口可见';
});
let exportedAssetSizes = null; // Definitions are immutable in this proof; camera and pose edits do not change them.
$('save').addEventListener('click', async () => {
  $('save').disabled = true; $('status').textContent = '正在导出六类完整可编辑GLB与当前画面…';
  const savingContext = { kind: state.kind, scene: state.scene, quality: state.quality, view: state.view };
  renderer.render(scene, camera); const image = renderer.domElement.toDataURL('image/png');
  const data = metrics();
  try {
    const exporter = new GLTFExporter(); const exportSizes = exportedAssetSizes ?? {};
    if (!exportedAssetSizes) for (const [kind, original] of Object.entries(originals)) {
      const binary = await exporter.parseAsync(original, { binary: true });
      const savedModel = await fetch('/save-model/' + kind, { method: 'POST', headers: { 'Content-Type': 'application/octet-stream' }, body: binary });
      if (!savedModel.ok) throw new Error(kind + '模型保存HTTP ' + savedModel.status);
      exportSizes[kind] = binary.byteLength;
    }
    exportedAssetSizes = exportSizes;
    data.glb_bytes = exportSizes;
    const response = await fetch('/save-proof', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ ...savingContext, model_sizes: exportSizes, image, metrics: data }) });
    if (!response.ok) throw new Error('本地保存HTTP ' + response.status);
    const saved = await response.json(); $('export-status').textContent = Object.values(exportSizes).reduce((a, b) => a + b, 0) / 1048576 < 100 ? (Object.values(exportSizes).reduce((a, b) => a + b, 0) / 1048576).toFixed(2) + ' MiB / 六类GLB' : '已导出';
    $('status').textContent = '已保存：output/' + saved.image + '，六类GLB及测量JSON';
  } catch (error) { $('status').textContent = '保存失败：' + error.message; console.error(error); }
  finally { $('save').disabled = false; writeMetrics(); }
});
$('reopen').addEventListener('click', async () => {
  const kind = state.kind; $('reopen').disabled = true;
  try {
    const loaded = await new GLTFLoader().loadAsync('output/' + kind + '.glb');
    const imported = loaded.scene.children[0];
    optimized[kind].high = optimize(imported, true);
    state.kind = kind; state.instanceId = defaultInstances[kind]; state.previewPose = false; state.scene = 'device'; state.quality = 'high'; $('quality').value = 'high';
    buildScene(); writeMetrics(); $('status').textContent = '已从磁盘重开：output/' + kind + '.glb · ' + names[kind];
  } catch (error) { $('status').textContent = 'GLB重开失败：' + error.message; console.error(error); }
  finally { $('reopen').disabled = false; }
});
function previewMotion(time, isAssembly) {
  const p = time % (isAssembly ? 3 : 2), down = isAssembly ? 0.9 : 0.6, hold = isAssembly ? 1.4 : 1.1, up = isAssembly ? 2.3 : 1.7;
  const ease = x => x * x * (3 - 2 * x);
  if (p < down) return { value: ease(p / down), label: isAssembly ? '主轴下压' : '夹爪闭合' };
  if (p < hold) return { value: 1, label: isAssembly ? '紧固位停留' : '保持闭合' };
  if (p < up) return { value: 1 - ease((p - hold) / (up - hold)), label: isAssembly ? '主轴退回' : '夹爪张开' };
  return { value: 0, label: isAssembly ? '待机' : '保持张开' };
}
$('record').addEventListener('click', async () => {
  const previous = { ...state }, previousPosition = camera.position.clone(), previousTarget = controls.target.clone();
  const wasRunning = simulationRunning; simulationRunning = false;
  const inputs = document.querySelectorAll('[data-scene],[data-device],[data-view],[data-instance-id],.simulation-bar button,.simulation-bar select,#quality,#shadow,#phase,#reopen,#save,#benchmark,#record,#line-record');
  inputs.forEach(b => b.disabled = true); controls.enabled = false; recordingVideo = true;
  $('record-result').hidden = true;
  const capture = document.createElement('canvas'); capture.width = 1920; capture.height = 1080;
  const context = capture.getContext('2d');
  const post = async (path, options = {}) => {
    const response = await fetch(path, { method: 'POST', ...options });
    if (!response.ok) throw new Error(path + ' HTTP ' + response.status);
    return response.json();
  };
  let encoderStarted = false, completion;
  try {
    $('status').textContent = '正在从磁盘载入装配台与机器人GLB…';
    for (const kind of ['assembly', 'robot']) {
      const loaded = await new GLTFLoader().loadAsync('output/' + kind + '.glb');
      optimized[kind].high = optimize(loaded.scene.children[0], true);
    }
    await post('/record-start'); encoderStarted = true;
    state.scene = 'device'; state.previewPose = true; state.quality = 'high'; state.shadow = true; renderer.shadowMap.enabled = true;
    let previousShot = -1, basePosition, baseTarget;
    for (let frame = 0; frame < 720; frame++) {
      const time = frame / 30, shot = Math.floor(time / 6), localTime = time % 6;
      const isAssembly = shot < 2;
      if (shot !== previousShot) {
        state.kind = isAssembly ? 'assembly' : 'robot'; state.phase = 0; buildScene();
        if (shot === 1) { controls.target.set(0.12, 1.07, 0.02); camera.position.set(1.1, 1.8, 1.3); }
        if (shot === 3) { controls.target.set(0.89, 1.07, 0); camera.position.set(1.4, 1.36, 0.76); }
        camera.lookAt(controls.target); basePosition = camera.position.clone(); baseTarget = controls.target.clone(); previousShot = shot;
      }
      const motion = previewMotion(localTime, isAssembly);
      state.phase = motion.value; applyPose();
      const offset = basePosition.clone().sub(baseTarget);
      if (shot === 0 || shot === 2) offset.applyAxisAngle(new THREE.Vector3(0, 1, 0), 0.18 * Math.sin((localTime / 6 - 0.5) * Math.PI));
      camera.position.copy(baseTarget).add(offset); camera.lookAt(baseTarget);
      renderer.info.reset(); renderer.render(scene, camera);
      context.drawImage(renderer.domElement, 0, 0);
      context.fillStyle = 'rgba(20,32,44,0.88)'; context.fillRect(44, 36, 670, 118); context.fillRect(44, 950, 1832, 86);
      context.fillStyle = '#ffffff'; context.font = 'bold 30px "Microsoft YaHei",sans-serif';
      context.fillText(isAssembly ? '定位与紧固装配台 · 主轴往复' : '六轴机器人 · 夹爪开合', 68, 84);
      context.font = '22px "Microsoft YaHei",sans-serif'; context.fillStyle = '#bfd2df';
      context.fillText((shot % 2 ? '部件近景' : '设备整体 / 相机环绕') + '   ·   ' + motion.label, 68, 128);
      context.fillStyle = '#ffffff'; context.fillText('实际GLB逐帧渲染 · 部件动作预览 · 尚未接业务仿真', 68, 988);
      context.font = '18px "Microsoft YaHei",sans-serif'; context.fillStyle = '#bfd2df';
      context.fillText('1920 × 1080   |   30 fps视频输出   |   ' + time.toFixed(1) + ' / 24.0 s', 68, 1018);
      context.fillStyle = '#4b6577'; context.fillRect(1370, 983, 460, 8); context.fillStyle = '#63d9b0'; context.fillRect(1370, 983, 460 * (frame + 1) / 720, 8);
      const png = await new Promise(resolve => capture.toBlob(resolve, 'image/png'));
      await post('/record-frame', { headers: { 'Content-Type': 'image/png', 'X-Frame-Index': String(frame) }, body: png });
      if (frame % 15 === 0) $('status').textContent = '正在录制：' + (frame + 1) + '/720帧 · ' + Math.round((frame + 1) / 7.2) + '% · ' + motion.label;
    }
    completion = await post('/record-finish'); encoderStarted = false;
    $('record-video').src = 'output/' + completion.file + '?v=' + Date.now(); $('record-result').hidden = false;
  } catch (error) {
    completion = { error: error.message }; console.error(error);
    if (encoderStarted) await post('/record-cancel');
  } finally {
    Object.assign(state, previous); $('quality').value = state.quality; $('shadow').checked = state.shadow;
    $('phase').value = String(state.phase); $('phase-value').textContent = Math.round(state.phase * 100) + '%';
    renderer.shadowMap.enabled = state.shadow; buildScene(); camera.position.copy(previousPosition); controls.target.copy(previousTarget); camera.lookAt(controls.target);
    controls.enabled = true; recordingVideo = false; simulationRunning = wasRunning; lastFrame = performance.now(); inputs.forEach(b => b.disabled = false); updateBusinessUI(); writeMetrics();
    $('status').textContent = completion?.error ? '录制失败：' + completion.error : '视频已保存：output/device-motion.mp4 · 24秒 / 1080p / 30fps';
  }
});
let cancelLineRecording = false;
$('line-record-cancel').addEventListener('click', () => { cancelLineRecording = true; $('line-record-cancel').disabled = true; });
$('line-record').addEventListener('click', async () => {
  const previous = { ...state }, previousRun = lineRun, wasRunning = simulationRunning;
  const inputs = document.querySelectorAll('[data-scene],[data-device],[data-view],[data-instance-id],.simulation-bar button,.simulation-bar select,#quality,#shadow,#phase,#reopen,#save,#benchmark,#record,#line-record');
  const post = async (path, options = {}) => {
    const response = await fetch(path, { method: 'POST', ...options });
    if (!response.ok) throw new Error(path + ' HTTP ' + response.status);
    return response.json();
  };
  simulationRunning = false; recordingVideo = true; controls.enabled = false; cancelLineRecording = false;
  inputs.forEach(b => b.disabled = true); $('line-record-cancel').disabled = false; $('line-record-result').hidden = true;
  let completion;
  try {
    $('status').textContent = '正在从磁盘载入六类GLB，准备L03完整业务录制…';
    for (const kind of Object.keys(builders)) {
      const loaded = await new GLTFLoader().loadAsync('output/' + kind + '.glb');
      optimized[kind].high = optimize(loaded.scene.children[0], true);
    }
    lineRun = new LineSimulation(lineConfig, 'output_blocked');
    Object.assign(state, { scene: 'line', previewPose: false, quality: 'high', shadow: true }); renderer.shadowMap.enabled = true;
    $('line-scenario').value = 'output_blocked'; buildScene();
    completion = await recordLine({ config: lineConfig, run: lineRun, view: lineView, camera, renderer, scene, post,
      isCancelled: () => cancelLineRecording,
      onProgress: (frame, total, snapshot, title) => {
        updateBusinessUI();
        $('status').textContent = `录制L03：${frame}/${total}帧 · ${Math.round(frame / total * 100)}% · 业务${(snapshot.now_ms / 1000).toFixed(1)}s · ${title}`;
      },
    });
    $('line-record-video').src = 'output/' + completion.file + '?v=' + Date.now(); $('line-record-result').hidden = false;
  } catch (error) {
    completion = { error: error.message }; console.error(error);
    lineRun = previousRun; Object.assign(state, previous); simulationRunning = wasRunning;
  } finally {
    recordingVideo = false; controls.enabled = true; $('line-record-cancel').disabled = true;
    $('quality').value = state.quality; $('shadow').checked = state.shadow;
    buildScene(); inputs.forEach(b => b.disabled = false); updateBusinessUI(); writeMetrics(); lastFrame = performance.now();
    $('status').textContent = completion?.error ? 'L03录制失败：' + completion.error : 'L03完整运行视频已保存：output/L03-full-run.mp4 · 143秒 / 1080p / 30fps · 330秒六套进入托盘';
  }
});
buildScene();
controls.addEventListener('start', () => { state.view = 'free'; });
state.startup_ms = performance.now() - started;
writeMetrics();
requestAnimationFrame(animate);
