/**
 * IV surface backends for the options volatility lab (Wave 2.5).
 *
 * Prefer WebGPU in the Tauri webview when available. Canvas mesh remains
 * mandatory fallback. A native `prismatik-renderer` / wgpu crate bridge is
 * deferred — set `VITE_IV_SURFACE_FORCE_CANVAS=1` or pass `preferWebGpu: false`
 * when wiring host-side GPU buffers later (see crates/prismatik-renderer stub).
 */

import {
  buildIvSurfaceMesh,
  ivColorCss,
  ivColorRgba,
  type IvQuote,
  type IvSurfaceMesh,
} from "./mesh";

/** Feature flag: force Canvas even when `navigator.gpu` exists. */
export const IV_SURFACE_FORCE_CANVAS =
  typeof import.meta !== "undefined" &&
  Boolean((import.meta as ImportMeta & { env?: Record<string, string> }).env?.VITE_IV_SURFACE_FORCE_CANVAS);

export type IvSurfaceBackend = "webgpu" | "canvas" | "unavailable";

export type IvSurfaceRenderResult = {
  backend: IvSurfaceBackend;
  mesh: IvSurfaceMesh;
};

export function detectIvSurfaceBackend(preferWebGpu = true): IvSurfaceBackend {
  if (IV_SURFACE_FORCE_CANVAS || !preferWebGpu) return "canvas";
  if (typeof navigator !== "undefined" && "gpu" in navigator) return "webgpu";
  return "canvas";
}

type Theme = {
  background: string;
  text: string;
  grid: string;
  border: string;
};

function readTheme(el: HTMLElement): Theme {
  const style = getComputedStyle(el);
  const read = (name: string, fallback: string) => style.getPropertyValue(name).trim() || fallback;
  return {
    background: read("--color-surface-1", "#ffffff"),
    text: read("--color-text-tertiary", "#7a8499"),
    grid: read("--color-border-default", "#d5dbe6"),
    border: read("--color-border-strong", "#b8c0cf"),
  };
}

export type PlotRect = { x: number; y: number; w: number; h: number };

export function plotRectFor(width: number, height: number, dpr: number): PlotRect {
  return {
    x: 52 * dpr,
    y: 28 * dpr,
    w: width - 72 * dpr,
    h: height - 64 * dpr,
  };
}

export function paintAxes(
  ctx: CanvasRenderingContext2D,
  mesh: IvSurfaceMesh,
  plot: PlotRect,
  theme: Theme,
  dpr: number,
  backendLabel: string,
): void {
  ctx.strokeStyle = theme.border;
  ctx.lineWidth = 1 * dpr;
  ctx.strokeRect(plot.x, plot.y, plot.w, plot.h);

  ctx.fillStyle = theme.text;
  ctx.font = `${11 * dpr}px ui-monospace, monospace`;
  ctx.fillText("Strike →", plot.x, plot.y + plot.h + 16 * dpr);
  ctx.save();
  ctx.translate(14 * dpr, plot.y + plot.h / 2);
  ctx.rotate(-Math.PI / 2);
  ctx.fillText("Term →", 0, 0);
  ctx.restore();

  const labelEvery = Math.max(1, Math.floor(mesh.strikes.length / 6));
  for (let xi = 0; xi < mesh.strikes.length; xi += labelEvery) {
    const x = plot.x + ((xi + 0.5) / mesh.strikes.length) * plot.w;
    ctx.fillText(String(mesh.strikes[xi]), x - 10 * dpr, plot.y + plot.h + 28 * dpr);
  }
  for (let yi = 0; yi < mesh.terms.length; yi++) {
    const y = plot.y + ((yi + 0.5) / mesh.terms.length) * plot.h;
    ctx.fillText(mesh.terms[yi].slice(5), 18 * dpr, y + 4 * dpr);
  }

  ctx.fillText(
    `IV ${(mesh.minIv * 100).toFixed(1)}–${(mesh.maxIv * 100).toFixed(1)}% · ${backendLabel}`,
    plot.x,
    14 * dpr,
  );

  for (let i = 0; i < 32; i++) {
    const t = mesh.minIv + (i / 31) * Math.max(1e-9, mesh.maxIv - mesh.minIv);
    ctx.fillStyle = ivColorCss(t, mesh.minIv, mesh.maxIv);
    ctx.fillRect(plot.x + plot.w - 96 * dpr + i * 3 * dpr, 6 * dpr, 3 * dpr, 10 * dpr);
  }
}

