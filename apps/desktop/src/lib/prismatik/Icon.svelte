<script lang="ts" module>
  /**
   * Tiny inline-SVG icon set (lucide-svelte is not a dependency of this
   * repo). Stroke-based 24x24 paths, `currentColor`, matching the repo's
   * WindowChrome inline-SVG convention.
   */
  export type IconName = 'settings' | 'wifi' | 'x' | 'minus' | 'square';

  const PATHS: Record<IconName, string> = {
    // gear: circle + 8 spokes
    settings:
      'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z' +
      'M12 2.5v2.6M12 18.9v2.6M2.5 12h2.6M18.9 12h2.6' +
      'M5.3 5.3l1.8 1.8M16.9 16.9l1.8 1.8M18.7 5.3l-1.8 1.8M7.1 16.9l-1.8 1.8',
    wifi: 'M2.5 9a15 15 0 0 1 19 0M5.5 12.5a10 10 0 0 1 13 0M8.6 16a5 5 0 0 1 6.8 0M12 19.5h.01',
    x: 'M5 5l14 14M19 5L5 19',
    minus: 'M5 12h14',
    square: 'M6 6h12v12H6z',
  };
</script>

<script lang="ts">
  let { name, size = 14, strokeWidth = 1.8 }: { name: IconName; size?: number; strokeWidth?: number } =
    $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={PATHS[name]} />
</svg>
