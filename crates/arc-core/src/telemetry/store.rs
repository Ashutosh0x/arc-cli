//! `redb`-backed persistent telemetry store.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use redb::{Database, ReadableTable, TableDefinition};

use super::types::{ProviderStats, RequestRecord, TelemetrySummary};
use crate::error::ArcError;

/// Table: auto-incrementing u64 key → CBOR-encoded [`RequestRecord`].
const REQUESTS: TableDefinition<u64, &[u8]> = TableDefinition::new("requests_v1");

/// Table: `"next_id"` → u64 counter.
const META: TableDefinition<&str, u64> = TableDefinition::new("meta_v1");

/// Durable telemetry store backed by `redb`.
pub struct TelemetryStore {
    db: Database,
    path: PathBuf,
}

impl TelemetryStore {
    /// Open (or create) the telemetry database at the given directory.
    ///
    /// The actual file will be `<dir>/telemetry.redb`.
    pub fn open(dir: &Path) -> Result<Self, ArcError> {
        std::fs::create_dir_all(dir).map_err(|e| {
            ArcError::System(format!("cannot create telemetry dir {}: {e}", dir.display()))
        })?;

        let path = dir.join("telemetry.redb");
        let db = Database::create(&path).map_err(|e| {
            ArcError::Database(format!(
                "cannot open telemetry db {}: {e}",
                path.display()
            ))
        })?;

        // Ensure tables exist.
        let txn = db.begin_write().map_err(|e| ArcError::Database(e.to_string()))?;
        {
            let _ = txn.open_table(REQUESTS);
            let _ = txn.open_table(META);
        }
        txn.commit().map_err(|e| ArcError::Database(e.to_string()))?;

        Ok(Self { db, path })
    }

    /// Record a completed LLM request.
    pub fn record(&self, record: &RequestRecord) -> Result<(), ArcError> {
        let encoded =
            serde_json::to_vec(record).map_err(|e| ArcError::System(e.to_string()))?;

        let txn = self
            .db
            .begin_write()
            .map_err(|e| ArcError::Database(e.to_string()))?;

        {
            let mut meta = txn
                .open_table(META)
                .map_err(|e| ArcError::Database(e.to_string()))?;
            let mut requests = txn
                .open_table(REQUESTS)
                .map_err(|e| ArcError::Database(e.to_string()))?;

            let next_id = meta
                .get("next_id")
                .map_err(|e| ArcError::Database(e.to_string()))?
                .map(|v| v.value())
                .unwrap_or(0);

            requests
                .insert(next_id, encoded.as_slice())
                .map_err(|e| ArcError::Database(e.to_string()))?;

            meta.insert("next_id", next_id + 1)
                .map_err(|e| ArcError::Database(e.to_string()))?;
        }

        txn.commit()
            .map_err(|e| ArcError::Database(e.to_string()))?;

        Ok(())
    }

    /// Aggregate all records into a [`TelemetrySummary`].
    pub fn aggregate_all(&self) -> Result<TelemetrySummary, ArcError> {
        self.aggregate_since(0)
    }

    /// Aggregate records since `since_ms` (unix millis).
    pub fn aggregate_since(&self, since_ms: u64) -> Result<TelemetrySummary, ArcError> {
        let txn = self
            .db
            .begin_read()
            .map_err(|e| ArcError::Database(e.to_string()))?;

        let table = txn
            .open_table(REQUESTS)
            .map_err(|e| ArcError::Database(e.to_string()))?;

        let mut summary = TelemetrySummary {
            providers: std::collections::HashMap::new(),
            total_requests: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_usd: 0.0,
            first_record_ms: None,
            last_record_ms: None,
        };

        for entry in table.iter().map_err(|e| ArcError::Database(e.to_string()))? {
            let (_key, value) = entry.map_err(|e| ArcError::Database(e.to_string()))?;
            let record: RequestRecord = serde_json::from_slice(value.value())
                .map_err(|e| ArcError::System(format!("corrupt telemetry record: {e}")))?;

            if record.timestamp_ms < since_ms {
                continue;
            }

            // Update global totals.
            summary.total_requests += 1;
            summary.total_input_tokens += record.input_tokens as u64;
            summary.total_output_tokens += record.output_tokens as u64;
            summary.total_cost_usd += record.cost_usd;

            // Track time range.
            summary.first_record_ms = Some(
                summary
                    .first_record_ms
                    .map_or(record.timestamp_ms, |prev| prev.min(record.timestamp_ms)),
            );
            summary.last_record_ms = Some(
                summary
                    .last_record_ms
                    .map_or(record.timestamp_ms, |prev| prev.max(record.timestamp_ms)),
            );

            // Per-provider stats.
            let stats = summary
                .providers
                .entry(record.provider.clone())
                .or_insert_with(|| ProviderStats {
                    provider: record.provider.clone(),
                    ..Default::default()
                });

            stats.total_requests += 1;
            stats.total_input_tokens += record.input_tokens as u64;
            stats.total_output_tokens += record.output_tokens as u64;
            stats.total_cost_usd += record.cost_usd;
            stats.latencies_ms.push(record.latency_ms);

            if record.success {
                stats.successful_requests += 1;
            } else {
                stats.failed_requests += 1;
            }
        }

        Ok(summary)
    }

    /// Records in the last N days.
    pub fn aggregate_last_days(&self, days: u64) -> Result<TelemetrySummary, ArcError> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let since_ms = now_ms.saturating_sub(days * 24 * 60 * 60 * 1000);
        self.aggregate_since(since_ms)
    }

    /// Total number of persisted records.
    pub fn record_count(&self) -> Result<u64, ArcError> {
        let txn = self
            .db
            .begin_read()
            .map_err(|e| ArcError::Database(e.to_string()))?;
        let table = txn
            .open_table(META)
            .map_err(|e| ArcError::Database(e.to_string()))?;
        Ok(table
            .get("next_id")
            .map_err(|e| ArcError::Database(e.to_string()))?
            .map(|v| v.value())
            .unwrap_or(0))
    }

    /// Path to the database file on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
