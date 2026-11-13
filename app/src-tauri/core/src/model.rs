//! Shapes shared with the frontend (`app/src/api/types.ts`). Keep both in sync.
use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    /// 0..1
    pub total: f64,
    pub user: f64,
    pub system: f64,
    /// 0..1 per core, performance cores first.
    pub per_core: Vec<f64>,
    pub p_cores: usize,
    /// °C, average of the CPU die sensors. None when no sensor could be read.
    pub temperature: Option<f64>,
}

#[derive(Serialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Pressure {
    #[default]
    Normal,
    Warning,
    Critical,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub total: u64,
    pub app: u64,
    pub wired: u64,
    pub compressed: u64,
    pub cached: u64,
    pub swap_used: u64,
    pub pressure: Pressure,
}

impl MemoryInfo {
    pub fn used(&self) -> u64 {
        self.app + self.wired + self.compressed
    }
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub total: u64,
    pub free: u64,
    pub read_per_sec: f64,
    pub write_per_sec: f64,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub in_per_sec: f64,
    pub out_per_sec: f64,
    pub total_in: u64,
    pub total_out: u64,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInfo {
    /// 0..1
    pub percent: f64,
    pub charging: bool,
    pub on_adapter: bool,
    /// Max capacity / design capacity, 0..1
    pub health: f64,
    pub cycles: u32,
    /// Whole-machine power in watts.
    pub power_draw: f64,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessKind {
    App,
    System,
    Mine,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRow {
    pub pid: i32,
    pub name: String,
    /// Percent of one core, like Activity Monitor (can exceed 100).
    pub cpu: f64,
    pub memory: u64,
    pub threads: u32,
    pub user: String,
    pub kind: ProcessKind,
    /// URL of the app icon (served by the app's `appicon://` scheme), None for plain executables.
    pub icon: Option<String>,
    pub cpu_time: f64,
    pub disk_read: u64,
    pub disk_written: u64,
    pub started_at: u64,
    /// Full executable path, empty when macOS refuses to tell us.
    pub path: String,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Machine {
    pub model: String,
    pub chip: String,
    pub uptime: u64,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// unix ms
    pub time: u64,
    pub machine: Machine,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disk: DiskInfo,
    pub network: NetworkInfo,
    pub battery: Option<BatteryInfo>,
    pub processes: Vec<ProcessRow>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_with_frontend_field_names() {
        let s = Snapshot::default();
        let json = serde_json::to_value(&s).unwrap();
        assert!(json["cpu"].get("perCore").is_some());
        assert!(json["cpu"].get("pCores").is_some());
        assert!(json["memory"].get("swapUsed").is_some());
        assert_eq!(json["memory"]["pressure"], "normal");
        assert!(json["disk"].get("readPerSec").is_some());
        assert!(json["network"].get("totalIn").is_some());
        assert!(json["battery"].is_null());
        assert_eq!(serde_json::to_value(ProcessKind::Mine).unwrap(), "mine");
    }
}
