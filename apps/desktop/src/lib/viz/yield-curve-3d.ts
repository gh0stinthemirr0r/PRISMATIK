/**
 * 3D Yield Curve Surface — government bond yields across maturities and time.
 * X = maturity, Y = yield (height), Z = time. Animated surface morphing.
 * Color = steepness (green=normal, red=inverted).
 */
import * as THREE from 'three';
import { PALETTE, createScene, createCamera, createRenderer, addDramaticLighting, orbitCamera, autoResize, disposeScene, createParticleField } from './effects';

export interface YieldPoint { maturity: number; yield: number; date: number; }

export interface YieldCurveConfig {
  curves: YieldPoint[][];   // array of daily curves
  labels: string[];          // date labels
}

export function createYieldCurveSurface(container: HTMLElement, config: YieldCurveConfig) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene();
  const camera = createCamera(50, W / H, [35, 20, 35], [0, 3, 0]);
  const renderer = createRenderer(W, H);
  container.appendChild(renderer.domElement);
  addDramaticLighting(scene);
  scene.add(createParticleField(600, 80, PALETTE.amber, 0.05, 0.1));

  const { curves, labels } = config;
  const maturities = curves[0]?.map(p => p.maturity) ?? [];
  const nMaturities = maturities.length;
  const nDates = curves.length;

  if (nMaturities < 2 || nDates < 2) {
    container.innerHTML = '<div style="color:#6b7280;padding:20px;font-family:monospace">Insufficient yield curve data</div>';
    return { destroy() {} };
  }

  // build surface
  const geo = new THREE.PlaneGeometry(30, 20, nMaturities - 1, nDates - 1);
  const pos = geo.attributes.position;
  const colors = new Float32Array(pos.count * 3);

  function updateSurface(curveIdx: number) {
    const curve = curves[Math.min(curveIdx, curves.length - 1)];
    const maxYield = Math.max(...curves.flat().map(p => p.yield));

    for (let j = 0; j < nDates; j++) {
      for (let i = 0; i < nMaturities; i++) {
        const idx = j * nMaturities + i;
        const ci = Math.min(j, curves.length - 1);
        const yieldVal = curves[ci]?.[i]?.yield ?? 0;
        const x = (i / (nMaturities - 1)) * 30 - 15;
        const y = (yieldVal / maxYield) * 15;
        const z = (j / (nDates - 1)) * 20 - 10;
        pos.setXYZ(idx, x, y, z);

        // color by steepness (yield diff between long and short)
        const shortYield = curves[ci]?.[0]?.yield ?? 0;
        const longYield = curves[ci]?.[nMaturities - 1]?.yield ?? 0;
        const steepness = longYield - shortYield;
        const t = (steepness + 2) / 4; // normalize -2..2 to 0..1
        colors[idx * 3] = t < 0.5 ? 1 : 1 - (t - 0.5) * 2;
        colors[idx * 3 + 1] = t < 0.5 ? t * 2 : 1 - (t - 0.5) * 2;
        colors[idx * 3 + 2] = t < 0.5 ? 1 - t * 2 : 0;
      }
    }
    pos.needsUpdate = true;
    geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    geo.computeVertexNormals();
  }

  updateSurface(0);

  const mat = new THREE.MeshPhongMaterial({
    vertexColors: true, transparent: true, opacity: 0.85,
    side: THREE.DoubleSide, shininess: 60,
  });
  const mesh = new THREE.Mesh(geo, mat);
  scene.add(mesh);

  // wireframe overlay
  const wireMat = new THREE.MeshBasicMaterial({
    color: PALETTE.cyan, wireframe: true, transparent: true, opacity: 0.05,
    blending: THREE.AdditiveBlending, depthWrite: false,
  });
  scene.add(new THREE.Mesh(geo.clone(), wireMat));

  // axes
  const axisMat = new THREE.LineBasicMaterial({ color: 0x374151, transparent: true, opacity: 0.3 });
  [
    [new THREE.Vector3(-15, 0, 10), new THREE.Vector3(15, 0, 10)],
    [new THREE.Vector3(-15, 0, -10), new THREE.Vector3(-15, 0, 10)],
    [new THREE.Vector3(-15, 0, -10), new THREE.Vector3(-15, 15, -10)],
  ].forEach(pts => {
    scene.add(new THREE.Line(new THREE.BufferGeometry().setFromPoints(pts), axisMat));
  });

  let frame: number;
  const startTime = performance.now();
  let currentCurve = 0;

  function animate() {
    frame = requestAnimationFrame(animate);
    const t = (performance.now() - startTime) / 1000;

    // animate through time
    if (Math.floor(t * 2) % 3 === 0) {
      const newCurve = Math.floor(t * 0.5) % curves.length;
      if (newCurve !== currentCurve) {
        currentCurve = newCurve;
        updateSurface(currentCurve);
      }
    }

    orbitCamera(camera, startTime, 38, 0.04, 10);
    renderer.render(scene, camera);
  }
  animate();
  const removeResize = autoResize(container, camera, renderer);

  return { destroy() { cancelAnimationFrame(frame); removeResize(); disposeScene(scene); renderer.dispose(); container.removeChild(renderer.domElement); } };
}
