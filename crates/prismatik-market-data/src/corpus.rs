//! Lawful, point-in-time corpus acquisition contracts.
//!
//! This module deliberately contains no network or filesystem code. It defines
//! the policy gate and append-only observation semantics that every concrete
//! collector and storage adapter must honor.

use prismatik_determinism::{Clock, ContentHash};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use time::OffsetDateTime;

/// Stable identifier for an approved source or registry.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceId(String);

impl SourceId {
    /// Construct a non-empty source identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, CorpusError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CorpusError::EmptySourceId);
        }
        Ok(Self(value))
    }

    /// Return the stable identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Monotonically increasing source-policy version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourcePolicyVersion(u32);

impl SourcePolicyVersion {
    /// Construct a non-zero source-policy version.
    pub fn new(value: u32) -> Result<Self, CorpusError> {
        if value == 0 {
            return Err(CorpusError::ZeroPolicyVersion);
        }
        Ok(Self(value))
    }

    /// Return the numeric version.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Approved mechanism used to retrieve an observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionMethod {
    /// Official or publisher-provided API.
    Api,
    /// Official bulk download or public dataset.
    Bulk,
    /// Publisher-provided RSS or Atom feed.
    Rss,
    /// Publisher-provided sitemap.
    Sitemap,
    /// Contractually licensed feed.
    Licensed,
    /// Public page whose terms and robots policy permit retrieval.
    PermittedWeb,
}

/// Maximum payload retention authorized by a source policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadRetention {
    /// Keep provenance metadata and a payload hash, but no raw object.
    MetadataOnly,
    /// A raw payload object may be retained for deterministic replay.
    RawPayloadPermitted,
}

/// Maximum redistribution authorized by a source policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedistributionPolicy {
    /// No redistribution outside the local evidence store.
    Prohibited,
    /// Only the explicitly permitted metadata may be redistributed.
    PermittedMetadataOnly,
    /// Raw payload redistribution is authorized by the governing terms.
    RawPayloadPermitted,
}

/// Optional metadata whose retention must be explicitly authorized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermittedMetadataField {
    /// Publisher-provided record identifier.
    PublisherRecordId,
    /// Selected HTTP or feed provenance headers.
    ResponseHeaders,
    /// Source language tag.
    Language,
    /// Source-declared title.
    Title,
    /// Source-declared byline.
    Byline,
    /// Copyright- and license-bounded excerpt.
    Excerpt,
    /// Source-declared or extracted entity labels.
    Entities,
}

/// A reviewed, time-bounded acquisition policy for one source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePolicy {
    source_id: SourceId,
    version: SourcePolicyVersion,
    allowed_methods: BTreeSet<AcquisitionMethod>,
    payload_retention: PayloadRetention,
    redistribution: RedistributionPolicy,
    permitted_metadata_fields: BTreeSet<PermittedMetadataField>,
    terms_owner: String,
    reviewed_at: OffsetDateTime,
    valid_from: OffsetDateTime,
    valid_until: OffsetDateTime,
    review_reference: String,
}

