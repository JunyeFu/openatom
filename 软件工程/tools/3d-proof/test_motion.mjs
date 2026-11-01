import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import * as THREE from './vendor/three.module.js';
import { robotPose, assemblyPose } from './equipment-motion.js';

function loadGeometry(kind) {
  const bytes = readFileSync(new URL(`./output/${kind}.glb`, import.meta.url));
  const jsonLength = bytes.readUInt32LE(12), gltf = JSON.parse(bytes.subarray(20, 20 + jsonLength));
  const binaryStart = 28 + jsonLength;
  const geometries = gltf.meshes.map(mesh => {
    const index = mesh.primitives[0].attributes.POSITION, a = gltf.accessors[index], view = gltf.bufferViews[a.bufferView];
    const offset = binaryStart + (view.byteOffset ?? 0) + (a.byteOffset ?? 0), stride = view.byteStride ?? 12;
    const positions = new Float32Array(a.count * 3);
    for (let n = 0; n < a.count; n++) for (let axis = 0; axis < 3; axis++) positions[n * 3 + axis] = bytes.readFloatLE(offset + n * stride + axis * 4);
    return new THREE.BufferGeometry().setAttribute('position', new THREE.BufferAttribute(positions, 3));
  });
  const nodes = gltf.nodes.map(n => {
    const object = n.mesh == null ? new THREE.Group() : new THREE.Mesh(geometries[n.mesh]);
    object.name = n.name; object.userData = structuredClone(n.extras ?? {});
    if (n.matrix) new THREE.Matrix4().fromArray(n.matrix).decompose(object.position, object.quaternion, object.scale);
    else { object.position.fromArray(n.translation ?? [0, 0, 0]); object.quaternion.fromArray(n.rotation ?? [0, 0, 0, 1]); object.scale.fromArray(n.scale ?? [1, 1, 1]); }
    return object;
  });
  gltf.nodes.forEach((n, i) => (n.children ?? []).forEach(child => nodes[i].add(nodes[child])));
  const scene = new THREE.Group(); gltf.scenes[gltf.scene ?? 0].nodes.forEach(i => scene.add(nodes[i]));
  return { root: scene.children[0], nodes };
}

const near = (a, b, message) => assert(Math.abs(a - b) < 2e-5, `${message}: ${a} vs ${b}`);
const robot = loadGeometry('robot');
for (let i = 1; i <= 6; i++) {
  const joint = robot.root.getObjectByName('joint-' + i); assert(joint);
  if (i > 1) assert.equal(joint.parent.name, 'joint-' + (i - 1));
  near(new THREE.Vector3(...joint.userData.axis).length(), 1, 'joint axis unit length');
}
const targets = [[.89, .935, 0], [-.8, .9155, .93], [-.8, 1.65, .93], [.56, 1.65, .9], [.56, .8605, .9]];
const poseChecks = [];
for (const target of targets) {
  const tip = robotPose(robot.root, target, 1);
  tip.toArray().forEach((value, i) => near(value, target[i], 'tool follows adopted target'));
  const pads = robot.nodes.filter(n => n.name === 'jaw-contact-pad').map(n => new THREE.Box3().setFromObject(n));
  const left = pads.find(b => b.getCenter(new THREE.Vector3()).x < target[0]), right = pads.find(b => b !== left);
  near(left.max.x, target[0] - .172, 'left jaw touches shared control-box wall');
  near(right.min.x, target[0] + .172, 'right jaw touches shared control-box wall');
  const wire = robot.root.getObjectByName('robot-dress-pack'), attribute = wire.geometry.getAttribute('position');
  const centers = [];
  for (let row = 0; row <= wire.geometry.parameters.tubularSegments; row++) {
    const center = new THREE.Vector3();
    for (let col = 0; col < 8; col++) center.add(new THREE.Vector3().fromBufferAttribute(attribute, row * 9 + col));
    center.divideScalar(8); centers.push(wire.localToWorld(center));
  }
  for (const clamp of robot.nodes.filter(n => n.name === 'dress-pack-clamp')) {
    const center = clamp.getWorldPosition(new THREE.Vector3());
    const distances = centers.slice(1).map((end, i) => {
      const start = centers[i], delta = end.clone().sub(start);
      const fraction = THREE.MathUtils.clamp(center.clone().sub(start).dot(delta) / delta.lengthSq(), 0, 1);
      return start.clone().addScaledVector(delta, fraction).distanceTo(center);
    });
    assert(Math.min(...distances) < .002, `animated cable passes through moving clamp: ${Math.min(...distances)}m`);
  }
  poseChecks.push({ target_meter: target, tool_error_meter: tip.distanceTo(new THREE.Vector3(...target)), jaw_and_cable_contact: 'passed' });
}
const assembly = loadGeometry('assembly');
const screws = [[.12, .15], [-.12, .15], [-.12, -.01], [.12, -.01]];
const fasteningChecks = [];
for (let i = 0; i < 4; i++) {
  assemblyPose(assembly.root, .2 + (i + .5) / 4 * .6, true, 'assemble');
  assembly.root.updateMatrixWorld(true);
  const bit = assembly.root.getObjectByName('screwdriver-bit'), bounds = new THREE.Box3().setFromObject(bit), center = bounds.getCenter(new THREE.Vector3());
  near(center.x, screws[i][0], 'driver X targets adopted screw'); near(center.z, screws[i][1], 'driver Z targets adopted screw');
  const clearance = bounds.min.y - .9564;
  assert(clearance >= 0 && clearance < .002, 'driver does not penetrate adopted screw');
  fasteningChecks.push({ screw_index: i + 1, target_xz_meter: screws[i], clearance_mm: clearance * 1000 });
}
const result = { scope: 'actual_exported_GLB_nominal_motion_and_contact_checks_not_industrial_kinematics',
  robot_joint_hierarchy: 'six_named_parent_child_joints', robot_poses: poseChecks, four_point_fastening: fasteningChecks, passed: true };
writeFileSync(new URL('./output/line-motion-checks.json', import.meta.url), JSON.stringify(result, null, 2));
console.log(JSON.stringify(result, null, 2));
