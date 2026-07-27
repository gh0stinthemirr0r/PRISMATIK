<script lang="ts">
  let {
    eventTime,
    maxAge,
    label = "Stale",
  }: {
    eventTime: string;
    maxAge: number;
    label?: string;
  } = $props();

  const ageMs = $derived(Date.now() - new Date(eventTime).getTime());
  const isStale = $derived(Number.isFinite(ageMs) && ageMs > maxAge);
  const ageLabel = $derived(formatDuration(ageMs));

  function formatDuration(milliseconds: number): string {
    const minutes = Math.floor(milliseconds / 60_000);
    if (minutes < 1) return "<1m";
    if (minutes < 60) return `${minutes}m`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ${minutes % 60}m`;
  }
</script>

{#if isStale}
  <span class="stale" role="status" title={`Data is ${ageLabel} old`}>
    {label} · {ageLabel}
  </span>
{/if}

<style>
  .stale {
    color: var(--color-stale);
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
</style>
