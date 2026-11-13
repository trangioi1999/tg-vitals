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

/// Max capacity relative to design. Apple silicon reports `MaxCapacity` as a percent (100)
/// and the real mAh in `AppleRawMaxCapacity`.
pub fn battery_health(raw_max: Option<i64>, max: Option<i64>, design: Option<i64>) -> f64 {
    let Some(design) = design.filter(|&d| d > 0) else { return 1.0 };
    let max = raw_max.filter(|&m| m > 0).or(max.filter(|&m| m > 100));
    max.map(|m| (m as f64 / design as f64).clamp(0.0, 1.0)).unwrap_or(1.0)
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
    use crate::iokit::{self, HidThermal, Service};
    use crate::model::BatteryInfo;
    use std::time::Instant;

    pub struct Sensors {
        thermal: Option<HidThermal>,
        disk_previous: Option<(u64, u64, Instant)>,
        has_battery: bool,
    }

    impl Default for Sensors {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Sensors {
        pub fn new() -> Self {
            // Intel Macs expose temperatures through SMC, not these IOHID sensors.
            let thermal = if cfg!(target_arch = "aarch64") { HidThermal::new() } else { None };
            Self { thermal, ..Self::new_without_thermal() }
        }

        pub fn new_without_thermal() -> Self {
            Self { thermal: None, disk_previous: None, has_battery: true }
        }

        pub fn temperature(&self) -> Option<f64> {
            cpu_temperature(&self.thermal.as_ref()?.read())
        }

        /// All temperature sensors, for debugging new chips.
        pub fn all_temperatures(&self) -> Vec<(String, f64)> {
            self.thermal.as_ref().map(|t| t.read()).unwrap_or_default()
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

        pub fn battery(&mut self) -> Option<BatteryInfo> {
            if !self.has_battery {
                return None;
            }
            let Some(service) = Service::all("AppleSmartBattery").into_iter().next() else {
                self.has_battery = false;
                return None;
            };
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
                health: battery_health(
                    iokit::get_i64(p, "AppleRawMaxCapacity"),
                    Some(max),
                    iokit::get_i64(p, "DesignCapacity"),
                ),
                cycles: iokit::get_i64(p, "CycleCount").unwrap_or(0).max(0) as u32,
                power_draw: power_watts(system_load, amperage, iokit::get_i64(p, "Voltage")),
            })
        }
    }

    /// (total, free) bytes of the startup volume's data partition.
    pub fn volume_space() -> (u64, u64) {
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
    fn health() {
        assert!((battery_health(Some(4700), Some(100), Some(5000)) - 0.94).abs() < 1e-9);
        assert!((battery_health(None, Some(4500), Some(5000)) - 0.9).abs() < 1e-9);
        assert_eq!(battery_health(None, Some(100), Some(5000)), 1.0);
        assert_eq!(battery_health(Some(1), None, None), 1.0);
    }

    #[test]
    fn power() {
        assert_eq!(power_watts(Some(14_500), Some(-1000), Some(12_000)), 14.5);
        assert_eq!(power_watts(None, Some(-1000), Some(12_000)), 12.0);
        assert_eq!(power_watts(Some(0), None, None), 0.0);
    }
}
