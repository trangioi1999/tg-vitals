//! Combines every reader into one [`Snapshot`]. Keep one `Sampler` alive and call
//! [`Sampler::sample`] about once a second: rates and CPU% are deltas between calls.
use crate::cpu::{self, CpuTicks};
use crate::model::*;
use crate::process::ProcessSampler;
use crate::sensors::{volume_space, Sensors};
use crate::{machine, memory, network, sys};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub struct Sampler {
    cpu_previous: Option<Vec<CpuTicks>>,
    p_cores: usize,
    e_cores: usize,
    net_previous: Option<(u64, u64, Instant)>,
    sensors: Sensors,
    processes: ProcessSampler,
    model: String,
    chip: String,
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Sampler {
    pub fn new() -> Self {
        let levels = sys::sysctl_u64("hw.nperflevels").unwrap_or(1);
        let p_cores = sys::sysctl_u64("hw.perflevel0.logicalcpu").unwrap_or(0) as usize;
        let e_cores = if levels > 1 { sys::sysctl_u64("hw.perflevel1.logicalcpu").unwrap_or(0) as usize } else { 0 };
        let mut sampler = Self {
            cpu_previous: None,
            p_cores,
            e_cores,
            net_previous: None,
            sensors: Sensors::new(),
            processes: ProcessSampler::new(),
            model: machine::family_from_model(&machine::hw_model()),
            chip: machine::chip(),
        };
        // Prime the delta counters so the first real sample has sensible rates.
        sampler.cpu_previous = cpu::read_ticks();
        sampler
    }

    /// Replace the guessed model name ("Mac") with the marketing name, once known.
    pub fn set_model_name(&mut self, name: String) {
        self.model = name;
    }

    /// `with_processes` = false skips the (comparatively expensive) process list,
    /// used when only the menu bar is visible.
    pub fn sample(&mut self, with_processes: bool, icon_url: &dyn Fn(&str) -> String) -> Snapshot {
        Snapshot {
            time: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0),
            machine: Machine { model: self.model.clone(), chip: self.chip.clone(), uptime: machine::uptime_seconds() },
            cpu: self.cpu(),
            memory: memory::read().unwrap_or_default(),
            disk: self.disk(),
            network: self.network(),
            battery: self.sensors.battery(),
            processes: if with_processes { self.processes.sample(icon_url) } else { Vec::new() },
        }
    }

    pub fn sensors(&self) -> &Sensors {
        &self.sensors
    }

    fn cpu(&mut self) -> CpuInfo {
        let current = cpu::read_ticks();
        let usage = match (&self.cpu_previous, &current) {
            (Some(old), Some(new)) if old.len() == new.len() => cpu::usage(old, new),
            _ => cpu::CpuUsage { per_core: vec![0.0; current.as_ref().map_or(0, |c| c.len())], ..Default::default() },
        };
        self.cpu_previous = current;
        let cores = usage.per_core.len();
        CpuInfo {
            total: (usage.user + usage.system).min(1.0),
            user: usage.user,
            system: usage.system,
            per_core: cpu::performance_first(&usage.per_core, self.e_cores),
            p_cores: if self.p_cores == 0 || self.p_cores > cores { cores } else { self.p_cores },
            temperature: self.sensors.temperature(),
        }
    }

    fn disk(&mut self) -> DiskInfo {
        let (total, free) = volume_space();
        let (read, write) = self.sensors.disk_io();
        DiskInfo { total, free, read_per_sec: read, write_per_sec: write }
    }

    fn network(&mut self) -> NetworkInfo {
        let Some((bytes_in, bytes_out)) = network::read_counters() else { return NetworkInfo::default() };
        let now = Instant::now();
        let (in_rate, out_rate) = match self.net_previous {
            Some((i, o, at)) => {
                let secs = now.duration_since(at).as_secs_f64();
                (network::rate(i, bytes_in, secs), network::rate(o, bytes_out, secs))
            }
            None => (0.0, 0.0),
        };
        self.net_previous = Some((bytes_in, bytes_out, now));
        NetworkInfo { in_per_sec: in_rate, out_per_sec: out_rate, total_in: bytes_in, total_out: bytes_out }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs against the real machine (macOS only): values must be in sane ranges.
    #[test]
    fn live_snapshot_is_sane() {
        let mut sampler = Sampler::new();
        sampler.sample(true, &|p| p.to_string());
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let s = sampler.sample(true, &|p| format!("appicon://localhost/{p}"));

        assert!(!s.cpu.per_core.is_empty());
        assert!((0.0..=1.0).contains(&s.cpu.total), "cpu {}", s.cpu.total);
        assert!(s.cpu.p_cores <= s.cpu.per_core.len());
        assert!(s.memory.total > 1 << 30);
        assert!(s.memory.used() > 0 && s.memory.used() <= s.memory.total);
        assert!(s.disk.total > 0 && s.disk.free <= s.disk.total);
        assert!(s.network.in_per_sec >= 0.0);
        assert!(s.machine.uptime > 0);
        assert!(!s.machine.chip.is_empty());
        assert!(s.processes.len() > 20, "only {} processes", s.processes.len());
        let me = s.processes.iter().find(|p| p.pid == std::process::id() as i32).expect("own process listed");
        assert!(me.memory > 0 && me.threads > 0);
        if let Some(b) = &s.battery {
            assert!((0.0..=1.0).contains(&b.percent));
        }
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"perCore\""));
        eprintln!(
            "cpu {:.0}% · mem {}/{} · temp {:?} · {} procs · battery {:?}\nsensors: {:?}",
            s.cpu.total * 100.0, s.memory.used(), s.memory.total, s.cpu.temperature, s.processes.len(), s.battery,
            sampler.sensors().all_temperatures()
        );
    }

    #[test]
    fn renders_an_app_icon() {
        let png = crate::icon::png_for_path("/System/Library/CoreServices/Finder.app", 64).expect("icon");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(png.len() > 500);
    }
}
