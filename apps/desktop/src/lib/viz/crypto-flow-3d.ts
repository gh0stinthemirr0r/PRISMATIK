/**
 * 3D Crypto Flow Rivers — on-chain flows as luminous particle streams.
 * GPU particle system with instanced rendering. Flows between exchanges,
 * wallets, protocols. Width = volume, color = direction (buy/sell).
 * Custom vertex shader for particle animation on GPU.
 */
import * as THREE from 'three';
import { PALETTE, createScene, createCamera, createRenderer, addDramaticLighting, autoResize, disposeScene, createParticleField } from './effects';

export interface CryptoFlow {
  from: string;
  to: string;
  volume: number;
  direction: 'inflow' | 'outflow';
  chain: string;
}

export interface CryptoNode {
  id: string;
  label: string;
  type: 'exchange' | 'wallet' | 'protocol' | 'bridge';
  balance: number;
  x: number;
  y: number;
  z: number;
}

export interface CryptoFlowConfig {
  nodes: CryptoNode[];
  flows: CryptoFlow[];
}

const NODE_COLORS: Record<string, number> = {
  exchange: 0x00f0ff,
  wallet: 0xa855f7,
  protocol: 0x34d399,
  bridge: 0xfbbf24,
};

// GPU particle shader for flow animation
const flowVertexShader = `
  attribute float speed;
  attribute float offset;
  uniform float time;
  varying float vAlpha;
  varying vec3 vColor;
  void main() {
    float t = mod(time * speed + offset, 1.0);
    vec3 pos = mix(position, position, t);
    vAlpha = sin(t * 3.14159) * 0.8;
    vColor = color;
    vec4 mvPosition = modelViewMatrix * vec4(pos, 1.0);
    gl_Position = projectionMatrix * mvPosition;
    gl_PointSize = (4.0 / -mvPosition.z) * 100.0;
  }
`;

const flowFragmentShader = `
  varying float vAlpha;
  varying vec3 vColor;
  void main() {
    float d = length(gl_PointCoord - vec2(0.5));
    if (d > 0.5) discard;
    float glow = smoothstep(0.5, 0.0, d);
    gl_FragColor = vec4(vColor, vAlpha * glow);
  }
`;

