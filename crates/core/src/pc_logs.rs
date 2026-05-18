//! Request log types for PC-host observability.

use serde::{Deserialize, Serialize};

/// A single request log entry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayLogEntry {
    pub timestamp_unix: u64,
    pub request_id: String,
    pub operation: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub duration_ms: u64,
}

/// Collection of recent log entries.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayLogs {
    pub entries: Vec<PcGatewayLogEntry>,
    pub total_stored: usize,
}

/// Ring buffer for storing recent request logs.
pub struct LogRing {
    entries: Vec<PcGatewayLogEntry>,
    capacity: usize,
}

impl LogRing {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            capacity: capacity.max(1),
        }
    }

    pub fn push(&mut self, entry: PcGatewayLogEntry) {
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    pub fn snapshot(&self) -> PcGatewayLogs {
        PcGatewayLogs {
            entries: self.entries.clone(),
            total_stored: self.entries.len(),
        }
    }
}
