<script lang="ts" generics="T">
  import type { Snippet } from "svelte";

  let {
    items,
    rowHeight = 48,
    height = 480,
    overscan = 5,
    children,
  }: {
    items: T[];
    rowHeight?: number;
    height?: number;
    overscan?: number;
    children: Snippet<[T, number]>;
  } = $props();

  let scrollTop = $state(0);
  const start = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - overscan));
  const visibleCount = $derived(Math.ceil(height / rowHeight) + overscan * 2);
  const end = $derived(Math.min(items.length, start + visibleCount));
  const visibleItems = $derived(items.slice(start, end));
</script>

<div
  class="virtual-list"
  style={`height:${height}px`}
  onscroll={(event) => (scrollTop = event.currentTarget.scrollTop)}
>
  <div class="virtual-list__spacer" style={`height:${items.length * rowHeight}px`}>
    <div
      class="virtual-list__window"
      style={`transform:translateY(${start * rowHeight}px)`}
    >
      {#each visibleItems as item, index}
        <div class="virtual-list__row" style={`height:${rowHeight}px`}>
          {@render children(item, start + index)}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .virtual-list {
    overflow: auto;
    contain: strict;
  }
  .virtual-list__spacer {
    position: relative;
  }
  .virtual-list__window {
    position: absolute;
    inset: 0 0 auto;
    will-change: transform;
  }
  .virtual-list__row {
    overflow: hidden;
  }
</style>
