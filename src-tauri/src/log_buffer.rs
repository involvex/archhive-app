use crate::models::LogEntry;
use parking_lot::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_ENTRIES: usize = 1000;

pub struct LogBuffer {
    entries: Mutex<Vec<LogEntry>>,
}

static INSTANCE: std::sync::OnceLock<LogBuffer> = std::sync::OnceLock::new();

impl LogBuffer {
    pub fn instance() -> &'static LogBuffer {
        INSTANCE.get_or_init(|| Self {
            entries: Mutex::new(Vec::with_capacity(MAX_LOG_ENTRIES)),
        })
    }

    pub fn push(&self, level: &str, target: &str, message: String) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let entry = LogEntry {
            timestamp,
            level: level.to_string(),
            message,
            target: target.to_string(),
        };
        let mut entries = self.entries.lock();
        let len = entries.len();
        if len >= MAX_LOG_ENTRIES {
            entries.drain(0..len - MAX_LOG_ENTRIES + 1);
        }
        entries.push(entry);
    }

    pub fn get_recent(&self, limit: usize) -> Vec<LogEntry> {
        let entries = self.entries.lock();
        let start = entries.len().saturating_sub(limit);
        entries[start..].to_vec()
    }

    pub fn get_all(&self) -> String {
        let entries = self.entries.lock();
        entries
            .iter()
            .map(|e| format!("[{}] {} {} {}\n", e.level, e.timestamp, e.target, e.message))
            .collect()
    }

    pub fn clear(&self) {
        self.entries.lock().clear();
    }
}