impl SourcePolicy {
    /// Construct a reviewed policy with an explicit validity window.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_id: SourceId,
        version: SourcePolicyVersion,
        allowed_methods: impl IntoIterator<Item = AcquisitionMethod>,
        payload_retention: PayloadRetention,
        redistribution: RedistributionPolicy,
        permitted_metadata_fields: impl IntoIterator<Item = PermittedMetadataField>,
        terms_owner: impl Into<String>,
        reviewed_at: OffsetDateTime,
        valid_from: OffsetDateTime,
        valid_until: OffsetDateTime,
        review_reference: impl Into<String>,
    ) -> Result<Self, CorpusError> {
        let allowed_methods = allowed_methods.into_iter().collect::<BTreeSet<_>>();
        if allowed_methods.is_empty() {
            return Err(CorpusError::NoAcquisitionMethods);
        }
        if valid_until <= valid_from {
            return Err(CorpusError::InvalidPolicyWindow);
        }
        let review_reference = review_reference.into();
        if review_reference.trim().is_empty() {
            return Err(CorpusError::EmptyReviewReference);
        }
        if redistribution == RedistributionPolicy::RawPayloadPermitted
            && payload_retention != PayloadRetention::RawPayloadPermitted
        {
            return Err(CorpusError::RedistributionExceedsRetention);
        }
        let policy = Self {
            source_id,
            version,
            allowed_methods,
            payload_retention,
            redistribution,
            permitted_metadata_fields: permitted_metadata_fields.into_iter().collect(),
            terms_owner: terms_owner.into(),
            reviewed_at,
            valid_from,
            valid_until,
            review_reference,
        };
        policy.validate()?;
        Ok(policy)
    }

    /// Return the source governed by this policy.
    pub fn source_id(&self) -> &SourceId {
        &self.source_id
    }

    /// Return this policy's version.
    pub const fn version(&self) -> SourcePolicyVersion {
        self.version
    }

    /// Return the allowed acquisition methods.
    pub fn allowed_methods(&self) -> &BTreeSet<AcquisitionMethod> {
        &self.allowed_methods
    }

    /// Return the raw-payload retention posture.
    pub const fn payload_retention(&self) -> PayloadRetention {
        self.payload_retention
    }

    /// Return the redistribution posture.
    pub const fn redistribution(&self) -> RedistributionPolicy {
        self.redistribution
    }

    /// Return optional metadata fields approved for retention.
    pub fn permitted_metadata_fields(&self) -> &BTreeSet<PermittedMetadataField> {
        &self.permitted_metadata_fields
    }

    /// Return the legal or contractual authority responsible for the terms.
    pub fn terms_owner(&self) -> &str {
        &self.terms_owner
    }

    /// Return the time at which this policy was reviewed.
    pub const fn reviewed_at(&self) -> OffsetDateTime {
        self.reviewed_at
    }

    /// Return the inclusive policy start time.
    pub const fn valid_from(&self) -> OffsetDateTime {
        self.valid_from
    }

    /// Return the exclusive policy expiration time.
    pub const fn valid_until(&self) -> OffsetDateTime {
        self.valid_until
    }

    /// Return the human-auditable policy or terms review reference.
    pub fn review_reference(&self) -> &str {
        &self.review_reference
    }

    fn authorize(
        &self,
        method: AcquisitionMethod,
        observation_time: OffsetDateTime,
        has_raw_object: bool,
        used_metadata_fields: &BTreeSet<PermittedMetadataField>,
    ) -> Result<(), CorpusError> {
        if observation_time < self.valid_from {
            return Err(CorpusError::PolicyNotYetValid {
                source_id: self.source_id.clone(),
                version: self.version,
            });
        }
        if observation_time >= self.valid_until {
            return Err(CorpusError::PolicyExpired {
                source_id: self.source_id.clone(),
                version: self.version,
            });
        }
        if !self.allowed_methods.contains(&method) {
            return Err(CorpusError::AcquisitionMethodDenied {
                source_id: self.source_id.clone(),
                method,
            });
        }
        if has_raw_object && self.payload_retention == PayloadRetention::MetadataOnly {
            return Err(CorpusError::RawPayloadRetentionDenied {
                source_id: self.source_id.clone(),
            });
        }
        if let Some(field) = used_metadata_fields
            .difference(&self.permitted_metadata_fields)
            .next()
        {
            return Err(CorpusError::MetadataFieldDenied {
                source_id: self.source_id.clone(),
                field: *field,
            });
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), CorpusError> {
        if self.source_id.as_str().trim().is_empty() {
            return Err(CorpusError::EmptySourceId);
        }
        if self.version.get() == 0 {
            return Err(CorpusError::ZeroPolicyVersion);
        }
        if self.allowed_methods.is_empty() {
            return Err(CorpusError::NoAcquisitionMethods);
        }
        if self.valid_until <= self.valid_from {
            return Err(CorpusError::InvalidPolicyWindow);
        }
        if self.review_reference.trim().is_empty() {
            return Err(CorpusError::EmptyReviewReference);
        }
        if self.terms_owner.trim().is_empty() {
            return Err(CorpusError::EmptyTermsOwner);
        }
        if self.reviewed_at > self.valid_from {
            return Err(CorpusError::ReviewAfterPolicyActivation);
        }
        if self.redistribution == RedistributionPolicy::RawPayloadPermitted
            && self.payload_retention != PayloadRetention::RawPayloadPermitted
        {
            return Err(CorpusError::RedistributionExceedsRetention);
        }
        Ok(())
    }
}

/// Immutable registry of approved source-policy versions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePolicyRegistry {
    policies: BTreeMap<SourceId, BTreeMap<SourcePolicyVersion, SourcePolicy>>,
}

impl SourcePolicyRegistry {
    /// Register a new source-policy version without replacing an older version.
    pub fn register(&mut self, policy: SourcePolicy) -> Result<(), CorpusError> {
        policy.validate()?;
        let versions = self.policies.entry(policy.source_id.clone()).or_default();
        if versions.contains_key(&policy.version) {
            return Err(CorpusError::DuplicatePolicyVersion {
                source_id: policy.source_id,
                version: policy.version,
            });
        }
        versions.insert(policy.version, policy);
        Ok(())
    }

    /// Resolve and authorize the exact policy pinned by an observation.
    pub fn authorize(&self, observation: &Observation) -> Result<&SourcePolicy, CorpusError> {
        let versions = self
            .policies
            .get(&observation.source_id)
            .ok_or_else(|| CorpusError::UnregisteredSource(observation.source_id.clone()))?;
        let policy = versions
            .get(&observation.source_policy_version)
            .ok_or_else(|| CorpusError::UnregisteredPolicyVersion {
                source_id: observation.source_id.clone(),
                version: observation.source_policy_version,
            })?;
        policy.authorize(
            observation.acquisition_method,
            observation.observation_time,
            observation.raw_object_ref.is_some(),
            &observation.metadata_fields(),
        )?;
        Ok(policy)
    }

