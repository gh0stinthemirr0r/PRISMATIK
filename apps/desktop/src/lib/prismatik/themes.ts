/**
 * PRISMATIK — theme / font / accent definitions.
 *
 * Pure data module (no runes). Reactive aesthetics state + persistence
 * lives in `aesthetics.svelte.ts`. Each theme ships a full palette of CSS
 * custom-property values; the same palette object is consumed directly by
 * the canvas renderers so charts always match the DOM theme.
 */

export interface ThemeDef {
  id: string;
  label: string;
  blurb: string;
  family?: string;
  vars: Record<string, string>;
}

export interface FontDef {
  id: string;
  label: string;
  family: string;
  blurb: string;
}

export interface AccentDef {
  id: string;
  label: string;
  /** null → use the theme's own accent colors. */
  color: string | null;
  color2: string | null;
}

export const MONO_STACK =
  "'IBM Plex Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";

function palette(
  id: string,
  label: string,
  blurb: string,
  family: string,
  colors: [string, string, string, string, string, string, string, string, string, string, string],
): ThemeDef {
  const [bg, bg2, surface, surface2, border, text, dim, accent, accent2, up, down] = colors;
  return {
    id, label, blurb, family,
    vars: {
      '--p-bg': bg, '--p-bg2': bg2, '--p-surface': surface, '--p-surface2': surface2,
      '--p-border': border, '--p-text': text, '--p-dim': dim, '--p-accent': accent,
      '--p-accent2': accent2, '--p-up': up, '--p-down': down,
      '--p-glow': `${accent}55`, '--p-grid': `${accent}18`,
    },
  };
}

