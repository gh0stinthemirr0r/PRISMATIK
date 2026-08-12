/**
 * 3D Sector Rotation Wheel — sectors as colored arcs on a rotating ring.
 * Performance = height, momentum = glow intensity. GPU-animated rotation.
 * Inner ring = current allocation, outer ring = target. Spokes = flows.
 */
import * as THREE from 'three';
import { PALETTE, createScene, createCamera, createRenderer, addDramaticLighting, orbitCamera, autoResize, disposeScene, createParticleField, createGlow } from './effects';

export interface SectorData {
  name: string;
  performance: number;   // -1 to 1
  momentum: number;       // 0-1
  weight: number;         // 0-1
  change: number;         // -1 to 1
  color: string;
}

export interface SectorRotationConfig {
  sectors: SectorData[];
}

export function createSectorRotation(container: HTMLElement, config: SectorRotationConfig) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene();
  const camera = createCamera(50, W / H, [0, 25, 45], [0, 0, 0]);
  const renderer = createRenderer(W, H);
  container.appendChild(renderer.domElement);
  addDramaticLighting(scene);
  scene.add(createParticleField(1000, 80, PALETTE.cyan, 0.06, 0.12));

  const { sectors } = config;
  const n = sectors.length;
  const innerR = 10, outerR = 16;

  // center hub
  const hubGeo = new THREE.CylinderGeometry(3, 3, 0.5, 32);
  const hubMat = new THREE.MeshPhongMaterial({
    color: PALETTE.surface2, emissive: PALETTE.cyan, emissiveIntensity: 0.1,
    transparent: true, opacity: 0.8,
  });
  const hub = new THREE.Mesh(hubGeo, hubMat);
  scene.add(hub);

  // sector arcs
  sectors.forEach((sector, i) => {
    const angle = (i / n) * Math.PI * 2;
    const nextAngle = ((i + 1) / n) * Math.PI * 2;
    const midAngle = (angle + nextAngle) / 2;

    // inner arc (current allocation)
    const innerGeo = new THREE.RingGeometry(innerR, innerR + sector.weight * 4, 32, 1, angle, nextAngle - angle);
    const color = parseInt(sector.color.replace('#', ''), 16);
    const innerMat = new THREE.MeshPhongMaterial({
      color, emissive: color, emissiveIntensity: 0.2 + sector.momentum * 0.4,
      transparent: true, opacity: 0.7, side: THREE.DoubleSide,
    });
    const innerMesh = new THREE.Mesh(innerGeo, innerMat);
    innerMesh.rotation.x = -Math.PI / 2;
    scene.add(innerMesh);

    // performance pillar
    const pillarH = Math.abs(sector.performance) * 15;
    const pillarGeo = new THREE.CylinderGeometry(0.3, 0.3, pillarH, 8);
    const pillarMat = new THREE.MeshPhongMaterial({
      color, emissive: color, emissiveIntensity: 0.3,
      transparent: true, opacity: 0.6,
    });
    const pillar = new THREE.Mesh(pillarGeo, pillarMat);
    const pillarR = (innerR + outerR) / 2;
    pillar.position.set(
      Math.cos(midAngle) * pillarR,
      pillarH / 2,
      Math.sin(midAngle) * pillarR,
    );
    scene.add(pillar);

    // glow at top of pillar
    if (sector.momentum > 0.5) {
      const glow = createGlow(color, 2, sector.momentum * 0.3);
      glow.position.copy(pillar.position);
      glow.position.y = pillarH;
      scene.add(glow);
    }

    // change arrow (spoke)
    if (Math.abs(sector.change) > 0.1) {
      const arrowLen = sector.change * 5;
      const arrowGeo = new THREE.CylinderGeometry(0.05, 0.15, Math.abs(arrowLen), 6);
      const arrowMat = new THREE.MeshBasicMaterial({
        color: sector.change > 0 ? PALETTE.emerald : PALETTE.red,
        transparent: true, opacity: 0.5,
      });
      const arrow = new THREE.Mesh(arrowGeo, arrowMat);
      arrow.position.set(
        Math.cos(midAngle) * (outerR + 2),
        0,
        Math.sin(midAngle) * (outerR + 2),
      );
      arrow.rotation.z = sector.change > 0 ? 0 : Math.PI;
      scene.add(arrow);
    }

    // label
    const canvas = document.createElement('canvas');
    canvas.width = 256; canvas.height = 64;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#ffffff';
    ctx.font = '18px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(sector.name, 128, 30);
    ctx.font = '14px monospace';
    ctx.fillStyle = sector.performance > 0 ? '#34d399' : '#f87171';
    ctx.fillText(`${(sector.performance * 100).toFixed(1)}%`, 128, 52);
    const tex = new THREE.CanvasTexture(canvas);
    const spriteMat = new THREE.SpriteMaterial({ map: tex, transparent: true, opacity: 0.7 });
    const label = new THREE.Sprite(spriteMat);
    label.scale.set(6, 1.5, 1);
    label.position.set(Math.cos(midAngle) * (outerR + 5), 0, Math.sin(midAngle) * (outerR + 5));
    scene.add(label);
  });

  // animate
  let frame: number;
  const startTime = performance.now();
  function animate() {
    frame = requestAnimationFrame(animate);
    const t = (performance.now() - startTime) / 1000;
    hub.rotation.y = t * 0.1;
    orbitCamera(camera, startTime, 45, 0.03, 15);
    renderer.render(scene, camera);
  }
  animate();
  const removeResize = autoResize(container, camera, renderer);

  return { destroy() { cancelAnimationFrame(frame); removeResize(); disposeScene(scene); renderer.dispose(); container.removeChild(renderer.domElement); } };
}
