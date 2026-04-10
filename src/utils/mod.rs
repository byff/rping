use std::net::IpAddr;
use ipnetwork::IpNetwork;
use regex::Regex;

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

/// Extract and clean IP addresses from messy text containing Chinese, mixed characters, etc.
/// Returns cleaned text with one IP/CIDR/domain per line.
pub fn extract_and_clean_ips(input: &str) -> String {
    // Match IPv4, IPv4/CIDR, or domain-like patterns
    let ip_re = Regex::new(
        r"(?x)
        (\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}(?:/\d{1,2})?)  # IPv4 or IPv4/CIDR
        |
        ([a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z]{2,})+)  # domain
        "
    ).unwrap();

    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for cap in ip_re.captures_iter(input) {
        let matched = cap.get(0).unwrap().as_str().to_string();

        // Validate: if it looks like an IP, check octets
        if let Some(ip_part) = cap.get(1) {
            let ip_str = ip_part.as_str();
            let base_ip = ip_str.split('/').next().unwrap_or(ip_str);
            let octets: Vec<&str> = base_ip.split('.').collect();
            if octets.len() == 4 {
                let valid = octets.iter().all(|o| {
                    o.parse::<u16>().map(|v| v <= 255).unwrap_or(false)
                });
                if !valid {
                    continue;
                }
            }
        }

        if seen.insert(matched.clone()) {
            results.push(matched);
        }
    }

    results.join("\n")
}

/// Parse input text into a list of IP addresses.
/// Returns (targets, skipped_count).
/// If strip_first_last is true, CIDR ranges will exclude network and broadcast addresses.
pub fn parse_targets(input: &str, strip_first_last: bool) -> (Vec<(String, IpAddr)>, usize) {
    let mut results = Vec::new();
    let mut skipped = 0usize;
    let lines: Vec<&str> = input
        .lines()
        .flat_map(|l| l.split(','))
        .flat_map(|l| l.split(';'))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for entry in lines {
        if let Ok(network) = entry.parse::<IpNetwork>() {
            if network_ip_count(&network) > 65534 {
                skipped += 1;
                continue;
            }
            let ips: Vec<IpAddr> = network.iter().collect();
            let len = ips.len();
            if strip_first_last && len > 2 {
                for ip in &ips[1..len-1] {
                    results.push((ip.to_string(), *ip));
                }
            } else {
                for ip in ips {
                    results.push((ip.to_string(), ip));
                }
            }
        } else if let Ok(ip) = entry.parse::<IpAddr>() {
            results.push((ip.to_string(), ip));
        } else {
            if let Ok(addrs) = dns_lookup::lookup_host(entry) {
                if let Some(ip) = addrs.into_iter().next() {
                    results.push((entry.to_string(), ip));
                } else {
                    skipped += 1;
                }
            } else {
                skipped += 1;
            }
        }
    }

    (results, skipped)
}

/// Count how many IPs a CIDR would expand to
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
        if ip_count > 0 && ip_count * 2 >= sample_size {
            ip_cols.push((col_idx, header.clone()));
        }
    }

    ip_cols
}
