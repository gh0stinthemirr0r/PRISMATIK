/**
 * 3D Narrative Constellation — stories as luminous stars in a cosmic field.
 * Nebula clouds, connecting filaments, phase-colored pulsars, parallax starfield.
 */
import * as THREE from 'three';
import { PALETTE, createGlow, createGlowSphere, createScene, createCamera, createRenderer, orbitCamera, autoResize, disposeScene } from './effects';

export interface StoryStar {
  id: string; title: string; novelty: number; phase: string;
  entities: string[]; sources: number; age: number; sentiment: number;
}

const PHASE_COLORS: Record<string, number> = {
  emergence: 0x00f0ff, amplification: 0xfbbf24,
  saturation: 0xef4444, decay: 0x6b7280, dormancy: 0x374151,
};

export function createConstellation(container: HTMLElement, config: { stories: StoryStar[] }) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene(0x020208);
  const camera = createCamera(60, W / H, [0, 15, 55], [0, 5, 0]);
  const renderer = createRenderer(W, H, 0x020208);
  container.appendChild(renderer.domElement);

  // multi-layer parallax starfield
  [
    { count: 4000, spread: 300, size: 0.08, color: 0x333355, opacity: 0.5 },
    { count: 2000, spread: 200, size: 0.15, color: PALETTE.cyan, opacity: 0.15 },
    { count: 800,  spread: 150, size: 0.25, color: PALETTE.violet, opacity: 0.1 },
  ].forEach(layer => {
    const positions = new Float32Array(layer.count * 3);
    for (let i = 0; i < layer.count; i++) {
      positions[i * 3]     = (Math.random() - 0.5) * layer.spread;
      positions[i * 3 + 1] = (Math.random() - 0.5) * layer.spread;
      positions[i * 3 + 2] = (Math.random() - 0.5) * layer.spread;
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    const mat = new THREE.PointsMaterial({
      color: layer.color, size: layer.size,
      transparent: true, opacity: layer.opacity,
      blending: THREE.AdditiveBlending, depthWrite: false, sizeAttenuation: true,
    });
    scene.add(new THREE.Points(geo, mat));
  });

  // nebula glow clouds — soft additive spheres in the background
  [
    { pos: [-20, 10, -30] as [number,number,number], color: PALETTE.cyan, size: 20, opacity: 0.04 },
    { pos: [25, -5, -20] as [number,number,number], color: PALETTE.violet, size: 15, opacity: 0.03 },
    { pos: [0, 20, -40] as [number,number,number], color: PALETTE.emerald, size: 25, opacity: 0.025 },
  ].forEach(nebula => {
    const geo = new THREE.SphereGeometry(nebula.size, 16, 16);
    const mat = new THREE.MeshBasicMaterial({
      color: nebula.color, transparent: true, opacity: nebula.opacity,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    const mesh = new THREE.Mesh(geo, mat);
    mesh.position.set(...nebula.pos);
    scene.add(mesh);
  });

  // story stars
  const starMeshes = new Map<string, THREE.Mesh>();
  config.stories.forEach(story => {
    const x = story.sentiment * 22 + (Math.random() - 0.5) * 10;
    const y = story.novelty * 28 + 2;
    const z = -(story.age / 24) * 18 + (Math.random() - 0.5) * 6;

    const size = 0.2 + story.sources * 0.06;
    const color = PHASE_COLORS[story.phase] ?? PALETTE.cyan;
    const star = createGlowSphere(size, color, 0.4 + story.novelty * 0.4, 12);
    star.position.set(x, y, z);
    scene.add(star);
    starMeshes.set(story.id, star);

    // outer glow halo
    star.add(createGlow(color, size * 7, story.novelty * 0.2));

    // bright core for high-novelty
    if (story.novelty > 0.7) {
      const coreGeo = new THREE.SphereGeometry(size * 0.3, 8, 8);
      const coreMat = new THREE.MeshBasicMaterial({ color: PALETTE.white });
      star.add(new THREE.Mesh(coreGeo, coreMat));
    }
  });

  // connecting filaments between related stories
  for (let i = 0; i < config.stories.length; i++) {
    for (let j = i + 1; j < config.stories.length; j++) {
      const a = config.stories[i], b = config.stories[j];
      const shared = a.entities.filter(e => b.entities.includes(e));
      if (shared.length === 0) continue;
      const mA = starMeshes.get(a.id)!, mB = starMeshes.get(b.id)!;
      const mid = mA.position.clone().add(mB.position).multiplyScalar(0.5);
      mid.y += 2;
      const curve = new THREE.QuadraticBezierCurve3(mA.position, mid, mB.position);
      const tubeGeo = new THREE.TubeGeometry(curve, 32, 0.01 + shared.length * 0.015, 4, false);
      const tubeMat = new THREE.MeshBasicMaterial({
        color: PALETTE.cyan, transparent: true, opacity: shared.length * 0.08,
        blending: THREE.AdditiveBlending, depthWrite: false,
      });
      scene.add(new THREE.Mesh(tubeGeo, tubeMat));
    }
  }

  let frame: number;
  const startTime = performance.now();
  function animate() {
    frame = requestAnimationFrame(animate);
    orbitCamera(camera, startTime, 55, 0.025, 12);
    starMeshes.forEach((mesh, id) => {
      const story = config.stories.find(s => s.id === id);
      if (!story) return;
      mesh.scale.setScalar(1 + Math.sin((performance.now() - startTime) / 1000 * 1.2 + mesh.position.x) * 0.08 * story.novelty);
    });
    renderer.render(scene, camera);
  }
  animate();
  const removeResize = autoResize(container, camera, renderer);

  return { destroy() { cancelAnimationFrame(frame); removeResize(); disposeScene(scene); renderer.dispose(); container.removeChild(renderer.domElement); } };
}
