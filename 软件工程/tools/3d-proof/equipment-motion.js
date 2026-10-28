import * as THREE from './vendor/three.module.js';

export function rigRobot(root) {
  const positions = [[0, .18, 0], [0, .65, 0], [-.28, 1.40, 0], [.52, 1.58, 0], [.89, 1.33, 0], [.89, 1.17, 0]];
  const rollLength = Math.hypot(.8, .18);
  const axes = [[0, 1, 0], [0, 0, 1], [0, 0, 1], [.8 / rollLength, .18 / rollLength, 0], [0, 0, 1], [0, 1, 0]];
  const parts = [...root.children];
  const joints = positions.map((p, i) => {
    const group = new THREE.Group(); group.name = 'joint-' + (i + 1); group.position.set(...p);
    group.userData = { motion: 'robot_joint', joint_index: i + 1, axis: axes[i],
      nominal_limits_radian: [-Math.PI, Math.PI] };
    root.add(group); return group;
  });
  root.updateMatrixWorld(true);
  for (let i = 1; i < joints.length; i++) joints[i - 1].attach(joints[i]);
  for (const part of parts) {
    const n = part.name;
    if (n === 'robot-baseplate' || n === 'axis1-base') continue;
    if (n === 'robot-dress-pack') { part.userData.motion = 'flexible_robot_cable'; continue; }
    let index = n === 'upper-arm' || n.startsWith('axis2-') ? 1 : n === 'forearm' || n.startsWith('axis3-') ? 2 :
      n === 'wrist-link' || n.startsWith('axis4-') ? 3 : n.startsWith('axis5-') ? 4 :
      n.startsWith('axis6-') || n === 'tool-flange' || n === 'parallel-gripper' || n.startsWith('gripper-') ? 5 : 0;
    if (n === 'bolt-head' || n === 'hex-socket' || n.startsWith('dress-pack-')) {
      index = [1, 2, 3, 4].reduce((best, i) => part.position.distanceTo(new THREE.Vector3(...positions[i])) < part.position.distanceTo(new THREE.Vector3(...positions[best])) ? i : best, 1);
      if (part.position.y < .2) continue;
    }
    if (n.startsWith('dress-pack-')) {
      part.userData.cable_order = part.position.x < 0 ? 0 : part.position.x < .6 ? 1 : 2;
      index = part.userData.cable_order + 1;
    }
    joints[index].attach(part);
  }
  root.updateMatrixWorld(true);
  return root;
}

export function rigAssembly(root) {
  const xy = new THREE.Group(); xy.name = 'xy-positioning-carriage'; xy.userData.motion = 'gantry_xy'; root.add(xy);
  const prefixes = ['z-axis-backplate', 'linear-guide', 'z-slide-and-driver', 'drag-chain', 'slide-flexible-lead'];
  root.updateMatrixWorld(true);
  for (const part of [...root.children]) {
    if (part === xy) continue;
    if (prefixes.some(prefix => part.name.startsWith(prefix)) ||
        (['bolt-head', 'hex-socket'].includes(part.name) && part.position.y > 1.2 && Math.abs(part.position.z + .328) < .03)) xy.attach(part);
  }
  root.getObjectByName('control-box-product').userData.motion = 'sample_geometry';
  return root;
}

export function rigConveyor(root, material) {
  for (const part of root.children) if (part.name.includes('roller')) part.userData.motion = 'roller';
  for (let i = 0; i < 8; i++) {
    const cleat = new THREE.Mesh(new THREE.BoxGeometry(.018, .003, .50), material);
    cleat.name = 'belt-cleat-' + i; cleat.position.set(-1 + i * .26, .809, 0);
    cleat.userData = { motion: 'belt_cleat', index: i }; root.add(cleat);
  }
  return root;
}

export function markSamples(root, names) {
  root.traverse(part => {
    if (names.includes(part.name)) part.userData.motion = 'sample_geometry';
    if (part.name === 'tape-reel' || part.name === 'tape-pressure-roller') part.userData.motion = 'roller';
    if (part.name === 'ring-light') part.userData.motion = 'inspection_light';
  });
  return root;
}

