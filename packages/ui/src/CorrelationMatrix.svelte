<script lang="ts">
  type Node = {
    id: string;
    label: string;
    value: string;
    x: number;
    y: number;
    tone: "cyan" | "violet" | "emerald" | "amber";
  };
  type Edge = { from: string; to: string; strength: number };

  let {
    nodes,
    edges,
  }: {
    nodes: Node[];
    edges: Edge[];
  } = $props();

  let selected = $state("");
  const selectedNode = $derived(nodes.find((node) => node.id === selected) ?? nodes[0]);
  const selectedEdges = $derived(edges.filter((edge) => edge.from === selected || edge.to === selected));

  function edgeStyle(edge: Edge) {
    const from = nodes.find((node) => node.id === edge.from);
    const to = nodes.find((node) => node.id === edge.to);
    if (!from || !to) return "";
    const dx = to.x - from.x;
    const dy = to.y - from.y;
    const length = Math.sqrt(dx * dx + dy * dy);
    const angle = Math.atan2(dy, dx) * (180 / Math.PI);
    return `left:${from.x}%;top:${from.y}%;width:${length}%;transform:rotate(${angle}deg);--strength:${edge.strength};--edge-duration:${(3.2 - edge.strength * 1.9).toFixed(2)}s`;
  }
</script>

