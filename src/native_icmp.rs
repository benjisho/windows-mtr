use serde_json::{Value, json};
use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    pub count: usize,
    pub max_hops: u8,
    pub timeout: Duration,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hop {
    pub ttl: u8,
    pub address: Option<Ipv4Addr>,
    pub sent: usize,
    pub received: Vec<f64>,
}

impl Hop {
    pub fn loss_pct(&self) -> f64 {
        100.0 * (self.sent.saturating_sub(self.received.len())) as f64 / self.sent.max(1) as f64
    }

    pub fn best(&self) -> Option<f64> {
        self.received.iter().copied().reduce(f64::min)
    }

    pub fn worst(&self) -> Option<f64> {
        self.received.iter().copied().reduce(f64::max)
    }

    pub fn avg(&self) -> Option<f64> {
        (!self.received.is_empty())
            .then(|| self.received.iter().sum::<f64>() / self.received.len() as f64)
    }

    pub fn last(&self) -> Option<f64> {
        self.received.last().copied()
    }
}

pub fn resolve_ipv4(target: &str) -> anyhow::Result<Ipv4Addr> {
    if let Ok(IpAddr::V4(addr)) = target.parse() {
        return Ok(addr);
    }

    target
        .to_socket_addrs()?
        .find_map(|addr| match addr.ip() {
            IpAddr::V4(ip) => Some(ip),
            IpAddr::V6(_) => None,
        })
        .ok_or_else(|| anyhow::anyhow!("no IPv4 address found for {target}"))
}

#[cfg(windows)]
pub fn trace(target: &str, config: &Config) -> anyhow::Result<Vec<Hop>> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        ICMP_ECHO_REPLY, IP_OPTION_INFORMATION, IP_SUCCESS, IP_TTL_EXPIRED_TRANSIT,
        IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
    };

    let target = resolve_ipv4(target)?;
    let handle = unsafe { IcmpCreateFile() };
    if handle == INVALID_HANDLE_VALUE {
        anyhow::bail!("IcmpCreateFile failed: {}", std::io::Error::last_os_error());
    }

    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe { IcmpCloseHandle(self.0) };
        }
    }
    let _handle = Handle(handle);

    let timeout_ms = config.timeout.as_millis().clamp(1, u32::MAX as u128) as u32;
    let mut hops = Vec::new();
    for ttl in 1..=config.max_hops {
        let sent = config.count.max(1);
        let mut hop = Hop {
            ttl,
            address: None,
            sent,
            received: Vec::new(),
        };

        for _ in 0..sent {
            let mut options = IP_OPTION_INFORMATION {
                Ttl: ttl,
                Tos: 0,
                Flags: 0,
                OptionsSize: 0,
                OptionsData: std::ptr::null_mut(),
            };
            let payload = [0u8; 32];
            let mut reply_buffer = vec![0u8; size_of::<ICMP_ECHO_REPLY>() + payload.len() + 8];
            let replies = unsafe {
                IcmpSendEcho(
                    handle,
                    u32::from_ne_bytes(target.octets()),
                    payload.as_ptr().cast::<c_void>(),
                    payload.len() as u16,
                    &mut options,
                    reply_buffer.as_mut_ptr().cast::<c_void>(),
                    reply_buffer.len() as u32,
                    timeout_ms,
                )
            };
            if replies == 0 {
                continue;
            }

            let reply = unsafe {
                std::ptr::read_unaligned(reply_buffer.as_ptr().cast::<ICMP_ECHO_REPLY>())
            };
            if reply.Status == IP_SUCCESS || reply.Status == IP_TTL_EXPIRED_TRANSIT {
                hop.address = Some(Ipv4Addr::from(reply.Address.to_ne_bytes()));
                hop.received.push(reply.RoundTripTime as f64);
            }
        }

        let reached_target = hop.address == Some(target);
        hops.push(hop);
        if reached_target {
            break;
        }
    }

    Ok(hops)
}

#[cfg(not(windows))]
pub fn trace(_target: &str, _config: &Config) -> anyhow::Result<Vec<Hop>> {
    anyhow::bail!("native Windows ICMP probing is only available on Windows")
}

pub fn json_report(target: &str, hops: &[Hop]) -> Value {
    let hops = hops
        .iter()
        .map(|hop| {
            json!({
                "ttl": hop.ttl,
                "host": hop.address.map(|address| address.to_string()),
                "loss_pct": hop.loss_pct(),
                "sent": hop.sent,
                "recv": hop.received.len(),
                "last": hop.last(),
                "avg": hop.avg(),
                "best": hop.best(),
                "worst": hop.worst(),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "schema_version": "1.0",
        "report": {
            "target": target,
            "protocol": "icmp",
            "backend": "windows-icmp-helper",
            "hops": hops,
        }
    })
}

pub fn format_report(target: &str, hops: &[Hop]) -> String {
    let mut report = format!("windows-mtr ICMP report for {target}\n");
    report.push_str("Hop  Host             Loss%  Snt  Recv  Last   Avg   Best  Wrst\n");
    for hop in hops {
        let host = hop
            .address
            .map(|address| address.to_string())
            .unwrap_or_else(|| "???".to_string());
        report.push_str(&format!(
            "{:<4} {:<16} {:>5.1} {:>4} {:>5} {:>5} {:>5} {:>5} {:>5}\n",
            hop.ttl,
            host,
            hop.loss_pct(),
            hop.sent,
            hop.received.len(),
            format_ms(hop.last()),
            format_ms(hop.avg()),
            format_ms(hop.best()),
            format_ms(hop.worst()),
        ));
    }
    report
}

fn format_ms(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "???".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_metrics_use_received_samples_only() {
        let hop = Hop {
            ttl: 1,
            address: None,
            sent: 4,
            received: vec![10.0, 30.0],
        };
        assert_eq!(hop.loss_pct(), 50.0);
        assert_eq!(hop.best(), Some(10.0));
        assert_eq!(hop.avg(), Some(20.0));
        assert_eq!(hop.worst(), Some(30.0));
        assert_eq!(hop.last(), Some(30.0));
    }

    #[test]
    fn json_report_keeps_unresponsive_hops() {
        let report = json_report(
            "8.8.8.8",
            &[Hop {
                ttl: 1,
                address: None,
                sent: 1,
                received: vec![],
            }],
        );
        assert_eq!(report["report"]["protocol"], "icmp");
        assert_eq!(report["report"]["hops"][0]["loss_pct"], 100.0);
        assert!(report["report"]["hops"][0]["host"].is_null());
    }
}
