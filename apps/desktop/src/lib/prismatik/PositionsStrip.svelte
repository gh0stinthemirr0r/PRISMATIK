<script lang="ts">
  /**
   * Paper positions, from the real paper OMS.
   *
   * The OMS reports value in micros and marks each position against a quote it
   * can cite (`markEvidenceId`); a position it could not mark is shown as
   * unmarked rather than valued off a stale or invented price.
   */
  import { onMount } from 'svelte';
  import { market, fmtSigned, fmtNum, NO_VALUE } from './market.svelte';

  interface PaperPositionView {
    symbol: string;
    quantity: string;
    markPriceMicros: number | null;
    marketValueMicros: string | null;
    cashFlowMicros: string;
    unrealizedPnlMicros: string | null;
    markEvidenceId: string | null;
  }

  interface PaperOmsView {
    mode: string;
    liveExecutionAvailable: boolean;
    positions: PaperPositionView[];
    quoteProviderCount: number;
    grossExposureMicros: string;
    netMarketValueMicros: string;
    totalUnrealizedPnlMicros: string | null;
    markedPositionCount: number;
    unmarkedPositionCount: number;
    message: string;
  }

  let oms = $state<PaperOmsView | null>(null);
  let error = $state<string | null>(null);

  /** Micros are integer-encoded to survive IPC without float drift. */
  function fromMicros(v: string | number | null): number | null {
    if (v === null) return null;
    const n = typeof v === 'number' ? v : Number.parseFloat(v);
    return Number.isFinite(n) ? n / 1_000_000 : null;
  }

  async function load(): Promise<void> {
    try {
      const { invoke, isTauri } = await import('@tauri-apps/api/core');
      if (!isTauri()) return;
      oms = await invoke<PaperOmsView>('get_paper_oms');
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(() => {
    void load();
    // Marks follow the quote poll; re-read on the same cadence.
    const timer = setInterval(() => void load(), 30_000);
    return () => clearInterval(timer);
  });

  const positions = $derived(oms?.positions ?? []);
  const unrealized = $derived(fromMicros(oms?.totalUnrealizedPnlMicros ?? null));
  const netValue = $derived(fromMicros(oms?.netMarketValueMicros ?? null));
  const grossExposure = $derived(fromMicros(oms?.grossExposureMicros ?? null));
</script>

<footer class="pk-strip">
  <div class="pk-strip-label">
    <span class="pk-strip-mode">{oms?.mode?.toUpperCase() ?? 'PAPER'}</span> POSITIONS
  </div>

  {#if error}
    <div class="pk-strip-empty">Paper OMS unavailable — {error}</div>
  {:else if positions.length === 0}
    <div class="pk-strip-empty">
      {oms?.message ?? 'No paper positions'}
    </div>
  {:else}
    {#each positions as pos (pos.symbol)}
      {@const pnl = fromMicros(pos.unrealizedPnlMicros)}
      {@const mark = fromMicros(pos.markPriceMicros)}
      <div class="pk-pos" class:unmarked={mark === null}>
        <div class="sym">
          <span>{pos.symbol}</span>
          <span class="qty">{pos.quantity}</span>
        </div>
        <div class="sub">
          {#if mark === null}
            unmarked · no citable quote
          {:else}
            MARK {fmtNum(mark, 2)}
          {/if}
        </div>
        <div class="pnl" class:pk-up={(pnl ?? 0) >= 0} class:pk-down={(pnl ?? 0) < 0}>
          {pnl === null ? NO_VALUE : fmtSigned(pnl)}
        </div>
      </div>
    {/each}
  {/if}

  <div class="pk-acct">
    <div>
      <div class="lbl">Net value</div>
      <div class="val">{netValue === null ? NO_VALUE : `$${fmtNum(netValue, 0)}`}</div>
    </div>
    <div>
      <div class="lbl">Gross exposure</div>
      <div class="val">{grossExposure === null ? NO_VALUE : `$${fmtNum(grossExposure, 0)}`}</div>
    </div>
    <div>
      <div class="lbl">Unrealized</div>
      <div
        class="val"
        class:pk-up={(unrealized ?? 0) >= 0}
        class:pk-down={(unrealized ?? 0) < 0}
      >
        {unrealized === null ? NO_VALUE : fmtSigned(unrealized, 0)}
      </div>
    </div>
    <div>
      <div class="lbl">Marks</div>
      <div class="val" title={`${oms?.unmarkedPositionCount ?? 0} unmarked`}>
        {oms ? `${oms.markedPositionCount}/${positions.length}` : NO_VALUE}
      </div>
    </div>
  </div>
</footer>

<style>
  .pk-strip-mode {
    color: var(--p-accent);
  }
  .pk-strip-empty {
    display: flex;
    align-items: center;
    padding: 0 14px;
    color: var(--p-dim);
    font-size: var(--fz-sm);
  }
  .pk-pos.unmarked .sub {
    color: var(--p-dim);
    font-style: italic;
  }
</style>
