<script lang="ts">
  import { ArrowRight, CircleDashed, DatabaseZap, FlaskConical, Orbit, Play, ScanSearch, ShieldCheck } from 'lucide-svelte';
  import { market, fmtPct, fmtPrice } from '$lib/prismatik/market.svelte';

  let {
    active,
    title,
    description,
    requirement = "A governed live provider adapter must be connected before this surface can display data.",
  }: { active: string; title: string; description: string; requirement?: string } = $props();

  const blueprints: Record<string, { eyebrow: string; panels: [string, string][]; action: string }> = {
    equity: { eyebrow: 'GLOBAL SECURITY GRAPH', action: 'Open instrument universe', panels: [['Market structure', 'Session-aware price, volume, breadth and factor context.'], ['Company evidence', 'Fundamentals, estimates, ownership and filing lineage.'], ['Signal lattice', 'Cross-horizon momentum, quality, regime and anomaly state.']] },
    options: { eyebrow: 'VOLATILITY LABORATORY', action: 'Configure options source', panels: [['Volatility surface', 'Moneyness, tenor, skew, term structure and uncertainty.'], ['Flow topology', 'Trades, quotes, open interest and executable-liquidity context.'], ['Dealer state', 'Scenario gamma, vanna, charm and expiry concentration.']] },
    macro: { eyebrow: 'GLOBAL REGIME ENGINE', action: 'Configure macro sources', panels: [['Release matrix', 'Vintage-safe observations, consensus, surprise and revision state.'], ['Regime field', 'Growth, inflation, liquidity and policy transition probabilities.'], ['Cross-asset transmission', 'Rates, FX, commodities, credit and equity factor response.']] },
    filings: { eyebrow: 'INSTITUTIONAL EVIDENCE', action: 'Configure SEC EDGAR', panels: [['Ownership graph', '13F positions, changes, concentration and filing provenance.'], ['Insider ledger', 'Issuer transactions with role, timing and source evidence.'], ['Document intelligence', 'Point-in-time facts, deltas, citations and semantic risk.']] },
    backtest: { eyebrow: 'DETERMINISTIC REPLAY', action: 'Create research run', panels: [['Strategy contract', 'Typed inputs, capabilities, parameters and pinned artifacts.'], ['Execution model', 'Calendars, fees, spread, impact and fill assumptions.'], ['Validation chamber', 'Walk-forward splits, leakage gates, calibration and manifest.']] },
    strategy: { eyebrow: 'AUTONOMOUS STRATEGY FORGE', action: 'Author Strategy IR', panels: [['Agent council', 'Bull, bear, risk and evidence roles with typed disagreement.'], ['Constraint lattice', 'Universe, exposure, turnover, loss and autonomy boundaries.'], ['Promotion path', 'Validate, replay, paper soak and explicit execution approval.']] },
    simulation: { eyebrow: 'PROBABILISTIC SCENARIO CHAMBER', action: 'Start simulation run', panels: [['Regime composer', 'Deterministic shocks, transitions, correlations and scenario weights.'], ['Path observatory', 'Distribution fans, drawdowns, tails and path-dependent exposure.'], ['Decision stress', 'Strategy response, limits, failure states and recovery policy.']] },
    calibration: { eyebrow: 'FORECAST TRUTH ENGINE', action: 'Open calibration run', panels: [['Reliability field', 'Brier score, calibration bins, ECE and confidence intervals.'], ['Cohort atlas', 'Skill by asset, horizon, regime, source and participant.'], ['Drift monitor', 'Coverage failures, structural breaks and abstention policy.']] },
    models: { eyebrow: 'MODEL GOVERNANCE PLANE', action: 'Register model', panels: [['Artifact registry', 'Version, hash, license, tokenizer and runtime compatibility.'], ['Evaluation matrix', 'Baselines, leakage checks, calibration and regime robustness.'], ['Deployment gates', 'Cost, latency, drift, rollback and production eligibility.']] },
    analogs: { eyebrow: 'TEMPORAL MEMORY FIELD', action: 'Search historical states', panels: [['State embedding', 'Price, regime, structure and evidence-aware similarity.'], ['Analog constellation', 'Nearest episodes with distance and coverage uncertainty.'], ['Forward distribution', 'Outcome paths, dispersion and conditional failure cases.']] },
    portfolio: { eyebrow: 'RISK OBSERVATORY', action: 'Import or connect account', panels: [['Exposure topology', 'Asset, factor, currency, venue and concentration decomposition.'], ['Scenario radar', 'Stress loss, liquidity, gap and correlation-break behavior.'], ['Reconciliation', 'Broker-versus-local cash, position and order truth.']] },
    journal: { eyebrow: 'DECISION MEMORY', action: 'Create journal entry', panels: [['Decision timeline', 'Hypothesis, evidence, action and outcome in one immutable chain.'], ['Behavioral mirror', 'FOMO, revenge, leverage and session-pattern detection.'], ['Learning loop', 'Mistakes, counterfactuals, playbook updates and recurrence.']] },
    orders: { eyebrow: 'GUARDED EXECUTION', action: 'Configure paper broker', panels: [['Intent queue', 'Unsigned drafts, approvals, limits and account readiness.'], ['Execution state', 'Acknowledged, working, partial, filled, rejected and unknown.'], ['Control plane', 'Kill switch, reconciliation, restricted assets and audit trail.']] },
    marketplace: { eyebrow: 'NATIVE EXTENSION FABRIC', action: 'Review extension policy', panels: [['Verified modules', 'Signed packages with declared capabilities and provenance.'], ['Research bridges', 'Isolated comparison tools that cannot enter the trusted core.'], ['Trust inspection', 'License, signature, permissions, SBOM and compatibility.']] },
  };

  const blueprint = $derived(blueprints[active] ?? blueprints.strategy);
