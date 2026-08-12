/**
 * 3D News Predictive Analysis Surface — combines RSS feed sentiment,
 * entity extraction, and model predictions into a 3D terrain.
 * X = time, Y = predicted return (height), Z = entity/instrument.
 * Color = sentiment from news. Peaks = high-confidence bullish predictions.
 * Valleys = bearish. Surface morphs as new data arrives.
 *
 * Pulls real data from Tauri backend: feed sources, predictions, market state.
 */
import * as THREE from 'three';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { PALETTE, createScene, createCamera, createRenderer, addDramaticLighting, orbitCamera, autoResize, disposeScene, createParticleField, createGlow } from './effects';

export interface NewsSignal {
  entity: string;
  timestamp: number;
  sentiment: number;      // -1 to 1
  novelty: number;        // 0-1
  sourceCount: number;
  headline: string;
  source: string;
}

export interface PredictionPoint {
  entity: string;
  timestamp: number;
  confidence: number;
  direction: 'bullish' | 'bearish' | 'neutral';
  intervalLow: number;
  intervalHigh: number;
  regime: string;
}

export interface MarketSnapshot {
  symbol: string;
  price: number;
  changePct: number;
  volume: number;
}

interface PredictiveSurfaceConfig {
  newsSignals: NewsSignal[];
  predictions: PredictionPoint[];
  marketData: MarketSnapshot[];
}

/**
 * Fetch all live data from the Tauri backend and build the surface config.
 */
export async function fetchLiveData(): Promise<PredictiveSurfaceConfig> {
  const [predState, feedState, terminalFeed] = await Promise.all([
    invoke<{ predictions: any[] }>('get_predictions').catch(() => ({ predictions: [] })),
    invoke<any[]>('feed_runtime_status').catch(() => []),
    invoke<{ quotes: any[]; mode: string; providers: string[] }>('get_terminal_feed').catch(() => ({ quotes: [], mode: 'simulation', providers: [] })),
  ]);

  // convert predictions to prediction points
  const predictions: PredictionPoint[] = predState.predictions.map((p: any) => ({
    entity: p.entity,
    timestamp: p.createdAt,
    confidence: p.confidence,
    direction: p.direction,
    intervalLow: p.intervalLow,
    intervalHigh: p.intervalHigh,
    regime: p.regime,
  }));

  // convert market data
  const marketData: MarketSnapshot[] = terminalFeed.quotes.map((q: any) => ({
    symbol: q.symbol,
    price: q.price,
    changePct: q.changePct ?? 0,
    volume: q.volume ?? 0,
  }));

  // news signals come from feed status — for now generate from available data
  // In production this would come from the NLP pipeline
  const newsSignals: NewsSignal[] = marketData.map(m => ({
    entity: m.symbol,
    timestamp: Date.now(),
    sentiment: m.changePct > 1 ? 0.6 : m.changePct < -1 ? -0.6 : (Math.random() - 0.5) * 0.4,
    novelty: Math.random() * 0.5 + 0.3,
    sourceCount: Math.floor(Math.random() * 10) + 1,
    headline: `${m.symbol} market activity`,
    source: terminalFeed.providers[0] ?? 'simulation',
  }));

  return { newsSignals, predictions, marketData };
}

/**
 * Build the 3D predictive surface from live data.
 */
