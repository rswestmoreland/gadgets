//! Append-only audit ledger.
//!
//! The ledger writes structured JSONL audit events and verifies a simple hash
//! chain. This crate does not execute tools, call providers, or authorize
//! actions. It only persists and verifies audit records.

use gadgets_core::{AuditActor, AuditEvent, AuditTarget};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const AUDIT_SCHEMA_VERSION: &str = "gadgets.framework/audit/v0.1";
pub const DEFAULT_LEDGER_RELATIVE_PATH: &str = ".gadgets/ledger/events.jsonl";

#[derive(Debug)]
pub enum LedgerError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidLedger(Vec<String>),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "ledger I/O error: {err}"),
            Self::Json(err) => write!(f, "ledger JSON error: {err}"),
            Self::InvalidLedger(errors) => {
                write!(f, "ledger verification failed before append: {}", errors.join("; "))
            }
        }
    }
}

impl Error for LedgerError {}

impl From<std::io::Error> for LedgerError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for LedgerError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerVerification {
    pub ledger_path: PathBuf,
    pub events_checked: usize,
    pub valid: bool,
    pub errors: Vec<String>,
}

impl LedgerVerification {
    pub fn ok(ledger_path: PathBuf, events_checked: usize) -> Self {
        Self {
            ledger_path,
            events_checked,
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn invalid(ledger_path: PathBuf, events_checked: usize, errors: Vec<String>) -> Self {
        Self {
            ledger_path,
            events_checked,
            valid: false,
            errors,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerSummaryRow {
    pub event_id: String,
    pub timestamp: String,
    pub event_type: String,
    pub actor: String,
    pub decision: String,
    pub summary: String,
}

pub fn default_ledger_path(project_root: &Path) -> PathBuf {
    project_root.join(DEFAULT_LEDGER_RELATIVE_PATH)
}

pub fn new_audit_event(
    event_id: impl Into<String>,
    timestamp: impl Into<String>,
    event_type: impl Into<String>,
    actor_kind: impl Into<String>,
    actor_id: impl Into<String>,
    run_id: impl Into<String>,
    decision: impl Into<String>,
    summary: impl Into<String>,
) -> AuditEvent {
    AuditEvent {
        event_id: event_id.into(),
        schema_version: AUDIT_SCHEMA_VERSION.to_string(),
        timestamp: timestamp.into(),
        event_type: event_type.into(),
        actor: AuditActor {
            kind: actor_kind.into(),
            id: actor_id.into(),
        },
        target: None,
        run_id: run_id.into(),
        decision: decision.into(),
        summary: summary.into(),
        previous_event_hash: None,
        event_hash: String::new(),
    }
}

pub fn with_target(
    mut event: AuditEvent,
    kind: impl Into<String>,
    id: impl Into<String>,
) -> AuditEvent {
    event.target = Some(AuditTarget {
        kind: kind.into(),
        id: id.into(),
    });
    event
}

pub fn append_event(path: &Path, mut event: AuditEvent) -> Result<AuditEvent, LedgerError> {
    ensure_parent_dir(path)?;
    let verification = verify_ledger(path)?;
    if !verification.valid {
        return Err(LedgerError::InvalidLedger(verification.errors));
    }
    let previous_hash = read_last_event_hash(path)?;
    event.previous_event_hash = previous_hash;
    event.event_hash = compute_event_hash(&event)?;

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, &event)?;
    file.write_all(b"\n")?;
    Ok(event)
}

pub fn read_events(path: &Path) -> Result<Vec<AuditEvent>, LedgerError> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        events.push(serde_json::from_str(&line)?);
    }

    Ok(events)
}

pub fn summarize_events(path: &Path) -> Result<Vec<LedgerSummaryRow>, LedgerError> {
    let events = read_events(path)?;
    Ok(events
        .into_iter()
        .map(|event| LedgerSummaryRow {
            event_id: event.event_id,
            timestamp: event.timestamp,
            event_type: event.event_type,
            actor: format!("{}:{}", event.actor.kind, event.actor.id),
            decision: event.decision,
            summary: event.summary,
        })
        .collect())
}

pub fn verify_ledger(path: &Path) -> Result<LedgerVerification, LedgerError> {
    if !path.exists() {
        return Ok(LedgerVerification::ok(path.to_path_buf(), 0));
    }

    let events = read_events(path)?;
    let mut errors = Vec::new();
    let mut expected_previous_hash: Option<String> = None;

    for (idx, event) in events.iter().enumerate() {
        if event.event_hash.is_empty() {
            errors.push(format!("event {idx} has empty event_hash"));
            continue;
        }

        if event.previous_event_hash != expected_previous_hash {
            errors.push(format!(
                "event {idx} previous hash mismatch: expected {:?}, found {:?}",
                expected_previous_hash, event.previous_event_hash
            ));
        }

        let recomputed = compute_event_hash(event)?;
        if event.event_hash != recomputed {
            errors.push(format!(
                "event {idx} hash mismatch for {}: expected {}, recomputed {}",
                event.event_id, event.event_hash, recomputed
            ));
        }

        expected_previous_hash = Some(event.event_hash.clone());
    }

    if errors.is_empty() {
        Ok(LedgerVerification::ok(path.to_path_buf(), events.len()))
    } else {
        Ok(LedgerVerification::invalid(path.to_path_buf(), events.len(), errors))
    }
}

pub fn compute_event_hash(event: &AuditEvent) -> Result<String, LedgerError> {
    let mut canonical = event.clone();
    canonical.event_hash.clear();
    let bytes = serde_json::to_vec(&canonical)?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("sha256:{}", to_hex(&digest)))
}

fn read_last_event_hash(path: &Path) -> Result<Option<String>, LedgerError> {
    let events = read_events(path)?;
    Ok(events.last().map(|event| event.event_hash.clone()))
}

fn ensure_parent_dir(path: &Path) -> Result<(), LedgerError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_ledger_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "gadgets-ledger-{name}-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn missing_ledger_verifies_as_empty() {
        let path = temp_ledger_path("missing");
        let result = verify_ledger(&path).unwrap();
        assert!(result.valid);
        assert_eq!(result.events_checked, 0);
    }

    #[test]
    fn appends_and_verifies_events() {
        let path = temp_ledger_path("append");
        let first = new_audit_event(
            "aud_1",
            "2026-05-12T00:00:00Z",
            "run.started",
            "gadget",
            "coordinator",
            "run_1",
            "allowed",
            "Run started.",
        );
        let second = new_audit_event(
            "aud_2",
            "2026-05-12T00:00:01Z",
            "run.completed",
            "gadget",
            "coordinator",
            "run_1",
            "allowed",
            "Run completed.",
        );

        let first = append_event(&path, first).unwrap();
        let second = append_event(&path, second).unwrap();

        assert!(first.previous_event_hash.is_none());
        assert_eq!(second.previous_event_hash, Some(first.event_hash));

        let result = verify_ledger(&path).unwrap();
        assert!(result.valid);
        assert_eq!(result.events_checked, 2);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_to_append_to_invalid_ledger() {
        let path = temp_ledger_path("refuse-append");
        let event = new_audit_event(
            "aud_1",
            "2026-05-12T00:00:00Z",
            "run.started",
            "gadget",
            "coordinator",
            "run_1",
            "allowed",
            "Run started.",
        );
        append_event(&path, event).unwrap();

        let mut text = fs::read_to_string(&path).unwrap();
        text = text.replace("Run started.", "Run secretly changed.");
        fs::write(&path, text).unwrap();

        let next = new_audit_event(
            "aud_2",
            "2026-05-12T00:00:01Z",
            "run.completed",
            "gadget",
            "coordinator",
            "run_1",
            "allowed",
            "Run completed.",
        );
        let err = append_event(&path, next).unwrap_err();
        assert!(err.to_string().contains("verification failed"));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn detects_tampered_event() {
        let path = temp_ledger_path("tamper");
        let event = new_audit_event(
            "aud_1",
            "2026-05-12T00:00:00Z",
            "run.started",
            "gadget",
            "coordinator",
            "run_1",
            "allowed",
            "Run started.",
        );
        append_event(&path, event).unwrap();

        let mut text = fs::read_to_string(&path).unwrap();
        text = text.replace("Run started.", "Run secretly changed.");
        fs::write(&path, text).unwrap();

        let result = verify_ledger(&path).unwrap();
        assert!(!result.valid);
        assert_eq!(result.events_checked, 1);
        assert!(result.errors.iter().any(|err| err.contains("hash mismatch")));

        fs::remove_file(path).unwrap();
    }
}
