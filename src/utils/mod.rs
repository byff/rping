use std::net::IpAddr;
use ipnetwork::IpNetwork;

fn network_ip_count(network: &IpNetwork) -> u128 {
    match network {
        IpNetwork::V4(n) => {
            let prefix = n.prefix();
            if prefix >= 32 { 1 } else { 1u128 << (32 - prefix) }
        }
        IpNetwork::V6(n) => {
            let prefix = n.prefix();
            if prefix >= 128 { 1 } else { 1u128 << (128 - prefix) }
        }
    }
}

/// Parse input text into a list of IP addresses.
/// Supports: single IP, CIDR notation, hostname, one per line or comma-separated.
pub fn parse_targets(input: &str) -> Vec<(String, IpAddr)> {
    let mut results = Vec::new();
    let lines: Vec<&str> = input
        .lines()
        .flat_map(|l| l.split(','))
        .flat_map(|l| l.split(';'))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for entry in lines {
        if let Ok(network) = entry.parse::<IpNetwork>() {
            // CIDR notation
            if network_ip_count(&network) > 65534 {
                // Too large, skip (caller should warn)
                continue;
            }
            for ip in network.iter() {
                results.push((ip.to_string(), ip));
            }
        } else if let Ok(ip) = entry.parse::<IpAddr>() {
            results.push((ip.to_string(), ip));
        } else {
            // Try DNS resolve
            if let Ok(addrs) = dns_lookup::lookup_host(entry) {
                if let Some(ip) = addrs.into_iter().next() {
                    results.push((entry.to_string(), ip));
                }
            }
        }
    }

    results
}

/// Count how many IPs a CIDR would expand to (for warning dialog)
pub fn count_cidr_ips(input: &str) -> usize {
    let mut count = 0usize;
    let lines: Vec<&str> = input
        .lines()
        .flat_map(|l| l.split(','))
        .flat_map(|l| l.split(';'))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for entry in lines {
        if let Ok(network) = entry.parse::<IpNetwork>() {
            count += network_ip_count(&network) as usize;
        } else {
            count += 1;
        }
    }
    count
}

/// Find columns in Excel data that contain IP addresses
pub fn find_ip_columns(headers: &[String], rows: &[Vec<String>]) -> Vec<(usize, String)> {
    let mut ip_cols = Vec::new();

    for (col_idx, header) in headers.iter().enumerate() {
        let mut ip_count = 0;
        let sample_size = rows.len().min(20);
        for row in rows.iter().take(sample_size) {
            if let Some(cell) = row.get(col_idx) {
                let trimmed = cell.trim();
                if trimmed.parse::<IpAddr>().is_ok()
                    || trimmed.parse::<IpNetwork>().is_ok()
                {
                    ip_count += 1;
                }
            }
        }
        // If more than 50% of sampled rows have IPs, consider it an IP column
        if ip_count > 0 && ip_count * 2 >= sample_size {
            ip_cols.push((col_idx, header.clone()));
        }
    }

    ip_cols
}