<div class="matrix">
  <div class="matrix__field" aria-label="Market correlation network">
    <div class="matrix__rings" aria-hidden="true"></div>
    {#each edges as edge}
      <div
        class="edge"
        class:active={edge.from === selected || edge.to === selected}
        style={edgeStyle(edge)}
        aria-hidden="true"
      >
        <span></span>
      </div>
    {/each}
    {#each nodes as node}
      <button
        type="button"
        class="node"
        class:active={selected === node.id}
        data-tone={node.tone}
        style={`left:${node.x}%;top:${node.y}%`}
        aria-label={`${node.label}, correlation ${node.value}`}
        onclick={() => (selected = node.id)}
      >
        <span class="node__core"></span>
        <strong>{node.label}</strong>
        <em>{node.value}</em>
      </button>
    {/each}
  </div>
  {#if selectedNode}
    <aside class="matrix__readout">
      <div class="readout__eyebrow">Selected signal</div>
      <div class="readout__title">{selectedNode.label}</div>
      <div class="readout__value">{selectedNode.value}</div>
      <div class="readout__rule"></div>
      {#each selectedEdges.slice(0, 4) as edge}
        <div class="relation">
          <span>{nodes.find((node) => node.id === (edge.from === selected ? edge.to : edge.from))?.label}</span>
          <strong>{edge.strength.toFixed(2)}</strong>
        </div>
      {/each}
      <p>30D rolling Pearson · winsorized at 2.5σ</p>
    </aside>
  {/if}
</div>

<style>
  .matrix {
    display: grid;
    grid-template-columns: minmax(360px, 1fr) 178px;
    gap: var(--space-4);
    min-height: 292px;
  }
  .matrix__field {
    position: relative;
    min-height: 292px;
    overflow: hidden;
    border-radius: var(--radius-md);
    background:
      linear-gradient(rgba(255,255,255,.035) 1px, transparent 1px),
      linear-gradient(90deg, rgba(255,255,255,.035) 1px, transparent 1px);
    background-size: 42px 42px;
  }
  .matrix__rings {
    position: absolute;
    inset: 50% auto auto 50%;
    width: 230px;
    height: 230px;
    border: 1px solid rgba(0, 240, 255, 0.055);
    border-radius: 50%;
    box-shadow:
      0 0 0 42px rgba(168, 85, 247, 0.025),
      0 0 0 84px rgba(0, 240, 255, 0.018);
    transform: translate(-50%, -50%);
  }
  .edge {
    position: absolute;
    z-index: 1;
    height: 1px;
    overflow: visible;
    background: linear-gradient(90deg, rgba(0,240,255,.07), rgba(168,85,247,calc(var(--strength) * .34)), rgba(0,240,255,.07));
    opacity: calc(0.25 + var(--strength) * 0.45);
    transform-origin: left center;
    transition: opacity var(--duration-base) var(--ease-standard);
  }
  .edge span {
    position: absolute;
    top: -1px;
    width: 18%;
    height: 3px;
    border-radius: 999px;
    background: linear-gradient(90deg, transparent, var(--color-brand-primary), transparent);
    filter: blur(0.3px);
    animation: edge-flow var(--edge-duration) linear infinite;
  }
  .edge.active {
    height: 1.5px;
    opacity: 1;
  }
  .node {
    --node-color: var(--color-brand-primary);
    position: absolute;
    z-index: 2;
    display: grid;
    min-width: 66px;
    justify-items: center;
    gap: 3px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-text-secondary);
    cursor: pointer;
    transform: translate(-50%, -50%);
    transition:
      color var(--duration-base) var(--ease-standard),
      transform var(--duration-base) var(--ease-standard);
  }
  .node[data-tone="violet"] { --node-color: var(--color-brand-accent); }
  .node[data-tone="emerald"] { --node-color: var(--color-success); }
  .node[data-tone="amber"] { --node-color: var(--color-warning); }
  .node:hover,
  .node.active {
    color: var(--color-text-primary);
    transform: translate(-50%, -50%) scale(1.07);
  }
  .node__core {
    width: 10px;
    height: 10px;
    margin-bottom: 2px;
    border: 2px solid color-mix(in oklab, var(--node-color) 70%, white);
    border-radius: 50%;
    background: #06080d;
    box-shadow: 0 0 0 5px color-mix(in oklab, var(--node-color) 8%, transparent), 0 0 18px var(--node-color);
  }
  .node.active .node__core {
    background: var(--node-color);
    animation: node-breathe 1.8s ease-in-out infinite;
  }
  .node strong {
    font-family: var(--font-mono);
    font-size: 0.625rem;
    font-weight: var(--font-weight-semibold);
    letter-spacing: 0.04em;
  }
  .node em {
    color: var(--node-color);
    font-family: var(--font-mono);
    font-size: 0.5625rem;
    font-style: normal;
  }
  .matrix__readout {
    align-self: stretch;
    padding: var(--space-4);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-md);
    background: rgba(5, 8, 14, 0.46);
  }
  .readout__eyebrow,
  .matrix__readout p {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: 0.5625rem;
    letter-spacing: 0.08em;
    line-height: 1.6;
    text-transform: uppercase;
  }
  .readout__title { margin-top: var(--space-3); font-weight: var(--font-weight-semibold); }
  .readout__value {
    margin-top: 3px;
    color: var(--color-brand-primary);
    font-family: var(--font-mono);
    font-size: var(--font-size-2xl);
  }
  .readout__rule {
    height: 1px;
    margin: var(--space-4) 0;
    background: linear-gradient(90deg, var(--color-brand-primary), transparent);
  }
  .relation {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
    padding: 6px 0;
    border-bottom: 1px solid rgba(255,255,255,.05);
    color: var(--color-text-secondary);
    font-family: var(--font-mono);
    font-size: 0.625rem;
  }
  .relation strong { color: var(--color-brand-accent); }
  .matrix__readout p { margin: var(--space-4) 0 0; }
  @keyframes edge-flow {
    from { left: -18%; opacity: 0; }
    20%, 80% { opacity: 0.85; }
    to { left: 100%; opacity: 0; }
  }
  @keyframes node-breathe {
    50% { box-shadow: 0 0 0 8px color-mix(in oklab, var(--node-color) 5%, transparent), 0 0 26px var(--node-color); }
  }
  @media (max-width: 720px) {
    .matrix { grid-template-columns: 1fr; }
    .matrix__readout { display: none; }
  }
</style>
