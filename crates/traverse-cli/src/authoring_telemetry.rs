//! Host-owned, aggregate-only authoring outcome telemetry (Specs 122 and 123).
//!
//! This module deliberately has no manifest, repository, or network inputs.
//! An integrating host supplies an opaque contributor ticket and normalized
//! milestone; the persisted bucket contains only aggregate counters and a
//! one-way ticket hash set.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub const MIN_DISTINCT_CONTRIBUTORS: usize = 20;
const RETENTION_DAYS: i64 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthoringMilestone {
    AuthoringStarted,
    ReviewFinding,
    Revision,
    TerminalOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthoringRoute {
    Manual,
    SkillAssisted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalOutcome {
    ReviewAccepted,
    MaterialRework,
    AbandonedDeferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionCountBucket {
    Zero,
    One,
    TwoToThree,
    FourPlus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElapsedTimeBucket {
    UnderHour,
    OneToFourHours,
    FourToTwentyFourHours,
    OverDay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    ContractDrift,
    AbiPackaging,
    TestCoverage,
    SecurityPolicy,
    Documentation,
}

/// Closed envelope supplied by a trusted integrating host. Free-form text and
/// identifiers are intentionally absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoringOutcomeEvent {
    pub milestone: AuthoringMilestone,
    pub authoring_route: Option<AuthoringRoute>,
    pub terminal_outcome: Option<TerminalOutcome>,
    pub revision_count_bucket: Option<RevisionCountBucket>,
    pub elapsed_time_bucket: Option<ElapsedTimeBucket>,
    pub finding_categories: BTreeSet<FindingCategory>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoringAggregateBucket {
    pub opened_at: DateTime<Utc>,
    ticket_hashes: BTreeSet<String>,
    authoring_route_counts: BTreeMap<AuthoringRoute, u64>,
    terminal_outcome_counts: BTreeMap<TerminalOutcome, u64>,
    revision_count_bucket_counts: BTreeMap<RevisionCountBucket, u64>,
    elapsed_time_bucket_counts: BTreeMap<ElapsedTimeBucket, u64>,
    finding_category_counts: BTreeMap<FindingCategory, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteAuthoringAggregate {
    pub opened_at: DateTime<Utc>,
    pub closes_at: DateTime<Utc>,
    pub distinct_contributor_count: usize,
    pub authoring_route_counts: BTreeMap<AuthoringRoute, u64>,
    pub terminal_outcome_counts: BTreeMap<TerminalOutcome, u64>,
    pub revision_count_bucket_counts: BTreeMap<RevisionCountBucket, u64>,
    pub elapsed_time_bucket_counts: BTreeMap<ElapsedTimeBucket, u64>,
    pub finding_category_counts: BTreeMap<FindingCategory, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyticsAccessAuditRecord {
    pub role: String,
    pub purpose: String,
    pub timestamp: DateTime<Utc>,
    pub bucket_opened_at: DateTime<Utc>,
    pub allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoringTelemetryError {
    EmptyTicket,
    UnpublishedBucket,
    Io(String),
}

impl AuthoringAggregateBucket {
    #[must_use]
    pub fn new(opened_at: DateTime<Utc>) -> Self {
        Self {
            opened_at,
            ticket_hashes: BTreeSet::new(),
            authoring_route_counts: BTreeMap::new(),
            terminal_outcome_counts: BTreeMap::new(),
            revision_count_bucket_counts: BTreeMap::new(),
            elapsed_time_bucket_counts: BTreeMap::new(),
            finding_category_counts: BTreeMap::new(),
        }
    }

    /// Records only aggregate categories. The opaque ticket is hashed before
    /// it reaches bucket state and never appears in an export.
    pub fn record(
        &mut self,
        opaque_ticket: &str,
        event: &AuthoringOutcomeEvent,
    ) -> Result<(), AuthoringTelemetryError> {
        if opaque_ticket.trim().is_empty() {
            return Err(AuthoringTelemetryError::EmptyTicket);
        }
        self.ticket_hashes.insert(ticket_hash(opaque_ticket));
        increment_option(&mut self.authoring_route_counts, event.authoring_route);
        increment_option(&mut self.terminal_outcome_counts, event.terminal_outcome);
        increment_option(
            &mut self.revision_count_bucket_counts,
            event.revision_count_bucket,
        );
        increment_option(
            &mut self.elapsed_time_bucket_counts,
            event.elapsed_time_bucket,
        );
        for category in &event.finding_categories {
            increment(&mut self.finding_category_counts, *category);
        }
        Ok(())
    }

    #[must_use]
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.opened_at + Duration::days(RETENTION_DAYS)
    }

    #[must_use]
    pub fn eligible_for_export(&self, now: DateTime<Utc>) -> bool {
        !self.is_expired(now) && self.ticket_hashes.len() >= MIN_DISTINCT_CONTRIBUTORS
    }

    pub fn remote_aggregate(
        &self,
        now: DateTime<Utc>,
    ) -> Result<RemoteAuthoringAggregate, AuthoringTelemetryError> {
        if !self.eligible_for_export(now) {
            return Err(AuthoringTelemetryError::UnpublishedBucket);
        }
        Ok(RemoteAuthoringAggregate {
            opened_at: self.opened_at,
            closes_at: now,
            distinct_contributor_count: self.ticket_hashes.len(),
            authoring_route_counts: self.authoring_route_counts.clone(),
            terminal_outcome_counts: self.terminal_outcome_counts.clone(),
            revision_count_bucket_counts: self.revision_count_bucket_counts.clone(),
            elapsed_time_bucket_counts: self.elapsed_time_bucket_counts.clone(),
            finding_category_counts: self.finding_category_counts.clone(),
        })
    }
}

/// Host opt-out deletes the complete unpublished bucket, preserving the
/// aggregate-only storage guarantee.
pub fn delete_unpublished_bucket_on_opt_out(
    bucket: &mut Option<AuthoringAggregateBucket>,
    now: DateTime<Utc>,
) -> Result<(), AuthoringTelemetryError> {
    let Some(current) = bucket.as_ref() else {
        return Ok(());
    };
    if current.eligible_for_export(now) {
        return Err(AuthoringTelemetryError::UnpublishedBucket);
    }
    *bucket = None;
    Ok(())
}

pub fn append_analytics_access_audit(
    path: &Path,
    record: &AnalyticsAccessAuditRecord,
) -> Result<(), AuthoringTelemetryError> {
    let line = serde_json::to_string(record)
        .map_err(|error| AuthoringTelemetryError::Io(error.to_string()))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| AuthoringTelemetryError::Io(error.to_string()))?;
    writeln!(file, "{line}").map_err(|error| AuthoringTelemetryError::Io(error.to_string()))
}

fn ticket_hash(ticket: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(ticket.as_bytes());
    let mut result = String::with_capacity(digest.len() * 2);
    for byte in digest {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    result
}
fn increment<K: Ord>(counts: &mut BTreeMap<K, u64>, key: K) {
    *counts.entry(key).or_default() += 1;
}
fn increment_option<K: Ord>(counts: &mut BTreeMap<K, u64>, key: Option<K>) {
    if let Some(key) = key {
        increment(counts, key);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use std::fs;

    fn event() -> AuthoringOutcomeEvent {
        AuthoringOutcomeEvent {
            milestone: AuthoringMilestone::TerminalOutcome,
            authoring_route: Some(AuthoringRoute::SkillAssisted),
            terminal_outcome: Some(TerminalOutcome::ReviewAccepted),
            revision_count_bucket: Some(RevisionCountBucket::One),
            elapsed_time_bucket: Some(ElapsedTimeBucket::OneToFourHours),
            finding_categories: BTreeSet::from([FindingCategory::ContractDrift]),
        }
    }

    #[test]
    fn only_opaque_ticket_hashes_and_aggregate_counts_are_retained() {
        let now = Utc::now();
        let mut bucket = AuthoringAggregateBucket::new(now);
        bucket.record("opaque-ticket", &event()).expect("record");
        let rendered = serde_json::to_string(&bucket).expect("serialize");
        assert!(!rendered.contains("opaque-ticket"));
        assert!(rendered.contains("contract_drift"));
    }

    #[test]
    fn export_fails_closed_below_twenty_and_excludes_ticket_hashes() {
        let now = Utc::now();
        let mut bucket = AuthoringAggregateBucket::new(now);
        for index in 0..(MIN_DISTINCT_CONTRIBUTORS - 1) {
            bucket
                .record(&format!("ticket-{index}"), &event())
                .expect("record");
        }
        assert_eq!(
            bucket.remote_aggregate(now),
            Err(AuthoringTelemetryError::UnpublishedBucket)
        );
        bucket.record("ticket-19", &event()).expect("record");
        let aggregate = bucket.remote_aggregate(now).expect("twenty tickets export");
        assert_eq!(
            aggregate.distinct_contributor_count,
            MIN_DISTINCT_CONTRIBUTORS
        );
        assert!(
            !serde_json::to_string(&aggregate)
                .expect("serialize")
                .contains("ticket")
        );
    }

    #[test]
    fn expiry_and_opt_out_prevent_unpublished_aggregate_retention() {
        let now = Utc::now();
        let bucket = AuthoringAggregateBucket::new(now);
        let mut retained = Some(bucket.clone());
        assert!(delete_unpublished_bucket_on_opt_out(&mut retained, now).is_ok());
        assert_eq!(retained, None);
        assert!(bucket.is_expired(now + Duration::days(RETENTION_DAYS)));
    }

    #[test]
    fn access_audit_is_separate_json_lines_evidence() {
        let path = std::env::temp_dir().join(format!(
            "traverse-audit-{}.jsonl",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let record = AnalyticsAccessAuditRecord {
            role: "traverse-analytics".to_string(),
            purpose: "quarterly-review".to_string(),
            timestamp: Utc::now(),
            bucket_opened_at: Utc::now(),
            allowed: true,
        };
        append_analytics_access_audit(&path, &record).expect("audit append");
        let line = fs::read_to_string(&path).expect("audit read");
        assert!(line.contains("traverse-analytics"));
        let _ = fs::remove_file(path);
    }
}