export function createCryptoFlowField(container: HTMLElement, config: CryptoFlowConfig) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene(0x030308);
  const camera = createCamera(55, W / H, [0, 30, 50], [0, 0, 0]);
  const renderer = createRenderer(W, H, 0x030308);
  container.appendChild(renderer.domElement);
  addDramaticLighting(scene);

  // background particles
  scene.add(createParticleField(2000, 120, PALETTE.violet, 0.06, 0.15));

  // ground grid
  const grid = new THREE.GridHelper(80, 40, 0x1a2332, 0x0a0f1e);
  (grid.material as THREE.LineBasicMaterial).transparent = true;
  (grid.material as THREE.LineBasicMaterial).opacity = 0.15;
  grid.position.y = -5;
  scene.add(grid);

  // nodes as glowing orbs
  const nodeMap = new Map<string, THREE.Mesh>();
  config.nodes.forEach(node => {
    const color = NODE_COLORS[node.type] ?? PALETTE.cyan;
    const size = 0.4 + Math.min(node.balance / 1e9, 3);
    const geo = new THREE.SphereGeometry(size, 16, 16);
    const mat = new THREE.MeshPhongMaterial({
      color, emissive: color, emissiveIntensity: 0.4,
      transparent: true, opacity: 0.85, shininess: 80,
    });
    const mesh = new THREE.Mesh(geo, mat);
    mesh.position.set(node.x, node.y, node.z);
    scene.add(mesh);
    nodeMap.set(node.id, mesh);

    // glow halo
    const glowMat = new THREE.SpriteMaterial({
      color, transparent: true, opacity: 0.2,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    const glow = new THREE.Sprite(glowMat);
    glow.scale.set(size * 6, size * 6, 1);
    mesh.add(glow);

    // label
    const canvas = document.createElement('canvas');
    canvas.width = 256; canvas.height = 64;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#ffffff';
    ctx.font = '20px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(node.label, 128, 40);
    const tex = new THREE.CanvasTexture(canvas);
    const spriteMat = new THREE.SpriteMaterial({ map: tex, transparent: true, opacity: 0.6 });
    const label = new THREE.Sprite(spriteMat);
    label.scale.set(8, 2, 1);
    label.position.y = size + 1.5;
    mesh.add(label);
  });

  // flow particle streams — GPU instanced particles
  config.flows.forEach(flow => {
    const fromNode = nodeMap.get(flow.from);
    const toNode = nodeMap.get(flow.to);
    if (!fromNode || !toNode) return;

    const start = fromNode.position;
    const end = toNode.position;
    const mid = start.clone().add(end).multiplyScalar(0.5);
    mid.y += 5 + flow.volume * 0.001;

    const curve = new THREE.QuadraticBezierCurve3(start, mid, end);
    const points = curve.getPoints(50);
    const particleCount = Math.min(200, Math.max(20, flow.volume * 0.01));

    // create particle positions along the curve
    const positions = new Float32Array(particleCount * 3);
    const speeds = new Float32Array(particleCount);
    const offsets = new Float32Array(particleCount);

    for (let i = 0; i < particleCount; i++) {
      const t = i / particleCount;
      const pt = curve.getPoint(t);
      positions[i * 3] = pt.x + (Math.random() - 0.5) * 0.5;
      positions[i * 3 + 1] = pt.y + (Math.random() - 0.5) * 0.5;
      positions[i * 3 + 2] = pt.z + (Math.random() - 0.5) * 0.5;
      speeds[i] = 0.3 + Math.random() * 0.7;
      offsets[i] = Math.random();
    }

    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    geo.setAttribute('speed', new THREE.BufferAttribute(speeds, 1));
    geo.setAttribute('offset', new THREE.BufferAttribute(offsets, 1));

    const color = flow.direction === 'inflow' ? new THREE.Color(PALETTE.emerald) : new THREE.Color(PALETTE.red);
    const mat = new THREE.ShaderMaterial({
      vertexShader: flowVertexShader,
      fragmentShader: flowFragmentShader,
      uniforms: {
        time: { value: 0 },
        color: { value: color },
      },
      transparent: true,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      vertexColors: true,
    });

    // set color attribute
    const colors = new Float32Array(particleCount * 3);
    const c = flow.direction === 'inflow' ? new THREE.Color(PALETTE.emerald) : new THREE.Color(PALETTE.red);
    for (let i = 0; i < particleCount; i++) {
      colors[i * 3] = c.r; colors[i * 3 + 1] = c.g; colors[i * 3 + 2] = c.b;
    }
    geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));

    const points_ = new THREE.Points(geo, mat);
    scene.add(points_);

    // glow tube along path
    const tubeGeo = new THREE.TubeGeometry(curve, 32, 0.03 + flow.volume * 0.0001, 6, false);
    const tubeMat = new THREE.MeshBasicMaterial({
      color: flow.direction === 'inflow' ? PALETTE.emerald : PALETTE.red,
      transparent: true, opacity: 0.15 + Math.min(flow.volume * 0.0005, 0.3),
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    scene.add(new THREE.Mesh(tubeGeo, tubeMat));
  });

  // animate
  let frame: number;
  const startTime = performance.now();
  const uniforms: THREE.IUniform[] = [];
  scene.traverse(obj => {
    if (obj instanceof THREE.Points && obj.material instanceof THREE.ShaderMaterial) {
      uniforms.push(obj.material.uniforms.time);
    }
  });

  function animate() {
    frame = requestAnimationFrame(animate);
    const t = (performance.now() - startTime) / 1000;
    camera.position.x = Math.sin(t * 0.04) * 50;
    camera.position.z = Math.cos(t * 0.04) * 50;
    camera.position.y = 20 + Math.sin(t * 0.02) * 5;
    camera.lookAt(0, 0, 0);

    // update GPU shader time
    uniforms.forEach(u => { u.value = t; });

    // pulse nodes
    nodeMap.forEach(mesh => {
      const pulse = 1 + Math.sin(t * 1.5 + mesh.position.x) * 0.05;
      mesh.scale.setScalar(pulse);
    });

    renderer.render(scene, camera);
  }
  animate();
  const removeResize = autoResize(container, camera, renderer);

  return { destroy() { cancelAnimationFrame(frame); removeResize(); disposeScene(scene); renderer.dispose(); container.removeChild(renderer.domElement); } };
}
