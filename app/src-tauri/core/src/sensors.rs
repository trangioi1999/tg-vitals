//! Battery, disk I/O, temperature and volume space (IOKit + statfs).

/// Average of the CPU die sensors. Names differ per chip generation.
pub fn cpu_temperature(sensors: &[(String, f64)]) -> Option<f64> {
    let valid = |t: &f64| *t > 5.0 && *t < 130.0;
    let pick = |pred: &dyn Fn(&str) -> bool| -> Option<f64> {
        let temps: Vec<f64> = sensors.iter().filter(|(n, t)| pred(n) && valid(t)).map(|(_, t)| *t).collect();
        (!temps.is_empty()).then(|| temps.iter().sum::<f64>() / temps.len() as f64)
    };
    pick(&|n| n.starts_with("pACC MTR Temp") || n.starts_with("eACC MTR Temp"))
        .or_else(|| pick(&|n| n.contains("tdie")))
        .or_else(|| pick(&|n| n.contains("CPU")))
}

/// SMC temperature keys on Intel Macs, most specific first. Per-core keys (`TC1C`…) exist on
/// Skylake and later (e.g. MacBook Pro 16" 2019); older chips only have die/proximity keys.
pub const SMC_CPU_KEYS: &[&str] = &[
    "TC1C", "TC2C", "TC3C", "TC4C", "TC5C", "TC6C", "TC7C", "TC8C", "TC9C", "TCAC", "TCBC", "TCCC", "TCDC",
    "TCEC", "TCFC", "TCGC", "TCXC", "TCXc", "TC0D", "TC0E", "TC0F", "TC0P", "TC0H",
];

/// Decode an SMC value by its type: `spXY` / `fpXY` fixed point (last hex digit = fraction bits),
/// `flt ` little-endian f32, `ui8 ` / `ui16` integers.
pub fn smc_decode(kind: &[u8; 4], bytes: &[u8]) -> Option<f64> {
    let be16 = || Some(u16::from_be_bytes(bytes.get(..2)?.try_into().ok()?));
    match kind {
        b"flt " => Some(f32::from_le_bytes(bytes.get(..4)?.try_into().ok()?) as f64),
        b"ui8 " => bytes.first().map(|&b| b as f64),
        b"ui16" => be16().map(f64::from),
        [b's', b'p', _, f] | [b'f', b'p', _, f] => {
            let frac = (*f as char).to_digit(16)?;
            let raw = be16()?;
            let value = if kind[0] == b's' { raw as i16 as f64 } else { raw as f64 };
            Some(value / f64::from(1u32 << frac))
        }
        _ => None,
    }
}

/// CPU temperature from SMC readings: average of the per-core sensors, else the first die /
/// PECI / proximity key that reads a sane value.
pub fn smc_cpu_temperature(readings: &[(String, f64)]) -> Option<f64> {
    let valid = |t: f64| t > 5.0 && t < 130.0;
    let cores: Vec<f64> = readings
        .iter()
        .filter(|(k, t)| k.len() == 4 && k.starts_with("TC") && k.ends_with('C') && k != "TCXC" && valid(*t))
        .map(|(_, t)| *t)
        .collect();
    if !cores.is_empty() {
        return Some(cores.iter().sum::<f64>() / cores.len() as f64);
    }
    SMC_CPU_KEYS.iter().find_map(|key| readings.iter().find(|(k, t)| k == key && valid(*t)).map(|(_, t)| *t))
}

/// Max capacity relative to design. Apple silicon reports `MaxCapacity` as a percent (100)
/// and the real mAh in `AppleRawMaxCapacity`.
pub fn battery_health(raw_max: Option<i64>, max: Option<i64>, design: Option<i64>) -> f64 {
    let Some(design) = design.filter(|&d| d > 0) else { return 1.0 };
    let max = raw_max.filter(|&m| m > 0).or(max.filter(|&m| m > 100));
    max.map(|m| (m as f64 / design as f64).clamp(0.0, 1.0)).unwrap_or(1.0)
}

