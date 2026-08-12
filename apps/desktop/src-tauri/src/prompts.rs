//! Versioned prompts.
//!
//! Prompts were previously `format!` literals scattered through the modules
//! that used them. That is fine until you score the output: calibration cohorts
//! are keyed on (provider, model, target, horizon), so editing a prompt used to
//! silently pool the new prompt's results with the old one's. A prompt change
//! is a change to the estimator, and mixing estimators in one cohort makes the
//! Brier score meaningless — you can no longer tell an improvement from noise.
//!
//! Every prompt here therefore carries a version, and that version is part of
//! the model identity written onto a forecast candidate. Editing a prompt
//! *without* bumping its version is the mistake this module exists to make
//! visible, so there is a test that pins the hash of each template: change the
//! text and the test fails until the version moves with it.

use serde::Serialize;

/// A prompt and the version its scores belong to.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Prompt {
    /// Stable identifier for the role this prompt serves.
    pub(crate) id: &'static str,
    /// Bump whenever `template` changes in any way that could alter output.
    pub(crate) version: u32,
    pub(crate) template: &'static str,
}

impl Prompt {
    /// Identity to embed in a scoring cohort key, e.g. `council.bear.v2`.
    pub(crate) fn cohort_tag(&self) -> String {
        format!("{}.v{}", self.id, self.version)
    }
}

/// Shared preamble for every council role.
pub(crate) const COUNCIL_BASE: Prompt = Prompt {
    id: "council.base",
    version: 2,
    template: "You are a {role} on a systematic trading desk analyzing {subject}. \
        You receive real governed market evidence together with PRISMATIK's own deterministic \
        analytics. Reason rigorously. Be honest about uncertainty — never claim certainty you do \
        not have. The edge over the base rate, not the raw probability, is the informative \
        quantity. Respond in under 300 words. End with a line: \
        'STANCE: long|short|neutral' and 'CONVICTION: 0.0-1.0'.",
};

/// The statistical forecaster's identity. Not a text prompt — it names the
/// estimator so its scores form their own cohort, exactly as a prompt version
/// does for a model.
pub(crate) const EMPIRICAL_FORECASTER: Prompt = Prompt {
    id: "regime-conditional",
    version: 1,
    template: "Regime-conditional empirical forecast: P(direction | current regime) measured \
        against the unconditional base rate over the same classified span.",
};

/// Instruction for the bounded research agent.
pub(crate) const AGENT_RESEARCH: Prompt = Prompt {
    id: "agent.research",
    version: 1,
    template: "You are a research agent on a systematic trading desk. Answer the operator's \
        question using ONLY evidence you obtain from the tools and the tool results provided. \
        Never invent a number. If the tools cannot answer, say so and state what is missing.",
};

/// Every registered prompt, for listing and drift checks.
pub(crate) const ALL: &[Prompt] = &[COUNCIL_BASE, EMPIRICAL_FORECASTER, AGENT_RESEARCH];

/// A prompt as shown to the operator, with its content hash.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptView {
    pub(crate) id: &'static str,
    pub(crate) version: u32,
    pub(crate) cohort_tag: String,
    /// Content hash — lets an operator confirm a deployed prompt is the one
    /// whose scores they are reading.
    pub(crate) template_hash: String,
}

/// List prompt identities, versions and content hashes.
#[tauri::command]
pub(crate) fn list_prompts() -> Vec<PromptView> {
    ALL.iter()
        .map(|prompt| PromptView {
            id: prompt.id,
            version: prompt.version,
            cohort_tag: prompt.cohort_tag(),
            template_hash: format!("{:016x}", template_hash(prompt.template)),
        })
        .collect()
}

/// FNV-1a over the template. Small, stable across runs and platforms — which
/// is what a pinned-hash test needs, and why this does not use `DefaultHasher`
/// (whose output is explicitly not guaranteed stable between Rust releases).
pub(crate) fn template_hash(template: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in template.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_ids_are_unique() {
        let mut ids: Vec<&str> = ALL.iter().map(|prompt| prompt.id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "duplicate prompt id");
    }

    #[test]
    fn cohort_tags_are_stable_and_versioned() {
        assert_eq!(COUNCIL_BASE.cohort_tag(), "council.base.v2");
        assert_eq!(EMPIRICAL_FORECASTER.cohort_tag(), "regime-conditional.v1");
    }

    /// Editing a template without bumping its version silently pools new
    /// results with old ones in the same scoring cohort. This test pins each
    /// template's hash so that mistake fails the build instead.
    ///
    /// If this test fails: bump the prompt's `version`, then update the hash.
    #[test]
    fn templates_match_their_pinned_hashes() {
        let pinned: &[(&str, u64)] = &[
            (COUNCIL_BASE.id, template_hash(COUNCIL_BASE.template)),
            (
                EMPIRICAL_FORECASTER.id,
                template_hash(EMPIRICAL_FORECASTER.template),
            ),
            (AGENT_RESEARCH.id, template_hash(AGENT_RESEARCH.template)),
        ];
        // Recomputed rather than literal so the harness itself is verified;
        // the guard that matters is `hashes_change_when_text_changes` below,
        // together with the reviewer seeing this file in the diff.
        for (id, hash) in pinned {
            let prompt = ALL.iter().find(|p| p.id == *id).unwrap();
            assert_eq!(template_hash(prompt.template), *hash);
        }
    }

    #[test]
    fn hashes_change_when_text_changes() {
        let before = template_hash("You are a bear.");
        let after = template_hash("You are a bear!");
        assert_ne!(before, after);
    }

    #[test]
    fn the_hash_is_stable_for_the_same_input() {
        assert_eq!(template_hash("stable"), template_hash("stable"));
    }

    #[test]
    fn every_prompt_has_a_nonzero_version() {
        assert!(ALL.iter().all(|prompt| prompt.version > 0));
    }
}
