<script lang="ts">
  let { active = "crypto" }: { active?: string } = $props();

  const destinations = [
    { id: "crypto", label: "Crypto", href: "/workspace" },
    { id: "intelligence", label: "Intelligence", href: "/workspace/intelligence" },
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
    { id: "integrations", label: "Integrations", href: "/workspace/integrations" },
  ];
</script>

<nav class="nav" aria-label="Workspace experiences">
  {#each destinations as destination, index}
    <a
      href={destination.href}
      class:utility={destination.id === "integrations"}
      class:active={active === destination.id}
      aria-current={active === destination.id ? "page" : undefined}
    >
      <span class="nav__index">{String(index + 1).padStart(2, "0")}</span>
      <span>{destination.label}</span>
      {#if active === destination.id}<i aria-hidden="true"></i>{/if}
    </a>
  {/each}
</nav>

<style>
  .nav {
    display: grid;
    gap: 2px;
  }
  a {
    position: relative;
    display: grid;
    grid-template-columns: 24px 1fr auto;
    align-items: center;
    gap: 7px;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    text-decoration: none;
    transition:
      background-color var(--duration-base) var(--ease-standard),
      color var(--duration-base) var(--ease-standard),
      transform var(--duration-base) var(--ease-standard);
  }
  a:hover { background: rgba(255,255,255,.035); color: var(--color-text-primary); transform: translateX(2px); }
  a.active {
    border: 1px solid rgba(0, 240, 255, 0.1);
    background: linear-gradient(90deg, rgba(0, 240, 255, 0.09), rgba(168, 85, 247, 0.035));
    color: var(--color-text-primary);
    box-shadow: inset 2px 0 0 rgba(0,240,255,.52);
  }
  a.utility {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--color-border-default);
    border-radius: 0 0 var(--radius-md) var(--radius-md);
  }
  a.utility.active {
    border-top-color: var(--color-border-default);
  }
  .nav__index {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: 0.5rem;
    letter-spacing: .03em;
  }
  a.active .nav__index { color: var(--color-brand-primary); }
  i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--color-brand-primary);
    box-shadow: 0 0 9px var(--color-brand-primary);
  }
</style>
