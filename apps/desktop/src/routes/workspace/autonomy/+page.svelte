<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke, isTauri } from '@tauri-apps/api/core';

  type Policy = { currency: string; operationsLimitMicros: number; tradingLimitMicros: number; perOperationLimitMicros: number; perTradeLimitMicros: number };
  type Snapshot = { policy: Policy; operationsUsedMicros: number; tradingUsedMicros: number; posture: 'active' | 'research_only' | 'local_only'; durable:boolean; journalRecords:number; journalHead:string };
  type ResearchConfig = { enabled:boolean;providerId:string;model:string;question:string;intervalSeconds:number;maxOutputTokens:number;maxCostMicros:number;requireStableForecasts:boolean;requiredCohortId:string };
  type ResearchStatus = { config:ResearchConfig;running:boolean;nextRunAt?:string;lastRunAt?:string;lastError?:string;gateState:string;gateReason?:string;successfulRuns:number;failedRuns:number;lastResult?:{text:string;providerId:string;model:string;generatedAt:string;evidenceCount:number};durable:boolean;journalRecords:number;journalHead:string;credentialsPersisted:boolean };
  type Cohort = {cohortId:string;target:string;horizonMinutes:number;providerId:string;model:string;sampleCount:number;state:string};
  let snapshot = $state<Snapshot | null>(null);
  let error = $state('');
  let saving = $state(false);
  let form = $state({ operations: 25, trading: 0, perOperation: 1, perTrade: 0 });
  let research = $state<ResearchStatus|null>(null); let activeModels=$state<string[]>([]); let cohorts=$state<Cohort[]>([]); let researchSaving=$state(false); let researchError=$state('');
  let researchForm=$state<ResearchConfig>({enabled:false,providerId:'',model:'',question:'Identify material cross-asset changes in the current real snapshot, challenge the strongest interpretation, and cite every observation used.',intervalSeconds:900,maxOutputTokens:700,maxCostMicros:250000,requireStableForecasts:false,requiredCohortId:''});
  const dollars = (micros: number) => `$${(micros / 1_000_000).toLocaleString('en-US', { maximumFractionDigits: 2 })}`;

  async function load() {
    if (!isTauri()) return;
    try {
      snapshot = await invoke<Snapshot>('get_autonomy_budget');
      form = { operations: snapshot.policy.operationsLimitMicros / 1e6, trading: snapshot.policy.tradingLimitMicros / 1e6, perOperation: snapshot.policy.perOperationLimitMicros / 1e6, perTrade: snapshot.policy.perTradeLimitMicros / 1e6 };
      research=await invoke<ResearchStatus>('get_autonomous_research'); researchForm={...research.config};
      const models=await invoke<Array<{providerId:string;active:boolean}>>('model_runtime_status'); activeModels=models.filter(row=>row.active).map(row=>row.providerId);
      cohorts=await invoke<Cohort[]>('forecast_calibration_health');
    } catch (cause) { error = String(cause); }
  }
  async function save() {
    saving = true; error = '';
    try {
      snapshot = await invoke<Snapshot>('configure_autonomy_budget', { policy: {
        currency: 'USD', operationsLimitMicros: Math.round(form.operations * 1e6), tradingLimitMicros: Math.round(form.trading * 1e6),
        perOperationLimitMicros: Math.max(1, Math.round(form.perOperation * 1e6)), perTradeLimitMicros: Math.max(1, Math.round(form.perTrade * 1e6)),
      }});
    } catch (cause) { error = String(cause); } finally { saving = false; }
  }
  async function saveResearch(){researchSaving=true;researchError='';try{research=await invoke<ResearchStatus>('configure_autonomous_research',{config:{...researchForm,maxCostMicros:Number(researchForm.maxCostMicros),maxOutputTokens:Number(researchForm.maxOutputTokens),intervalSeconds:Number(researchForm.intervalSeconds)}});researchForm={...research.config};}catch(cause){researchError=String(cause)}finally{researchSaving=false}}
  onMount(() => { void load(); const timer=setInterval(async()=>{if(isTauri())research=await invoke<ResearchStatus>('get_autonomous_research');},15000);return()=>clearInterval(timer); });
</script>

