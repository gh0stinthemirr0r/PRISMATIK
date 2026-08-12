<script lang="ts">
  import TerminalFrame from '$lib/prismatik/TerminalFrame.svelte';
  let { children } = $props();
</script>

<TerminalFrame>{@render children()}</TerminalFrame>
