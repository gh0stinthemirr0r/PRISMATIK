/**
 * 3D Volatility Mountain — a volatility surface as a luminous crystalline terrain.
 *
 * The renderer is axis-agnostic: it plots height over an (x, z) grid. PRISMATIK
 * feeds it a *realized* volatility surface (lookback × horizon) because no
 * connected provider supplies an options chain, so there is no implied vol to
 * plot. The field names are deliberately generic to keep that honest.
 * Wireframe overlay, contour glow rings, event pillars with pulse beacons.
 */
import * as THREE from 'three';
import { PALETTE, createScene, createCamera, createRenderer, addDramaticLighting, orbitCamera, autoResize, createGlow, createParticleField, disposeScene } from './effects';

/** One surface sample: `height` plotted over the (`x`, `z`) grid. */
export interface SurfacePoint { x: number; z: number; height: number; }
export interface VolMountainConfig {
  surface: SurfacePoint[];
  eventMarkers?: { x: number; z: number; label: string }[];
}

export function createVolMountain(container: HTMLElement, config: VolMountainConfig) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene();
  const camera = createCamera(50, W / H, [35, 28, 35], [0, 3, 0]);
  const renderer = createRenderer(W, H);
  container.appendChild(renderer.domElement);
  addDramaticLighting(scene);

  // ambient particles
  scene.add(createParticleField(1000, 100, PALETTE.violet, 0.1, 0.2));

  const xs = [...new Set(config.surface.map(p => p.x))].sort((a, b) => a - b);
  const zs = [...new Set(config.surface.map(p => p.z))].sort((a, b) => a - b);
  const wSegs = xs.length - 1, dSegs = zs.length - 1;

  const getHeight = (s: number, e: number) => {
    const p = config.surface.find(pt => pt.x === s && pt.z === e);
    return p ? p.height * 50 : 0;
  };

  // main surface
  const geo = new THREE.PlaneGeometry(32, 32, wSegs, dSegs);
  const pos = geo.attributes.position;
  const colors = new Float32Array(pos.count * 3);
  let maxH = 0;

  for (let i = 0; i <= dSegs; i++) {
    for (let j = 0; j <= wSegs; j++) {
      const idx = i * (wSegs + 1) + j;
      const s = xs[j] ?? xs[xs.length - 1];
      const e = zs[i] ?? zs[zs.length - 1];
      const h = getHeight(s, e);
      maxH = Math.max(maxH, h);
      pos.setXYZ(idx, (j / wSegs - 0.5) * 32, h, (i / dSegs - 0.5) * 32);
    }
  }
  geo.computeVertexNormals();

  for (let i = 0; i < pos.count; i++) {
    const h = pos.getY(i);
    const t = Math.min(h / (maxH || 1), 1);
    // deep blue → cyan → amber → hot red
    colors[i * 3]     = t < 0.33 ? t * 3 : t < 0.66 ? 1 : 1;
    colors[i * 3 + 1] = t < 0.33 ? 0.2 + t * 2 : t < 0.66 ? 0.8 + (t - 0.33) * 2 : 1 - (t - 0.66) * 3;
    colors[i * 3 + 2] = t < 0.33 ? 1 : t < 0.66 ? 1 - (t - 0.33) * 4 : 0;
  }
  geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));

  const mat = new THREE.MeshPhongMaterial({
    vertexColors: true, transparent: true, opacity: 0.92,
    side: THREE.DoubleSide, shininess: 100, specular: new THREE.Color(0x444444),
  });
  scene.add(new THREE.Mesh(geo, mat));

  // wireframe glow overlay
  const wireMat = new THREE.MeshBasicMaterial({
    color: PALETTE.cyan, wireframe: true, transparent: true, opacity: 0.06,
    blending: THREE.AdditiveBlending, depthWrite: false,
  });
  scene.add(new THREE.Mesh(geo.clone(), wireMat));

  // event pillars — glowing beacon columns
  config.eventMarkers?.forEach(m => {
    const si = xs.indexOf(m.x), ei = zs.indexOf(m.z);
    if (si < 0 || ei < 0) return;
    const x = (si / (xs.length - 1) - 0.5) * 32;
    const z = (ei / (zs.length - 1) - 0.5) * 32;
    const h = getHeight(m.x, m.z);

    // pillar beam
    const beamGeo = new THREE.CylinderGeometry(0.12, 0.12, 30, 8);
    const beamMat = new THREE.MeshBasicMaterial({
      color: PALETTE.amber, transparent: true, opacity: 0.35,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    const beam = new THREE.Mesh(beamGeo, beamMat);
    beam.position.set(x, 15, z);
    scene.add(beam);

    // beacon orb
    const orb = createGlowSphere(0.35, PALETTE.amber, 0.8);
    orb.position.set(x, h + 2, z);
    scene.add(orb);
    orb.add(createGlow(PALETTE.amber, 4, 0.3));
  });

  // animate
  let frame: number;
  const startTime = performance.now();
  function animate() {
    frame = requestAnimationFrame(animate);
    orbitCamera(camera, startTime, 40, 0.06, 12);
    renderer.render(scene, camera);
  }
  animate();
  const removeResize = autoResize(container, camera, renderer);

  return {
    destroy() { cancelAnimationFrame(frame); removeResize(); disposeScene(scene); renderer.dispose(); container.removeChild(renderer.domElement); },
  };
}

function createGlowSphere(r: number, c: number, ei: number) {
  const geo = new THREE.SphereGeometry(r, 16, 16);
  const mat = new THREE.MeshPhongMaterial({ color: c, emissive: c, emissiveIntensity: ei, transparent: true, opacity: 0.9 });
  return new THREE.Mesh(geo, mat);
}