export function drawCanvasMesh(
  canvas: HTMLCanvasElement,
  mesh: IvSurfaceMesh,
  backendLabel: string,
): void {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const dpr = typeof devicePixelRatio === "number" ? devicePixelRatio : 1;
  const width = (canvas.width = Math.max(1, Math.floor(canvas.clientWidth * dpr)));
  const height = (canvas.height = Math.max(1, Math.floor((canvas.clientHeight || 260) * dpr)));
  const theme = readTheme(canvas);
  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = theme.background;
  ctx.fillRect(0, 0, width, height);

  const plot = plotRectFor(width, height, dpr);
  const cellW = plot.w / Math.max(1, mesh.strikes.length);
  const cellH = plot.h / Math.max(1, mesh.terms.length);

  for (let yi = 0; yi < mesh.terms.length; yi++) {
    for (let xi = 0; xi < mesh.strikes.length; xi++) {
      const iv = mesh.values[yi][xi];
      if (iv == null) {
        ctx.fillStyle = theme.grid;
        ctx.globalAlpha = 0.35;
        ctx.fillRect(plot.x + xi * cellW, plot.y + yi * cellH, cellW - dpr, cellH - dpr);
        ctx.globalAlpha = 1;
        continue;
      }
      ctx.fillStyle = ivColorCss(iv, mesh.minIv, mesh.maxIv);
      ctx.fillRect(plot.x + xi * cellW, plot.y + yi * cellH, cellW - dpr, cellH - dpr);
    }
  }

  paintAxes(ctx, mesh, plot, theme, dpr, backendLabel);
}

/** Upload IV grid as an RGBA texture and blit a fullscreen quad. */
export async function drawWebGpuMesh(canvas: HTMLCanvasElement, mesh: IvSurfaceMesh): Promise<boolean> {
  const nav = navigator as Navigator & { gpu?: GPU };
  const gpu = nav.gpu;
  if (!gpu) return false;
  const adapter = await gpu.requestAdapter();
  if (!adapter) return false;
  const device = await adapter.requestDevice();
  const context = canvas.getContext("webgpu") as GPUCanvasContext | null;
  if (!context) return false;

  const dpr = typeof devicePixelRatio === "number" ? devicePixelRatio : 1;
  const width = Math.max(1, Math.floor(canvas.clientWidth * dpr));
  const height = Math.max(1, Math.floor((canvas.clientHeight || 260) * dpr));
  canvas.width = width;
  canvas.height = height;

  const format = gpu.getPreferredCanvasFormat();
  context.configure({ device, format, alphaMode: "opaque" });

  // Clear to surface background, then blit the IV texture into the plot rect only
  // so the 2D overlay axes line up with Canvas mesh layout.
  const theme = readTheme(canvas);
  const bg = theme.background.match(/\d+/g);
  const clearValue = bg && bg.length >= 3
    ? { r: Number(bg[0]) / 255, g: Number(bg[1]) / 255, b: Number(bg[2]) / 255, a: 1 }
    : { r: 1, g: 1, b: 1, a: 1 };
  const plot = plotRectFor(width, height, dpr);

  const texW = Math.max(1, mesh.strikes.length);
  const texH = Math.max(1, mesh.terms.length);
  const pixels = new Uint8Array(texW * texH * 4);
  for (let yi = 0; yi < texH; yi++) {
    for (let xi = 0; xi < texW; xi++) {
      const iv = mesh.values[yi]?.[xi];
      const offset = (yi * texW + xi) * 4;
      if (iv == null) {
        pixels.set([213, 219, 230, 255], offset);
        continue;
      }
      pixels.set(ivColorRgba(iv, mesh.minIv, mesh.maxIv), offset);
    }
  }

  const texture = device.createTexture({
    size: [texW, texH],
    format: "rgba8unorm",
    usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
  });
  device.queue.writeTexture({ texture }, pixels, { bytesPerRow: texW * 4 }, [texW, texH]);

  const shader = device.createShaderModule({
    code: `
struct VsOut { @builtin(position) pos: vec4f, @location(0) uv: vec2f };
@group(0) @binding(0) var samp: sampler;
@group(0) @binding(1) var tex: texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) i: u32) -> VsOut {
  var p = array<vec2f, 6>(
    vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0),
    vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0)
  );
  var u = array<vec2f, 6>(
    vec2f(0.0, 1.0), vec2f(1.0, 1.0), vec2f(0.0, 0.0),
    vec2f(0.0, 0.0), vec2f(1.0, 1.0), vec2f(1.0, 0.0)
  );
  var o: VsOut;
  o.pos = vec4f(p[i], 0.0, 1.0);
  o.uv = u[i];
  return o;
}
@fragment fn fs(i: VsOut) -> @location(0) vec4f {
  return textureSample(tex, samp, i.uv);
}
`,
  });

  const sampler = device.createSampler({ magFilter: "nearest", minFilter: "nearest" });
  const bindGroupLayout = device.createBindGroupLayout({
    entries: [
      { binding: 0, visibility: GPUShaderStage.FRAGMENT, sampler: {} },
      { binding: 1, visibility: GPUShaderStage.FRAGMENT, texture: {} },
    ],
  });
  const pipeline = device.createRenderPipeline({
    layout: device.createPipelineLayout({ bindGroupLayouts: [bindGroupLayout] }),
    vertex: { module: shader, entryPoint: "vs" },
    fragment: { module: shader, entryPoint: "fs", targets: [{ format }] },
  });
  const bindGroup = device.createBindGroup({
    layout: bindGroupLayout,
    entries: [
      { binding: 0, resource: sampler },
      { binding: 1, resource: texture.createView() },
    ],
  });

  const encoder = device.createCommandEncoder();
  const pass = encoder.beginRenderPass({
    colorAttachments: [
      {
        view: context.getCurrentTexture().createView(),
        clearValue,
        loadOp: "clear",
        storeOp: "store",
      },
    ],
  });
  pass.setPipeline(pipeline);
  pass.setBindGroup(0, bindGroup);
  pass.setViewport(plot.x, plot.y, Math.max(1, plot.w), Math.max(1, plot.h), 0, 1);
  pass.setScissorRect(
    Math.max(0, Math.floor(plot.x)),
    Math.max(0, Math.floor(plot.y)),
    Math.max(1, Math.floor(plot.w)),
    Math.max(1, Math.floor(plot.h)),
  );
  pass.draw(6);
  pass.end();
  device.queue.submit([encoder.finish()]);
  return true;
}