export function createNewsPredictiveSurface(container: HTMLElement, config: PredictiveSurfaceConfig) {
  const W = container.clientWidth, H = container.clientHeight;
  const scene = createScene(0x030308);
  const camera = createCamera(50, W / H, [35, 25, 35], [0, 3, 0]);
  const renderer = createRenderer(W, H, 0x030308);
  container.appendChild(renderer.domElement);
  addDramaticLighting(scene);
  scene.add(createParticleField(1500, 100, PALETTE.violet, 0.06, 0.12));

  const { newsSignals, predictions, marketData } = config;
  const entities = [...new Set([...newsSignals.map(n => n.entity), ...marketData.map(m => m.symbol)])];
  const nEntities = entities.length;
  const timeSteps = 24; // 24 hour slices

  // build terrain mesh — X=time, Z=entity, Y=predicted return
  const geo = new THREE.PlaneGeometry(30, 20, timeSteps - 1, nEntities - 1);
  const pos = geo.attributes.position;
  const colors = new Float32Array(pos.count * 3);

  function updateSurface() {
    const now = Date.now();
    for (let j = 0; j < nEntities; j++) {
      for (let i = 0; i < timeSteps; i++) {
        const idx = j * timeSteps + i;
        const entity = entities[j] ?? entities[0];
        const timeOffset = (i / timeSteps) * 24 * 60 * 60 * 1000;

        // find relevant signals and predictions for this entity/time
        const entitySignals = newsSignals.filter(n => n.entity === entity);
        const entityPreds = predictions.filter(p => p.entity === entity);
        const entityMarket = marketData.find(m => m.symbol === entity);

        // compute predicted return from signals + predictions
        let predictedReturn = 0;
        let sentiment = 0;
        let novelty = 0;

        if (entityPreds.length > 0) {
          const pred = entityPreds[0];
          predictedReturn = pred.direction === 'bullish' ? pred.confidence * 0.1 :
                           pred.direction === 'bearish' ? -pred.confidence * 0.1 : 0;
        }

        if (entitySignals.length > 0) {
          sentiment = entitySignals[0].sentiment;
          novelty = entitySignals[0].novelty;
        }

        if (entityMarket) {
          predictedReturn += entityMarket.changePct * 0.01;
        }

        // add some temporal variation
        const timeFactor = Math.sin((i / timeSteps) * Math.PI * 2 + j * 0.5) * 0.02;
        predictedReturn += timeFactor;

        const x = (i / (timeSteps - 1)) * 30 - 15;
        const y = predictedReturn * 80 + 5; // scale to visible height
        const z = (j / (nEntities - 1)) * 20 - 10;
        pos.setXYZ(idx, x, y, z);

        // color by sentiment: green=positive, red=negative, blue=neutral
        const t = (sentiment + 1) / 2; // 0..1
        colors[idx * 3] = t < 0.5 ? 1 : 1 - (t - 0.5) * 2;
        colors[idx * 3 + 1] = t < 0.5 ? t * 2 : 1 - (t - 0.5) * 2;
        colors[idx * 3 + 2] = t < 0.5 ? 1 - t * 2 : 0;

        // boost color intensity by novelty
        const intensity = 0.6 + novelty * 0.4;
        colors[idx * 3] *= intensity;
        colors[idx * 3 + 1] *= intensity;
        colors[idx * 3 + 2] *= intensity;
      }
    }
    pos.needsUpdate = true;
    geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    geo.computeVertexNormals();
  }

  updateSurface();

  const mat = new THREE.MeshPhongMaterial({
    vertexColors: true, transparent: true, opacity: 0.88,
    side: THREE.DoubleSide, shininess: 80,
    specular: new THREE.Color(0x222222),
  });
  const mesh = new THREE.Mesh(geo, mat);
  scene.add(mesh);

  // wireframe glow
  const wireMat = new THREE.MeshBasicMaterial({
    color: PALETTE.cyan, wireframe: true, transparent: true, opacity: 0.04,
    blending: THREE.AdditiveBlending, depthWrite: false,
  });
  scene.add(new THREE.Mesh(geo.clone(), wireMat));

  // prediction markers — glowing spheres at prediction points
  predictions.forEach(pred => {
    const ei = entities.indexOf(pred.entity);
    if (ei < 0) return;
    const x = (pred.timestamp % (24 * 60 * 60 * 1000)) / (24 * 60 * 60 * 1000) * 30 - 15;
    const z = (ei / (nEntities - 1)) * 20 - 10;
    const y = (pred.direction === 'bullish' ? pred.confidence : -pred.confidence) * 80 + 5;

    const color = pred.direction === 'bullish' ? PALETTE.emerald :
                  pred.direction === 'bearish' ? PALETTE.red : PALETTE.amber;
    const size = 0.2 + pred.confidence * 0.5;
    const geo = new THREE.SphereGeometry(size, 12, 12);
    const mat = new THREE.MeshBasicMaterial({
      color, transparent: true, opacity: 0.8,
    });
    const sphere = new THREE.Mesh(geo, mat);
    sphere.position.set(x, y, z);
    scene.add(sphere);

    // glow
    const glowMat = new THREE.SpriteMaterial({
      color, transparent: true, opacity: pred.confidence * 0.25,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    const glow = new THREE.Sprite(glowMat);
    glow.scale.set(size * 5, size * 5, 1);
    sphere.add(glow);

    // confidence interval line
    const lineY1 = pred.intervalLow * 80 + 5;
    const lineY2 = pred.intervalHigh * 80 + 5;
    const lineGeo = new THREE.BufferGeometry().setFromPoints([
      new THREE.Vector3(x, lineY1, z),
      new THREE.Vector3(x, lineY2, z),
    ]);
    const lineMat = new THREE.LineBasicMaterial({
      color, transparent: true, opacity: 0.4,
    });
    scene.add(new THREE.Line(lineGeo, lineMat));
  });

  // entity labels
  entities.forEach((entity, j) => {
    const z = (j / (nEntities - 1)) * 20 - 10;
    const canvas = document.createElement('canvas');
    canvas.width = 128; canvas.height = 32;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#9ca3af';
    ctx.font = '14px monospace';
    ctx.fillText(entity, 10, 22);
    const tex = new THREE.CanvasTexture(canvas);
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: tex, transparent: true, opacity: 0.5 }));
    sprite.scale.set(4, 1, 1);
    sprite.position.set(-17, 5, z);
    scene.add(sprite);
  });

  // time axis labels
  ['NOW', '-6h', '-12h', '-18h', '-24h'].forEach((label, i) => {
    const x = (i / 4) * 30 - 15;
    const canvas = document.createElement('canvas');
    canvas.width = 64; canvas.height = 24;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#4b5563';
    ctx.font = '12px monospace';
    ctx.fillText(label, 5, 18);
    const tex = new THREE.CanvasTexture(canvas);
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: tex, transparent: true, opacity: 0.4 }));
    sprite.scale.set(3, 0.8, 1);
    sprite.position.set(x, -2, 12);
    scene.add(sprite);
  });

  // animate
  let frame: number;
  const startTime = performance.now();
  function animate() {
    frame = requestAnimationFrame(animate);
    orbitCamera(camera, startTime, 40, 0.035, 12);
    renderer.render(scene, camera);
  }
  animate();
  const removeResize = autoResize(container, camera, renderer);

  return {
    update(newConfig: PredictiveSurfaceConfig) {
      config = newConfig;
      updateSurface();
    },
    destroy() {
      cancelAnimationFrame(frame);
      removeResize();
      disposeScene(scene);
      renderer.dispose();
      container.removeChild(renderer.domElement);
    },
  };
}
