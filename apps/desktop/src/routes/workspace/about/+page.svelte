<script lang="ts">
  /**
   * About — who built this, and what it will and will not claim.
   *
   * The provenance block reads its version from the running binary rather than
   * a hardcoded string, because an About page that drifts from the build is
   * worse than no About page at all. The stance section is not marketing: it
   * restates the three rules the rest of the codebase is written to, so that
   * someone reading the terminal for the first time knows what a number here
   * means before they act on one.
   */
  import { onMount } from 'svelte';
  import { getVersion, getTauriVersion } from '@tauri-apps/api/app';
  import { platform } from '$lib/prismatik/platform.svelte';

  let appVersion = $state<string | null>(null);
  let tauriVersion = $state<string | null>(null);

  onMount(async () => {
    if (!platform.isTauri) return;
    try {
      [appVersion, tauriVersion] = await Promise.all([
        getVersion(),
        getTauriVersion(),
      ]);
    } catch {
      // A failed version read is not worth an error state; the field simply
      // reports that it is unavailable rather than inventing a number.
    }
  });

  const STANCE: Array<[string, string]> = [
    [
      'Measured, not asserted',
      'Every probability on this terminal is scored against a climatology baseline and reported as skill over that baseline. A model that cannot beat the base rate is shown as unproven, by name, on its own cohort.',
    ],
    [
      'Absence is reported',
      'Empty is drawn as empty. Retrieval lists what it could not cover, forecasts declare when the sample is insufficient, and no surface fills a gap with a plausible number.',
    ],
    [
      'Quoted voices are data',
      'Agent transcripts, documents and feeds reach a model inside fences that mark them as material to respond to, never as instructions to follow.',
    ],
    [
      'Live execution is gated',
      'Autonomous trading re-evaluates every gate on every decision — arming window, broker connection, resolved-forecast count, measured skill, breaker, drawdown ladder, budget. Any failure downgrades to paper and says why.',
    ],
  ];
</script>

<svelte:head><title>About · PRISMATIK</title></svelte:head>

<div class="about prismatik">
  <header>
    <p class="eyebrow">Mythos Systems</p>
    <h1>PRISMATIK</h1>
    <p class="tagline">A measured terminal for markets.</p>
  </header>

  <section class="provenance">
    <dl>
      <div><dt>Author</dt><dd>Aaron Stovall</dd></div>
      <div><dt>Studio</dt><dd>Mythos Systems</dd></div>
      <div><dt>Year</dt><dd>2026</dd></div>
      <div>
        <dt>Version</dt>
        <dd>{appVersion ?? (platform.isTauri ? 'reading…' : 'browser preview')}</dd>
      </div>
      <div>
        <dt>Runtime</dt>
        <dd>{tauriVersion ? `Tauri ${tauriVersion}` : 'Tauri'}</dd>
      </div>
      <div><dt>Host</dt><dd>{platform.host ?? 'resolving…'}</dd></div>
    </dl>
  </section>

  <section class="stance">
    <h2>How to read this terminal</h2>
    {#each STANCE as [title, body] (title)}
      <article>
        <h3>{title}</h3>
        <p>{body}</p>
      </article>
    {/each}
  </section>

  <footer>
    <p>
      PRISMATIK is research and decision-support software. It does not provide
      investment advice, and nothing it displays is a recommendation to buy or
      sell any instrument.
    </p>
    <p class="copyright">© 2026 Aaron Stovall · Mythos Systems</p>
  </footer>
</div>

<style>
  .about {
    max-width: 62rem;
    margin: 0 auto;
    padding: 2.5rem 2rem 4rem;
    color: var(--p-text);
  }
  .eyebrow {
    margin: 0;
    color: var(--p-accent);
    font: 600 0.66rem/1 var(--font-mono);
    letter-spacing: 0.32em;
    text-transform: uppercase;
  }
  h1 {
    margin: 0.5rem 0 0;
    font-size: 2.4rem;
    font-weight: 300;
    letter-spacing: 0.28em;
  }
  .tagline {
    margin: 0.35rem 0 0;
    color: var(--p-dim);
    font-size: 0.82rem;
  }

  .provenance {
    margin-top: 2.25rem;
    border: 1px solid var(--p-border);
    border-radius: var(--p-card-radius);
    background: var(--p-panel-fill);
    padding: 1.25rem 1.5rem;
  }
  dl {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
    gap: 1rem 2rem;
    margin: 0;
  }
  dt {
    color: var(--p-dim);
    font: 600 0.6rem var(--font-mono);
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }
  dd {
    margin: 0.3rem 0 0;
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }

  .stance {
    margin-top: 2.25rem;
  }
  h2 {
    margin: 0 0 1rem;
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.24em;
    text-transform: uppercase;
    color: var(--p-dim);
  }
  .stance article {
    border-left: 2px solid var(--p-border);
    padding: 0 0 0 1rem;
    margin-bottom: 1.35rem;
  }
  .stance h3 {
    margin: 0;
    font-size: 0.84rem;
    font-weight: 600;
    color: var(--p-accent);
  }
  .stance p {
    margin: 0.3rem 0 0;
    color: var(--p-dim);
    font-size: 0.76rem;
    line-height: 1.65;
    max-width: 52rem;
  }

  footer {
    margin-top: 2.5rem;
    border-top: 1px solid var(--p-border);
    padding-top: 1.25rem;
  }
  footer p {
    margin: 0 0 0.6rem;
    color: var(--p-dim);
    font-size: 0.7rem;
    line-height: 1.6;
    max-width: 48rem;
  }
  .copyright {
    font-family: var(--font-mono);
    letter-spacing: 0.08em;
  }
</style>
