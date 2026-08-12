<script lang="ts">
  /**
   * Observed-quote sparkline.
   *
   * Draws only real observations, so a freshly tracked instrument shows a dot
   * or nothing at all until a second quote lands. It never pre-seeds a walk to
   * make the row look populated.
   */
  import { aesthetics } from './aesthetics.svelte';
  import type { Instrument } from './market.svelte';

  let { inst }: { inst: Instrument } = $props();

  let canvas: HTMLCanvasElement;

  function draw() {
    const ctx = canvas?.getContext('2d');
    if (!ctx) return;
    const W = canvas.width;
    const H = canvas.height;
    ctx.clearRect(0, 0, W, H);

    const hist = inst.history;
    const color = inst.up ? aesthetics.vars['--p-up'] : aesthetics.vars['--p-down'];

    if (hist.length === 0) return;
    if (hist.length === 1) {
      // One observation is a level, not a trend — say so with a single mark.
      ctx.beginPath();
      ctx.arc(W / 2, H / 2, 2.2, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.globalAlpha = 0.75;
      ctx.fill();
      ctx.globalAlpha = 1;
      return;
    }

    let min = Infinity;
    let max = -Infinity;
    for (const v of hist) {
      if (v < min) min = v;
      if (v > max) max = v;
    }
    const span = max - min || 1;
    const x = (i: number) => (i / (hist.length - 1)) * (W - 4) + 2;
    const y = (v: number) => H - 4 - ((v - min) / span) * (H - 8);
    ctx.beginPath();
    ctx.moveTo(x(0), y(hist[0]));
    for (let i = 1; i < hist.length; i++) ctx.lineTo(x(i), y(hist[i]));
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.6;
    ctx.stroke();
    ctx.lineTo(x(hist.length - 1), H);
    ctx.lineTo(x(0), H);
    ctx.closePath();
    ctx.globalAlpha = 0.13;
    ctx.fillStyle = color;
    ctx.fill();
    ctx.globalAlpha = 1;
  }

  $effect(() => {
    // fine-grained: redraw only when this instrument ticks or the theme flips
    void inst.tick;
    void aesthetics.theme;
    void aesthetics.accent;
    draw();
  });
</script>

<canvas class="pk-spark" bind:this={canvas} width="128" height="44"></canvas>

<style>
  /* Backing store is 2x the CSS box so the line stays crisp on HiDPI.
     Sized here rather than with a style attribute — the Tauri CSP sets
     `style-src 'self'`, which blocks inline style attributes. */
  .pk-spark {
    width: 64px;
    height: 22px;
  }
</style>