/// "Maximum Capacity" exactly as System Settings shows it, from `system_profiler SPPowerDataType -json`
/// (macOS computes it privately; it is not in IOKit). Returns 0..1.
pub fn parse_max_capacity(json: &str) -> Option<f64> {
    let root: serde_json::Value = serde_json::from_str(json).ok()?;
    root.get("SPPowerDataType")?.as_array()?.iter().find_map(|item| {
        let text = item.get("sppower_battery_health_info")?.get("sppower_battery_health_maximum_capacity")?.as_str()?;
        let percent: f64 = text.trim().trim_end_matches('%').trim().parse().ok()?;
        (percent > 0.0 && percent <= 100.0).then_some(percent / 100.0)
    })
}

/// Whole-machine watts: telemetry `SystemLoad` (mW) when present, else |amps × volts|.
pub fn power_watts(system_load_mw: Option<i64>, amperage_ma: Option<i64>, voltage_mv: Option<i64>) -> f64 {
    if let Some(mw) = system_load_mw.filter(|&mw| mw > 0) {
        return mw as f64 / 1000.0;
    }
    match (amperage_ma, voltage_mv) {
        (Some(a), Some(v)) => (a as f64 * v as f64).abs() / 1_000_000.0,
        _ => 0.0,
    }
}

#[cfg(target_os = "macos")]
pub use mac::{volume_space, Sensors};

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use crate::iokit::{self, HidThermal, Service, Smc};
    use crate::model::BatteryInfo;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    /// How often to re-ask `system_profiler` for battery health (it changes over weeks).
    const HEALTH_REFRESH: Duration = Duration::from_secs(600);

    fn macos_battery_health() -> Option<f64> {
        let out = std::process::Command::new("/usr/sbin/system_profiler").args(["SPPowerDataType", "-json"]).output().ok()?;
        out.status.success().then(|| parse_max_capacity(&String::from_utf8_lossy(&out.stdout)))?
    }

    pub struct Sensors {
        thermal: Option<HidThermal>,
        /// Intel only: temperatures come from the SMC.
        smc: Option<Smc>,
        disk_previous: Option<(u64, u64, Instant)>,
        has_battery: bool,
        /// Latest health from `system_profiler`, filled by a background thread.
        macos_health: Arc<Mutex<Option<f64>>>,
        health_asked: Option<Instant>,
    }

    impl Default for Sensors {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Sensors {
        pub fn new() -> Self {
            // Intel Macs expose temperatures through SMC, not these IOHID sensors.
            if cfg!(target_arch = "aarch64") {
                Self { thermal: HidThermal::new(), ..Self::new_without_thermal() }
            } else {
                Self { smc: Smc::open(), ..Self::new_without_thermal() }
            }
        }

        pub fn new_without_thermal() -> Self {
            Self {
                thermal: None,
                smc: None,
                disk_previous: None,
                has_battery: true,
                macos_health: Arc::default(),
                health_asked: None,
            }
        }

        pub fn temperature(&self) -> Option<f64> {
            if let Some(hid) = &self.thermal {
                return cpu_temperature(&hid.read());
            }
            smc_cpu_temperature(&self.smc.as_ref()?.temperatures(SMC_CPU_KEYS))
        }

        /// All temperature sensors, for debugging new chips.
        pub fn all_temperatures(&self) -> Vec<(String, f64)> {
            match (&self.thermal, &self.smc) {
                (Some(hid), _) => hid.read(),
                (None, Some(smc)) => smc.temperatures(SMC_CPU_KEYS),
                _ => Vec::new(),
            }
        }

        /// (read bytes/s, write bytes/s) summed over every block storage driver.
        pub fn disk_io(&mut self) -> (f64, f64) {
            let (mut read, mut write) = (0u64, 0u64);
            for service in Service::all("IOBlockStorageDriver") {
                if let Some(stats) = service.property("Statistics") {
                    read += iokit::get_i64(stats.get(), "Bytes (Read)").unwrap_or(0).max(0) as u64;
                    write += iokit::get_i64(stats.get(), "Bytes (Write)").unwrap_or(0).max(0) as u64;
                }
            }
            let now = Instant::now();
            let rates = match self.disk_previous {
                Some((r, w, at)) => {
                    let secs = now.duration_since(at).as_secs_f64();
                    (crate::network::rate(r, read, secs), crate::network::rate(w, write, secs))
                }
                None => (0.0, 0.0),
            };
            self.disk_previous = Some((read, write, now));
            rates
        }

        /// Re-reads System Settings' battery health off the sampling thread (~150 ms).
        fn refresh_macos_health(&mut self) {
            if self.health_asked.is_some_and(|at| at.elapsed() < HEALTH_REFRESH) {
                return;
            }
            self.health_asked = Some(Instant::now());
            let slot = Arc::clone(&self.macos_health);
            std::thread::spawn(move || {
                if let Some(h) = macos_battery_health() {
                    *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(h);
                }
            });
        }

        pub fn battery(&mut self) -> Option<BatteryInfo> {
            if !self.has_battery {
                return None;
            }
            let Some(service) = Service::all("AppleSmartBattery").into_iter().next() else {
                self.has_battery = false;
                return None;
            };
            self.refresh_macos_health();
            let macos_health = *self.macos_health.lock().unwrap_or_else(|e| e.into_inner());
            let props = service.properties()?;
            let p = props.get();
            let current = iokit::get_i64(p, "CurrentCapacity").unwrap_or(0);
            let max = iokit::get_i64(p, "MaxCapacity").filter(|&m| m > 0).unwrap_or(100);
            let telemetry = iokit::dict_get(p, "PowerTelemetryData");
            let system_load = telemetry.and_then(|t| iokit::get_i64(t, "SystemLoad"));
            let amperage = iokit::get_i64(p, "InstantAmperage").or_else(|| iokit::get_i64(p, "Amperage"));
            Some(BatteryInfo {
                percent: (current as f64 / max as f64).clamp(0.0, 1.0),
                charging: iokit::get_bool(p, "IsCharging").unwrap_or(false),
                on_adapter: iokit::get_bool(p, "ExternalConnected").unwrap_or(false),
                health: macos_health.unwrap_or_else(|| {
                    battery_health(iokit::get_i64(p, "AppleRawMaxCapacity"), Some(max), iokit::get_i64(p, "DesignCapacity"))
                }),
                cycles: iokit::get_i64(p, "CycleCount").unwrap_or(0).max(0) as u32,
                power_draw: power_watts(system_load, amperage, iokit::get_i64(p, "Voltage")),
            })
        }
    }

    /// (total, free) bytes of the startup volume, as Finder shows them: "free" includes purgeable
    /// space macOS frees on demand (`volumeAvailableCapacityForImportantUsage`). Falls back to statfs.
    pub fn volume_space() -> (u64, u64) {
        finder_volume_space().unwrap_or_else(statfs_volume_space)
    }

    fn finder_volume_space() -> Option<(u64, u64)> {
        use objc2_foundation::{
            NSArray, NSNumber, NSString, NSURLVolumeAvailableCapacityForImportantUsageKey, NSURLVolumeTotalCapacityKey,
            NSURL,
        };
        let url = NSURL::fileURLWithPath(&NSString::from_str("/"));
        // SAFETY: Foundation's constant keys, valid for the process lifetime.
        let (free_key, total_key) = unsafe { (NSURLVolumeAvailableCapacityForImportantUsageKey, NSURLVolumeTotalCapacityKey) };
        let values = url.resourceValuesForKeys_error(&NSArray::from_slice(&[free_key, total_key])).ok()?;
        let number = |key| -> Option<u64> {
            let n = values.objectForKey(key)?.downcast::<NSNumber>().ok()?.longLongValue();
            (n > 0).then_some(n as u64)
        };
        let (total, free) = (number(total_key)?, number(free_key)?);
        Some((total, free.min(total)))
    }

    fn statfs_volume_space() -> (u64, u64) {
        for path in ["/System/Volumes/Data\0", "/\0"] {
            let mut st = std::mem::MaybeUninit::<libc::statfs>::zeroed();
            // SAFETY: NUL-terminated path, valid out-pointer.
            if unsafe { libc::statfs(path.as_ptr().cast(), st.as_mut_ptr()) } == 0 {
                let st = unsafe { st.assume_init() };
                let bsize = st.f_bsize as u64;
                return (st.f_blocks * bsize, st.f_bavail * bsize);
            }
        }
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(n: &str, t: f64) -> (String, f64) {
        (n.into(), t)
    }

    #[test]
    fn picks_cpu_sensors() {
        let m1 = [s("pACC MTR Temp Sensor0", 50.0), s("eACC MTR Temp Sensor1", 40.0), s("GPU MTR Temp Sensor1", 70.0)];
        assert_eq!(cpu_temperature(&m1), Some(45.0));
        let m3 = [s("PMU tdie1", 60.0), s("PMU tdie2", 62.0), s("NAND CH0 temp", 30.0), s("PMU tdie3", -1.0)];
        assert_eq!(cpu_temperature(&m3), Some(61.0));
        assert_eq!(cpu_temperature(&[s("NAND CH0 temp", 30.0)]), None);
    }

    #[test]
    fn decodes_smc_values() {
        assert_eq!(smc_decode(b"sp78", &[0x3A, 0x80]), Some(58.5));
        assert_eq!(smc_decode(b"sp78", &[0xFF, 0x00]), Some(-1.0));
        assert_eq!(smc_decode(b"fpe2", &[0x00, 0x64]), Some(25.0));
        assert_eq!(smc_decode(b"flt ", &47.25f32.to_le_bytes()), Some(47.25));
        assert_eq!(smc_decode(b"ui8 ", &[7]), Some(7.0));
        assert_eq!(smc_decode(b"sp78", &[0x3A]), None);
        assert_eq!(smc_decode(b"ch8*", b"abcd"), None);
    }

    #[test]
    fn picks_smc_cpu_sensors() {
        let i9 = [s("TC1C", 60.0), s("TC2C", 64.0), s("TCXC", 90.0), s("TC0P", 50.0)];
        assert_eq!(smc_cpu_temperature(&i9), Some(62.0));
        let old = [s("TC0P", 50.0), s("TC0D", 70.0), s("TC1C", 0.0)];
        assert_eq!(smc_cpu_temperature(&old), Some(70.0));
        assert_eq!(smc_cpu_temperature(&[s("TCXC", 81.0), s("TC0P", 50.0)]), Some(81.0));
        assert_eq!(smc_cpu_temperature(&[s("TB0T", 30.0)]), None);
    }

    #[test]
    fn health() {
        assert!((battery_health(Some(4700), Some(100), Some(5000)) - 0.94).abs() < 1e-9);
        assert!((battery_health(None, Some(4500), Some(5000)) - 0.9).abs() < 1e-9);
        assert_eq!(battery_health(None, Some(100), Some(5000)), 1.0);
        assert_eq!(battery_health(Some(1), None, None), 1.0);
    }

    #[test]
    fn parses_system_profiler_health() {
        let json = r#"{"SPPowerDataType":[{"_name":"spbattery_information","sppower_battery_health_info":
            {"sppower_battery_cycle_count":433,"sppower_battery_health_maximum_capacity":"79%"}},
            {"_name":"sppower_information"}]}"#;
        assert_eq!(parse_max_capacity(json), Some(0.79));
        assert_eq!(parse_max_capacity(r#"{"SPPowerDataType":[{"_name":"sppower_information"}]}"#), None);
        assert_eq!(parse_max_capacity("not json"), None);
    }

    #[test]
    fn power() {
        assert_eq!(power_watts(Some(14_500), Some(-1000), Some(12_000)), 14.5);
        assert_eq!(power_watts(None, Some(-1000), Some(12_000)), 12.0);
        assert_eq!(power_watts(Some(0), None, None), 0.0);
    }
}