    /// Return a registered policy by exact source and version.
    pub fn get(&self, source_id: &SourceId, version: SourcePolicyVersion) -> Option<&SourcePolicy> {
        self.policies
            .get(source_id)
            .and_then(|versions| versions.get(&version))
    }
}

macro_rules! hash_id {
    ($name:ident, $docs:literal) => {
        #[doc = $docs]
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Construct an identifier from a stable 32-byte digest.
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Return the identifier bytes.
            pub const fn as_bytes(self) -> [u8; 32] {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.0 {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }
    };
}

hash_id!(
    ObservationId,
    "Content-derived identifier for one immutable observation."
);
hash_id!(
    RetrievalAttemptId,
    "Stable identifier for the retrieval attempt that produced an observation."
);
hash_id!(
    IngestManifestId,
    "Stable identifier for the pinned ingest manifest and collector configuration."
);

/// Copyright- and license-bounded metadata retained with an observation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermittedMetadata {
    /// Source-declared title, when permitted.
    pub title: Option<String>,
    /// Source-declared byline, when permitted.
    pub byline: Option<String>,
    /// Permitted excerpt, when one may lawfully be retained.
    pub excerpt: Option<String>,
    /// Source-declared or extracted entity labels.
    pub entities: Vec<String>,
}

/// Candidate observation awaiting policy authorization and immutable sealing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationDraft {
    /// Source being observed.
    pub source_id: SourceId,
    /// Exact source-policy version used for acquisition.
    pub source_policy_version: SourcePolicyVersion,
    /// Retrieval mechanism.
    pub acquisition_method: AcquisitionMethod,
    /// Canonical source URI.
    pub source_uri: String,
    /// Publisher-provided record identifier, when available.
    pub publisher_record_id: Option<String>,
    /// Time the described event occurred, when known.
    pub event_time: Option<OffsetDateTime>,
    /// Publisher-declared publication time, when known.
    pub publication_time: Option<OffsetDateTime>,
    /// Time PRISMATIK first received these bytes or permitted metadata.
    pub observation_time: OffsetDateTime,
    /// Selected provenance headers such as ETag and Last-Modified.
    pub response_headers: BTreeMap<String, String>,
    /// BCP-47 language tag, when known.
    pub language: Option<String>,
    /// Source media type.
    pub media_type: String,
    /// BLAKE3 hash of the observed payload, even when raw retention is denied.
    pub payload_hash: ContentHash,
    /// Content-addressed raw object reference, only when policy permits it.
    pub raw_object_ref: Option<String>,
    /// Metadata retained to the licensed or otherwise permitted extent.
    pub permitted_metadata: PermittedMetadata,
    /// Earlier observation corrected or changed by this observation.
    pub supersedes: Option<ObservationId>,
    /// Retrieval-attempt identity.
    pub retrieval_attempt_id: RetrievalAttemptId,
    /// Ingest-manifest identity.
    pub ingest_manifest_id: IngestManifestId,
}

impl ObservationDraft {
    /// Start an observation draft using the required determinism clock.
    ///
    /// Optional publisher, event, provenance, language, retention, metadata,
    /// and supersession fields can be populated before sealing.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        clock: &dyn Clock,
        source_id: SourceId,
        source_policy_version: SourcePolicyVersion,
        acquisition_method: AcquisitionMethod,
        source_uri: impl Into<String>,
        media_type: impl Into<String>,
        payload_hash: ContentHash,
        retrieval_attempt_id: RetrievalAttemptId,
        ingest_manifest_id: IngestManifestId,
    ) -> Result<Self, CorpusError> {
        let draft = Self {
            source_id,
            source_policy_version,
            acquisition_method,
            source_uri: source_uri.into(),
            publisher_record_id: None,
            event_time: None,
            publication_time: None,
            observation_time: clock.now(),
            response_headers: BTreeMap::new(),
            language: None,
            media_type: media_type.into(),
            payload_hash,
            raw_object_ref: None,
            permitted_metadata: PermittedMetadata::default(),
            supersedes: None,
            retrieval_attempt_id,
            ingest_manifest_id,
        };
        draft.validate()?;
        Ok(draft)
    }

    /// Validate required textual fields before policy authorization.
    pub fn validate(&self) -> Result<(), CorpusError> {
        if self.source_uri.trim().is_empty() {
            return Err(CorpusError::EmptySourceUri);
        }
        if self.media_type.trim().is_empty() {
            return Err(CorpusError::EmptyMediaType);
        }
        if self
            .raw_object_ref
            .as_ref()
            .is_some_and(|reference| reference.trim().is_empty())
        {
            return Err(CorpusError::EmptyRawObjectReference);
        }
        Ok(())
    }

    fn metadata_fields(&self) -> BTreeSet<PermittedMetadataField> {
        metadata_fields(
            self.publisher_record_id.is_some(),
            !self.response_headers.is_empty(),
            self.language.is_some(),
            &self.permitted_metadata,
        )
    }
}