export const THEMES: ThemeDef[] = [
  {
    id: 'cyberpunk',
    label: 'Cyberpunk',
    blurb: 'Toxic lime, ultraviolet glass',
    family: 'Neon / Retro',
    vars: {
      '--p-bg': '#030806',
      '--p-bg2': '#07110d',
      '--p-surface': '#0a1712',
      '--p-surface2': '#10231a',
      '--p-border': '#214936',
      '--p-text': '#eafff2',
      '--p-dim': '#6d9a82',
      '--p-accent': '#b7ff25',
      '--p-accent2': '#a855f7',
      '--p-up': '#5cffac',
      '--p-down': '#ff3d8d',
      '--p-glow': 'rgba(183, 255, 37, 0.34)',
      '--p-grid': 'rgba(183, 255, 37, 0.09)',
    },
  },
  {
    id: 'dark',
    label: 'Darkroom',
    blurb: 'Near-black, electric signal',
    vars: {
      '--p-bg': '#06070a',
      '--p-bg2': '#0a0c11',
      '--p-surface': '#0e1118',
      '--p-surface2': '#141926',
      '--p-border': '#1d2433',
      '--p-text': '#dfe6f2',
      '--p-dim': '#6b7690',
      '--p-accent': '#4dc8ff',
      '--p-accent2': '#8b7bff',
      '--p-up': '#23e08b',
      '--p-down': '#ff4d67',
      '--p-glow': 'rgba(77, 200, 255, 0.35)',
      '--p-grid': 'rgba(120, 140, 175, 0.10)',
    },
  },
  {
    id: 'light',
    label: 'Daylight',
    blurb: 'Paper-white, cool ink',
    vars: {
      '--p-bg': '#eef0f4',
      '--p-bg2': '#e4e7ee',
      '--p-surface': '#ffffff',
      '--p-surface2': '#f3f5f9',
      '--p-border': '#d5dae4',
      '--p-text': '#161b26',
      '--p-dim': '#68738a',
      '--p-accent': '#1d5fd6',
      '--p-accent2': '#6a4fd0',
      '--p-up': '#0a9d5c',
      '--p-down': '#d42a3e',
      '--p-glow': 'rgba(29, 95, 214, 0.18)',
      '--p-grid': 'rgba(40, 60, 100, 0.10)',
    },
  },
  {
    id: 'grey',
    label: 'Monochrome',
    blurb: 'Fifty shades of signal',
    vars: {
      '--p-bg': '#131416',
      '--p-bg2': '#18191c',
      '--p-surface': '#1c1d21',
      '--p-surface2': '#24262b',
      '--p-border': '#31343b',
      '--p-text': '#e4e4e6',
      '--p-dim': '#7c7f88',
      '--p-accent': '#e8e8ec',
      '--p-accent2': '#9a9da6',
      '--p-up': '#f2f2f4',
      '--p-down': '#63666e',
      '--p-glow': 'rgba(232, 232, 236, 0.14)',
      '--p-grid': 'rgba(200, 200, 210, 0.07)',
    },
  },
  {
    id: 'synthwave',
    label: 'Synthwave',
    blurb: 'Neon grid horizon',
    family: 'Neon / Retro',
    vars: {
      '--p-bg': '#0c0218',
      '--p-bg2': '#140328',
      '--p-surface': '#180632',
      '--p-surface2': '#21094a',
      '--p-border': '#3a1268',
      '--p-text': '#f2e8ff',
      '--p-dim': '#9a7cc4',
      '--p-accent': '#ff2ec4',
      '--p-accent2': '#00f0ff',
      '--p-up': '#00ffa3',
      '--p-down': '#ff3d71',
      '--p-glow': 'rgba(255, 46, 196, 0.45)',
      '--p-grid': 'rgba(0, 240, 255, 0.10)',
    },
  },
  palette('synthwave-84', 'Synthwave \'84', 'Sunset highway, chrome palm trees', 'Neon / Retro', ['#1a0b2e', '#0f0620', '#241042', '#2d1659', '#4a1f7c', '#fce4ec', '#b08bc7', '#ff006e', '#fb5607', '#06ffa5', '#ff4365']),
  palette('outrun', 'Outrun', 'Magnum opus: magenta sun, chrome grid', 'Neon / Retro', ['#0a0015', '#0d0020', '#14002e', '#1c0042', '#2e006b', '#e8d5ff', '#8e6db5', '#ff10f0', '#00ffff', '#39ff14', '#ff003c']),
  palette('vaporwave', 'Vaporwave', 'Pastel pink, teal statue, broken grid', 'Neon / Retro', ['#1e0b2e', '#170728', '#2a1245', '#371c5e', '#5a2d8a', '#fde4ff', '#b89dc9', '#ff71ce', '#01cdfe', '#05ffa1', '#ff6ac1']),
  palette('blade-runner', 'Blade Runner', 'Acid rain, orange neon, corporate decay', 'Neon / Retro', ['#0c0a08', '#14100b', '#1c1712', '#261f18', '#3d3328', '#fff4e0', '#998a72', '#ff8c00', '#ff4500', '#7fff00', '#ff2020']),
  palette('cyberpunk-night-city', 'Night City', 'Wake the flame, samurai', 'Neon / Retro', ['#060a0e', '#04080c', '#0a1016', '#111922', '#1d3344', '#d9f2ff', '#5c8ca5', '#fcee0a', '#ff003c', '#00f5d4', '#ff206e']),
  palette('cyberpunk-blade', 'Cyber Blade', 'Cold steel, electric blue edge', 'Neon / Retro', ['#050810', '#030610', '#0a1020', '#101830', '#1e2d4a', '#e0f0ff', '#5a7090', '#00bfff', '#4169e1', '#00ff7f', '#ff1493']),
  palette('tron', 'Tron', 'Grid of light, black vacuum', 'Neon / Retro', ['#000000', '#000005', '#000814', '#001020', '#002040', '#c0e0ff', '#4080a0', '#00ffff', '#0080ff', '#00ff80', '#ff4040']),
  palette('neon-noir', 'Neon Noir', 'Rain-soaked streetlight, tequila sunset', 'Neon / Retro', ['#0a0608', '#0d070a', '#14090e', '#1c0e14', '#2e1822', '#ffe0e8', '#8a6878', '#ff1744', '#ff6e40', '#1de9b6', '#e91e63']),
  palette('miami-vice', 'Miami Vice', 'Flamingo pink, ocean teal, pastel suit', 'Neon / Retro', ['#0d0518', '#0f0620', '#180a30', '#200e42', '#381860', '#ffe5f0', '#a080b0', '#ff4081', '#00bcd4', '#69f0ae', '#ff5252']),
  palette('grid-2049', 'Grid 2049', 'Holographic amber, dust and fog', 'Neon / Retro', ['#08070a', '#0c0b10', '#121018', '#1a181f', '#2a2630', '#e8e0f0', '#706878', '#ffa500', '#ff6347', '#20b2aa', '#dc143c']),
  {
    id: 'business',
    label: 'Business',
    blurb: 'Navy, steel, old gold',
    vars: {
      '--p-bg': '#0a1522',
      '--p-bg2': '#0d1b2c',
      '--p-surface': '#102134',
      '--p-surface2': '#16293f',
      '--p-border': '#223850',
      '--p-text': '#dce6f0',
      '--p-dim': '#7c92a8',
      '--p-accent': '#d4af37',
      '--p-accent2': '#5b8fc9',
      '--p-up': '#38c172',
      '--p-down': '#e3344b',
      '--p-glow': 'rgba(212, 175, 55, 0.22)',
      '--p-grid': 'rgba(120, 150, 190, 0.10)',
    },
  },
  {
    id: 'classics',
    label: 'Classics',
    blurb: 'Sepia tape & serif type',
    vars: {
      '--p-bg': '#221b10',
      '--p-bg2': '#2a2114',
      '--p-surface': '#efe4c8',
      '--p-surface2': '#e6d8b4',
      '--p-border': '#8a764e',
      '--p-text': '#2e2413',
      '--p-dim': '#7a6a48',
      '--p-accent': '#8a5a1e',
      '--p-accent2': '#4d5a30',
      '--p-up': '#3d6b2f',
      '--p-down': '#9c2b1e',
      '--p-glow': 'rgba(138, 90, 30, 0.20)',
      '--p-grid': 'rgba(60, 45, 20, 0.12)',
    },
  },
  palette('cat-latte', 'Catppuccin Latte', 'Rosewater on warm daylight', 'Soft / Pastels', ['#eff1f5','#e6e9ef','#ffffff','#e6e9ef','#bcc0cc','#4c4f69','#8c8fa1','#8839ef','#1e66f5','#40a02b','#d20f39']),
  palette('cat-frappe', 'Catppuccin Frappé', 'Cool pastel dusk', 'Soft / Pastels', ['#303446','#292c3c','#414559','#51576d','#626880','#c6d0f5','#949cbb','#ca9ee6','#8caaee','#a6d189','#e78284']),
  palette('cat-macchiato', 'Catppuccin Macchiato', 'Deep muted lavender', 'Soft / Pastels', ['#24273a','#1e2030','#363a4f','#494d64','#5b6078','#cad3f5','#939ab7','#c6a0f6','#8aadf4','#a6da95','#ed8796']),
  palette('cat-mocha', 'Catppuccin Mocha', 'Darkest pastel velvet', 'Soft / Pastels', ['#1e1e2e','#181825','#313244','#45475a','#585b70','#cdd6f4','#9399b2','#cba6f7','#89b4fa','#a6e3a1','#f38ba8']),
  palette('tokyo-storm', 'Tokyo Night Storm', 'Rain-blue metropolitan glass', 'Soft / Pastels', ['#24283b','#1f2335','#292e42','#3b4261','#414868','#c0caf5','#737aa2','#7aa2f7','#bb9af7','#9ece6a','#f7768e']),
  palette('tokyo-night', 'Tokyo Night', 'Electric midnight skyline', 'Soft / Pastels', ['#1a1b26','#16161e','#202330','#292e42','#3b4261','#c0caf5','#565f89','#7aa2f7','#bb9af7','#9ece6a','#f7768e']),
  palette('tokyo-day', 'Tokyo Night Day', 'Cool metropolitan daylight', 'Soft / Pastels', ['#d5d6db','#cbccd1','#e1e2e7','#c4c8da','#a8aecb','#3760bf','#6172b0','#2e7de9','#9854f1','#587539','#f52a65']),
  palette('nord', 'Nord', 'Arctic blue-gray focus', 'Soft / Pastels', ['#2e3440','#272c36','#3b4252','#434c5e','#4c566a','#eceff4','#8fbcbb','#88c0d0','#81a1c1','#a3be8c','#bf616a']),
  palette('rose-pine', 'Rosé Pine', 'Muted rose and pine night', 'Soft / Pastels', ['#191724','#12101c','#1f1d2e','#26233a','#403d52','#e0def4','#908caa','#ebbcba','#c4a7e7','#9ccfd8','#eb6f92']),
  palette('rose-pine-moon', 'Rosé Pine Moon', 'Lunar violet and vintage pink', 'Soft / Pastels', ['#232136','#1d1b2b','#2a273f','#393552','#44415a','#e0def4','#908caa','#ea9a97','#c4a7e7','#9ccfd8','#eb6f92']),
  palette('rose-pine-dawn', 'Rosé Pine Dawn', 'Warm paper and dusty rose', 'Soft / Pastels', ['#faf4ed','#f2e9e1','#fffaf3','#f2e9de','#dfdad9','#575279','#9893a5','#d7827e','#907aa9','#56949f','#b4637a']),
  palette('dracula', 'Dracula', 'Vivid violet nocturne', 'Vibrant Dark', ['#282a36','#21222c','#30323f','#44475a','#6272a4','#f8f8f2','#8f9bbc','#bd93f9','#8be9fd','#50fa7b','#ff5555']),
  palette('one-dark', 'One Dark Pro', 'Balanced Atom blues', 'Vibrant Dark', ['#282c34','#21252b','#2c313a','#3a3f4b','#4b5263','#abb2bf','#7f848e','#61afef','#c678dd','#98c379','#e06c75']),
  palette('monokai', 'Monokai', 'Charcoal and punch neon', 'Vibrant Dark', ['#272822','#1e1f1c','#2f3029','#3e3d32','#535447','#f8f8f2','#90908a','#f92672','#66d9ef','#a6e22e','#fd5ff0']),
  palette('monokai-pro', 'Monokai Pro', 'Refined warm neon carbon', 'Vibrant Dark', ['#2d2a2e','#221f22','#363337','#403e41','#5b595c','#fcfcfa','#939293','#ffd866','#ab9df2','#a9dc76','#ff6188']),
  palette('night-owl', 'Night Owl', 'Accessible low-light cobalt', 'Vibrant Dark', ['#011627','#00111d','#0b2942','#123b56','#1d3b53','#d6deeb','#637777','#82aaff','#c792ea','#addb67','#ef5350']),
  palette('light-owl', 'Light Owl', 'Accessible cool daylight', 'Vibrant Dark', ['#fbfbfb','#f0f0f0','#ffffff','#e8e8e8','#d3d3d3','#403f53','#90a7b2','#4876d6','#994cc3','#2aa298','#de3d3b']),
  palette('solarized-dark', 'Solarized Dark', 'Precision deep teal', 'Warm & Earthy', ['#002b36','#00212b','#073642','#0b4653','#586e75','#eee8d5','#839496','#b58900','#268bd2','#859900','#dc322f']),
  palette('solarized-light', 'Solarized Light', 'Precision warm parchment', 'Warm & Earthy', ['#fdf6e3','#eee8d5','#fffaf0','#eee8d5','#93a1a1','#586e75','#839496','#b58900','#268bd2','#859900','#dc322f']),
  palette('gruvbox', 'Gruvbox', 'Retro amber workshop', 'Warm & Earthy', ['#282828','#1d2021','#32302f','#3c3836','#504945','#ebdbb2','#928374','#fabd2f','#d3869b','#b8bb26','#fb4934']),
  palette('kanagawa', 'Kanagawa Wave', 'Ink, wave and maple', 'Warm & Earthy', ['#1f1f28','#16161d','#2a2a37','#363646','#54546d','#dcd7ba','#727169','#7e9cd8','#957fb8','#98bb6c','#e46876']),
  palette('everforest', 'Everforest', 'Quiet moss and forest loam', 'Warm & Earthy', ['#2d353b','#272e33','#343f44','#3d484d','#4f585e','#d3c6aa','#859289','#a7c080','#7fbbb3','#a7c080','#e67e80']),
  palette('github-dark', 'GitHub Dark', 'Functional neutral graphite', 'Minimal', ['#0d1117','#010409','#161b22','#21262d','#30363d','#e6edf3','#8b949e','#2f81f7','#a371f7','#3fb950','#f85149']),
  palette('github-light', 'GitHub Light', 'Functional clean canvas', 'Minimal', ['#ffffff','#f6f8fa','#ffffff','#f6f8fa','#d0d7de','#1f2328','#656d76','#0969da','#8250df','#1a7f37','#cf222e']),
  palette('vesper', 'Vesper', 'Black glass and warm amber', 'Minimal', ['#101010','#080808','#171717','#202020','#333333','#ffffff','#8b8b8b','#ffc799','#99ffe4','#8ff0a4','#ff8080']),
  palette('ayer-dark', 'Ayer Dark', 'Near-monochrome high clarity', 'Minimal', ['#111111','#090909','#181818','#222222','#393939','#f2f2f2','#888888','#d7d7d7','#9e9e9e','#ededed','#707070']),
  palette('ayer-light', 'Ayer Light', 'Gray-white editorial clarity', 'Minimal', ['#f7f7f7','#eeeeee','#ffffff','#e8e8e8','#cccccc','#202020','#777777','#333333','#777777','#181818','#999999']),
];

