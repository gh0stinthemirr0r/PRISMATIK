# PRISMATIK Agent Notes

Before modifying this repository, read
[`DOCS/AGENT_BRIEFING.md`](DOCS/AGENT_BRIEFING.md) in full. It is the canonical
agent handoff for architecture invariants, verified gate status, known traps,
proposal boundaries, and the product vision.

In particular:

- Part I contains operational constraints and outranks generic implementation
  directives when they conflict with PRISMATIK's invariants.
- Parts II and III are proposals unless the owner separately approves their
  implementation.
- Part IV records the owner-endorsed ingestion and corpus-moat direction,
  including the Claude discussion supplied on 2026-07-27. It requires lawful,
  append-only, observation-timestamped ingestion; attention/diffusion/silence
  analysis; global structured-public-data coverage; point-in-time-safe lead
  expansion; and calibrated context rather than latency claims.
- Strategic endorsement does not authorize accepting provider terms, hostile
  scraping, licensed-data purchases, personal-data collection, redistribution
  of copyrighted content, new crate commitments, or regulatory decisions.

Preserve the distinction between:

1. facts and decisions already verified or approved;
2. owner-endorsed strategic direction;
3. research proposals;
4. legal, licensing, regulatory, cost, and scope decisions that require a
   human.