export function robotPose(root, target, closed) {
  const j = Array.from({ length: 6 }, (_, i) => root.getObjectByName('joint-' + (i + 1)));
  const l1 = Math.hypot(.28, .75), l2 = Math.hypot(1.17, -.07);
  const theta1 = Math.atan2(.75, -.28), theta2 = Math.atan2(-.07, 1.17);
  const radial = Math.hypot(target[0], target[2]), height = target[1] + .395 - .65;
  const cosine = (radial * radial + height * height - l1 * l1 - l2 * l2) / (2 * l1 * l2);
  if (cosine < -1 - 1e-6 || cosine > 1 + 1e-6) throw new Error('L03示意机器人目标超出已建连杆可达范围');
  const elbow = -Math.acos(THREE.MathUtils.clamp(cosine, -1, 1));
  const shoulder = Math.atan2(height, radial) - Math.atan2(l2 * Math.sin(elbow), l1 + l2 * Math.cos(elbow));
  const q1 = Math.atan2(-target[2], target[0]), q2 = shoulder - theta1, q3 = elbow - (theta2 - theta1);
  const angles = [q1, q2, q3, 0, -(q2 + q3), -q1];
  j.forEach((joint, i) => joint.quaternion.setFromAxisAngle(new THREE.Vector3(...joint.userData.axis), angles[i]));
  for (const side of ['left', 'right']) {
    const jaw = root.getObjectByName('gripper-' + side + '-slide');
    if (!jaw.userData.rest_position) jaw.userData.rest_position = jaw.position.toArray();
    jaw.position.fromArray(jaw.userData.rest_position);
    jaw.position.x -= jaw.userData.side * closed * .022;
  }
  root.updateMatrixWorld(true);
  const wire = root.getObjectByName('robot-dress-pack');
  const clamps = [];
  root.traverse(part => { if (part.name === 'dress-pack-clamp') clamps.push(part); });
  clamps.sort((a, b) => a.userData.cable_order - b.userData.cable_order);
  const points = [root.worldToLocal(j[0].localToWorld(new THREE.Vector3(0, .14, -.14))),
    ...clamps.map(part => root.worldToLocal(part.getWorldPosition(new THREE.Vector3()))),
    root.worldToLocal(j[5].localToWorld(new THREE.Vector3(0, .03, -.10)))];
  if (wire) { // The simplified asset intentionally omits the cable.
    if (wire.userData.live_geometry) wire.geometry.dispose();
    wire.geometry = new THREE.TubeGeometry(new THREE.CatmullRomCurve3(points), 96, .018, 8, false);
    wire.userData.live_geometry = true;
  }
  return j[5].localToWorld(new THREE.Vector3(0, -.235, 0));
}

export function assemblyPose(root, progress, working, operation) {
  const xy = root.getObjectByName('xy-positioning-carriage');
  const z = root.getObjectByName('z-slide-and-driver');
  const bit = root.getObjectByName('screwdriver-bit');
  if (!working || operation === 'pack') { xy.position.set(0, 0, 0); z.position.y = 0; return; }
  const fastening = THREE.MathUtils.clamp((progress - .2) / .6, 0, 1);
  const index = Math.min(3, Math.floor(fastening * 4)), local = fastening * 4 - index;
  // The adopted control-box asset is 1:1; a fixture adapter raises its screw
  // heads to the existing 83mm Z-stroke. The old 1.35x sample is hidden.
  const targets = [[-.042, -.028], [-.282, -.028], [-.282, -.188], [-.042, -.188]];
  const previous = targets[Math.max(0, index - 1)], target = targets[index];
  const transition = THREE.MathUtils.smoothstep(local, 0, .18);
  xy.position.set(THREE.MathUtils.lerp(previous[0], target[0], transition), 0, THREE.MathUtils.lerp(previous[1], target[1], transition));
  const down = progress >= .2 && progress < .8 ? Math.min(THREE.MathUtils.smoothstep(local, .18, .36), 1 - THREE.MathUtils.smoothstep(local, .75, 1)) : 0;
  z.position.y = -z.userData.travel * down;
  bit.rotation.y = fastening * 4 * 18 * Math.PI;
}

export function conveyorPose(root, workMs, working) {
  root.traverse(part => {
    if (part.userData.motion === 'belt_cleat') part.position.x = -1.04 + ((part.userData.index / 8 + workMs / 4000) % 1) * 2.08;
    if (part.userData.motion === 'roller') {
      if (!part.userData.rest_quaternion) part.userData.rest_quaternion = part.quaternion.toArray();
      part.quaternion.fromArray(part.userData.rest_quaternion).premultiply(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 0, 1), workMs / 4000 * Math.PI * 8));
    }
    if (part.userData.motion === 'inspection_light') {
      if (!part.userData.own_material) { part.material = part.material.clone(); part.userData.own_material = true; }
      part.material.emissive.set(working ? '#d4edff' : '#000000'); part.material.emissiveIntensity = working ? .9 : 0;
    }
  });
}