export const FONTS: FontDef[] = [
  {
    id: 'mono',
    label: 'Plex Mono',
    family: MONO_STACK,
    blurb: 'IBM Plex Mono — the tape native',
  },
  {
    id: 'sans',
    label: 'Plex Sans',
    family: "'IBM Plex Sans', system-ui, -apple-system, sans-serif",
    blurb: 'IBM Plex Sans — quiet and precise',
  },
  {
    id: 'system',
    label: 'System Grotesk',
    family: "system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif",
    blurb: 'The OS voice, geometric',
  },
  {
    id: 'condensed',
    label: 'Condensed Display',
    family: "'Arial Narrow', 'Franklin Gothic Medium', 'Liberation Sans Narrow', sans-serif",
    blurb: 'Compressed display energy',
  },
  {
    id: 'serif',
    label: 'Newsprint Serif',
    family: "Georgia, 'Iowan Old Style', 'Times New Roman', serif",
    blurb: 'Old broadsheet, new terminal',
  },
];

export const ACCENTS: AccentDef[] = [
  { id: 'theme', label: 'Theme default', color: null, color2: null },
  { id: 'cyan', label: 'Ion Cyan', color: '#22d3ee', color2: '#818cf8' },
  { id: 'magenta', label: 'Hot Magenta', color: '#f0f', color2: '#00e5ff' },
  { id: 'gold', label: 'Bullion', color: '#e5b93b', color2: '#7ea4d4' },
  { id: 'lime', label: 'Acid Lime', color: '#a3e635', color2: '#34d399' },
];

