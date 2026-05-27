use deepseek_mobile_core::{
    PcGatewayDiscoveryReport, PcGatewayDiscoveryService, PcGatewayMdnsRecord,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidPcGatewayDiscoveryCommand {
    pub request_id: String,
    pub service_type: String,
    pub timeout_ms: u64,
}

impl AndroidPcGatewayDiscoveryCommand {
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            service_type: "_deepseek-pc-gateway._tcp.".to_string(),
            timeout_ms: 5_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidPcGatewayMdnsRecordPayload {
    pub instance_name: String,
    pub host: String,
    pub port: u16,
    pub txt: BTreeMap<String, String>,
}

impl AndroidPcGatewayMdnsRecordPayload {
    pub fn new(instance_name: impl Into<String>, host: impl Into<String>, port: u16) -> Self {
        Self {
            instance_name: instance_name.into(),
            host: host.into(),
            port,
            txt: BTreeMap::new(),
        }
    }

    pub fn into_core_record(self) -> PcGatewayMdnsRecord {
        let mut record = PcGatewayMdnsRecord::new(self.instance_name, self.host, self.port);
        for (key, value) in self.txt {
            record = record.with_txt(key, value);
        }
        record
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidPcGatewayDiscoveryCallback {
    Started {
        request_id: String,
        service_type: String,
    },
    Candidate {
        request_id: String,
        record: AndroidPcGatewayMdnsRecordPayload,
    },
    Completed {
        request_id: String,
        records: Vec<AndroidPcGatewayMdnsRecordPayload>,
    },
    Failed {
        request_id: String,
        message: String,
    },
}

impl AndroidPcGatewayDiscoveryCallback {
    pub fn request_id(&self) -> &str {
        match self {
            Self::Started { request_id, .. }
            | Self::Candidate { request_id, .. }
            | Self::Completed { request_id, .. }
            | Self::Failed { request_id, .. } => request_id,
        }
    }

    pub fn into_discovery_report(self) -> Option<PcGatewayDiscoveryReport> {
        match self {
            Self::Candidate { record, .. } => Some(
                PcGatewayDiscoveryService::default()
                    .from_mdns_records(vec![record.into_core_record()]),
            ),
            Self::Completed { records, .. } => Some(
                PcGatewayDiscoveryService::default().from_mdns_records(
                    records
                        .into_iter()
                        .map(AndroidPcGatewayMdnsRecordPayload::into_core_record)
                        .collect(),
                ),
            ),
            Self::Started { .. } | Self::Failed { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AndroidPcGatewayDiscoveryCallback, AndroidPcGatewayMdnsRecordPayload};

    #[test]
    fn candidate_callback_converts_to_core_report() {
        let callback = AndroidPcGatewayDiscoveryCallback::Candidate {
            request_id: "scan-1".to_string(),
            record: AndroidPcGatewayMdnsRecordPayload::new("Laptop", "192.168.1.10", 8787),
        };
        let report = callback.into_discovery_report().expect("report");
        assert_eq!(report.candidates.len(), 1);
        assert!(report.candidates[0]
            .endpoint
            .base_url
            .contains("192.168.1.10"));
    }
}