/// Immutable, policy-authorized corpus observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    observation_id: ObservationId,
    source_id: SourceId,
    source_policy_version: SourcePolicyVersion,
    acquisition_method: AcquisitionMethod,
    source_uri: String,
    publisher_record_id: Option<String>,
    event_time: Option<OffsetDateTime>,
    publication_time: Option<OffsetDateTime>,
    observation_time: OffsetDateTime,
    response_headers: BTreeMap<String, String>,
    language: Option<String>,
    media_type: String,
    payload_hash: ContentHash,
    raw_object_ref: Option<String>,
    permitted_metadata: PermittedMetadata,
    supersedes: Option<ObservationId>,
    retrieval_attempt_id: RetrievalAttemptId,
    ingest_manifest_id: IngestManifestId,
}

impl Observation {
    /// Authorize and seal an observation draft under an exact source policy.
    pub fn seal(draft: ObservationDraft, policy: &SourcePolicy) -> Result<Self, CorpusError> {
        draft.validate()?;
        if draft.source_id != policy.source_id || draft.source_policy_version != policy.version {
            return Err(CorpusError::PolicyIdentityMismatch);
        }
        policy.authorize(
            draft.acquisition_method,
            draft.observation_time,
            draft.raw_object_ref.is_some(),
            &draft.metadata_fields(),
        )?;
        let observation_id = observation_id_for(&draft);
        Ok(Self {
            observation_id,
            source_id: draft.source_id,
            source_policy_version: draft.source_policy_version,
            acquisition_method: draft.acquisition_method,
            source_uri: draft.source_uri,
            publisher_record_id: draft.publisher_record_id,
            event_time: draft.event_time,
            publication_time: draft.publication_time,
            observation_time: draft.observation_time,
            response_headers: draft.response_headers,
            language: draft.language,
            media_type: draft.media_type,
            payload_hash: draft.payload_hash,
            raw_object_ref: draft.raw_object_ref,
            permitted_metadata: draft.permitted_metadata,
            supersedes: draft.supersedes,
            retrieval_attempt_id: draft.retrieval_attempt_id,
            ingest_manifest_id: draft.ingest_manifest_id,
        })
    }

    /// Return the immutable observation identifier.
    pub const fn id(&self) -> ObservationId {
        self.observation_id
    }

    /// Return the source identifier.
    pub fn source_id(&self) -> &SourceId {
        &self.source_id
    }

    /// Return the pinned source-policy version.
    pub const fn source_policy_version(&self) -> SourcePolicyVersion {
        self.source_policy_version
    }

    /// Return the acquisition mechanism.
    pub const fn acquisition_method(&self) -> AcquisitionMethod {
        self.acquisition_method
    }

    /// Return the canonical source URI.
    pub fn source_uri(&self) -> &str {
        &self.source_uri
    }

    /// Return the publisher record identifier, when available.
    pub fn publisher_record_id(&self) -> Option<&str> {
        self.publisher_record_id.as_deref()
    }

    /// Return the described event time, when known.
    pub const fn event_time(&self) -> Option<OffsetDateTime> {
        self.event_time
    }

    /// Return the publisher-declared publication time, when known.
    pub const fn publication_time(&self) -> Option<OffsetDateTime> {
        self.publication_time
    }

    /// Return PRISMATIK's observation time.
    pub const fn observation_time(&self) -> OffsetDateTime {
        self.observation_time
    }

    /// Return selected provenance response headers.
    pub fn response_headers(&self) -> &BTreeMap<String, String> {
        &self.response_headers
    }

    /// Return the BCP-47 language tag, when known.
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Return the source media type.
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// Return the hash of the observed payload.
    pub const fn payload_hash(&self) -> ContentHash {
        self.payload_hash
    }

    /// Return the raw object reference, when retention is permitted.
    pub fn raw_object_ref(&self) -> Option<&str> {
        self.raw_object_ref.as_deref()
    }

    /// Return the permitted metadata.
    pub fn permitted_metadata(&self) -> &PermittedMetadata {
        &self.permitted_metadata
    }

    /// Return the superseded observation, when this is a correction or edit.
    pub const fn supersedes(&self) -> Option<ObservationId> {
        self.supersedes
    }

    /// Return the retrieval-attempt identifier.
    pub const fn retrieval_attempt_id(&self) -> RetrievalAttemptId {
        self.retrieval_attempt_id
    }

    /// Return the ingest-manifest identifier.
    pub const fn ingest_manifest_id(&self) -> IngestManifestId {
        self.ingest_manifest_id
    }

    /// Report source-clock anomalies without discarding the observation.
    pub fn temporal_anomalies(&self) -> BTreeSet<TemporalAnomaly> {
        let mut anomalies = BTreeSet::new();
        if self
            .event_time
            .is_some_and(|event_time| event_time > self.observation_time)
        {
            anomalies.insert(TemporalAnomaly::EventAfterObservation);
        }
        if self
            .publication_time
            .is_some_and(|publication_time| publication_time > self.observation_time)
        {
            anomalies.insert(TemporalAnomaly::PublicationAfterObservation);
        }
        anomalies
    }

