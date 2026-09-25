//! Medição do próprio processo: CPU, RSS e threads via /proc/self.
//! "100% de CPU" equivale a um núcleo completo ocupado.

use std::{fs, time::Instant};

pub struct ProcessMeter {
    last: Option<(Instant, u64)>,
    pub cpu: Option<f64>,
    pub rss_mib: Option<f64>,
    pub threads: Option<u64>,
}

impl ProcessMeter {
    pub fn new() -> Self {
        Self { last: None, cpu: None, rss_mib: None, threads: None }
    }

    /// Lê /proc/self/stat e /proc/self/status para atualizar as métricas.
    pub fn update(&mut self) {
        // CPU a partir de /proc/self/stat
        if let Ok(stat) = fs::read_to_string("/proc/self/stat") {
            // O nome do processo pode conter parênteses; buscamos após o último ')'.
            if let Some((_, tail)) = stat.rsplit_once(") ") {
                let fields: Vec<_> = tail.split_whitespace().collect();
                if let (Some(user), Some(system)) = (
                    fields.get(11).and_then(|v| v.parse::<u64>().ok()),
                    fields.get(12).and_then(|v| v.parse::<u64>().ok()),
                ) {
                    let now   = Instant::now();
                    let ticks = user.saturating_add(system);
                    if let Some((at, old)) = self.last {
                        let seconds = now.duration_since(at).as_secs_f64();
                        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
                        if hz > 0 && seconds > 0.0 {
                            self.cpu = Some(
                                100.0 * ticks.saturating_sub(old) as f64 / hz as f64 / seconds,
                            );
                        }
                    }
                    self.last = Some((now, ticks));
                }
            }
        }

        // RSS e threads a partir de /proc/self/status
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if let Some(kib) = line
                    .strip_prefix("VmRSS:")
                    .and_then(|v| v.split_whitespace().next())
                    .and_then(|v| v.parse::<f64>().ok())
                {
                    self.rss_mib = Some(kib / 1024.0);
                }
                if let Some(n) = line
                    .strip_prefix("Threads:")
                    .and_then(|v| v.trim().parse().ok())
                {
                    self.threads = Some(n);
                }
            }
        }
    }
}