<svelte:head><title>Autonomy control · PRISMATIK</title></svelte:head>
<div class="autonomy">
  <header><div><span>AUTONOMY / POLICY KERNEL</span><h1>Operating envelope</h1><p>Research and execution have separate monetary boundaries. Exhausting trading capital blocks new order promotion while evidence ingestion and analysis continue.</p></div><strong class:active={snapshot?.posture === 'active'}>{snapshot?.posture?.replace('_', ' ') ?? 'loading'}</strong></header>
  <section class="meters">
    <article><span>OPERATIONS</span><b>{snapshot ? dollars(snapshot.operationsUsedMicros) : '—'} <i>/ {snapshot ? dollars(snapshot.policy.operationsLimitMicros) : '—'}</i></b><p>Data, models, research, and external tools.</p></article>
    <article><span>TRADING</span><b>{snapshot ? dollars(snapshot.tradingUsedMicros) : '—'} <i>/ {snapshot ? dollars(snapshot.policy.tradingLimitMicros) : '—'}</i></b><p>Gross capital reservations for autonomous orders.</p></article>
    <article><span>DURABLE JOURNAL</span><b>{snapshot?.durable ? 'VERIFIED' : 'UNAVAILABLE'}</b><p>{snapshot ? `${snapshot.journalRecords} hash-chained mutations · ${snapshot.journalHead.slice(0, 20)}…` : 'Checking restart-safe state'}</p></article>
  </section>
  <section class="panel">
    <div><span>PERSISTENT ENVELOPE · USD</span><h2>Budget policy</h2><p>Saving is an explicit audited reset. Usage survives restarts; there is no silent calendar rollover. Trading still requires separate broker, risk, and owner gates.</p></div>
    <div class="fields">
      <label>Operations limit<input type="number" min="0" step="1" bind:value={form.operations} /></label>
      <label>Per-operation cap<input type="number" min="0.01" step="0.01" bind:value={form.perOperation} /></label>
      <label>Trading capital limit<input type="number" min="0" step="1" bind:value={form.trading} /></label>
      <label>Per-trade cap<input type="number" min="0" step="1" bind:value={form.perTrade} /></label>
    </div>
    {#if error}<p class="error">{error}</p>{/if}
    <button onclick={save} disabled={saving}>{saving ? 'APPLYING…' : 'APPLY & RESET PERIOD'}</button>
  </section>
  <section class="loop">
    <div class="loop-head"><div><span>AUTONOMOUS RESEARCH LOOP</span><h2>Evidence watch</h2><p>The mandate, lifecycle, failures, and latest cited result survive restarts in a verified journal. Provider credentials remain memory-only and must be reconnected after restart.</p></div><strong class:enabled={research?.config.enabled}>{research?.running?'RUNNING':research?.config.enabled?'ARMED':'DISABLED'}</strong></div>
    <div class="loop-form"><label class="toggle"><input type="checkbox" bind:checked={researchForm.enabled}/><span>Enable autonomous research</span></label><label>Active provider<select bind:value={researchForm.providerId}><option value="">Select session</option>{#each activeModels as id}<option>{id}</option>{/each}</select></label><label>Exact model ID<input bind:value={researchForm.model}/></label><label>Interval seconds<input type="number" min="300" max="604800" bind:value={researchForm.intervalSeconds}/></label><label>Max output tokens<input type="number" min="1" max="8192" bind:value={researchForm.maxOutputTokens}/></label><label>Worst-case reservation (USD)<input type="number" min="0.000001" step="0.01" value={researchForm.maxCostMicros/1e6} oninput={(event)=>researchForm.maxCostMicros=Math.round(Number(event.currentTarget.value)*1e6)}/></label><label class="loop-question">Recurring research mandate<textarea rows="3" bind:value={researchForm.question}></textarea></label><button onclick={saveResearch} disabled={researchSaving}>{researchSaving?'APPLYING…':'APPLY LOOP POLICY'}</button></div>
    <div class="calibration-gate"><label class="toggle"><input type="checkbox" bind:checked={researchForm.requireStableForecasts}/><span>Require stable calibrated forecast cohort before every autonomous cycle</span></label>{#if researchForm.requireStableForecasts}<label>Required cohort<select bind:value={researchForm.requiredCohortId}><option value="">Select cohort</option>{#each cohorts as cohort}<option value={cohort.cohortId}>{cohort.target} {cohort.horizonMinutes}m · {cohort.providerId}/{cohort.model} · n={cohort.sampleCount} · {cohort.state}</option>{/each}</select></label><p>Runs pause without spending model budget until this exact cohort has at least 40 resolved observations and passes rolling drift checks.</p>{/if}</div>
    {#if researchError}<p class="error">{researchError}</p>{/if}
    <div class="loop-status"><article><span>SUCCESSFUL</span><b>{research?.successfulRuns??0}</b></article><article><span>FAILED</span><b>{research?.failedRuns??0}</b></article><article><span>NEXT RUN</span><b>{research?.nextRunAt??'—'}</b></article><article><span>LAST RUN</span><b>{research?.lastRunAt??'—'}</b></article><article><span>STATE JOURNAL</span><b>{research?.durable ? `${research.journalRecords} VERIFIED` : 'UNAVAILABLE'}</b></article><article><span>CREDENTIAL POSTURE</span><b>{research?.credentialsPersisted ? 'PERSISTED' : 'MEMORY ONLY'}</b></article></div>
    {#if research?.gateState==='paused'}<p class="gate-pause">Calibration gate paused this mandate: {research.gateReason}</p>{/if}{#if research?.lastError}<p class="error">Last cycle: {research.lastError}</p>{/if}{#if research?.lastResult}<details><summary>Latest cited model research · {research.lastResult.providerId}/{research.lastResult.model}</summary><pre>{research.lastResult.text}</pre></details>{/if}
  </section>
  <section class="gates"><h2>Autonomy gates</h2><div><article><b>Research loop</b><span>Autonomous · evidence required</span></article><article><b>Strategy drafting</b><span>Autonomous · simulation first</span></article><article><b>Order promotion</b><span>Disabled until broker, risk, and capital gates pass</span></article><article><b>Live execution</b><span>Human-controlled activation boundary</span></article></div></section>
</div>

<style>
  .autonomy{height:100%;overflow:auto;padding:34px clamp(24px,3vw,48px) 80px;color:var(--p-text)}header{display:flex;justify-content:space-between;gap:30px;align-items:flex-start;border-bottom:1px solid var(--p-border);padding-bottom:28px}header span,.panel span,.loop-head span{font:600 .62rem var(--font-mono);letter-spacing:.14em;color:var(--p-accent)}h1{font-size:clamp(2.6rem,5vw,5.4rem);letter-spacing:-.07em;margin:10px 0}header p,.panel p,.loop p{max-width:760px;color:var(--p-dim);line-height:1.6}header>strong{padding:9px 14px;border:1px solid #ffb84d55;border-radius:99px;color:#ffcb7b;text-transform:uppercase;font:600 .66rem var(--font-mono)}header>strong.active{color:#67efba;border-color:#28e7a455}.meters{display:grid;grid-template-columns:repeat(3,1fr);gap:12px;margin:24px 0}.meters article,.panel,.gates,.loop{border:1px solid var(--p-border);background:var(--p-panel-fill);backdrop-filter:blur(var(--glass-blur));padding:22px}.meters span{font:.6rem var(--font-mono);color:var(--p-dim)}.meters b{display:block;font:600 1.25rem var(--font-mono);margin:12px 0}.meters i{font-style:normal;color:var(--p-dim)}.meters p{color:var(--p-dim);margin:0;font-size:.78rem}.panel{display:grid;grid-template-columns:minmax(240px,.7fr) 1fr auto;gap:28px;align-items:end}.panel h2,.gates h2,.loop h2{font-size:1.4rem;margin:8px 0}.fields{display:grid;grid-template-columns:repeat(2,1fr);gap:12px}.fields label,.loop-form label{display:grid;gap:6px;color:var(--p-dim);font-size:.7rem}.fields input,.loop-form input,.loop-form select,.loop-form textarea{border:1px solid var(--p-border);background:var(--p-surface2);color:var(--p-text);padding:10px;border-radius:6px;font:inherit}.panel button,.loop-form button{border:0;background:var(--p-accent);color:var(--p-bg);padding:12px 16px;border-radius:5px;font:700 .64rem var(--font-mono);cursor:pointer}.error{color:#ff637d!important}.loop{margin-top:14px}.loop-head{display:flex;justify-content:space-between;gap:20px}.loop-head strong{color:#ffcb7b;font:.62rem var(--font-mono)}.loop-head strong.enabled{color:#67efba}.loop-form{display:grid;grid-template-columns:repeat(3,1fr);gap:12px;margin-top:18px}.loop-form .toggle{display:flex;align-items:center;gap:8px}.loop-form .toggle input{width:auto}.loop-question{grid-column:1/-1}.loop-form button{grid-column:1/-1;justify-self:end}.loop-status{display:grid;grid-template-columns:repeat(3,1fr);gap:8px;margin-top:16px}.loop-status article{border-top:1px solid var(--p-border);padding-top:10px}.loop-status span,.loop-status b{display:block;font:.58rem var(--font-mono);color:var(--p-dim)}.loop-status b{color:var(--p-text);margin-top:6px;overflow-wrap:anywhere}details{margin-top:14px;border:1px solid var(--p-border);padding:12px}details summary{cursor:pointer;color:var(--p-accent);font:.62rem var(--font-mono)}details pre{white-space:pre-wrap;line-height:1.5}.gates{margin-top:14px}.gates>div{display:grid;grid-template-columns:repeat(4,1fr);gap:10px}.gates article{border-top:1px solid var(--p-border);padding-top:12px}.gates b,.gates span{display:block}.gates span{margin-top:8px;color:var(--p-dim);font-size:.72rem;line-height:1.4}@media(max-width:1000px){.meters,.gates>div{grid-template-columns:repeat(2,1fr)}.panel{grid-template-columns:1fr}.fields{max-width:700px}.loop-form{grid-template-columns:repeat(2,1fr)}}@media(max-width:620px){.meters,.gates>div,.fields,.loop-form,.loop-status{grid-template-columns:1fr}header,.loop-head{display:block}header>strong{display:inline-block;margin-top:12px}}
.calibration-gate{display:grid;gap:10px;margin-top:12px;padding:13px;border:1px solid var(--p-border);background:var(--p-surface2)}.calibration-gate label{display:flex;align-items:center;gap:8px;color:var(--p-dim);font-size:.7rem}.calibration-gate label:not(.toggle){display:grid}.calibration-gate select{padding:9px;border:1px solid var(--p-border);background:var(--p-surface);color:var(--p-text)}.calibration-gate p{margin:0;font-size:.68rem}.gate-pause{padding:11px;border:1px solid #ffcb7b66;background:#ffcb7b0a;color:#ffcb7b!important;font:.62rem var(--font-mono)}
</style>