    fn verify_integrity(&self) -> Result<(), CorpusError> {
        let draft = self.as_draft();
        draft.validate()?;
        let expected = observation_id_for(&draft);
        if self.observation_id != expected {
            return Err(CorpusError::ObservationIdMismatch {
                expected,
                found: self.observation_id,
            });
        }
        Ok(())
    }

    fn metadata_fields(&self) -> BTreeSet<PermittedMetadataField> {
        metadata_fields(
            self.publisher_record_id.is_some(),
            !self.response_headers.is_empty(),
            self.language.is_some(),
            &self.permitted_metadata,
        )
    }

    fn as_draft(&self) -> ObservationDraft {
        ObservationDraft {
            source_id: self.source_id.clone(),
            source_policy_version: self.source_policy_version,
            acquisition_method: self.acquisition_method,
            source_uri: self.source_uri.clone(),
            publisher_record_id: self.publisher_record_id.clone(),
            event_time: self.event_time,
            publication_time: self.publication_time,
            observation_time: self.observation_time,
            response_headers: self.response_headers.clone(),
            language: self.language.clone(),
            media_type: self.media_type.clone(),
            payload_hash: self.payload_hash,
            raw_object_ref: self.raw_object_ref.clone(),
            permitted_metadata: self.permitted_metadata.clone(),
            supersedes: self.supersedes,
            retrieval_attempt_id: self.retrieval_attempt_id,
            ingest_manifest_id: self.ingest_manifest_id,
        }
    }
}

/// A source-clock relationship that should be measured rather than hidden.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporalAnomaly {
    /// The source-declared event time is later than observation time.
    EventAfterObservation,
    /// The source-declared publication time is later than observation time.
    PublicationAfterObservation,
}

/// Result of an idempotent append operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendOutcome {
    /// A new immutable observation was appended.
    Inserted {
        /// Zero-based append position.
        position: usize,
    },
    /// The exact observation was already present.
    AlreadyPresent {
        /// Existing zero-based append position.
        position: usize,
    },
}

/// Deterministic in-memory model of the append-only observation contract.
///
/// Durable adapters should preserve the same behavior transactionally.
#[derive(Clone, Debug, Default)]
pub struct ObservationLog {
    observations: Vec<Observation>,
    positions: BTreeMap<ObservationId, usize>,
    superseded_by: BTreeMap<ObservationId, ObservationId>,
}

impl ObservationLog {
    /// Validate and append an observation without mutating earlier entries.
    pub fn append(
        &mut self,
        observation: Observation,
        policies: &SourcePolicyRegistry,
    ) -> Result<AppendOutcome, CorpusError> {
        observation.verify_integrity()?;
        policies.authorize(&observation)?;

        if let Some(position) = self.positions.get(&observation.id()).copied() {
            return Ok(AppendOutcome::AlreadyPresent { position });
        }

        if let Some(superseded_id) = observation.supersedes {
            let superseded_position = self
                .positions
                .get(&superseded_id)
                .copied()
                .ok_or(CorpusError::UnknownSupersededObservation(superseded_id))?;
            let superseded = &self.observations[superseded_position];
            if superseded.source_id != observation.source_id {
                return Err(CorpusError::CrossSourceSupersession);
            }
            if observation.observation_time < superseded.observation_time {
                return Err(CorpusError::SupersessionMovesBackwardInTime);
            }
            let same_publisher_record = observation.publisher_record_id.is_some()
                && observation.publisher_record_id == superseded.publisher_record_id;
            if !same_publisher_record && observation.source_uri != superseded.source_uri {
                return Err(CorpusError::SupersessionIdentityMismatch);
            }
            if let Some(existing) = self.superseded_by.get(&superseded_id) {
                return Err(CorpusError::SupersessionFork {
                    superseded: superseded_id,
                    existing: *existing,
                });
            }
        }

        let position = self.observations.len();
        if let Some(superseded_id) = observation.supersedes {
            self.superseded_by.insert(superseded_id, observation.id());
        }
        self.positions.insert(observation.id(), position);
        self.observations.push(observation);
        Ok(AppendOutcome::Inserted { position })
    }

    /// Return the number of immutable observations.
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Return whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    /// Resolve an immutable observation by id.
    pub fn get(&self, id: ObservationId) -> Option<&Observation> {
        self.positions
            .get(&id)
            .map(|position| &self.observations[*position])
    }

    /// Return the observation that directly supersedes an id, when present.
    pub fn superseded_by(&self, id: ObservationId) -> Option<ObservationId> {
        self.superseded_by.get(&id).copied()
    }

    /// Iterate in deterministic append order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Observation> {
        self.observations.iter()
    }
}

