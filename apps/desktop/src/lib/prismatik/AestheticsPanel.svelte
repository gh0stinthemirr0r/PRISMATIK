<script lang="ts">
  import { X } from 'lucide-svelte';
  import { aesthetics } from './aesthetics.svelte';
  import { THEMES, FONTS, ACCENTS, DENSITIES, MOTIONS, FONT_SCALES, GLASS_LEVELS, LAYOUTS } from './themes';

  const systemReduced =
    typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches;
  let themeQuery = $state('');
  let themeFamily = $state('All');
  const families = ['All', 'Core', 'Neon / Retro', 'Soft / Pastels', 'Vibrant Dark', 'Warm & Earthy', 'Minimal'];
  const visibleThemes = $derived(THEMES.filter((theme) =>
    (themeFamily === 'All' || (theme.family ?? 'Core') === themeFamily) &&
    `${theme.label} ${theme.blurb}`.toLowerCase().includes(themeQuery.trim().toLowerCase()),
  ));
</script>

<div
  class="pk-aes-backdrop"
  role="presentation"
  onclick={() => (aesthetics.panelOpen = false)}
></div>
<aside class="pk-aes" aria-label="Aesthetics panel">
  <div class="pk-aes-head">
    <span class="pk-aes-title">Aesthetics</span>
    <button class="pk-icon-btn" onclick={() => (aesthetics.panelOpen = false)} title="Close">
      <X size={14} />
    </button>
  </div>
  <div class="pk-aes-body">
    <section class="pk-aes-sec">
      <span class="pk-aes-label">Theme library · {THEMES.length} palettes</span>
      <input class="pk-theme-search" bind:value={themeQuery} placeholder="Search Catppuccin, Tokyo, Nord…" aria-label="Search themes" />
      <div class="pk-theme-families">
        {#each families as family}<button class="pk-btn" class:active={themeFamily === family} onclick={() => (themeFamily = family)}>{family}</button>{/each}
      </div>
      <div class="pk-theme-grid">
        {#each visibleThemes as theme (theme.id)}
          <button
            class="pk-theme-card"
            class:active={aesthetics.theme === theme.id}
            onclick={() => (aesthetics.theme = theme.id)}
          >
            <div class="pk-theme-swatch" style="background:{theme.vars['--p-bg']}">
              <i style="background:{theme.vars['--p-up']}"></i>
              <i style="background:{theme.vars['--p-down']}"></i>
              <i style="background:{theme.vars['--p-accent']}"></i>
              <i style="background:{theme.vars['--p-accent2']}"></i>
            </div>
            <div class="pk-theme-meta">
              <b>{theme.label}</b>
              <span>{theme.blurb}</span>
            </div>
          </button>
        {/each}
      </div>
    </section>

    <section class="pk-aes-sec">
      <span class="pk-aes-label">Typeface</span>
      {#each FONTS as font (font.id)}
        <button
          class="pk-font-row"
          class:active={aesthetics.font === font.id}
          style="font-family:{font.family}"
          onclick={() => (aesthetics.font = font.id)}
        >
          <span>{font.label} <small>Aag 0123456789</small></span>
          <small>{font.blurb}</small>
        </button>
      {/each}
    </section>

    <section class="pk-aes-sec">
      <span class="pk-aes-label">Density</span>
      <div class="pk-seg">
        {#each DENSITIES as d (d.id)}
          <button
            class="pk-btn"
            class:active={aesthetics.density === d.id}
            onclick={() => (aesthetics.density = d.id)}>{d.label}</button
          >
        {/each}
      </div>
    </section>

    <section class="pk-aes-sec">
      <span class="pk-aes-label">Type scale</span>
      <div class="pk-seg">
        {#each FONT_SCALES as size (size.id)}
          <button class="pk-btn" class:active={aesthetics.fontScale === size.id} onclick={() => (aesthetics.fontScale = size.id)}>{size.label}</button>
        {/each}
      </div>
    </section>

    <section class="pk-aes-sec">
      <span class="pk-aes-label">Glass depth</span>
      <div class="pk-seg">
        {#each GLASS_LEVELS as level (level.id)}
          <button class="pk-btn" class:active={aesthetics.glass === level.id} onclick={() => (aesthetics.glass = level.id)}>{level.label}</button>
        {/each}
      </div>
    </section>

    <section class="pk-aes-sec">
      <span class="pk-aes-label">Workspace geometry</span>
      <div class="pk-seg">
        {#each LAYOUTS as layout (layout.id)}
          <button class="pk-btn" class:active={aesthetics.layout === layout.id} onclick={() => (aesthetics.layout = layout.id)}>{layout.label}</button>
        {/each}
      </div>
    </section>

    <section class="pk-aes-sec">
      <span class="pk-aes-label">Motion</span>
      <div class="pk-seg">
        {#each MOTIONS as m (m.id)}
          <button
            class="pk-btn"
            class:active={aesthetics.motion === m.id}
            onclick={() => (aesthetics.motion = m.id)}>{m.label}</button
          >
        {/each}
      </div>
      {#if systemReduced}
        <small class="pk-dim" style="font-size:9.5px">
          Your OS requests reduced motion — honoured by default.
        </small>
      {/if}
    </section>

    <section class="pk-aes-sec">
      <span class="pk-aes-label">Accent</span>
      <div class="pk-accent-dots">
        {#each ACCENTS as acc (acc.id)}
          <button
            class="pk-accent-dot"
            class:active={aesthetics.accent === acc.id}
            class:theme-default={acc.id === 'theme'}
            style="background:{acc.color ?? 'transparent'}"
            title={acc.label}
            onclick={() => (aesthetics.accent = acc.id)}
          ></button>
        {/each}
      </div>
    </section>
  </div>
</aside>