export type Density = 'compact' | 'comfortable';
export type Motion = 'full' | 'reduced';
export type FontScale = 'small' | 'standard' | 'large' | 'xl';
export type GlassLevel = 'subtle' | 'balanced' | 'crystal';
export type LayoutMode = 'focus' | 'desk' | 'panorama';

export const DENSITIES: { id: Density; label: string }[] = [
  { id: 'compact', label: 'Compact' },
  { id: 'comfortable', label: 'Comfortable' },
];

export const MOTIONS: { id: Motion; label: string }[] = [
  { id: 'full', label: 'Full motion' },
  { id: 'reduced', label: 'Reduced' },
];

export const FONT_SCALES: { id: FontScale; label: string; value: string }[] = [
  { id: 'small', label: 'S', value: '0.88' },
  { id: 'standard', label: 'M', value: '1' },
  { id: 'large', label: 'L', value: '1.14' },
  { id: 'xl', label: 'XL', value: '1.3' },
];

export const GLASS_LEVELS: { id: GlassLevel; label: string; opacity: string; blur: string }[] = [
  { id: 'subtle', label: 'Subtle', opacity: '0.94', blur: '8px' },
  { id: 'balanced', label: 'Balanced', opacity: '0.78', blur: '18px' },
  { id: 'crystal', label: 'Crystal', opacity: '0.58', blur: '30px' },
];

export const LAYOUTS: { id: LayoutMode; label: string }[] = [
  { id: 'focus', label: 'Focus' },
  { id: 'desk', label: 'Desk' },
  { id: 'panorama', label: 'Panorama' },
];
