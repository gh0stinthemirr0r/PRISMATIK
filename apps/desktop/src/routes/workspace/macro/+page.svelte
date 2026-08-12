<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { Activity, DatabaseZap, RefreshCw, ShieldCheck } from "lucide-svelte";
  import Visualizations from "$lib/prismatik/Visualizations.svelte";

  type Point = { seriesId:string; date:string; value:string|null; realtimeStart:string; realtimeEnd:string|null; availableAt:string };
  type Persistence = { state:"persisted"|"policy_required"|"failed"; inserted:number; unchanged:number; totalObservations:number; retainedPayloads:boolean; policyVersion:number|null; message:string };
  type Snapshot = { provider:string; retrievedAt:string; series:string[]; points:Point[]; evidence:string; persistence:Persistence };
  const catalog = [
    ["DGS10", "10Y Treasury"], ["DGS2", "2Y Treasury"], ["FEDFUNDS", "Fed funds"],
    ["CPIAUCSL", "CPI"], ["UNRATE", "Unemployment"], ["GDP", "GDP"], ["BAMLH0A0HYM2", "HY spread"]
  ];
  let selected = $state(["DGS10", "DGS2", "FEDFUNDS", "CPIAUCSL"]);
  let snapshot = $state<Snapshot|null>(null);
  let loading = $state(false);
  let error = $state<string|null>(null);
  const grouped = $derived(Object.fromEntries(selected.map(id => [id, (snapshot?.points ?? []).filter(point => point.seriesId === id)])) as Record<string,Point[]>);
  const latest = $derived(selected.map(id => ({ id, point: grouped[id]?.at(-1), label: catalog.find(row => row[0] === id)?.[1] ?? id })));

  function toggle(id:string) {
    selected = selected.includes(id) ? selected.filter(value => value !== id) : selected.length < 12 ? [...selected, id] : selected;
  }
  async function load() {
    if (!isTauri()) { error = "Native desktop runtime required."; return; }
    if (!selected.length) { error = "Select at least one series."; return; }
    loading = true; error = null;
    try { snapshot = await invoke<Snapshot>("get_macro_series", { seriesIds: selected }); }
    catch (cause) { snapshot = null; error = cause instanceof Error ? cause.message : String(cause); }
    finally { loading = false; }
  }
  onMount(() => { void load(); });
</script>

