//! Speaker-aware transcript mention and silence analysis over supplied lawful text.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Cited transcript segment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptSegment {
    /// Stable segment identifier.
    pub id: String,
    /// Named speaker when available.
    pub speaker: Option<String>,
    /// Segment text supplied by a governed provider.
    pub text: String,
    /// Event timestamp.
    pub spoken_at: OffsetDateTime,
    /// Evidence record.
    pub evidence_id: String,
}

/// Expected topic and its accepted literal aliases.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentionExpectation {
    /// Canonical topic.
    pub topic: String,
    /// Case-insensitive literal aliases.
    pub aliases: Vec<String>,
    /// Expected minimum mentions.
    pub expected_minimum: u32,
}

/// Cited mention analysis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptAnalysis {
    /// Canonical topic.
    pub topic: String,
    /// Total matching segments.
    pub mention_count: u32,
    /// Matching segment/evidence pairs.
    pub citations: Vec<(String, String)>,
    /// Mention counts by known speaker.
    pub speaker_counts: std::collections::BTreeMap<String, u32>,
    /// True when mentions fall below expectation.
    pub silence_detected: bool,
}

/// Analyze literal aliases deterministically; no semantic match is invented.
pub fn analyze_transcript(
    segments: &[TranscriptSegment],
    expectation: &MentionExpectation,
) -> TranscriptAnalysis {
    let aliases = expectation
        .aliases
        .iter()
        .chain(std::iter::once(&expectation.topic))
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut citations = Vec::new();
    let mut speakers = std::collections::BTreeMap::new();
    for segment in segments {
        let lower = segment.text.to_ascii_lowercase();
        if aliases
            .iter()
            .any(|alias| !alias.is_empty() && lower.contains(alias))
        {
            citations.push((segment.id.clone(), segment.evidence_id.clone()));
            if let Some(speaker) = &segment.speaker {
                *speakers.entry(speaker.clone()).or_default() += 1;
            }
        }
    }
    let count = u32::try_from(citations.len()).unwrap_or(u32::MAX);
    TranscriptAnalysis {
        topic: expectation.topic.clone(),
        mention_count: count,
        citations,
        speaker_counts: speakers,
        silence_detected: count < expectation.expected_minimum,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cites_mentions_and_detects_silence() {
        let segment = TranscriptSegment {
            id: "s1".into(),
            speaker: Some("Chair".into()),
            text: "Inflation remains elevated".into(),
            spoken_at: OffsetDateTime::UNIX_EPOCH,
            evidence_id: "ev".into(),
        };
        let result = analyze_transcript(
            &[segment],
            &MentionExpectation {
                topic: "inflation".into(),
                aliases: vec![],
                expected_minimum: 2,
            },
        );
        assert_eq!(result.mention_count, 1);
        assert!(result.silence_detected);
        assert_eq!(result.citations[0].1, "ev");
    }
}
