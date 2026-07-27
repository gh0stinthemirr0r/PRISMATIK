//! Criterion benchmark: audit ledger append vs 1ms p99 budget (P0-DK-08).
//!
//! Run:
//! ```text
//! cargo bench -p prismatik-audit --bench audit_append
//! ```
//!
//! Report: `crates/prismatik-audit/benches/AUDIT_APPEND_P99.md`
//! and `DOCS/waves/P0_DK_08_Audit_Append_Benchmark.md`.

#![allow(clippy::disallowed_methods)] // criterion / wall-clock timing only

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use prismatik_audit::{
    Actor, AuditAction, AuditEntry, AuditLedger, InMemoryAuditLedger, Outcome, RedactedJson,
    SubjectRef,
};
use prismatik_determinism::ContentHash;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::runtime::Runtime;

fn sample_entry(prev: ContentHash, code: &str) -> AuditEntry {
    AuditEntry {
        occurred_at: OffsetDateTime::UNIX_EPOCH,
        actor: Actor::System {
            component: "bench".into(),
        },
        action: AuditAction::Operational { code: code.into() },
        subject: SubjectRef::None,
        outcome: Outcome::Allowed,
        prev_hash: prev,
        detail: RedactedJson::default(),
    }
}

fn bench_append(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime");
    let mut group = c.benchmark_group("audit_append");
    group.sample_size(200);
    group.measurement_time(Duration::from_secs(8));
    group.warm_up_time(Duration::from_secs(2));

    // Single append on a fresh ledger (interactive-path budget).
    group.bench_function(BenchmarkId::new("in_memory", "single_append"), |b| {
        b.iter(|| {
            let ledger = InMemoryAuditLedger::new();
            let prev = ledger.tip_hash();
            let receipt = rt
                .block_on(ledger.append(sample_entry(prev, "evt")))
                .expect("append");
            black_box(receipt);
        });
    });

    // Append onto a pre-warmed tree (~1024 leaves) — closer to steady-state.
    group.bench_function(BenchmarkId::new("in_memory", "append_at_1k"), |b| {
        b.iter_batched(
            || {
                let ledger = InMemoryAuditLedger::new();
                let mut prev = ledger.tip_hash();
                for i in 0..1024 {
                    let receipt = rt
                        .block_on(ledger.append(sample_entry(prev, &format!("warm-{i}"))))
                        .expect("warm append");
                    prev = prismatik_audit::hash_leaf(&receipt.leaf_hash);
                }
                (ledger, prev)
            },
            |(ledger, prev)| {
                let receipt = rt
                    .block_on(ledger.append(sample_entry(prev, "measured")))
                    .expect("append");
                black_box(receipt);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_append);
criterion_main!(benches);
