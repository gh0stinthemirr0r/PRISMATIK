<script lang="ts">
  /**
   * Primary navigation.
   *
   * Twenty-four destinations in one flat icon column gave no sense of where you
   * were or what was near what. They are grouped here by the question the desk
   * is asking — what is the market doing, what do we believe, what are we
   * testing, what are we doing about it — and the rail expands on hover so the
   * labels are readable without giving up the collapsed width.
   *
   * There is no "Visuals" destination: each visualization now lives in the
   * workspace whose data it draws, and no feed badge: that state is reported
   * once, in the top bar, rather than twice in two different vocabularies.
   */
  import {
    BrainCircuit,
    Boxes,
    Info,
    CandlestickChart,
    ChartNoAxesCombined,
    ClipboardList,
    BookOpen,
    Users,
    Radar,
    FlaskConical,
    Gauge,
    History,
    Landmark,
    Library,
    Network,
    Orbit,
    PlugZap,
    Puzzle,
    ScrollText,
    Settings2,
    ShieldCheck,
    TestTube2,
    Waypoints,
    Scale,
    Rss,
    Bot,
    Newspaper,
    Cpu,
    TrendingUp,
  } from 'lucide-svelte';
  import { aesthetics } from './aesthetics.svelte';
  import { page } from '$app/state';

  const sections = [
    {
      label: 'Markets',
      modules: [
        { label: 'Terminal', detail: 'Tracked instruments', href: '/workspace', icon: ChartNoAxesCombined },
        { label: 'Equities', detail: 'Security graph', href: '/workspace/equity', icon: CandlestickChart },
        { label: 'Screener', detail: 'Find candidates', href: '/workspace/screener', icon: Radar },
        { label: 'Options', detail: 'Volatility surface', href: '/workspace/options', icon: Orbit },
        { label: 'Macro', detail: 'Rates and regimes', href: '/workspace/macro', icon: Landmark },
        { label: 'Filings', detail: 'Institutional evidence', href: '/workspace/filings', icon: ScrollText },
      ],
    },
    {
      label: 'Intelligence',
      modules: [
        { label: 'Intelligence', detail: 'Evidence fabric', href: '/workspace/intelligence', icon: BrainCircuit },
        { label: 'News', detail: 'Reviewed sources', href: '/workspace/news', icon: Newspaper },
        { label: 'Feeds', detail: 'Source policy', href: '/workspace/feeds', icon: Rss },
        { label: 'Predictions', detail: 'Live model calls', href: '/workspace/predictions', icon: TrendingUp },
        { label: 'Markets', detail: 'Cross-venue truth', href: '/workspace/prediction', icon: Scale },
        { label: 'Analogs', detail: 'Temporal memory', href: '/workspace/analogs', icon: History },
      ],
    },
    {
      label: 'Research',
      modules: [
        { label: 'Strategy', detail: 'Agent authoring', href: '/workspace/strategy', icon: Network },
        { label: 'Backtest', detail: 'Temporal replay', href: '/workspace/backtest', icon: TestTube2 },
        { label: 'Simulation', detail: 'Deterministic lab', href: '/workspace/simulation', icon: FlaskConical },
        { label: 'Calibration', detail: 'Forecast truth', href: '/workspace/calibration', icon: Gauge },
        { label: 'Models', detail: 'Governed registry', href: '/workspace/models', icon: Boxes },
        { label: 'Council', detail: 'Agent debate', href: '/workspace/agent', icon: Bot },
        { label: 'Analysts', detail: 'Scored specialists', href: '/workspace/analysts', icon: Users },
        { label: 'Knowledge', detail: 'Institutional memory', href: '/workspace/knowledge', icon: BookOpen },
      ],
    },
    {
      label: 'Execution',
      modules: [
        { label: 'Trader', detail: 'Autonomous engine', href: '/workspace/trader', icon: Cpu },
        { label: 'Orders', detail: 'Guarded execution', href: '/workspace/orders', icon: Waypoints },
        { label: 'Risk', detail: 'Limits and exposure', href: '/workspace/portfolio', icon: ShieldCheck },
        { label: 'Agent log', detail: 'What it did and why', href: '/workspace/agent-log', icon: ClipboardList },
        { label: 'Journal', detail: 'Decision memory', href: '/workspace/journal', icon: Library },
      ],
    },
    {
      label: 'System',
      modules: [
        { label: 'Integrations', detail: 'Provider plane', href: '/workspace/integrations', icon: PlugZap },
        { label: 'Autonomy', detail: 'Budgets and gates', href: '/workspace/autonomy', icon: Settings2 },
        { label: 'Extensions', detail: 'Native ecosystem', href: '/workspace/marketplace', icon: Puzzle },
        { label: 'About', detail: 'Build and provenance', href: '/workspace/about', icon: Info },
      ],
    },
  ];

  function isActive(href: string): boolean {
    return (
      page.url.pathname === href ||
      (href !== '/workspace' && page.url.pathname.startsWith(`${href}/`))
    );
  }

