use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tokio::sync::Semaphore;

#[derive(Debug, Clone)]
pub struct PingTarget {
    pub index: usize,
    pub hostname: String,
    pub ip: IpAddr,
    pub stats: Arc<RwLock<PingStats>>,
}

#[derive(Debug, Clone, Default)]
pub struct PingStats {
    pub success_count: u64,
    pub fail_count: u64,
    pub total_sent: u64,
    pub last_rtt_us: Option<u64>,
    pub max_rtt_us: u64,
    pub min_rtt_us: u64,
    pub total_rtt_us: u64,
    pub is_alive: bool,
}

impl PingStats {
    pub fn fail_rate(&self) -> f64 {
        if self.total_sent == 0 {
            0.0
        } else {
            (self.fail_count as f64 / self.total_sent as f64) * 100.0
        }
    }

    pub fn avg_rtt_us(&self) -> u64 {
        if self.success_count == 0 {
            0
        } else {
            self.total_rtt_us / self.success_count
        }
    }

    pub fn record_success(&mut self, rtt_us: u64) {
        self.total_sent += 1;
        self.success_count += 1;
        self.last_rtt_us = Some(rtt_us);
        self.total_rtt_us += rtt_us;
        self.is_alive = true;

        if self.success_count == 1 {
            self.min_rtt_us = rtt_us;
            self.max_rtt_us = rtt_us;
        } else {
            if rtt_us < self.min_rtt_us {
                self.min_rtt_us = rtt_us;
            }
            if rtt_us > self.max_rtt_us {
                self.max_rtt_us = rtt_us;
            }
        }
    }

    pub fn record_failure(&mut self) {
        self.total_sent += 1;
        self.fail_count += 1;
        self.last_rtt_us = None;
        self.is_alive = false;
    }
}

pub struct PingEngine {
    targets: Vec<PingTarget>,
    running: Arc<RwLock<bool>>,
    timeout: Duration,
    interval: Duration,
    packet_size: usize,
    max_concurrent: usize,
}

impl PingEngine {
    pub fn new(
        timeout_ms: u64,
        interval_ms: u64,
        packet_size: usize,
        max_concurrent: usize,
    ) -> Self {
        Self {
            targets: Vec::new(),
            running: Arc::new(RwLock::new(false)),
            timeout: Duration::from_millis(timeout_ms),
            interval: Duration::from_millis(interval_ms),
            packet_size,
            max_concurrent,
        }
    }

    pub fn set_targets(&mut self, targets: Vec<PingTarget>) {
        self.targets = targets;
    }

    pub fn targets(&self) -> &[PingTarget] {
        &self.targets
    }

    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    pub fn stop(&self) {
        *self.running.write() = false;
    }

    pub fn start(&self, runtime: &tokio::runtime::Handle) {
        *self.running.write() = true;
        let targets = self.targets.clone();
        let running = self.running.clone();
        let timeout = self.timeout;
        let interval = self.interval;
        let payload_size = self.packet_size;
        let max_concurrent = self.max_concurrent;

        runtime.spawn(async move {
            let semaphore = Arc::new(Semaphore::new(max_concurrent));

            while *running.read() {
                let start = Instant::now();
                let mut handles = Vec::new();

                for target in &targets {
                    let sem = semaphore.clone();
                    let stats = target.stats.clone();
                    let ip = target.ip;
                    let running = running.clone();

                    let handle = tokio::spawn(async move {
                        if !*running.read() {
                            return;
                        }
                        let _permit = sem.acquire().await.ok();
                        let result = do_ping(ip, timeout, payload_size).await;
                        let mut s = stats.write();
                        match result {
                            Ok(rtt) => s.record_success(rtt),
                            Err(_) => s.record_failure(),
                        }
                    });
                    handles.push(handle);
                }

                for h in handles {
                    let _ = h.await;
                }

                let elapsed = start.elapsed();
                if elapsed < interval {
                    tokio::time::sleep(interval - elapsed).await;
                }
            }
        });
    }
}

async fn do_ping(ip: IpAddr, timeout: Duration, payload_size: usize) -> Result<u64, String> {
    use surge_ping::{Client, Config, PingIdentifier, PingSequence, ICMP};

    let config = Config::builder().kind(match ip {
        IpAddr::V4(_) => ICMP::V4,
        IpAddr::V6(_) => ICMP::V6,
    }).build();

    let client = Client::new(&config).map_err(|e| e.to_string())?;
    let payload = vec![0u8; payload_size];
    let mut pinger = client.pinger(ip, PingIdentifier(rand_id())).await;
    pinger.timeout(timeout);

    let start = Instant::now();
    match pinger.ping(PingSequence(0), &payload).await {
        Ok((_packet, dur)) => Ok(dur.as_micros() as u64),
        Err(_) => {
            let elapsed = start.elapsed();
            Err(format!("timeout after {:?}", elapsed))
        }
    }
}

fn rand_id() -> u16 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let s = RandomState::new();
    let mut h = s.build_hasher();
    h.write_u64(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64);
    h.finish() as u16
}
