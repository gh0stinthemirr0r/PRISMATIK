/**
 * 3D Options Flow Visualization — options trades as 3D glyphs.
 * Calls = green pillars rising, puts = red pillars falling.
 * Size = volume, position = strike × expiry. Open interest as heat surface.
 */
import * as THREE from 'three';
import { PALETTE, createScene, createCamera, createRenderer, addDramaticLighting, orbitCamera, autoResize, disposeScene, createGlow, createParticleField } from './effects';

export interface OptionTrade {
  strike: number;
  expiry: number;  // days
  type: 'call' | 'put';
  volume: number;
  premium: number;
  iv: number;
}

export interface OptionsFlowConfig {
  trades: OptionTrade[];
  spotPrice: number;
}

export function createOptionsFlow(container: HTMLElement, config: OptionsFlowConfig) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene();
  const camera = createCamera(50, W / H, [30, 20, 30], [0, 0, 0]);
  const renderer = createRenderer(W, H);
  container.appendChild(renderer.domElement);
  addDramaticLighting(scene);
  scene.add(createParticleField(800, 80, PALETTE.violet, 0.05, 0.1));

  const strikes = [...new Set(config.trades.map(t => t.strike))].sort((a, b) => a - b);
  const expiries = [...new Set(config.trades.map(t => t.expiry))].sort((a, b) => a - b);
  const maxVol = Math.max(...config.trades.map(t => t.volume));

  // spot price plane
  const spotX = ((config.spotPrice - strikes[0]) / (strikes[strikes.length - 1] - strikes[0])) * 30 - 15;
  const spotPlane = new THREE.Mesh(
    new THREE.PlaneGeometry(0.1, 20),
    new THREE.MeshBasicMaterial({ color: PALETTE.white, transparent: true, opacity: 0.2, side: THREE.DoubleSide }),
  );
  spotPlane.position.set(spotX, 5, 0);
  spotPlane.rotation.y = Math.PI / 2;
  scene.add(spotPlane);

  config.trades.forEach(trade => {
    const si = strikes.indexOf(trade.strike);
    const ei = expiries.indexOf(trade.expiry);
    if (si < 0 || ei < 0) return;

    const x = (si / (strikes.length - 1)) * 30 - 15;
    const z = (ei / (expiries.length - 1)) * 20 - 10;
    const isCall = trade.type === 'call';
    const color = isCall ? PALETTE.emerald : PALETTE.red;
    const height = (trade.volume / maxVol) * 12 + 0.5;

    // pillar
    const geo = new THREE.CylinderGeometry(
      0.15 + (trade.volume / maxVol) * 0.5,
      0.15 + (trade.volume / maxVol) * 0.5,
      height, 8,
    );
    const mat = new THREE.MeshPhongMaterial({
      color, emissive: color, emissiveIntensity: 0.3 + (trade.volume / maxVol) * 0.4,
      transparent: true, opacity: 0.7,
    });
    const pillar = new THREE.Mesh(geo, mat);
    pillar.position.set(x, isCall ? height / 2 : -height / 2, z);
    scene.add(pillar);

    // glow at tip
    if (trade.volume / maxVol > 0.3) {
      const glow = createGlow(color, 2, 0.2);
      glow.position.copy(pillar.position);
      glow.position.y += isCall ? height / 2 : -height / 2;
      scene.add(glow);
    }
  });

  // axes
  const axisMat = new THREE.LineBasicMaterial({ color: 0x374151, transparent: true, opacity: 0.4 });
  // strike axis
  scene.add(new THREE.Line(
    new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(-15, 0, 12), new THREE.Vector3(15, 0, 12)]),
    axisMat,
  ));
  // expiry axis
  scene.add(new THREE.Line(
    new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(-15, 0, -10), new THREE.Vector3(-15, 0, 10)]),
    axisMat,
  ));

  // labels
  ['STRIKE', 'EXPIRY', 'SPOT'].forEach((label, i) => {
    const canvas = document.createElement('canvas');
    canvas.width = 128; canvas.height = 32;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#6b7280';
    ctx.font = '14px monospace';
    ctx.fillText(label, 10, 22);
    const tex = new THREE.CanvasTexture(canvas);
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: tex, transparent: true, opacity: 0.5 }));
    sprite.scale.set(4, 1, 1);
    const positions = [[0, 0, 14], [-17, 0, 0], [spotX + 2, 12, 0]];
    sprite.position.set(...(positions[i] as [number, number, number]));
    scene.add(sprite);
  });

  let frame: number;
  const startTime = performance.now();
  function animate() {
    frame = requestAnimationFrame(animate);
    orbitCamera(camera, startTime, 35, 0.04, 10);
    renderer.render(scene, camera);
  }
  animate();
  const removeResize = autoResize(container, camera, renderer);

  return { destroy() { cancelAnimationFrame(frame); removeResize(); disposeScene(scene); renderer.dispose(); container.removeChild(renderer.domElement); } };
}