</script>

<nav class="pk-mission" aria-label="PRISMATIK systems">
  <div class="pk-mission-items">
    {#each sections as section (section.label)}
      <div class="pk-mission-section">
        <span class="pk-mission-section-label" aria-hidden="true">{section.label}</span>
        {#each section.modules as module (module.href)}
          <a
            href={module.href}
            class:active={isActive(module.href)}
            title={`${module.label} — ${module.detail}`}
          >
            <module.icon size={16} strokeWidth={1.6} />
            <span><b>{module.label}</b><small>{module.detail}</small></span>
          </a>
        {/each}
      </div>
    {/each}
  </div>

  <button
    class="pk-mission-config"
    onclick={() => (aesthetics.panelOpen = true)}
    title="Open appearance controls"
    aria-label="Open appearance controls"
  >
    <Settings2 size={16} />
  </button>
</nav>

<style>
  /* Collapsed the rail is an icon column; on hover or keyboard focus it widens
     in place so labels and section headers become readable. `width` is animated
     rather than the grid column so the workspace does not reflow. */
  .pk-mission {
    position: relative;
    z-index: 40;
    width: 56px;
    transition: width 160ms ease;
  }
  .pk-mission:hover,
  .pk-mission:focus-within {
    width: 186px;
    box-shadow: 8px 0 26px rgba(0, 0, 0, 0.34);
  }
  @media (prefers-reduced-motion: reduce) {
    .pk-mission {
      transition: none;
    }
  }

  /* Collapsed, a section is a 44px content column whose children are fixed at
     42px. Without centring here they sit flush left while the orbit badge and
     settings button — direct children of the centred rail — sit a pixel in, so
     the icon column visibly fails to line up. */
  .pk-mission-section {
    display: flex;
    width: 100%;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .pk-mission:hover .pk-mission-section,
  .pk-mission:focus-within .pk-mission-section {
    align-items: stretch;
  }
  .pk-mission-section + .pk-mission-section {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--p-border);
  }
  .pk-mission-section-label {
    height: 0;
    overflow: hidden;
    padding: 0 8px;
    color: var(--p-dim);
    font: 700 8px var(--font-mono);
    letter-spacing: 0.18em;
    opacity: 0;
    text-transform: uppercase;
    transition:
      height 160ms ease,
      opacity 160ms ease;
  }
  .pk-mission:hover .pk-mission-section-label,
  .pk-mission:focus-within .pk-mission-section-label {
    height: 15px;
    opacity: 0.75;
  }

  /* Labels: hidden while collapsed, revealed with the rail. */
  .pk-mission :global(a span) {
    display: none;
  }
  .pk-mission:hover :global(a span),
  .pk-mission:focus-within :global(a span) {
    display: flex;
    min-width: 0;
    flex-direction: column;
  }
  .pk-mission :global(a span b) {
    font: 600 10.5px var(--font-ui);
    letter-spacing: 0.02em;
  }
  .pk-mission :global(a span small) {
    overflow: hidden;
    color: var(--p-dim);
    font-size: 8.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Expanded, every control shares one icon column so the glyphs stay on a
     single vertical axis. The orbit badge and settings button are included:
     leaving them 42px-centred while the links go full width put them visibly
     off-axis from everything between them. */
  .pk-mission:hover :global(.pk-mission-items a),
  .pk-mission:focus-within :global(.pk-mission-items a),
  .pk-mission:hover :global(.pk-mission-config),
  .pk-mission:focus-within :global(.pk-mission-config) {
    width: 100%;
    justify-content: start;
    grid-auto-flow: column;
    grid-template-columns: 16px minmax(0, 1fr);
    gap: 9px;
    padding: 0 8px;
    place-items: center start;
  }
  /* The section header sits over the icon column, not over the label column. */
  .pk-mission-section-label {
    align-self: stretch;
  }

</style>
