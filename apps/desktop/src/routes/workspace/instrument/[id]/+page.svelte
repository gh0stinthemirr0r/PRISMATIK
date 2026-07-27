<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, PriceChart, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";
  import type { CandlestickPoint } from "@prismatik/chart-contracts";
  type Payload={title:string;subtitle:string;value:string;provider:string;retrievedAt:string;bars:CandlestickPoint[];greeks?:Array<{occSymbol:string;delta:number;gamma:number;theta:number;vega:number;liquidityScore:number}>;contracts?:unknown[]};
  let data=$state<Payload|null>(null);let error=$state<string|null>(null);
  const id=$derived($page.params.id);const assetClass=$derived($page.url.searchParams.get("assetClass")??"crypto");
  onMount(async()=>{try{
    if(assetClass==="equity"){const r=await invoke<any>("get_equity_bars",{symbol:id});const last=r.bars.at(-1);data={title:r.symbol,subtitle:"Equity · "+r.currency,value:last?`$${last.close.toFixed(2)}`:"—",provider:r.provider,retrievedAt:r.retrievedAt,bars:r.bars};}
    else if(assetClass==="option"){const [r,flow,dealer]=await Promise.all([invoke<any>("get_options_chain",{underlying:id}),invoke<any>("get_options_flow",{underlying:id}),invoke<any>("get_dealer_exposure",{underlying:id})]);data={title:r.underlying,subtitle:`Listed options · ${r.contracts.length} contracts · ${flow.prints.length} prints · GEX ${dealer.netGex.toExponential(2)}`,value:`$${r.spot.toFixed(2)} spot`,provider:r.provider,retrievedAt:r.retrievedAt,bars:[],contracts:r.contracts,greeks:r.contracts.slice(0,3)};}
    else if(assetClass==="macro"){const r=await invoke<any>("get_macro_series",{seriesId:id});const last=r.points.at(-1);data={title:r.seriesId,subtitle:r.title,value:last?`${last.value}`:"—",provider:r.provider,retrievedAt:r.retrievedAt,bars:[]};}
    else{const [detail,chart,markets]=await Promise.all([invoke<any>("get_coin_detail",{coingeckoId:id}),invoke<any>("get_crypto_ohlc",{coingeckoId:id,days:30}),invoke<any[]>("get_crypto_markets")]);const q=markets.find(x=>x.coingeckoId===id);data={title:detail.symbol.toUpperCase(),subtitle:`Crypto · ${detail.name}`,value:q?`$${Number(q.price).toLocaleString()}`:"—",provider:detail.provider,retrievedAt:detail.retrievedAt,bars:chart.candles};}
  }catch(cause){error=cause instanceof Error?cause.message:String(cause)}})
</script>
<svelte:head><title>{id} instrument · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active={assetClass==="equity"?"equity":assetClass==="option"?"options":assetClass==="macro"?"macro":"crypto"}/>{/snippet}
  {#snippet status()}<EvidenceChip status="confirmed" label={`asset class · ${assetClass}`}/>{/snippet}
  <main>{#if error}<h1>Instrument unavailable</h1><p>{error}</p>{:else if !data}<p>Loading {id}…</p>{:else}<header><div><div class="kicker">{data.subtitle}</div><h1>{data.title}</h1></div><div class="value">{data.value}<EvidenceChip status="confirmed" label={data.provider}/><StaleDataMarker eventTime={data.retrievedAt} maxAge={86_400_000}/></div></header>{#if data.bars.length}<section><div class="kicker">Price history</div><PriceChart candles={data.bars} height={420}/></section>{:else if data.greeks}<section><div class="kicker">Contract greeks</div><div class="grid">{#each data.greeks as row}<article><code>{row.occSymbol}</code><p>Δ {row.delta} · Γ {row.gamma} · Θ {row.theta} · ν {row.vega}</p><EvidenceChip status="confirmed" label={`${(row.liquidityScore*100).toFixed(0)} liq`}/></article>{/each}</div></section>{:else}<section class="placeholder"><div class="kicker">Instrument</div><p>No chart series for this asset class; metadata above is live from the offline command fixtures.</p></section>{/if}{/if}</main>
</WorkspaceShell>
<style>
  .rail-label,.kicker{margin:0 0 var(--space-3);color:var(--color-text-tertiary);font-size:var(--font-size-xs);letter-spacing:.06em;text-transform:uppercase}main{padding:var(--space-5) var(--space-6)}header{display:flex;justify-content:space-between;align-items:end;padding-bottom:var(--space-5);border-bottom:1px solid var(--color-border-default)}h1{margin:0;font-size:var(--font-size-3xl)}.value{display:grid;gap:var(--space-2);text-align:right;font:600 var(--font-size-2xl) var(--font-mono)}section{margin-top:var(--space-5)}.placeholder{max-width:620px;padding:var(--space-5);border:1px dashed var(--color-border-strong);color:var(--color-text-secondary)}.grid{display:grid;gap:var(--space-3)}article{display:flex;justify-content:space-between;gap:var(--space-4);align-items:center;padding:var(--space-3);border:1px solid var(--color-border-default)}article p{margin:0;color:var(--color-text-secondary);font-family:var(--font-mono);font-size:var(--font-size-sm)}
</style>
