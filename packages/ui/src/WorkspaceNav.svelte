<script lang="ts">
  let { active = "crypto" }: { active?: string } = $props();

  const destinations = [
    { id: "crypto", label: "Crypto", href: "/workspace" },
    { id: "equity", label: "Equity", href: "/workspace/equity" },
    { id: "options", label: "Options", href: "/workspace/options" },
    { id: "macro", label: "Macro", href: "/workspace/macro" },
    { id: "filings", label: "Filings", href: "/workspace/filings" },
    { id: "backtest", label: "Backtest", href: "/workspace/backtest" },
    { id: "strategy", label: "Strategy", href: "/workspace/strategy" },
    { id: "simulation", label: "Simulation", href: "/workspace/simulation" },
    { id: "calibration", label: "Calibration", href: "/workspace/calibration" },
    { id: "models", label: "Models", href: "/workspace/models" },
    { id: "analogs", label: "Analogs", href: "/workspace/analogs" },
    { id: "portfolio", label: "Portfolio", href: "/workspace/portfolio" },
    { id: "journal", label: "Journal", href: "/workspace/journal" },
    { id: "orders", label: "Orders", href: "/workspace/orders" },
    { id: "marketplace", label: "Marketplace", href: "/workspace/marketplace" },
  ];
</script>

<nav class="nav" aria-label="Workspace experiences">
  {#each destinations as destination, index}
    <a
      href={destination.href}
      class:active={active === destination.id}
      aria-current={active === destination.id ? "page" : undefined}
    >
      <span>{String(index + 1).padStart(2, "0")}</span>{destination.label}
    </a>
  {/each}
</nav>

<style>
  .nav { display: grid; gap: 2px; }
  a {
    display: flex;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    text-decoration: none;
  }
  a:hover,
  a.active {
    background: rgba(0, 240, 255, 0.07);
    color: var(--color-text-primary);
  }
  a span {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: 0.5rem;
  }
</style>
