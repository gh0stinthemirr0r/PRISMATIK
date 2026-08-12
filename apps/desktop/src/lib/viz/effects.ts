/**
 * Shared visual effects utilities for all 3D/2D visualizations.
 * Color palettes, glow materials, particle systems, post-processing.
 */
import * as THREE from 'three';

// ── PRISMATIK spectral palette ────────────────────────────────

export const PALETTE = {
  bg:         0x050510,
  bgAlt:      0x0a0f1e,
  surface0:   0x0d1117,
  surface1:   0x111827,
  surface2:   0x1a2332,
  border:     0x2d3748,
  textDim:    0x6b7280,
  text:       0xe5e7eb,
  cyan:       0x00f0ff,
  cyanDark:   0x0891b2,
  violet:     0xa855f7,
  violetDark: 0x7c3aed,
  emerald:    0x34d399,
  emeraldDark:0x059669,
  amber:      0xfbbf24,
  amberDark:  0xd97706,
  red:        0xef4444,
  redDark:    0xdc2626,
  rose:       0xf43f5e,
  white:      0xffffff,
};

export const REGIME_COLORS: Record<string, number> = {
  calm_trending:       PALETTE.emerald,
  calm_mean_revert:    PALETTE.cyan,
  volatile_trending:   PALETTE.amber,
  volatile_mean_revert:PALETTE.rose,
  crisis:              PALETTE.red,
  // Deliberately desaturated: an unclassified instrument must never read as a
  // confident regime call. Anything without enough history lands here.
  unclassified:        0x6b7690,
};

// ── Glow sprite factory ───────────────────────────────────────

export function createGlow(color: number, size: number, opacity = 0.25): THREE.Sprite {
  const mat = new THREE.SpriteMaterial({
    color,
    transparent: true,
    opacity,
    blending: THREE.AdditiveBlending,
    depthWrite: false,
  });
  const sprite = new THREE.Sprite(mat);
  sprite.scale.set(size, size, 1);
  return sprite;
}

// ── Glowing sphere ────────────────────────────────────────────

export function createGlowSphere(
  radius: number,
  color: number,
  emissiveIntensity = 0.4,
  segments = 24,
): THREE.Mesh {
  const geo = new THREE.SphereGeometry(radius, segments, segments);
  const mat = new THREE.MeshPhongMaterial({
    color,
    emissive: color,
    emissiveIntensity,
    transparent: true,
    opacity: 0.9,
    shininess: 80,
  });
  return new THREE.Mesh(geo, mat);
}

// ── Glowing line ──────────────────────────────────────────────

export function createGlowLine(
  points: THREE.Vector3[],
  color: number,
  opacity = 0.5,
  lineWidth = 1,
): THREE.Line {
  const geo = new THREE.BufferGeometry().setFromPoints(points);
  const mat = new THREE.LineBasicMaterial({
    color,
    transparent: true,
    opacity,
    linewidth: lineWidth,
  });
  return new THREE.Line(geo, mat);
}

// ── Tube with glow ────────────────────────────────────────────

export function createGlowTube(
  curve: THREE.Curve<THREE.Vector3>,
  radius: number,
  color: number,
  opacity = 0.6,
  glowRadius?: number,
): THREE.Group {
  const group = new THREE.Group();
  const tubeGeo = new THREE.TubeGeometry(curve, 64, radius, 8, false);
  const tubeMat = new THREE.MeshPhongMaterial({
    color,
    emissive: color,
    emissiveIntensity: 0.3,
    transparent: true,
    opacity,
    shininess: 60,
  });
  group.add(new THREE.Mesh(tubeGeo, tubeMat));

  if (glowRadius) {
    const glowGeo = new THREE.TubeGeometry(curve, 64, glowRadius, 8, false);
    const glowMat = new THREE.MeshBasicMaterial({
      color,
      transparent: true,
      opacity: opacity * 0.25,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });
    group.add(new THREE.Mesh(glowGeo, glowMat));
  }
  return group;
}

// ── Particle field ────────────────────────────────────────────

