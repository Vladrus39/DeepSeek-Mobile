//! Advertise the PC gateway on the LAN via mDNS so Android NSD can discover it.

use anyhow::{Context, Result};
use deepseek_mobile_core::PC_GATEWAY_MDNS_SERVICE;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Keeps the mDNS daemon alive for the process lifetime.
pub struct MdnsAdvertisement {
    _daemon: ServiceDaemon,
}

pub fn register_pc_gateway(
    bind_addr: SocketAddr,
    gateway_id: &str,
    gateway_label: &str,
) -> Result<Option<MdnsAdvertisement>> {
    if std::env::var("DEEPSEEK_PC_HOST_DISABLE_MDNS")
        .ok()
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
    {
        return Ok(None);
    }

    let ip = lan_ipv4_for_mdns(bind_addr)?;
    let port = bind_addr.port();
    if bind_addr.ip().is_unspecified() {
        eprintln!(
            "deepseek-pc-host mDNS: bind is {bind_addr}; advertising on LAN IPv4 {ip}:{port}"
        );
    }
    let daemon = ServiceDaemon::new().context("create mDNS daemon")?;

    let instance_name = sanitize_instance_name(gateway_label, gateway_id);
    let host_name = format!("{gateway_id}.local.");
    let properties = [("gateway_id", gateway_id), ("label", gateway_label)];

    let service = ServiceInfo::new(
        PC_GATEWAY_MDNS_SERVICE,
        &instance_name,
        &host_name,
        ip.to_string(),
        port,
        &properties[..],
    )
    .context("build mDNS ServiceInfo")?;

    daemon
        .register(service)
        .context("register PC gateway mDNS service")?;

    println!("deepseek-pc-host mDNS: {instance_name} on {ip}:{port} ({PC_GATEWAY_MDNS_SERVICE})");

    Ok(Some(MdnsAdvertisement { _daemon: daemon }))
}

fn lan_ipv4_for_mdns(bind_addr: SocketAddr) -> Result<Ipv4Addr> {
    match bind_addr.ip() {
        IpAddr::V4(v4) if !v4.is_unspecified() && !v4.is_loopback() => Ok(v4),
        _ => primary_ipv4().context(
            "no LAN IPv4 for mDNS (bind a specific interface IP or ensure Wi-Fi/Ethernet is up)",
        ),
    }
}

fn primary_ipv4() -> Option<Ipv4Addr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(v4) if !v4.is_loopback() => Some(v4),
        _ => None,
    }
}

fn sanitize_instance_name(label: &str, gateway_id: &str) -> String {
    let raw = if label.trim().is_empty() {
        gateway_id.to_string()
    } else {
        label.trim().to_string()
    };
    let mut cleaned: String = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    if cleaned.is_empty() {
        cleaned = "deepseek-pc".to_string();
    }
    if cleaned.len() > 15 {
        cleaned.truncate(15);
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::{lan_ipv4_for_mdns, sanitize_instance_name};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn instance_name_is_short_and_safe() {
        let name = sanitize_instance_name("Developer PC #1", "pc-local");
        assert!(name.len() <= 15);
        assert!(name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-'));
    }

    #[test]
    fn specific_bind_uses_that_ipv4() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)), 8787);
        assert_eq!(
            lan_ipv4_for_mdns(addr).expect("ipv4"),
            Ipv4Addr::new(192, 168, 1, 50)
        );
    }

    #[test]
    fn loopback_bind_falls_back_to_primary_or_errors() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8787);
        let _ = lan_ipv4_for_mdns(addr);
    }
}
