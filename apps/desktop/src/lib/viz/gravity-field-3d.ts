/**
 * 3D Market Gravity Field — assets as luminous nodes in a force-directed nebula.
 * Bloom trails, particle halos, correlation filaments, regime-colored aurora.
 */
import * as THREE from 'three';
import { PALETTE, REGIME_COLORS, createGlowSphere, createGlow, createGlowTube, createParticleField, createScene, createCamera, createRenderer, addDramaticLighting, orbitCamera, autoResize, disposeScene } from './effects';

export interface AssetNode {
  symbol: string; name: string; price: number; changePct: number;
  volume: number; correlation: Record<string, number>;
  regime: string; narrativeMomentum: number; sector: string;
}

const SECTOR_POS: Record<string, [number, number, number]> = {
  tech:     [0, 0, 0],
  crypto:   [18, 5, -8],
  energy:   [-14, -3, 12],
  finance:  [8, -6, 16],
  intl:     [-10, 8, -14],
};

/** Three stable offsets in [-0.5, 0.5], derived from the symbol alone. */
function symbolJitter(symbol: string): [number, number, number] {
  let h = 2166136261;
  for (let i = 0; i < symbol.length; i++) {
    h ^= symbol.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  const next = () => {
    h ^= h << 13;
    h ^= h >>> 17;
    h ^= h << 5;
    return ((h >>> 0) % 10000) / 10000 - 0.5;
  };
  return [next(), next(), next()];
}

export function createGravityField(container: HTMLElement, config: { nodes: AssetNode[]; onNodeClick?: (s: string) => void }) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene();
  const camera = createCamera(55, W / H, [0, 25, 55], [0, 0, 0]);
  const renderer = createRenderer(W, H);
  container.appendChild(renderer.domElement);
  addDramaticLighting(scene);

  // background particles — deep space dust
  scene.add(createParticleField(3000, 200, PALETTE.cyan, 0.12, 0.3));
  scene.add(createParticleField(1500, 150, PALETTE.violet, 0.08, 0.2));

  // grid floor — faint
  const grid = new THREE.GridHelper(80, 40, PALETTE.surface2, PALETTE.surface1);
  (grid.material as THREE.LineBasicMaterial).transparent = true;
  (grid.material as THREE.LineBasicMaterial).opacity = 0.2;
  grid.position.y = -12;
  scene.add(grid);

  // nodes
  const nodeMeshes = new Map<string, THREE.Mesh>();
  const nodeData = new Map<string, AssetNode>();

  config.nodes.forEach(node => {
    nodeData.set(node.symbol, node);
    const sectorBase = SECTOR_POS[node.sector] ?? [0, 0, 0];
    // Offsets are seeded from the symbol so a given universe always lays out
    // identically. A field that reshuffles on every re-render cannot be read
    // for structure, which is the entire point of the view.
    const jitter = symbolJitter(node.symbol);
    const x = sectorBase[0] + jitter[0] * 14;
    const y = sectorBase[1] + jitter[1] * 10;
    const z = sectorBase[2] + jitter[2] * 14;

    const size = 0.25 + Math.min(node.volume / 5e7, 2.5);
    const color = REGIME_COLORS[node.regime] ?? PALETTE.cyan;
    const sphere = createGlowSphere(size, color, 0.3 + node.narrativeMomentum * 0.5);
    sphere.position.set(x, y, z);
    sphere.userData = { symbol: node.symbol };
    scene.add(sphere);
    nodeMeshes.set(node.symbol, sphere);

    // narrative momentum glow halo
    if (node.narrativeMomentum > 0.3) {
      sphere.add(createGlow(color, size * 5, node.narrativeMomentum * 0.2));
    }

    // price change flash ring
    if (Math.abs(node.changePct) > 1.5) {
      const ringGeo = new THREE.RingGeometry(size * 1.3, size * 1.6, 32);
      const ringMat = new THREE.MeshBasicMaterial({
        color: node.changePct > 0 ? PALETTE.emerald : PALETTE.red,
        transparent: true, opacity: 0.4,
        side: THREE.DoubleSide, blending: THREE.AdditiveBlending, depthWrite: false,
      });
      const ring = new THREE.Mesh(ringGeo, ringMat);
      ring.lookAt(camera.position);
      sphere.add(ring);
    }
  });

  // correlation filaments — glowing tubes
  config.nodes.forEach(node => {
    const meshA = nodeMeshes.get(node.symbol);
    if (!meshA) return;
    Object.entries(node.correlation).forEach(([other, corr]) => {
      const meshB = nodeMeshes.get(other);
      if (!meshB || node.symbol >= other) return;
      const abs = Math.abs(corr);
      if (abs < 0.25) return;
      const mid = meshA.position.clone().add(meshB.position).multiplyScalar(0.5);
      mid.y += 3 + abs * 4;
      const curve = new THREE.QuadraticBezierCurve3(meshA.position, mid, meshB.position);
      const color = corr > 0 ? PALETTE.cyan : PALETTE.rose;
      const tube = createGlowTube(curve, 0.015 + abs * 0.06, color, abs * 0.4, 0.08 + abs * 0.15);
      scene.add(tube);
    });
  });

  // sector labels as glowing points
  Object.entries(SECTOR_POS).forEach(([sector, pos]) => {
    const glow = createGlow(PALETTE.textDim, 0.5, 0.1);
    glow.position.set(...pos);
    scene.add(glow);
  });

  // animate
  let frame: number;
  const startTime = performance.now();

  function animate() {
    frame = requestAnimationFrame(animate);
    const t = (performance.now() - startTime) / 1000;
    orbitCamera(camera, startTime, 55, 0.04, 15);

    nodeMeshes.forEach((mesh, sym) => {
      const node = nodeData.get(sym);
      if (!node) return;
      // breathing pulse
      const pulse = 1 + Math.sin(t * 1.5 + mesh.position.x * 0.5) * 0.06 * node.narrativeMomentum;
      mesh.scale.setScalar(pulse);
      // gentle vertical drift
      mesh.position.y += Math.sin(t * 0.6 + mesh.position.z * 0.3) * 0.003;
    });

    renderer.render(scene, camera);
  }
  animate();

  const removeResize = autoResize(container, camera, renderer);

  // click
  const onClick = (e: MouseEvent) => {
    const rect = container.getBoundingClientRect();
    const mouse = new THREE.Vector2(
      ((e.clientX - rect.left) / rect.width) * 2 - 1,
      -((e.clientY - rect.top) / rect.height) * 2 + 1,
    );
    const raycaster = new THREE.Raycaster();
    raycaster.setFromCamera(mouse, camera);
    const hits = raycaster.intersectObjects(Array.from(nodeMeshes.values()));
    if (hits.length > 0 && hits[0].object.userData.symbol && config.onNodeClick) {
      config.onNodeClick(hits[0].object.userData.symbol);
    }
  };
  container.addEventListener('click', onClick);

  return {
    destroy() {
      cancelAnimationFrame(frame);
      removeResize();
      container.removeEventListener('click', onClick);
      disposeScene(scene);
      renderer.dispose();
      container.removeChild(renderer.domElement);
    },
  };
}