/// Source-policy, observation, and append-only validation failures.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CorpusError {
    /// A source identifier was blank.
    #[error("source id cannot be empty")]
    EmptySourceId,
    /// Policy version zero is reserved as invalid.
    #[error("source-policy version must be non-zero")]
    ZeroPolicyVersion,
    /// A source policy approved no acquisition mechanisms.
    #[error("source policy must approve at least one acquisition method")]
    NoAcquisitionMethods,
    /// Policy expiration was not later than activation.
    #[error("source-policy validity window is empty or reversed")]
    InvalidPolicyWindow,
    /// The policy omitted its auditable terms or legal review reference.
    #[error("source policy requires a review reference")]
    EmptyReviewReference,
    /// The policy omitted the legal or contractual owner of the terms.
    #[error("source policy requires a terms owner")]
    EmptyTermsOwner,
    /// A policy claimed activation before its recorded review.
    #[error("source policy review must occur no later than policy activation")]
    ReviewAfterPolicyActivation,
    /// Redistribution permission exceeded retention permission.
    #[error("raw redistribution cannot be permitted when raw retention is denied")]
    RedistributionExceedsRetention,
    /// The source-policy version already exists and cannot be overwritten.
    #[error("policy {source_id} v{version:?} is already registered")]
    DuplicatePolicyVersion {
        /// Source identifier.
        source_id: SourceId,
        /// Duplicate version.
        version: SourcePolicyVersion,
    },
    /// No policy has ever been registered for the source.
    #[error("source is not registered: {0}")]
    UnregisteredSource(SourceId),
    /// The exact policy version pinned by the observation is absent.
    #[error("source policy is not registered: {source_id} v{version:?}")]
    UnregisteredPolicyVersion {
        /// Source identifier.
        source_id: SourceId,
        /// Missing version.
        version: SourcePolicyVersion,
    },
    /// The observation predates policy activation.
    #[error("source policy is not yet valid: {source_id} v{version:?}")]
    PolicyNotYetValid {
        /// Source identifier.
        source_id: SourceId,
        /// Policy version.
        version: SourcePolicyVersion,
    },
    /// The observation occurred at or after policy expiration.
    #[error("source policy has expired: {source_id} v{version:?}")]
    PolicyExpired {
        /// Source identifier.
        source_id: SourceId,
        /// Policy version.
        version: SourcePolicyVersion,
    },
    /// The observation used a mechanism not approved by policy.
    #[error("acquisition method {method:?} is denied for {source_id}")]
    AcquisitionMethodDenied {
        /// Source identifier.
        source_id: SourceId,
        /// Denied acquisition method.
        method: AcquisitionMethod,
    },
    /// A raw object was retained under a metadata-only policy.
    #[error("raw payload retention is denied for {source_id}")]
    RawPayloadRetentionDenied {
        /// Source identifier.
        source_id: SourceId,
    },
    /// Optional metadata was retained without explicit policy permission.
    #[error("metadata field {field:?} is denied for {source_id}")]
    MetadataFieldDenied {
        /// Source identifier.
        source_id: SourceId,
        /// Denied optional metadata field.
        field: PermittedMetadataField,
    },
    /// The observation and supplied policy identities did not match.
    #[error("observation source or policy version does not match supplied policy")]
    PolicyIdentityMismatch,
    /// The source URI was blank.
    #[error("source URI cannot be empty")]
    EmptySourceUri,
    /// The media type was blank.
    #[error("media type cannot be empty")]
    EmptyMediaType,
    /// A present raw object reference was blank.
    #[error("raw object reference cannot be blank")]
    EmptyRawObjectReference,
    /// Serialized or otherwise supplied observation fields did not match its id.
    #[error("observation id mismatch: expected {expected}, found {found}")]
    ObservationIdMismatch {
        /// Recomputed identifier.
        expected: ObservationId,
        /// Supplied identifier.
        found: ObservationId,
    },
    /// A supersession referenced an observation not yet in the log.
    #[error("superseded observation is not present: {0}")]
    UnknownSupersededObservation(ObservationId),
    /// One source attempted to supersede a different source's record.
    #[error("an observation cannot supersede a record from another source")]
    CrossSourceSupersession,
    /// The new observation was recorded before the observation it supersedes.
    #[error("a supersession cannot move backward in observation time")]
    SupersessionMovesBackwardInTime,
    /// Neither publisher record id nor source URI connected the two versions.
    #[error("supersession must preserve publisher record id or source URI")]
    SupersessionIdentityMismatch,
    /// A second child attempted to fork an existing supersession chain.
    #[error("supersession fork for {superseded}; existing direct successor is {existing}")]
    SupersessionFork {
        /// Observation already superseded.
        superseded: ObservationId,
        /// Existing direct successor.
        existing: ObservationId,
    },
}

