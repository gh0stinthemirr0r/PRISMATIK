/**
 * 3D Global Contagion Globe — atmospheric sphere with region beacons
 * and glowing contagion arcs. Atmosphere rim, pulse waves, stress heat.
 */
import * as THREE from 'three';
import { PALETTE, createGlow, createGlowSphere, createScene, createCamera, createRenderer, autoResize, disposeScene } from './effects';

export interface RegionNode { id: string; name: string; lat: number; lng: number; stress: number; sector: string; }
export interface ContagionArc { from: string; to: string; strength: number; speed: number; }
export interface ContagionConfig { regions: RegionNode[]; arcs: ContagionArc[]; }

function latLngToVec3(lat: number, lng: number, r: number): THREE.Vector3 {
  const phi = (90 - lat) * Math.PI / 180;
  const theta = (lng + 180) * Math.PI / 180;
  return new THREE.Vector3(-r * Math.sin(phi) * Math.cos(theta), r * Math.cos(phi), r * Math.sin(phi) * Math.sin(theta));
}

export function createContagionMap(container: HTMLElement, config: ContagionConfig) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene(0x020208);
  const camera = createCamera(45, W / H, [0, 0, 48]);
  const renderer = createRenderer(W, H, 0x020208);
  container.appendChild(renderer.domElement);

  const R = 16;

  // globe core
  const globeGeo = new THREE.SphereGeometry(R, 64, 64);
  const globeMat = new THREE.MeshPhongMaterial({
    color: 0x0a1628, emissive: 0x040810, shininess: 15,
    transparent: true, opacity: 0.97,
  });
  const globe = new THREE.Mesh(globeGeo, globeMat);
  scene.add(globe);

  // wireframe grid
  const wireGeo = new THREE.SphereGeometry(R + 0.06, 36, 36);
  const wireMat = new THREE.MeshBasicMaterial({
    color: PALETTE.cyan, wireframe: true, transparent: true, opacity: 0.06,
    blending: THREE.AdditiveBlending, depthWrite: false,
  });
  scene.add(new THREE.Mesh(wireGeo, wireMat));

  // atmosphere rim glow
  const atmosGeo = new THREE.SphereGeometry(R + 1.5, 48, 48);
  const atmosMat = new THREE.MeshBasicMaterial({
    color: PALETTE.cyan, transparent: true, opacity: 0.04,
    blending: THREE.AdditiveBlending, depthWrite: false, side: THREE.BackSide,
  });
  scene.add(new THREE.Mesh(atmosGeo, atmosMat));

  // region beacons
  const nodeMap = new Map<string, THREE.Mesh>();
  config.regions.forEach(region => {
    const pos = latLngToVec3(region.lat, region.lng, R + 0.3);
    const size = 0.2 + region.stress * 0.4;
    const color = region.stress > 0.7 ? PALETTE.red : region.stress > 0.4 ? PALETTE.amber : PALETTE.emerald;
    const beacon = createGlowSphere(size, color, 0.5 + region.stress * 0.4, 12);
    beacon.position.copy(pos);
    scene.add(beacon);
    nodeMap.set(region.id, beacon);

    // stress glow
    beacon.add(createGlow(color, size * 6, region.stress * 0.25));

    // stress pulse ring for high-stress regions
    if (region.stress > 0.6) {
      const ringGeo = new THREE.RingGeometry(size * 1.5, size * 2, 32);
      const ringMat = new THREE.MeshBasicMaterial({
        color, transparent: true, opacity: 0.3,
        side: THREE.DoubleSide, blending: THREE.AdditiveBlending, depthWrite: false,
      });
      const ring = new THREE.Mesh(ringGeo, ringMat);
      ring.lookAt(0, 0, 0);
      beacon.add(ring);
    }
  });

  // contagion arcs
  config.arcs.forEach(arc => {
    const fromNode = nodeMap.get(arc.from);
    const toNode = nodeMap.get(arc.to);
    if (!fromNode || !toNode) return;
    const start = fromNode.position;
    const end = toNode.position;
    const mid = start.clone().add(end).multiplyScalar(0.5);
    mid.normalize().multiplyScalar(R + 3 + arc.strength * 5);
    const curve = new THREE.QuadraticBezierCurve3(start, mid, end);
    const tubeGeo = new THREE.TubeGeometry(curve, 48, 0.02 + arc.strength * 0.1, 6, false);
    const color = arc.strength > 0.7 ? PALETTE.red : arc.strength > 0.4 ? PALETTE.amber : PALETTE.cyan;
    const tubeMat = new THREE.MeshBasicMaterial({
      color, transparent: true, opacity: 0.25 + arc.strength * 0.45,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    scene.add(new THREE.Mesh(tubeGeo, tubeMat));

    // glow tube
    const glowGeo = new THREE.TubeGeometry(curve, 48, 0.08 + arc.strength * 0.2, 6, false);
    const glowMat = new THREE.MeshBasicMaterial({
      color, transparent: true, opacity: 0.08 + arc.strength * 0.12,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    scene.add(new THREE.Mesh(glowGeo, glowMat));
  });

  // lighting
  scene.add(new THREE.AmbientLight(0x1a1a2e, 0.3));
  const sun = new THREE.DirectionalLight(0xffffff, 0.5);
  sun.position.set(10, 5, 10);
  scene.add(sun);
  scene.add(new THREE.PointLight(PALETTE.cyan, 0.6, 80));
  scene.add(new THREE.PointLight(PALETTE.violet, 0.3, 60));

  let frame: number;
  const startTime = performance.now();
  function animate() {
    frame = requestAnimationFrame(animate);
    const t = (performance.now() - startTime) / 1000;
    globe.rotation.y = t * 0.015;
    camera.lookAt(0, 0, 0);
    camera.position.x = Math.sin(t * 0.03) * 48;
    camera.position.z = Math.cos(t * 0.03) * 48;
    camera.position.y = Math.sin(t * 0.02) * 8;
    renderer.render(scene, camera);
  }
  animate();
  const removeResize = autoResize(container, camera, renderer);

  return { destroy() { cancelAnimationFrame(frame); removeResize(); disposeScene(scene); renderer.dispose(); container.removeChild(renderer.domElement); } };
}
