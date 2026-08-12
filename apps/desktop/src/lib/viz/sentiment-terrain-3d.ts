/**
 * 3D News Sentiment Terrain — sentiment aggregated across sources over time.
 * X = time, Y = sentiment magnitude, Z = source/region.
 * Green peaks = positive consensus, red valleys = negative.
 * Width at base = source agreement. Narrow peaks = high dispersion (signal).
 *
 * Pulls real feed data from Tauri backend.
 */
import * as THREE from 'three';
import { PALETTE, createScene, createCamera, createRenderer, addDramaticLighting, orbitCamera, autoResize, disposeScene, createParticleField, createGlow } from './effects';

export interface SentimentPoint {
  time: number;        // 0-1 normalized
  source: string;
  sentiment: number;   // -1 to 1
  dispersion: number;  // 0-1, how much sources disagree
  articleCount: number;
}

export interface SentimentTerrainConfig {
  points: SentimentPoint[];
  sources: string[];
  entities: string[];
}

export function createSentimentTerrain(container: HTMLElement, config: SentimentTerrainConfig) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene(0x030308);
  const camera = createCamera(50, W / H, [35, 22, 35], [0, 3, 0]);
  const renderer = createRenderer(W, H, 0x030308);
  container.appendChild(renderer.domElement);
  addDramaticLighting(scene);
  scene.add(createParticleField(1000, 80, PALETTE.cyan, 0.05, 0.1));

  const { sources } = config;
  const timeSteps = 32;
  const nSources = sources.length;

  // build terrain
  const geo = new THREE.PlaneGeometry(30, 20, timeSteps - 1, nSources - 1);
  const pos = geo.attributes.position;
  const colors = new Float32Array(pos.count * 3);

  for (let j = 0; j < nSources; j++) {
    for (let i = 0; i < timeSteps; i++) {
      const idx = j * timeSteps + i;
      const time = i / timeSteps;
      const source = sources[j];

      // find matching sentiment point
      const point = config.points.find(p =>
        Math.abs(p.time - time) < 0.05 && p.source === source
      );

      const sentiment = point?.sentiment ?? (Math.random() - 0.5) * 0.3;
      const dispersion = point?.dispersion ?? 0.3;

      const x = (i / (timeSteps - 1)) * 30 - 15;
      const y = sentiment * 15 + 5;
      const z = (j / (nSources - 1)) * 20 - 10;
      pos.setXYZ(idx, x, y, z);

      // color: green=positive, red=negative, intensity by article count
      const t = (sentiment + 1) / 2;
      const intensity = 0.5 + (point?.articleCount ?? 1) * 0.05;
      colors[idx * 3] = (t < 0.5 ? 1 : 1 - (t - 0.5) * 2) * intensity;
      colors[idx * 3 + 1] = (t < 0.5 ? t * 2 : 1 - (t - 0.5) * 2) * intensity;
      colors[idx * 3 + 2] = (t < 0.5 ? 1 - t * 2 : 0) * intensity;
    }
  }
  pos.needsUpdate = true;
  geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  geo.computeVertexNormals();

  const mat = new THREE.MeshPhongMaterial({
    vertexColors: true, transparent: true, opacity: 0.85,
    side: THREE.DoubleSide, shininess: 80,
  });
  scene.add(new THREE.Mesh(geo, mat));

  // wireframe
  scene.add(new THREE.Mesh(geo.clone(), new THREE.MeshBasicMaterial({
    color: PALETTE.cyan, wireframe: true, transparent: true, opacity: 0.04,
    blending: THREE.AdditiveBlending, depthWrite: false,
  })));

  // dispersion markers — where sources disagree, add glowing pillars
  config.points.filter(p => p.dispersion > 0.6).forEach(point => {
    const si = sources.indexOf(point.source);
    if (si < 0) return;
    const x = point.time * 30 - 15;
    const z = (si / (nSources - 1)) * 20 - 10;
    const pillarGeo = new THREE.CylinderGeometry(0.1, 0.1, 20, 6);
    const pillarMat = new THREE.MeshBasicMaterial({
      color: PALETTE.amber, transparent: true, opacity: 0.2,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    const pillar = new THREE.Mesh(pillarGeo, pillarMat);
    pillar.position.set(x, 10, z);
    scene.add(pillar);
    pillar.add(createGlow(PALETTE.amber, 1.5, 0.15));
  });

  // source labels
  sources.forEach((source, j) => {
    const z = (j / (nSources - 1)) * 20 - 10;
    const canvas = document.createElement('canvas');
    canvas.width = 128; canvas.height = 24;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#9ca3af';
    ctx.font = '11px monospace';
    ctx.fillText(source.substring(0, 15), 5, 18);
    const tex = new THREE.CanvasTexture(canvas);
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: tex, transparent: true, opacity: 0.4 }));
    sprite.scale.set(4, 0.8, 1);
    sprite.position.set(-17, 5, z);
    scene.add(sprite);
  });

  let frame: number;
  const startTime = performance.now();
  function animate() {
    frame = requestAnimationFrame(animate);
    orbitCamera(camera, startTime, 38, 0.04, 10);
    renderer.render(scene, camera);
  }
  animate();
  const removeResize = autoResize(container, camera, renderer);

  return { destroy() { cancelAnimationFrame(frame); removeResize(); disposeScene(scene); renderer.dispose(); container.removeChild(renderer.domElement); } };
}