<svelte:head><title>Macro regime · PRISMATIK</title></svelte:head>
<main class="terminal">
  <header><div><p class="eyebrow">GLOBAL REGIME ENGINE</p><h1>Macro intelligence</h1><p class="lede">Vintage-aware economic observations retrieved directly from a connected FRED session. Missing or disconnected evidence stays empty.</p></div><div class:live={!!snapshot} class="mode"><i></i><span>{snapshot ? "FRED LIVE" : "NO MACRO INPUT"}</span><strong>{snapshot ? new Date(snapshot.retrievedAt).toLocaleTimeString() : "Connect provider"}</strong></div></header>
  <section class="toolbar" aria-label="Macro series controls">
    <div class="series">{#each catalog as row}<button class:active={selected.includes(row[0])} onclick={() => toggle(row[0])}>{row[0]}<small>{row[1]}</small></button>{/each}</div>
    <button class="refresh" onclick={load} disabled={loading}><RefreshCw size={14} class={loading ? "spin" : ""}/>{loading ? "Retrieving" : "Refresh real data"}</button>
  </section>
  {#if error}<section class="notice"><DatabaseZap size={18}/><div><strong>Macro evidence unavailable</strong><p>{error}</p></div><a href="/workspace/integrations">Open integrations</a></section>{/if}
  {#if snapshot && snapshot.persistence.state !== "persisted"}<section class="notice"><ShieldCheck size={18}/><div><strong>Live data is not retained</strong><p>{snapshot.persistence.message}</p></div><a href="/workspace/integrations">Review source policy</a></section>{/if}
  <section class="metrics">
    {#each latest as item}
      <article><span>{item.label}</span><strong>{item.point?.value ?? "—"}</strong><small>{item.point ? `OBS ${item.point.date} · AVAILABLE ${item.point.availableAt}` : "AWAITING OBSERVATION"}</small></article>
    {/each}
  </section>
  <section class="panel viz-panel">
    <div class="panel-head"><div><span>TREASURY TERM STRUCTURE</span><strong>Observed FRED curve</strong></div><ShieldCheck size={16}/></div>
    <div class="viz-host"><Visualizations only={["yield"]} /></div>
  </section>

  <section class="panel">
    <div class="panel-head"><div><span>POINT-IN-TIME RELEASE MATRIX</span><strong>{snapshot?.points.length ?? 0} normalized observations</strong></div><ShieldCheck size={16}/></div>
    {#if snapshot?.points.length}
      <div class="table"><div class="tr th"><span>Series</span><span>Observation</span><span>Value</span><span>Available</span><span>Vintage end</span></div>{#each snapshot.points.slice().reverse().slice(0,160) as point}<div class="tr"><b>{point.seriesId}</b><span>{point.date}</span><strong>{point.value ?? "missing"}</strong><span>{point.availableAt}</span><span>{point.realtimeEnd ?? "current"}</span></div>{/each}</div>
    {:else}<div class="empty"><Activity size={28}/><strong>No governed macro observations</strong><p>Connect FRED, select series, then refresh. PRISMATIK will not substitute simulated macro releases.</p></div>{/if}
    <footer>{snapshot ? `${snapshot.evidence} · ${snapshot.persistence.message}` : "FRED adapter · BudgetGovernor · vintage-safe normalization"}</footer>
  </section>
</main>

<style>
  .viz-panel{margin-bottom:10px}.viz-host{height:340px}.terminal{min-height:100%;padding:clamp(18px,2.5vw,36px);overflow:auto;color:var(--p-text)}header{display:flex;justify-content:space-between;gap:22px;padding-bottom:20px;border-bottom:1px solid var(--p-border)}.eyebrow{margin:0;color:var(--p-accent);font:700 9px var(--font-mono);letter-spacing:.18em}.terminal h1{margin:8px 0 6px;font-size:clamp(28px,4vw,50px);letter-spacing:-.05em}.lede{max-width:720px;margin:0;color:var(--p-dim);line-height:1.55}.mode{display:grid;grid-template-columns:auto 1fr;align-content:center;gap:3px 8px;min-width:175px;padding:12px;border:1px solid var(--p-border);border-radius:10px;background:var(--p-panel-fill)}.mode i{grid-row:1/3;width:7px;height:7px;margin:auto;border-radius:50%;background:#f59e0b}.mode.live i{background:var(--p-up);box-shadow:0 0 10px var(--p-up)}.mode span,.mode strong{font:700 8px var(--font-mono);letter-spacing:.12em}.mode strong{color:var(--p-dim)}.toolbar{display:flex;align-items:stretch;gap:10px;margin:16px 0}.series{display:flex;flex:1;gap:6px;overflow:auto}.series button,.refresh{padding:9px 11px;border:1px solid var(--p-border);border-radius:7px;background:var(--p-panel-fill);color:var(--p-dim);font:700 9px var(--font-mono);cursor:pointer;white-space:nowrap}.series button small{display:block;margin-top:3px;font-size:7px;font-weight:500}.series button.active{border-color:color-mix(in srgb,var(--p-accent) 55%,var(--p-border));color:var(--p-accent);box-shadow:inset 0 -2px var(--p-accent)}.refresh{display:flex;align-items:center;gap:7px;color:var(--p-accent)}.refresh:disabled{opacity:.6}.refresh :global(.spin){animation:spin 1s linear infinite}.notice{display:grid;grid-template-columns:auto 1fr auto;align-items:center;gap:12px;margin-bottom:12px;padding:14px;border:1px solid color-mix(in srgb,#f59e0b 38%,var(--p-border));border-radius:9px;background:var(--p-panel-fill);color:#f59e0b}.notice p{margin:4px 0 0;color:var(--p-dim);font-size:11px}.notice a{color:var(--p-accent);font:700 9px var(--font-mono)}.metrics{display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:8px;margin-bottom:10px}.metrics article,.panel{border:1px solid var(--p-border);border-radius:10px;background:var(--p-panel-fill);backdrop-filter:blur(var(--p-glass-blur))}.metrics article{display:grid;gap:5px;padding:14px}.metrics span,.metrics small,.panel-head span,footer{color:var(--p-dim);font:600 8px var(--font-mono);letter-spacing:.1em}.metrics strong{font:700 21px var(--font-mono)}.panel{overflow:hidden}.panel-head{display:flex;align-items:center;justify-content:space-between;padding:15px;border-bottom:1px solid var(--p-border);color:var(--p-accent)}.panel-head strong{display:block;margin-top:4px;color:var(--p-text);font:700 11px var(--font-mono)}.table{max-height:460px;overflow:auto}.tr{display:grid;grid-template-columns:.75fr 1fr .8fr 1fr 1fr;gap:12px;padding:9px 15px;border-bottom:1px solid var(--p-grid);font:500 10px var(--font-mono)}.tr b{color:var(--p-accent)}.tr strong{font-size:11px}.th{position:sticky;top:0;background:var(--p-surface);color:var(--p-dim);font-size:8px;text-transform:uppercase}.empty{display:grid;place-items:center;gap:9px;padding:70px 20px;color:var(--p-dim);text-align:center}.empty strong{color:var(--p-text)}.empty p{max-width:520px;margin:0;font-size:11px}footer{padding:12px 15px;border-top:1px solid var(--p-border)}@keyframes spin{to{transform:rotate(360deg)}}@media(max-width:760px){header,.toolbar{display:block}.mode{margin-top:12px}.refresh{margin-top:8px}.tr{grid-template-columns:.7fr 1fr .8fr}.tr span:nth-child(n+4){display:none}}
</style>
