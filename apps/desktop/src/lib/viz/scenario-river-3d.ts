/**
 * 3D Scenario River — Monte Carlo paths as luminous branching flows.
 * Volumetric tube glow, particle spray at branch points, depth fog.
 */
import * as THREE from 'three';
import { PALETTE, createGlowTube, createScene, createCamera, createRenderer, orbitCamera, autoResize, createParticleField, disposeScene } from './effects';

export interface SimPath { id: string; points: { t: number; value: number; probability: number }[]; regime: string; }
export interface RiverConfig { paths: SimPath[]; realizedPath?: { t: number; value: number }[]; }

const REGIME_COLORS: Record<string, number> = {
  calm: PALETTE.emerald, volatile: PALETTE.amber, crisis: PALETTE.red, recovery: PALETTE.cyan,
};

export function createScenarioRiver(container: HTMLElement, config: RiverConfig) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene();
  const camera = createCamera(50, W / H, [40, 22, 40], [0, 0, 0]);
  const renderer = createRenderer(W, H);
  container.appendChild(renderer.domElement);

  // ambient particles — probability dust
  scene.add(createParticleField(2000, 80, PALETTE.cyan, 0.08, 0.15));

  // grid floor
  const grid = new THREE.GridHelper(60, 30, PALETTE.surface2, PALETTE.surface1);
  (grid.material as THREE.LineBasicMaterial).transparent = true;
  (grid.material as THREE.LineBasicMaterial).opacity = 0.2;
  grid.position.y = -6;
  scene.add(grid);

  // simulation paths — glowing tubes
  config.paths.forEach(path => {
    const color = REGIME_COLORS[path.regime] ?? PALETTE.cyan;
    const points = path.points.map(p =>
      new THREE.Vector3((p.t - 0.5) * 40, p.value * 12, (p.probability - 0.5) * 25)
    );
    if (points.length < 2) return;
    const curve = new THREE.CatmullRomCurve3(points);
    const group = createGlowTube(curve, 0.06 + path.points[0].probability * 0.2, color, 0.2 + path.points[0].probability * 0.4, 0.3 + path.points[0].probability * 0.5);
    scene.add(group);
  });

  // realized path — bright white core with cyan glow
  if (config.realizedPath && config.realizedPath.length > 1) {
    const points = config.realizedPath.map(p => new THREE.Vector3((p.t - 0.5) * 40, p.value * 12, 0));
    const curve = new THREE.CatmullRomCurve3(points);
    const group = createGlowTube(curve, 0.18, PALETTE.white, 0.9, 0.6);
    scene.add(group);

    // particle spray along realized path
    const sprayGeo = new THREE.BufferGeometry();
    const sprayCount = 500;
    const sprayPos = new Float32Array(sprayCount * 3);
    for (let i = 0; i < sprayCount; i++) {
      const t = Math.random();
      const pt = curve.getPoint(t);
      sprayPos[i * 3]     = pt.x + (Math.random() - 0.5) * 2;
      sprayPos[i * 3 + 1] = pt.y + (Math.random() - 0.5) * 2;
      sprayPos[i * 3 + 2] = pt.z + (Math.random() - 0.5) * 2;
    }
    sprayGeo.setAttribute('position', new THREE.BufferAttribute(sprayPos, 3));
    const sprayMat = new THREE.PointsMaterial({
      color: PALETTE.cyan, size: 0.06, transparent: true, opacity: 0.3,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    scene.add(new THREE.Points(sprayGeo, sprayMat));
  }

  let frame: number;
  const startTime = performance.now();
  function animate() {
    frame = requestAnimationFrame(animate);
    orbitCamera(camera, startTime, 48, 0.05, 10);
    renderer.render(scene, camera);
  }
  animate();
  const removeResize = autoResize(container, camera, renderer);

  return { destroy() { cancelAnimationFrame(frame); removeResize(); disposeScene(scene); renderer.dispose(); container.removeChild(renderer.domElement); } };
}