export function createParticleField(
  count: number,
  spread: number,
  color: number,
  size = 0.15,
  opacity = 0.5,
): THREE.Points {
  const positions = new Float32Array(count * 3);
  for (let i = 0; i < count; i++) {
    positions[i * 3]     = (Math.random() - 0.5) * spread;
    positions[i * 3 + 1] = (Math.random() - 0.5) * spread;
    positions[i * 3 + 2] = (Math.random() - 0.5) * spread;
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  const mat = new THREE.PointsMaterial({
    color,
    size,
    transparent: true,
    opacity,
    blending: THREE.AdditiveBlending,
    depthWrite: false,
    sizeAttenuation: true,
  });
  return new THREE.Points(geo, mat);
}

// ── Grid floor ────────────────────────────────────────────────

export function createGridFloor(size: number, divisions: number): THREE.GridHelper {
  const grid = new THREE.GridHelper(size, divisions, PALETTE.surface2, PALETTE.surface1);
  (grid.material as THREE.LineBasicMaterial).transparent = true;
  (grid.material as THREE.LineBasicMaterial).opacity = 0.4;
  return grid;
}

// ── Scene setup ───────────────────────────────────────────────

export function createScene(bg: number = PALETTE.bg): THREE.Scene {
  const scene = new THREE.Scene();
  scene.fog = new THREE.FogExp2(bg, 0.004);
  return scene;
}

export function createCamera(
  fov: number,
  aspect: number,
  pos: [number, number, number],
  lookAt?: [number, number, number],
): THREE.PerspectiveCamera {
  const cam = new THREE.PerspectiveCamera(fov, aspect, 0.1, 1000);
  cam.position.set(...pos);
  if (lookAt) cam.lookAt(...lookAt);
  return cam;
}

export function createRenderer(
  width: number,
  height: number,
  bg: number = PALETTE.bg,
): THREE.WebGLRenderer {
  const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
  renderer.setSize(width, height);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.setClearColor(bg, 1);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.2;
  return renderer;
}

// ── Dramatic lighting ─────────────────────────────────────────

export function addDramaticLighting(scene: THREE.Scene): void {
  scene.add(new THREE.AmbientLight(0x1a1a2e, 0.3));

  const key = new THREE.DirectionalLight(0xffffff, 0.6);
  key.position.set(20, 30, 10);
  scene.add(key);

  const fill = new THREE.PointLight(PALETTE.cyan, 0.8, 80);
  fill.position.set(-15, 20, -10);
  scene.add(fill);

  const rim = new THREE.PointLight(PALETTE.violet, 0.5, 60);
  rim.position.set(10, -10, 20);
  scene.add(rim);
}

// ── Slow orbit camera ─────────────────────────────────────────

export function orbitCamera(
  camera: THREE.PerspectiveCamera,
  startTime: number,
  radius: number,
  speed = 0.05,
  yBase = 0,
): void {
  const t = (performance.now() - startTime) / 1000;
  camera.position.x = Math.sin(t * speed) * radius;
  camera.position.z = Math.cos(t * speed) * radius;
  camera.position.y = yBase + Math.sin(t * speed * 0.3) * 5;
  camera.lookAt(0, 0, 0);
}

// ── Resize handler ────────────────────────────────────────────

export function autoResize(
  container: HTMLElement,
  camera: THREE.PerspectiveCamera,
  renderer: THREE.WebGLRenderer,
): () => void {
  const handler = () => {
    const w = container.clientWidth;
    const h = container.clientHeight;
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    renderer.setSize(w, h);
  };
  window.addEventListener('resize', handler);
  return () => window.removeEventListener('resize', handler);
}

// ── Cleanup ───────────────────────────────────────────────────

export function disposeScene(scene: THREE.Scene): void {
  scene.traverse((obj) => {
    if (obj instanceof THREE.Mesh) {
      obj.geometry?.dispose();
      if (Array.isArray(obj.material)) {
        obj.material.forEach(m => m.dispose());
      } else {
        obj.material?.dispose();
      }
    }
  });
}