</script>

<main class="pk-experience">
  <header class="pk-experience-head">
    <div><span class="kicker">{blueprint.eyebrow}</span><h1>{title}</h1><p>{description}</p></div>
    <div class="pk-experience-mode"><i></i><span>NOT YET BUILT</span><strong>NO DATA PATH</strong></div>
  </header>

  <section class="pk-experience-metrics">
    <article><span>Selected context</span><strong>{market.selected?.symbol ?? '—'}</strong><small>{market.selected ? `${fmtPrice(market.selected)} · ${fmtPct(market.selected.changePct)}` : 'No instrument tracked'}</small></article>
    <article><span>Evidence coverage</span><strong>0%</strong><small>Awaiting governed observations</small></article>
    <article><span>Autonomy state</span><strong>CONTAINED</strong><small>No executable route</small></article>
    <article><span>Timeframe</span><strong>{market.timeframe}</strong><small>{market.feedMessage}</small></article>
  </section>

  <div class="pk-experience-toolbar">
    <button><ScanSearch size={14} /> Search evidence</button><button><Orbit size={14} /> Change visualization</button><button><ShieldCheck size={14} /> Inspect policy</button>
    <a href="/workspace/integrations"><DatabaseZap size={14} /> Provider plane</a>
  </div>

  <section class="pk-experience-grid">
    {#each blueprint.panels as panel, index (panel[0])}
      <article class="pk-domain-panel">
        <header><span>0{index + 1}</span><b>{panel[0]}</b><CircleDashed size={13} /></header>
        <div class="pk-domain-viz" aria-hidden="true"><i></i><i></i><i></i><i></i><span></span></div>
        <p>{panel[1]}</p>
        <footer><span>AWAITING EVIDENCE</span><small>Native interface ready</small></footer>
      </article>
    {/each}
  </section>

  <section class="pk-experience-callout">
    <div class="pk-callout-icon"><FlaskConical size={20} /></div>
    <div><span>SAFE PREVIEW STATE</span><strong>{requirement}</strong><p>The surrounding market motion is synthetic and isolated from evidence, research manifests, and execution. Connect an approved source or start a native run to populate this workbench.</p></div>
    <button><Play size={14} /> {blueprint.action}</button><a href="/workspace/integrations">Configure inputs <ArrowRight size={13} /></a>
  </section>
</main>

<style>
  .pk-experience{min-height:100%;padding:clamp(18px,2.4vw,34px);overflow:auto}.pk-experience-head{display:flex;justify-content:space-between;gap:24px;padding-bottom:22px;border-bottom:1px solid var(--p-border)}.kicker{color:var(--p-accent);font:700 9px var(--font-mono);letter-spacing:.2em}.pk-experience h1{margin:8px 0 7px;font-size:clamp(26px,3.4vw,48px);letter-spacing:-.05em;line-height:1}.pk-experience-head p{max-width:720px;margin:0;color:var(--p-dim);line-height:1.55}.pk-experience-mode{display:grid;align-content:center;grid-template-columns:auto auto;gap:3px 8px;min-width:190px;padding:12px 14px;border:1px solid var(--p-border);border-radius:10px;background:var(--p-panel-fill);backdrop-filter:blur(var(--p-glass-blur))}.pk-experience-mode i{grid-row:1/3;width:7px;height:7px;margin:auto;border-radius:50%;background:var(--p-accent);box-shadow:0 0 9px var(--p-accent)}.pk-experience-mode span,.pk-experience-mode strong{font:700 8px var(--font-mono);letter-spacing:.12em}.pk-experience-mode strong{color:var(--p-dim)}.pk-experience-metrics{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:8px;margin:16px 0}.pk-experience-metrics article,.pk-domain-panel,.pk-experience-callout{border:1px solid var(--p-border);border-radius:10px;background:var(--p-panel-fill);backdrop-filter:blur(var(--p-glass-blur)) saturate(145%)}.pk-experience-metrics article{display:grid;gap:4px;padding:13px}.pk-experience-metrics span,.pk-domain-panel footer,.pk-experience-callout span{color:var(--p-dim);font:600 8px var(--font-mono);letter-spacing:.13em;text-transform:uppercase}.pk-experience-metrics strong{font:700 16px var(--font-mono)}.pk-experience-metrics small{color:var(--p-dim);font-size:9px}.pk-experience-toolbar{display:flex;gap:6px;margin-bottom:12px}.pk-experience-toolbar button,.pk-experience-toolbar a,.pk-experience-callout button,.pk-experience-callout a{display:inline-flex;align-items:center;gap:6px;padding:7px 10px;border:1px solid var(--p-border);border-radius:6px;background:var(--p-surface);color:var(--p-dim);font:600 9px var(--font-mono);text-decoration:none;cursor:pointer}.pk-experience-toolbar a{margin-left:auto;color:var(--p-accent)}.pk-experience-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:10px}.pk-domain-panel{min-height:260px;padding:14px}.pk-domain-panel header{display:grid;grid-template-columns:auto 1fr auto;align-items:center;gap:8px;color:var(--p-dim);font:600 9px var(--font-mono);letter-spacing:.1em;text-transform:uppercase}.pk-domain-panel header b{color:var(--p-text)}.pk-domain-viz{position:relative;height:120px;margin:18px 0;border-block:1px solid var(--p-border);overflow:hidden;background:linear-gradient(var(--p-grid) 1px,transparent 1px),linear-gradient(90deg,var(--p-grid) 1px,transparent 1px);background-size:24px 24px}.pk-domain-viz i{position:absolute;bottom:12px;width:9%;border-radius:2px 2px 0 0;background:linear-gradient(var(--p-accent),transparent);opacity:.45;animation:domain-pulse 3s ease-in-out infinite}.pk-domain-viz i:nth-child(1){left:8%;height:36%}.pk-domain-viz i:nth-child(2){left:31%;height:74%;animation-delay:-1s}.pk-domain-viz i:nth-child(3){left:55%;height:51%;animation-delay:-2s}.pk-domain-viz i:nth-child(4){left:80%;height:88%;animation-delay:-.5s}.pk-domain-viz span{position:absolute;inset:45% -10% auto;height:1px;background:linear-gradient(90deg,transparent,var(--p-accent),var(--p-accent2),transparent);transform:rotate(-7deg);box-shadow:0 0 12px var(--p-glow)}.pk-domain-panel p{min-height:44px;color:var(--p-dim);font-size:10px;line-height:1.55}.pk-domain-panel footer{display:flex;justify-content:space-between;padding-top:12px;border-top:1px solid var(--p-border)}.pk-domain-panel footer span{color:var(--p-accent)}.pk-experience-callout{display:grid;grid-template-columns:auto minmax(0,1fr) auto auto;align-items:center;gap:14px;margin-top:10px;padding:16px}.pk-callout-icon{display:grid;width:42px;height:42px;place-items:center;border:1px solid color-mix(in srgb,var(--p-accent) 30%,var(--p-border));border-radius:50%;color:var(--p-accent)}.pk-experience-callout strong{display:block;margin:4px 0;font-size:11px}.pk-experience-callout p{margin:0;color:var(--p-dim);font-size:9px;line-height:1.5}.pk-experience-callout button{color:var(--p-accent)}@keyframes domain-pulse{50%{opacity:.8;transform:scaleY(.82)}}@media(max-width:1050px){.pk-experience-grid{grid-template-columns:1fr 1fr}.pk-domain-panel:last-child{grid-column:1/-1}.pk-experience-metrics{grid-template-columns:1fr 1fr}.pk-experience-callout{grid-template-columns:auto 1fr}.pk-experience-callout button,.pk-experience-callout>a{grid-row:2}}@media(max-width:720px){.pk-experience-head{display:block}.pk-experience-mode{margin-top:14px}.pk-experience-grid{grid-template-columns:1fr}.pk-domain-panel:last-child{grid-column:auto}.pk-experience-toolbar button:nth-child(n+2){display:none}.pk-experience-metrics{grid-template-columns:1fr 1fr}.pk-experience-callout{grid-template-columns:1fr}.pk-callout-icon{display:none}.pk-experience-callout button,.pk-experience-callout>a{grid-row:auto}}
</style>