fn observation_id_for(draft: &ObservationDraft) -> ObservationId {
    let mut hasher = blake3::Hasher::new();
    hash_text(&mut hasher, draft.source_id.as_str());
    hasher.update(&draft.source_policy_version.get().to_le_bytes());
    hasher.update(&[acquisition_method_tag(draft.acquisition_method)]);
    hash_text(&mut hasher, &draft.source_uri);
    hash_optional_text(&mut hasher, draft.publisher_record_id.as_deref());
    hash_optional_time(&mut hasher, draft.event_time);
    hash_optional_time(&mut hasher, draft.publication_time);
    hasher.update(&draft.observation_time.unix_timestamp_nanos().to_le_bytes());
    for (name, value) in &draft.response_headers {
        hash_text(&mut hasher, name);
        hash_text(&mut hasher, value);
    }
    hash_optional_text(&mut hasher, draft.language.as_deref());
    hash_text(&mut hasher, &draft.media_type);
    hasher.update(draft.payload_hash.as_slice());
    hash_optional_text(&mut hasher, draft.raw_object_ref.as_deref());
    hash_optional_text(&mut hasher, draft.permitted_metadata.title.as_deref());
    hash_optional_text(&mut hasher, draft.permitted_metadata.byline.as_deref());
    hash_optional_text(&mut hasher, draft.permitted_metadata.excerpt.as_deref());
    for entity in &draft.permitted_metadata.entities {
        hash_text(&mut hasher, entity);
    }
    match draft.supersedes {
        Some(id) => {
            hasher.update(&[1]);
            hasher.update(&id.as_bytes());
        },
        None => {
            hasher.update(&[0]);
        },
    }
    hasher.update(&draft.retrieval_attempt_id.as_bytes());
    hasher.update(&draft.ingest_manifest_id.as_bytes());
    ObservationId::from_bytes(*hasher.finalize().as_bytes())
}

fn metadata_fields(
    has_publisher_record_id: bool,
    has_response_headers: bool,
    has_language: bool,
    metadata: &PermittedMetadata,
) -> BTreeSet<PermittedMetadataField> {
    let mut fields = BTreeSet::new();
    if has_publisher_record_id {
        fields.insert(PermittedMetadataField::PublisherRecordId);
    }
    if has_response_headers {
        fields.insert(PermittedMetadataField::ResponseHeaders);
    }
    if has_language {
        fields.insert(PermittedMetadataField::Language);
    }
    if metadata.title.is_some() {
        fields.insert(PermittedMetadataField::Title);
    }
    if metadata.byline.is_some() {
        fields.insert(PermittedMetadataField::Byline);
    }
    if metadata.excerpt.is_some() {
        fields.insert(PermittedMetadataField::Excerpt);
    }
    if !metadata.entities.is_empty() {
        fields.insert(PermittedMetadataField::Entities);
    }
    fields
}

const fn acquisition_method_tag(method: AcquisitionMethod) -> u8 {
    match method {
        AcquisitionMethod::Api => 1,
        AcquisitionMethod::Bulk => 2,
        AcquisitionMethod::Rss => 3,
        AcquisitionMethod::Sitemap => 4,
        AcquisitionMethod::Licensed => 5,
        AcquisitionMethod::PermittedWeb => 6,
    }
}

fn hash_text(hasher: &mut blake3::Hasher, value: &str) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn hash_optional_text(hasher: &mut blake3::Hasher, value: Option<&str>) {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            hash_text(hasher, value);
        },
        None => {
            hasher.update(&[0]);
        },
    }
}

