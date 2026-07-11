use crate::error::{AppError, AppResult};
use crate::models::LanHost;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

pub const SERVICE_TYPE: &str = "_archhive._tcp.local.";
pub const LAN_PORT: u16 = 8787;

/// Browse mDNS and probe the local /24 subnet for ArcHive LAN hosts on `LAN_PORT`.
pub fn discover_lan_hosts(timeout_ms: u64) -> AppResult<Vec<LanHost>> {
    let mut by_url: HashMap<String, LanHost> = HashMap::new();

    let mdns_budget = timeout_ms.saturating_mul(2) / 5;
    if let Ok(mdns_hosts) = discover_mdns(mdns_budget) {
        for host in mdns_hosts {
            by_url.insert(host.url.clone(), host);
        }
    }

    let probe_budget = timeout_ms.saturating_sub(mdns_budget);
    for host in probe_subnet(LAN_PORT, probe_budget)? {
        by_url.insert(host.url.clone(), host);
    }

    let mut list: Vec<LanHost> = by_url.into_values().collect();
    list.sort_by(|a, b| a.ip.cmp(&b.ip));
    Ok(list)
}

fn discover_mdns(timeout_ms: u64) -> AppResult<Vec<LanHost>> {
    let mdns = ServiceDaemon::new().map_err(|e| AppError::Other(e.to_string()))?;
    let receiver = mdns
        .browse(SERVICE_TYPE)
        .map_err(|e| AppError::Other(e.to_string()))?;

    let mut hosts: HashMap<String, LanHost> = HashMap::new();
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);

    while Instant::now() < deadline {
        let wait = deadline
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(250));
        match receiver.recv_timeout(wait) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let name = info.get_fullname().to_string();
                let port = info.get_port();
                for ip in info.get_addresses_v4() {
                    if ip.is_loopback() || ip.is_unspecified() {
                        continue;
                    }
                    let url = format!("http://{ip}:{port}");
                    hosts.insert(
                        url.clone(),
                        LanHost {
                            name: name.clone(),
                            url,
                            ip: ip.to_string(),
                            port,
                        },
                    );
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    let _ = mdns.shutdown();
    Ok(hosts.into_values().collect())
}

fn probe_subnet(port: u16, timeout_ms: u64) -> AppResult<Vec<LanHost>> {
    let local = local_ipv4_for_mdns();
    if local.is_loopback() || local.is_unspecified() {
        return Ok(vec![]);
    }

    let octets = local.octets();
    let prefix = format!("{}.{}.{}", octets[0], octets[1], octets[2]);
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let per_ip = Duration::from_millis(120);

    let (tx, rx) = std::sync::mpsc::channel::<LanHost>();

    for batch_start in (1_i32..=254).step_by(32) {
        if Instant::now() >= deadline {
            break;
        }
        std::thread::scope(|scope| {
            for host_octet in batch_start..=batch_start.saturating_add(31).min(254) {
                let tx = tx.clone();
                let prefix = prefix.clone();
                scope.spawn(move || {
                    let ip = format!("{prefix}.{host_octet}");
                    if let Some(found) = probe_archhive_at(&ip, port, per_ip) {
                        let _ = tx.send(found);
                    }
                });
            }
        });
    }
    drop(tx);

    let mut hosts: HashMap<String, LanHost> = HashMap::new();
    while let Ok(host) = rx.recv_timeout(Duration::from_millis(50)) {
        hosts.insert(host.url.clone(), host);
        if Instant::now() >= deadline {
            break;
        }
    }

    Ok(hosts.into_values().collect())
}

fn probe_archhive_at(ip: &str, port: u16, timeout: Duration) -> Option<LanHost> {
    let addr: SocketAddr = format!("{ip}:{port}").parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&addr, timeout).ok()?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let request = format!(
        "GET /api/health HTTP/1.1\r\nHost: {ip}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).ok()?;

    let mut buf = vec![0u8; 2048];
    let read = stream.read(&mut buf).ok()?;
    if read == 0 {
        return None;
    }
    let response = String::from_utf8_lossy(&buf[..read]);
    if !response.contains("\"status\"") && !response.contains("ok") {
        return None;
    }
    if response.contains("\"lan\":true") || response.contains("\"lan\": true") {
        return Some(LanHost {
            name: format!("ArcHive @ {ip}"),
            url: format!("http://{ip}:{port}"),
            ip: ip.to_string(),
            port,
        });
    }
    None
}

/// Best-effort primary LAN IPv4 for mDNS registration.
pub fn local_ipv4_for_mdns() -> Ipv4Addr {
    let ip = std::net::UdpSocket::bind("0.0.0.0:0")
        .ok()
        .and_then(|socket| {
            socket.connect("8.8.8.8:80").ok()?;
            socket.local_addr().ok()
        })
        .and_then(|addr| match addr.ip() {
            std::net::IpAddr::V4(v4) => Some(v4),
            _ => None,
        });

    match ip {
        Some(v4) if !v4.is_loopback() && !v4.is_unspecified() => v4,
        _ => Ipv4Addr::new(127, 0, 0, 1),
    }
}