/** Paint axes onto a separate 2D overlay canvas (WebGPU cannot share a context). */
export function drawAxesOverlay(
  canvas: HTMLCanvasElement,
  mesh: IvSurfaceMesh,
  backendLabel: string,
): void {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const dpr = typeof devicePixelRatio === "number" ? devicePixelRatio : 1;
  const width = (canvas.width = Math.max(1, Math.floor(canvas.clientWidth * dpr)));
  const height = (canvas.height = Math.max(1, Math.floor((canvas.clientHeight || 260) * dpr)));
  ctx.clearRect(0, 0, width, height);
  paintAxes(ctx, mesh, plotRectFor(width, height, dpr), readTheme(canvas), dpr, backendLabel);
}

export async function renderIvSurface(
  canvas: HTMLCanvasElement,
  quotes: IvQuote[],
  preferWebGpu = true,
  overlay?: HTMLCanvasElement | null,
): Promise<IvSurfaceRenderResult> {
  const mesh = buildIvSurfaceMesh(quotes);
  const wanted = detectIvSurfaceBackend(preferWebGpu);

  if (wanted === "webgpu") {
    try {
      const ok = await drawWebGpuMesh(canvas, mesh);
      if (ok) {
        if (overlay) drawAxesOverlay(overlay, mesh, "WebGPU surface");
        return { backend: "webgpu", mesh };
      }
    } catch {
      // Fall through — common on older WebView2 builds without WebGPU.
    }
  }

  if (overlay) {
    const ctx = overlay.getContext("2d");
    if (ctx) {
      overlay.width = overlay.clientWidth;
      overlay.height = overlay.clientHeight || 260;
      ctx.clearRect(0, 0, overlay.width, overlay.height);
    }
  }

  drawCanvasMesh(
    canvas,
    mesh,
    wanted === "webgpu" ? "Canvas mesh · WebGPU unavailable" : "Canvas mesh",
  );
  return { backend: "canvas", mesh };
}

type GPU = {
  requestAdapter(): Promise<GPUAdapter | null>;
  getPreferredCanvasFormat(): GPUTextureFormat;
};
type GPUAdapter = { requestDevice(): Promise<GPUDevice> };
type GPUDevice = {
  createTexture(desc: unknown): GPUTexture;
  createShaderModule(desc: { code: string }): GPUShaderModule;
  createSampler(desc: unknown): GPUSampler;
  createBindGroupLayout(desc: unknown): GPUBindGroupLayout;
  createPipelineLayout(desc: unknown): GPUPipelineLayout;
  createRenderPipeline(desc: unknown): GPURenderPipeline;
  createBindGroup(desc: unknown): GPUBindGroup;
  createCommandEncoder(): GPUCommandEncoder;
  queue: { writeTexture(...args: unknown[]): void; submit(bufs: unknown[]): void };
};
type GPUTexture = { createView(): unknown };
type GPUShaderModule = object;
type GPUSampler = object;
type GPUBindGroupLayout = object;
type GPUPipelineLayout = object;
type GPURenderPipeline = object;
type GPUBindGroup = object;
type GPUCommandEncoder = {
  beginRenderPass(desc: unknown): GPURenderPass;
  finish(): unknown;
};
type GPURenderPass = {
  setPipeline(p: GPURenderPipeline): void;
  setBindGroup(i: number, g: GPUBindGroup): void;
  setViewport(x: number, y: number, w: number, h: number, minD: number, maxD: number): void;
  setScissorRect(x: number, y: number, w: number, h: number): void;
  draw(n: number): void;
  end(): void;
};
type GPUCanvasContext = {
  configure(desc: unknown): void;
  getCurrentTexture(): { createView(): unknown };
};
type GPUTextureFormat = string;
declare const GPUTextureUsage: { TEXTURE_BINDING: number; COPY_DST: number };
declare const GPUShaderStage: { FRAGMENT: number };
