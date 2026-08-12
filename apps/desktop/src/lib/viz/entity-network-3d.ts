/**
 * 3D News Entity Co-occurrence Network — entities that appear together
 * in news stories form a 3D graph. Edge thickness = co-occurrence frequency.
 * Node size = mention volume. Color = aggregate sentiment.
 * Predictive: clusters that form often precede correlated price moves.
 *
 * Pulls real feed data from Tauri backend.
 */
import * as THREE from 'three';
import { PALETTE, createScene, createCamera, createRenderer, addDramaticLighting, orbitCamera, autoResize, disposeScene, createGlow, createGlowSphere, createParticleField } from './effects';

export interface EntityMention {
  entity: string;
  count: number;
  sentiment: number;
  novelty: number;
  sources: string[];
}

export interface CoOccurrence {
  entityA: string;
  entityB: string;
  count: number;
  avgSentiment: number;
  predictiveStrength: number;  // how often co-occurrence preceded correlated moves
}

export interface EntityNetworkConfig {
  entities: EntityMention[];
  coOccurrences: CoOccurrence[];
}

export function createEntityNetwork(container: HTMLElement, config: EntityNetworkConfig) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene(0x020208);
  const camera = createCamera(55, W / H, [0, 20, 50], [0, 0, 0]);
  const renderer = createRenderer(W, H, 0x020208);
  container.appendChild(renderer.domElement);
  addDramaticLighting(scene);

  // background stars
  scene.add(createParticleField(3000, 200, 0x222244, 0.08, 0.3));
  scene.add(createParticleField(500, 100, PALETTE.cyan, 0.04, 0.08));

  // nebula clouds
  [
    { pos: [-20, 10, -25] as [number,number,number], color: PALETTE.cyan, size: 18, opacity: 0.03 },
    { pos: [15, -5, -20] as [number,number,number], color: PALETTE.violet, size: 14, opacity: 0.025 },
  ].forEach(n => {
    const geo = new THREE.SphereGeometry(n.size, 16, 16);
    const mat = new THREE.MeshBasicMaterial({
      color: n.color, transparent: true, opacity: n.opacity,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    const mesh = new THREE.Mesh(geo, mat);
    mesh.position.set(...n.pos);
    scene.add(mesh);
  });

  // position entities in 3D — cluster by sentiment
  const entityPositions = new Map<string, THREE.Vector3>();
  const entityMeshes = new Map<string, THREE.Mesh>();

  config.entities.forEach((entity, i) => {
    const angle = (i / config.entities.length) * Math.PI * 2;
    const radius = 8 + entity.count * 0.3;
    const x = Math.cos(angle) * radius + (Math.random() - 0.5) * 4;
    const y = entity.sentiment * 12 + (Math.random() - 0.5) * 3;
    const z = Math.sin(angle) * radius + (Math.random() - 0.5) * 4;

    const pos = new THREE.Vector3(x, y, z);
    entityPositions.set(entity.entity, pos);

    const size = 0.3 + entity.count * 0.05;
    const color = entity.sentiment > 0.2 ? PALETTE.emerald :
                  entity.sentiment < -0.2 ? PALETTE.red :
                  entity.novelty > 0.5 ? PALETTE.amber : PALETTE.cyan;

    const sphere = createGlowSphere(size, color, 0.3 + entity.novelty * 0.4, 16);
    sphere.position.copy(pos);
    scene.add(sphere);
    entityMeshes.set(entity.entity, sphere);

    // glow halo
    sphere.add(createGlow(color, size * 6, 0.15 + entity.novelty * 0.15));

    // label
    const canvas = document.createElement('canvas');
    canvas.width = 128; canvas.height = 32;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#ffffff';
    ctx.font = 'bold 16px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(entity.entity, 64, 22);
    const tex = new THREE.CanvasTexture(canvas);
    const label = new THREE.Sprite(new THREE.SpriteMaterial({ map: tex, transparent: true, opacity: 0.6 }));
    label.scale.set(5, 1.2, 1);
    label.position.y = size + 1.5;
    sphere.add(label);
  });

  // co-occurrence edges — glowing tubes
  config.coOccurrences.forEach(co => {
    const posA = entityPositions.get(co.entityA);
    const posB = entityPositions.get(co.entityB);
    if (!posA || !posB) return;

    const mid = posA.clone().add(posB).multiplyScalar(0.5);
    mid.y += 2 + co.predictiveStrength * 3;
    const curve = new THREE.QuadraticBezierCurve3(posA, mid, posB);

    const color = co.avgSentiment > 0.1 ? PALETTE.emerald :
                  co.avgSentiment < -0.1 ? PALETTE.red : PALETTE.cyan;

    // tube
    const tubeGeo = new THREE.TubeGeometry(curve, 32, 0.02 + co.count * 0.01, 6, false);
    const tubeMat = new THREE.MeshBasicMaterial({
      color, transparent: true, opacity: 0.15 + co.predictiveStrength * 0.4,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    scene.add(new THREE.Mesh(tubeGeo, tubeMat));

    // outer glow tube
    const glowGeo = new THREE.TubeGeometry(curve, 32, 0.06 + co.count * 0.02, 6, false);
    const glowMat = new THREE.MeshBasicMaterial({
      color, transparent: true, opacity: 0.05 + co.predictiveStrength * 0.08,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    scene.add(new THREE.Mesh(glowGeo, glowMat));

    // predictive strength indicator — particle at midpoint
    if (co.predictiveStrength > 0.5) {
      const beacon = createGlowSphere(0.15, PALETTE.amber, 0.6);
      beacon.position.copy(mid);
      scene.add(beacon);
      beacon.add(createGlow(PALETTE.amber, 2, 0.2));
    }
  });

  // animate
  let frame: number;
  const startTime = performance.now();
  function animate() {
    frame = requestAnimationFrame(animate);
    const t = (performance.now() - startTime) / 1000;
    orbitCamera(camera, startTime, 50, 0.025, 10);

    // pulse entities
    entityMeshes.forEach((mesh, entity) => {
      const e = config.entities.find(en => en.entity === entity);
      if (!e) return;
      mesh.scale.setScalar(1 + Math.sin(t * 1.2 + mesh.position.x) * 0.05 * e.novelty);
    });

    renderer.render(scene, camera);
  }
  animate();
  const removeResize = autoResize(container, camera, renderer);

  return { destroy() { cancelAnimationFrame(frame); removeResize(); disposeScene(scene); renderer.dispose(); container.removeChild(renderer.domElement); } };
}
