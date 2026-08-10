<script lang="ts">
  import { Activity, BrainCircuit, Boxes, CandlestickChart, ChartNoAxesCombined, FlaskConical, Gauge, History, Landmark, Library, Network, Orbit, PlugZap, Puzzle, ScrollText, Settings2, ShieldCheck, TestTube2, Waypoints, Scale, Rss, Bot, Newspaper, Cpu } from 'lucide-svelte';
  import { aesthetics } from './aesthetics.svelte';
  import { page } from '$app/state';

  const modules = [
    { label: 'Markets', detail: 'Multi-asset pulse', href: '/workspace', icon: ChartNoAxesCombined },
    { label: 'Equities', detail: 'Security graph', href: '/workspace/equity', icon: CandlestickChart },
    { label: 'Trader', detail: 'Autonomous engine', href: '/workspace/trader', icon: Cpu },
    { label: 'Agent', detail: 'Council debate', href: '/workspace/agent', icon: Bot },
    { label: 'Briefings', detail: 'Daily digest', href: '/workspace/briefings', icon: Newspaper },
    { label: 'Intelligence', detail: 'Evidence fabric', href: '/workspace/intelligence', icon: BrainCircuit },
    { label: 'Feeds', detail: 'Source attention', href: '/workspace/feeds', icon: Rss },
    { label: 'Prediction', detail: 'Cross-venue truth', href: '/workspace/prediction', icon: Scale },
    { label: 'Strategy', detail: 'Agent authoring', href: '/workspace/strategy', icon: Network },
    { label: 'Simulation', detail: 'Deterministic lab', href: '/workspace/simulation', icon: FlaskConical },
    { label: 'Backtest', detail: 'Temporal replay', href: '/workspace/backtest', icon: TestTube2 },
    { label: 'Risk', detail: 'Limits & exposure', href: '/workspace/portfolio', icon: ShieldCheck },
    { label: 'Orders', detail: 'Guarded execution', href: '/workspace/orders', icon: Waypoints },
    { label: 'Options', detail: 'Volatility surface', href: '/workspace/options', icon: Orbit },
    { label: 'Macro', detail: 'Global regimes', href: '/workspace/macro', icon: Landmark },
    { label: 'Filings', detail: 'Institutional evidence', href: '/workspace/filings', icon: ScrollText },
    { label: 'Calibration', detail: 'Forecast truth', href: '/workspace/calibration', icon: Gauge },
    { label: 'Models', detail: 'Governed registry', href: '/workspace/models', icon: Boxes },
    { label: 'Analogs', detail: 'Temporal memory', href: '/workspace/analogs', icon: History },
    { label: 'Journal', detail: 'Decision memory', href: '/workspace/journal', icon: Library },
    { label: 'Integrations', detail: 'Provider plane', href: '/workspace/integrations', icon: PlugZap },
    { label: 'Autonomy', detail: 'Budgets & gates', href: '/workspace/autonomy', icon: Settings2 },
    { label: 'Extensions', detail: 'Native ecosystem', href: '/workspace/marketplace', icon: Puzzle },
  ];
</script>

<nav class="pk-mission" aria-label="PRISMATIK systems">
  <div class="pk-mission-orbit" title="Autonomy state">
    <Activity size={15} />
    <span>SIM</span>
  </div>
  <div class="pk-mission-items">
    {#each modules as module (module.href)}
      <a href={module.href} class:active={page.url.pathname === module.href || (module.href !== '/workspace' && page.url.pathname.startsWith(`${module.href}/`))} title={`${module.label} — ${module.detail}`}>
        <module.icon size={16} strokeWidth={1.6} />
        <span><b>{module.label}</b><small>{module.detail}</small></span>
      </a>
    {/each}
  </div>
  <button class="pk-mission-config" onclick={() => (aesthetics.panelOpen = true)} title="Open appearance controls" aria-label="Open appearance controls">
    <Settings2 size={16} />
  </button>
</nav>
