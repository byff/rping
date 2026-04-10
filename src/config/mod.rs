use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ping: PingConfig,
    pub display: DisplayConfig,
    pub export: ExportConfig,
    pub remember_addresses: bool,
    pub last_addresses: Vec<String>,
    pub last_import_dir: Option<PathBuf>,
    pub window_width: f32,
    pub window_height: f32,
    pub debug_mode: bool,
    pub cidr_strip_first_last: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingConfig {
    pub timeout_ms: u64,
    pub packet_size: usize,
    pub interval_ms: u64,
    pub max_concurrent: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub show_hostname: bool,
    pub show_ip: bool,
    pub show_success_count: bool,
    pub show_fail_count: bool,
    pub show_fail_rate: bool,
    pub show_total_sent: bool,
    pub show_last_rtt: bool,
    pub show_max_rtt: bool,
    pub show_min_rtt: bool,
    pub show_avg_rtt: bool,
    pub show_index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub export_hostname: bool,
    pub export_ip: bool,
    pub export_success_count: bool,
    pub export_fail_count: bool,
    pub export_fail_rate: bool,
    pub export_total_sent: bool,
    pub export_last_rtt: bool,
    pub export_max_rtt: bool,
    pub export_min_rtt: bool,
    pub export_avg_rtt: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ping: PingConfig::default(),
            display: DisplayConfig::default(),
            export: ExportConfig::default(),
            remember_addresses: true,
            last_addresses: Vec::new(),
            last_import_dir: None,
            window_width: 1100.0,
            window_height: 680.0,
            debug_mode: false,
            cidr_strip_first_last: true,
        }
    }
}

impl Default for PingConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 3000,
            packet_size: 32,
            interval_ms: 1000,
            max_concurrent: 200,
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            show_hostname: true,
            show_ip: true,
            show_success_count: true,
            show_fail_count: true,
            show_fail_rate: true,
            show_total_sent: true,
            show_last_rtt: true,
            show_max_rtt: true,
            show_min_rtt: true,
            show_avg_rtt: true,
            show_index: true,
        }
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            export_hostname: true,
            export_ip: true,
            export_success_count: true,
            export_fail_count: true,
            export_fail_rate: true,
            export_total_sent: true,
            export_last_rtt: false,
            export_max_rtt: true,
            export_min_rtt: true,
            export_avg_rtt: true,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        exe_dir.join("pingtest_config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }
}
