//! PC gateway discovery primitives.
//!
//! Android/iOS/desktop platform code can provide mDNS/NSD service records to
//! this core module. The core then converts them into validated
//! `PcGatewayEndpointCandidate` values and can probe candidates through the
//! gateway `/health` endpoint. This keeps platform discovery separate from the
//! transport policy and route scoring logic.

use crate::pc_gateway::{
    validate_gateway_base_url_for_transport, PcGatewayEndpointCandidate, PcGatewayHealth,
    PcGatewayTransportMode,
};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;

pub const PC_GATEWAY_MDNS_SERVICE: &str = "_deepseek-pc-gateway._tcp.local.";
pub const DEFAULT_PC_GATEWAY_PORT: u16 = 8787;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcGatewayDiscoverySource {
    Mdns,
    Manual,
    SubnetScan,
    SavedCandidate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcGatewayDiscoveryStatus {
    Found,
    Rejected,
    ProbeFailed,
    Online,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayMdnsRecord {
    pub instance_name: String,
    pub host: String,
    pub port: u16,
    pub txt: BTreeMap<String, String>,
}

impl PcGatewayMdnsRecord {
    pub fn new(instance_name: impl Into<String>, host: impl Into<String>, port: u16) -> Self {
        Self {
            instance_name: instance_name.into(),
            host: host.into(),
            port,
            txt: BTreeMap::new(),
        }
    }

    pub fn with_txt(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.txt.insert(key.into(), value.into());
        self
    }

    pub fn endpoint_candidate(&self) -> PcGatewayEndpointCandidate {
        let https = self
            .txt
            .get("tls")
            .map(|value| value == "true" || value == "1")
            .unwrap_or(false);
        let mode = if https {
            PcGatewayTransportMode::LocalNetworkHttps
        } else {
            PcGatewayTransportMode::LocalNetworkHttp
        };
        let scheme = if https { "https" } else { "http" };
        PcGatewayEndpointCandidate::new(
            format!("mdns:{}", self.instance_name),
            format!("{}://{}:{}", scheme, normalize_host_for_url(&self.host), self.port),
            mode,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayDiscoveryCandidate {
    pub source: PcGatewayDiscoverySource,
    pub endpoint: PcGatewayEndpointCandidate,
    pub status: PcGatewayDiscoveryStatus,
    pub message: Option<String>,
    pub latency_ms: Option<u128>,
    pub health: Option<PcGatewayHealth>,
}

impl PcGatewayDiscoveryCandidate {
    pub fn found(source: PcGatewayDiscoverySource, endpoint: PcGatewayEndpointCandidate) -> Self {
        Self {
            source,
            endpoint,
            status: PcGatewayDiscoveryStatus::Found,
            message: None,
            latency_ms: None,
            health: None,
        }
    }

    pub fn rejected(
        source: PcGatewayDiscoverySource,
        endpoint: PcGatewayEndpointCandidate,
        message: impl Into<String>,
    ) -> Self {
        Self {
            source,
            endpoint,
            status: PcGatewayDiscoveryStatus::Rejected,
            message: Some(message.into()),
            latency_ms: None,
            health: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayDiscoveryReport {
    pub service_name: String,
    pub candidates: Vec<PcGatewayDiscoveryCandidate>,
}

impl Default for PcGatewayDiscoveryReport {
    fn default() -> Self {
        Self {
            service_name: PC_GATEWAY_MDNS_SERVICE.to_string(),
            candidates: Vec::new(),
        }
    }
}

impl PcGatewayDiscoveryReport {
    pub fn endpoint_candidates(&self) -> Vec<PcGatewayEndpointCandidate> {
        let mut endpoints = Vec::new();
        for candidate in self.candidates.iter() {
            if matches!(
                candidate.status,
                PcGatewayDiscoveryStatus::Found | PcGatewayDiscoveryStatus::Online
            ) && !endpoints
                .iter()
                .any(|endpoint: &PcGatewayEndpointCandidate| endpoint.base_url == candidate.endpoint.base_url)
            {
                endpoints.push(candidate.endpoint.clone());
            }
        }
        endpoints
    }

    pub fn online_candidates(&self) -> Vec<PcGatewayDiscoveryCandidate> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.status == PcGatewayDiscoveryStatus::Online)
            .cloned()
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct PcGatewayDiscoveryService {
    allow_http_on_local_network: bool,
    http: Client,
}

impl Default for PcGatewayDiscoveryService {
    fn default() -> Self {
        Self::new(true)
    }
}

impl PcGatewayDiscoveryService {
    pub fn new(allow_http_on_local_network: bool) -> Self {
        Self {
            allow_http_on_local_network,
            http: Client::new(),
        }
    }

    pub fn from_mdns_records(&self, records: Vec<PcGatewayMdnsRecord>) -> PcGatewayDiscoveryReport {
        let mut report = PcGatewayDiscoveryReport::default();
        for record in records {
            let endpoint = record.endpoint_candidate();
            report.candidates.push(self.validate_candidate(PcGatewayDiscoverySource::Mdns, endpoint));
        }
        report
    }

    pub fn from_manual_hosts(&self, hosts: Vec<String>, port: u16) -> PcGatewayDiscoveryReport {
        let mut report = PcGatewayDiscoveryReport::default();
        for host in hosts {
            let endpoint = PcGatewayEndpointCandidate::new(
                format!("manual:{}", host),
                format!("http://{}:{}", normalize_host_for_url(&host), port),
                PcGatewayTransportMode::LocalNetworkHttp,
            );
            report.candidates.push(self.validate_candidate(PcGatewayDiscoverySource::Manual, endpoint));
        }
        report
    }

    pub fn subnet_scan_candidates(&self, subnet_prefix: &str, port: u16, range: std::ops::RangeInclusive<u8>) -> PcGatewayDiscoveryReport {
        let mut report = PcGatewayDiscoveryReport::default();
        let prefix = subnet_prefix.trim().trim_end_matches('.');
        for octet in range {
            let host = format!("{}.{}", prefix, octet);
            let endpoint = PcGatewayEndpointCandidate::new(
                format!("subnet:{}", host),
                format!("http://{}:{}", host, port),
                PcGatewayTransportMode::LocalNetworkHttp,
            )
            .with_priority(PcGatewayTransportMode::LocalNetworkHttp.default_priority().saturating_sub(5));
            report.candidates.push(self.validate_candidate(PcGatewayDiscoverySource::SubnetScan, endpoint));
        }
        report
    }

    pub async fn probe_candidates(&self, mut report: PcGatewayDiscoveryReport) -> PcGatewayDiscoveryReport {
        for candidate in report.candidates.iter_mut() {
            if candidate.status == PcGatewayDiscoveryStatus::Rejected {
                continue;
            }
            let started = Instant::now();
            match self.probe_endpoint(&candidate.endpoint).await {
                Ok(health) => {
                    candidate.status = PcGatewayDiscoveryStatus::Online;
                    candidate.latency_ms = Some(started.elapsed().as_millis());
                    candidate.health = Some(health);
                    candidate.message = None;
                }
                Err(error) => {
                    candidate.status = PcGatewayDiscoveryStatus::ProbeFailed;
                    candidate.latency_ms = Some(started.elapsed().as_millis());
                    candidate.message = Some(error.to_string());
                }
            }
        }
        report
    }

    fn validate_candidate(
        &self,
        source: PcGatewayDiscoverySource,
        endpoint: PcGatewayEndpointCandidate,
    ) -> PcGatewayDiscoveryCandidate {
        match validate_gateway_base_url_for_transport(
            &endpoint.base_url,
            &endpoint.transport_mode,
            self.allow_http_on_local_network,
        ) {
            Ok(()) => PcGatewayDiscoveryCandidate::found(source, endpoint),
            Err(error) => PcGatewayDiscoveryCandidate::rejected(source, endpoint, error),
        }
    }

    async fn probe_endpoint(&self, endpoint: &PcGatewayEndpointCandidate) -> Result<PcGatewayHealth> {
        let url = format!("{}/health", endpoint.base_url.trim_end_matches('/'));
        let response = self.http.get(url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow!("health HTTP status {}", response.status()));
        }
        Ok(response.json::<PcGatewayHealth>().await?)
    }
}

fn normalize_host_for_url(host: &str) -> String {
    let host = host.trim().trim_end_matches('.');
    if host.contains(':') && !host.starts_with('[') {
        format!("[{}]", host)
    } else {
        host.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{PcGatewayDiscoveryService, PcGatewayDiscoveryStatus, PcGatewayMdnsRecord, DEFAULT_PC_GATEWAY_PORT};

    #[test]
    fn mdns_records_become_local_http_candidates() {
        let service = PcGatewayDiscoveryService::default();
        let report = service.from_mdns_records(vec![PcGatewayMdnsRecord::new(
            "Laptop",
            "192.168.1.10",
            DEFAULT_PC_GATEWAY_PORT,
        )]);
        assert_eq!(report.candidates.len(), 1);
        assert_eq!(report.candidates[0].status, PcGatewayDiscoveryStatus::Found);
        assert!(report.candidates[0].endpoint.base_url.contains("192.168.1.10"));
    }

    #[test]
    fn public_http_manual_hosts_are_rejected() {
        let service = PcGatewayDiscoveryService::default();
        let report = service.from_manual_hosts(vec!["example.com".to_string()], DEFAULT_PC_GATEWAY_PORT);
        assert_eq!(report.candidates[0].status, PcGatewayDiscoveryStatus::Rejected);
    }

    #[test]
    fn subnet_scan_generates_private_candidates() {
        let service = PcGatewayDiscoveryService::default();
        let report = service.subnet_scan_candidates("192.168.1", DEFAULT_PC_GATEWAY_PORT, 10..=12);
        assert_eq!(report.candidates.len(), 3);
        assert!(report.endpoint_candidates().len() == 3);
    }
}
