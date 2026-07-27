<script lang="ts">
  export type CommandItem = {
    id: string;
    label: string;
    hint?: string;
    group?: string;
  };

  let {
    open = $bindable(false),
    commands = [],
    extraCommands = [],
    onSelect,
    onQueryChange,
  }: {
    open?: boolean;
    commands?: CommandItem[];
    extraCommands?: CommandItem[];
    onSelect?: (command: CommandItem) => void;
    onQueryChange?: (query: string) => void;
  } = $props();

  let query = $state("");
  let active = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const locals = commands.filter((c) => {
      if (!q) return true;
      return (
        c.label.toLowerCase().includes(q) ||
        (c.hint?.toLowerCase().includes(q) ?? false) ||
        (c.group?.toLowerCase().includes(q) ?? false)
      );
    });
    const extras = extraCommands.filter((e) => !locals.some((l) => l.id === e.id));
    return [...locals, ...extras];
  });

  $effect(() => {
    if (!open) {
      query = "";
      active = 0;
    } else {
      queueMicrotask(() => inputEl?.focus());
    }
  });

  $effect(() => {
    onQueryChange?.(query);
  });

  $effect(() => {
    if (active >= filtered.length) active = Math.max(0, filtered.length - 1);
  });

  function choose(item: CommandItem) {
    onSelect?.(item);
    open = false;
  }

  function onKeydown(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === "Escape") {
      event.preventDefault();
      open = false;
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      active = Math.min(active + 1, Math.max(0, filtered.length - 1));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      active = Math.max(active - 1, 0);
      return;
    }
    if (event.key === "Enter" && filtered[active]) {
      event.preventDefault();
      choose(filtered[active]);
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="palette" role="dialog" aria-modal="true" aria-label="Command palette">
    <button class="palette__backdrop" type="button" aria-label="Close" onclick={() => (open = false)}></button>
    <div class="palette__panel">
      <input
        class="palette__input"
        type="search"
        placeholder="Search assets or run a command…"
        bind:value={query}
        bind:this={inputEl}
      />
      <ul class="palette__list" role="listbox">
        {#each filtered as item, index (item.id)}
          <li>
            <button
              type="button"
              class="palette__item"
              class:active={index === active}
              role="option"
              aria-selected={index === active}
              onclick={() => choose(item)}
              onmouseenter={() => (active = index)}
            >
              <span class="palette__label">
                {#if item.group}<span class="palette__group">{item.group}</span>{/if}
                {item.label}
              </span>
              {#if item.hint}<span class="palette__hint">{item.hint}</span>{/if}
            </button>
          </li>
        {:else}
          <li class="palette__empty">
            {query.trim() ? "No matching commands or assets" : "No commands"}
          </li>
        {/each}
      </ul>
    </div>
  </div>
{/if}

<style>
  .palette {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: start center;
    padding-top: 12vh;
    animation: palette-in var(--duration-base) var(--ease-standard) both;
  }
  .palette__backdrop {
    position: absolute;
    inset: 0;
    border: 0;
    background:
      radial-gradient(circle at 50% 18%, rgba(0,240,255,.05), transparent 34rem),
      rgba(1, 3, 7, .74);
    backdrop-filter: blur(8px);
    cursor: default;
  }
  .palette__panel {
    position: relative;
    width: min(620px, calc(100vw - 2rem));
    overflow: hidden;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-lg);
    background: rgba(9, 13, 22, .88);
    box-shadow: var(--shadow-panel), 0 0 80px -42px rgba(0,240,255,.65);
    backdrop-filter: blur(28px) saturate(170%);
  }
  .palette__input {
    width: 100%;
    padding: 20px;
    border: 0;
    border-bottom: 1px solid var(--color-border-default);
    background: transparent;
    color: var(--color-text-primary);
    font: inherit;
    font-size: var(--font-size-lg);
    outline: none;
  }
  .palette__list {
    max-height: 380px;
    margin: 0;
    padding: var(--space-3);
    overflow: auto;
    list-style: none;
  }
  .palette__item {
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: 11px 12px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-primary);
    cursor: pointer;
    text-align: left;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      border-color var(--duration-fast) var(--ease-standard),
      transform var(--duration-fast) var(--ease-standard);
  }
  .palette__item.active,
  .palette__item:hover {
    border-color: rgba(0,240,255,.09);
    background: linear-gradient(90deg, rgba(0,240,255,.08), rgba(168,85,247,.045));
    transform: translateX(2px);
  }
  .palette__label {
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-2);
    font-weight: var(--font-weight-medium);
  }
  .palette__group {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-regular);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .palette__hint {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
  }
  .palette__empty {
    padding: var(--space-4) var(--space-3);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-sm);
  }
  @keyframes palette-in {
    from { opacity: 0; transform: translateY(-8px) scale(.99); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }
</style>