fn hash_optional_time(hasher: &mut blake3::Hasher, value: Option<OffsetDateTime>) {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            hasher.update(&value.unix_timestamp_nanos().to_le_bytes());
        },
        None => {
            hasher.update(&[0]);
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_determinism::FrozenClock;
    use time::Duration;

    fn source_id() -> SourceId {
        SourceId::new("sec-edgar").unwrap()
    }

    fn version() -> SourcePolicyVersion {
        SourcePolicyVersion::new(1).unwrap()
    }

    fn policy(retention: PayloadRetention) -> SourcePolicy {
        SourcePolicy::new(
            source_id(),
            version(),
            [AcquisitionMethod::Api, AcquisitionMethod::Rss],
            retention,
            RedistributionPolicy::PermittedMetadataOnly,
            [
                PermittedMetadataField::PublisherRecordId,
                PermittedMetadataField::ResponseHeaders,
                PermittedMetadataField::Language,
                PermittedMetadataField::Title,
                PermittedMetadataField::Byline,
                PermittedMetadataField::Excerpt,
                PermittedMetadataField::Entities,
            ],
            "U.S. Securities and Exchange Commission",
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH + Duration::days(365),
            "legal/source-reviews/sec-edgar-v1",
        )
        .unwrap()
    }

    fn draft(at: OffsetDateTime, payload: &[u8]) -> ObservationDraft {
        let clock = FrozenClock::new(at);
        let mut draft = ObservationDraft::new(
            &clock,
            source_id(),
            version(),
            AcquisitionMethod::Api,
            "https://data.sec.gov/submissions/CIK0000320193.json",
            "application/json",
            ContentHash::from_bytes(payload),
            RetrievalAttemptId::from_bytes([7; 32]),
            IngestManifestId::from_bytes([9; 32]),
        )
        .unwrap();
        draft.publisher_record_id = Some("CIK0000320193".into());
        draft.publication_time = Some(at);
        draft.response_headers = BTreeMap::from([("etag".into(), "\"v1\"".into())]);
        draft.language = Some("en".into());
        draft
    }

    #[test]
    fn policy_versions_are_append_only() {
        let mut registry = SourcePolicyRegistry::default();
        registry
            .register(policy(PayloadRetention::MetadataOnly))
            .unwrap();
        assert!(matches!(
            registry.register(policy(PayloadRetention::RawPayloadPermitted)),
            Err(CorpusError::DuplicatePolicyVersion { .. })
        ));
    }

    #[test]
    fn policy_registry_round_trips_without_losing_exact_versions() {
        let mut registry = SourcePolicyRegistry::default();
        registry
            .register(policy(PayloadRetention::MetadataOnly))
            .unwrap();
        let encoded = serde_json::to_vec(&registry).unwrap();
        let decoded: SourcePolicyRegistry = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, registry);
        assert!(decoded.get(&source_id(), version()).is_some());
    }

    #[test]
    fn unregistered_and_expired_policies_are_hard_denials() {
        let policy = policy(PayloadRetention::MetadataOnly);
        let observation =
            Observation::seal(draft(OffsetDateTime::UNIX_EPOCH, b"v1"), &policy).unwrap();
        let mut log = ObservationLog::default();
        assert!(matches!(
            log.append(observation, &SourcePolicyRegistry::default()),
            Err(CorpusError::UnregisteredSource(_))
        ));

        let at_expiration = OffsetDateTime::UNIX_EPOCH + Duration::days(365);
        assert!(matches!(
            Observation::seal(draft(at_expiration, b"v2"), &policy),
            Err(CorpusError::PolicyExpired { .. })
        ));
    }

    #[test]
    fn metadata_only_policy_rejects_raw_object_retention() {
        let policy = policy(PayloadRetention::MetadataOnly);
        let mut draft = draft(OffsetDateTime::UNIX_EPOCH, b"copyrighted body");
        draft.raw_object_ref = Some("objects/blake3/example".into());
        assert!(matches!(
            Observation::seal(draft, &policy),
            Err(CorpusError::RawPayloadRetentionDenied { .. })
        ));
    }

    #[test]
    fn policy_rejects_unapproved_optional_metadata() {
        let policy = SourcePolicy::new(
            source_id(),
            version(),
            [AcquisitionMethod::Api],
            PayloadRetention::MetadataOnly,
            RedistributionPolicy::Prohibited,
            [],
            "U.S. Securities and Exchange Commission",
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH + Duration::days(365),
            "legal/source-reviews/sec-edgar-v1",
        )
        .unwrap();
        assert!(matches!(
            Observation::seal(draft(OffsetDateTime::UNIX_EPOCH, b"v1"), &policy),
            Err(CorpusError::MetadataFieldDenied { .. })
        ));
    }

    #[test]
    fn deterministic_identity_makes_retries_idempotent() {
        let policy = policy(PayloadRetention::RawPayloadPermitted);
        let observation =
            Observation::seal(draft(OffsetDateTime::UNIX_EPOCH, b"v1"), &policy).unwrap();
        let same = Observation::seal(draft(OffsetDateTime::UNIX_EPOCH, b"v1"), &policy).unwrap();
        assert_eq!(observation.id(), same.id());

        let mut registry = SourcePolicyRegistry::default();
        registry.register(policy).unwrap();
        let mut log = ObservationLog::default();
        assert_eq!(
            log.append(observation, &registry).unwrap(),
            AppendOutcome::Inserted { position: 0 }
        );
        assert_eq!(
            log.append(same, &registry).unwrap(),
            AppendOutcome::AlreadyPresent { position: 0 }
        );
    }

    #[test]
    fn supersession_preserves_history_and_rejects_forks() {
        let policy = policy(PayloadRetention::MetadataOnly);
        let first = Observation::seal(draft(OffsetDateTime::UNIX_EPOCH, b"v1"), &policy).unwrap();
        let mut second_draft = draft(OffsetDateTime::UNIX_EPOCH + Duration::minutes(5), b"v2");
        second_draft.supersedes = Some(first.id());
        let second = Observation::seal(second_draft, &policy).unwrap();
        let mut fork_draft = draft(
            OffsetDateTime::UNIX_EPOCH + Duration::minutes(6),
            b"v2-fork",
        );
        fork_draft.supersedes = Some(first.id());
        let fork = Observation::seal(fork_draft, &policy).unwrap();

        let mut registry = SourcePolicyRegistry::default();
        registry.register(policy).unwrap();
        let mut log = ObservationLog::default();
        log.append(first.clone(), &registry).unwrap();
        log.append(second.clone(), &registry).unwrap();

        assert_eq!(log.len(), 2);
        assert_eq!(log.get(first.id()), Some(&first));
        assert_eq!(log.superseded_by(first.id()), Some(second.id()));
        assert!(matches!(
            log.append(fork, &registry),
            Err(CorpusError::SupersessionFork { .. })
        ));
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn source_clock_skew_is_preserved_as_anomaly() {
        let policy = policy(PayloadRetention::MetadataOnly);
        let mut draft = draft(OffsetDateTime::UNIX_EPOCH, b"v1");
        draft.publication_time = Some(OffsetDateTime::UNIX_EPOCH + Duration::minutes(3));
        let observation = Observation::seal(draft, &policy).unwrap();
        assert_eq!(
            observation.temporal_anomalies(),
            BTreeSet::from([TemporalAnomaly::PublicationAfterObservation])
        );
    }
}
