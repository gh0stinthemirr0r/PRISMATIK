/**
 * PRISMATIK — reactive aesthetics state.
 *
 * Holds the user's theme / font / density / motion / accent choices,
 * resolves them into a flat map of CSS custom-property values (also read
 * directly by canvas renderers), and persists to localStorage under the
 * `prismatik:` key prefix.
 */

import {
  THEMES,
  FONTS,
  ACCENTS,
  FONT_SCALES,
  GLASS_LEVELS,
  MONO_STACK,
  type Density,
  type FontScale,
  type GlassLevel,
  type LayoutMode,
  type Motion,
} from './themes';

const STORAGE_KEY = 'prismatik:prefs';

interface Prefs {
  theme: string;
  font: string;
  density: Density;
  motion: Motion;
  accent: string;
  fontScale: FontScale;
  glass: GlassLevel;
  layout: LayoutMode;
}

function systemPrefersReduced(): boolean {
  try {
    return typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches;
  } catch {
    return false;
  }
}

function loadPrefs(): Partial<Prefs> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Partial<Prefs>) : {};
  } catch {
    return {};
  }
}

class Aesthetics {
  theme = $state('dark');
  font = $state('mono');
  density = $state<Density>('comfortable');
  motion = $state<Motion>(systemPrefersReduced() ? 'reduced' : 'full');
  accent = $state('theme');
  fontScale = $state<FontScale>('standard');
  glass = $state<GlassLevel>('balanced');
  layout = $state<LayoutMode>('desk');
  panelOpen = $state(false);

  themeDef = $derived(THEMES.find((t) => t.id === this.theme) ?? THEMES[0]);
  fontDef = $derived(FONTS.find((f) => f.id === this.font) ?? FONTS[0]);
  accentDef = $derived(ACCENTS.find((a) => a.id === this.accent) ?? ACCENTS[0]);

  /** Flat resolved variable map — applied inline on the root element and
   *  read by canvas code so charts and DOM never drift apart. */
  vars = $derived.by((): Record<string, string> => {
    const out: Record<string, string> = { ...this.themeDef.vars };
    if (this.accentDef.color) out['--p-accent'] = this.accentDef.color;
    if (this.accentDef.color2) out['--p-accent2'] = this.accentDef.color2;
    out['--font-ui'] = this.fontDef.family;
    out['--font-mono'] = MONO_STACK;
    out['--p-font-scale'] = FONT_SCALES.find((item) => item.id === this.fontScale)?.value ?? '1';
    const glass = GLASS_LEVELS.find((item) => item.id === this.glass) ?? GLASS_LEVELS[1];
    out['--p-panel-alpha'] = glass.opacity;
    out['--p-glass-blur'] = glass.blur;
    out['--p-panel-fill'] = `color-mix(in srgb, ${out['--p-surface']} ${Number(glass.opacity) * 100}%, transparent)`;
    return out;
  });

  reduced = $derived(this.motion === 'reduced');

  constructor() {
    const saved = loadPrefs();
    if (saved.theme && THEMES.some((t) => t.id === saved.theme)) this.theme = saved.theme;
    if (saved.font && FONTS.some((f) => f.id === saved.font)) this.font = saved.font;
    if (saved.density === 'compact' || saved.density === 'comfortable') this.density = saved.density;
    if (saved.motion === 'full' || saved.motion === 'reduced') this.motion = saved.motion;
    if (saved.accent && ACCENTS.some((a) => a.id === saved.accent)) this.accent = saved.accent;
    if (FONT_SCALES.some((item) => item.id === saved.fontScale)) this.fontScale = saved.fontScale!;
    if (GLASS_LEVELS.some((item) => item.id === saved.glass)) this.glass = saved.glass!;
    if (saved.layout === 'focus' || saved.layout === 'desk' || saved.layout === 'panorama') this.layout = saved.layout;

    $effect.root(() => {
      $effect(() => {
        const prefs: Prefs = {
          theme: this.theme,
          font: this.font,
          density: this.density,
          motion: this.motion,
          accent: this.accent,
          fontScale: this.fontScale,
          glass: this.glass,
          layout: this.layout,
        };
        try {
          localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
        } catch {
          /* storage unavailable — session-only prefs */
        }
      });
    });
  }
}

export const aesthetics = new Aesthetics();
