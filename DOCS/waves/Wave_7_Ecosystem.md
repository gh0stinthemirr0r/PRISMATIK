# Wave 7 — Ecosystem

**Maps to v1.0 phases:** P9 (Ecosystem and Marketplace)
**Business outcome:** third business. Third-party extension and published contracts.
**Duration:** ~41 days.
**Date:** 2026-07-26

---

## Objective

Open the ecosystem. Third parties can build, sign, submit, and publish plugins without Mythos Systems writing code. Third parties can verify a PRISMATIK research bundle using only the published manifest schema and the standalone verifier, with no PRISMATIK installation.

This is the wave where the moat deepens from "we built it" to "the community built around it." Competitors can copy features but not the body of reproducible, third-party-verifiable research artifacts the community has produced. Each passing month adds to that corpus.

## Entry Criteria

Wave 6 exit gate **plus demonstrated third-party demand.** A marketplace with no plugin developers is a gallery of empty shelves.

## Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P9-SS-01 | SS | Plugin marketplace: submission, review, signing, revocation | 10 | L |
| P9-SS-02 | SS | Plugin capability review workflow and publisher identity | 6 | M |
| P9-OD-01 | OD | Publish plugin SDK, chart contracts, indicator trait, manifest schema under Apache-2.0 | 6 | M |
| P9-OD-02 | OD | Public documentation site, examples, contribution guide | 8 | M |
| P9-QM-01 | QM | Third-party manifest verification service and public tree-head publication | 6 | L |
| P9-EX-01 | EX | In-application marketplace surface with capability disclosure before install | 5 | M |

## The Strategic Logic

Wave 7 is small in engineering days (~41) but large in strategic significance. It is the wave where three things become possible that were not possible before:

1. **Third-party verifiability of PRISMATIK research artifacts.** A researcher publishes a PRISMATIK research bundle. A regulator, an acquirer, a counterparty, or an academic peer verifies it using only the published Apache-2.0 manifest schema and the standalone `prismatik-cli verify` tool — no PRISMATIK installation, no network access, no trust in Mythos Systems required. The bundle contains the manifest, the pinned artifact references with their hashes, the metrics, the audit inclusion proof, and the dual signature. This is the concrete engineering of v1.0 §14.4, and it costs one schema publication.

2. **An ecosystem of plugin developers.** The plugin SDK, chart contracts, indicator trait, and manifest schema are published under Apache-2.0. Third parties build plugins against the published contracts. The capability-diffing gate (Wave 3) ensures that a plugin update that newly imports a network capability fails the marketplace review automatically. Publisher identity is verified.

3. **Citable artifacts in academic and regulatory contexts.** The manifest schema becomes a citable artifact. Researchers reference PRISMATIK research bundles in papers, with verifiable reproducibility as a property of the citation. This is uncommon in the retail/prosumer category and directly extends the Verifiability Moat (Enterprise Overview §7.1).

## The License Boundary (Restated)

Per v1.0 Architecture §8 amendment three: the trusted core, the Determinism Kernel, the strategy IR, the risk policy, the evidence graph, and the design system are proprietary. **The plugin SDK, the chart contracts, the indicator trait, and the manifest schema are published under Apache-2.0.** Publishing the contracts and keeping the implementation is the posture that builds an ecosystem without giving away the product, and it makes the manifest schema citable by third parties who need to verify a PRISMATIK research artifact — which is the point of v1.0 §14.4.

## Definition of Done

| # | Criterion | Verified by |
|---|---|---|
| 1 | A third party can build, sign, submit, and publish a plugin without Mythos Systems writing code | End-to-end external beta (≥1 third-party plugin shipped) |
| 2 | A third party can verify a PRISMATIK research bundle using only the published schema and the standalone verifier, with no PRISMATIK installation | External verification test, recorded |
| 3 | The marketplace surface shows capability disclosure before install | UI review |
| 4 | Plugin capability-diffing gate blocks an update that newly imports a network capability | Committed negative test |
| 5 | Public tree-head publication allows third-party audit of the manifest verification service | Public tree head live, third-party audit recorded |
| 6 | Public documentation site covers SDK, contracts, schema, and contribution guide | Documentation review |

## What Comes After Wave 7

The wave framework ends here, but the product does not. Wave 7's completion opens the door to the items filed as "Explicitly Not Now" in the Enterprise Overview Part VIII:

- **Causal market graph** (v0.1 §37.1) — research project, post-Wave 7
- **Market digital twin** (v0.1 §37.2) — depends on microstructure data PRISMATIK doesn't have
- **Federated strategy learning** (v0.1 §37.3) — requires multiple enterprise customers
- **Confidential computing / TEE broker integration** (v0.1 §37.4) — research-stage for LLM workloads in 2026; 2027+ pilot
- **Multi-agent research council** (v0.1 §37.7) — single AI analyst proven by Wave 4
- **Pine Script compatibility** (v0.4 §12) — provenance and licensing risk exceeds value
- **Additional TSFM families** — five artifacts prove the registry; more is dilution

These are deliberately deferred, not cancelled. The wave framework's job is to make the deferral explicit and the revisit trigger clear, so the same debate doesn't recur quarterly.

## Risks

| Risk | Response |
|---|---|
| No third-party plugin developers materialize; marketplace remains empty | This is the entry-criteria test. If Wave 7 opens with no demonstrated demand, it does not open. |
| Published contracts constrain future evolution (Apache-2.0 schema can't break compatibly) | Versioning from day one. Schema versions are explicit; the standalone verifier supports a range. Breaking changes require a major version bump and a migration path. |
| Plugin supply-chain attack via a signed-but-malicious plugin | Capability-diffing gate is the load-bearing check. Defense-in-depth: sandbox (wasmtime), capability scoping, publisher identity, marketplace review, signed artifacts. No single control is sufficient. |
| Manifest schema becomes a citation standard but evolves incompatibly | Schema versioning + deprecation policy. Public tree-head publication gives third parties a stable trust anchor. |

## Commercial Metrics

| Metric | Wave 7+ target |
|---|---|
| Third-party plugins published | 10+ within 12 months |
| Research bundles verified by third parties | 1,000+ in the wild |
| Revenue | Marketplace revenue share on paid plugins; free plugins free |
| Community contributors | 20+ external contributors |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26*
