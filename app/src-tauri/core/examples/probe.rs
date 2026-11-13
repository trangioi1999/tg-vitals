//! Step-by-step check of every macOS reader; prints before each call so a crash names its source.
//! Run: cargo run -p vitals-core --example probe
fn main() {
    #[cfg(target_os = "macos")]
    mac::run();
}

#[cfg(target_os = "macos")]
mod mac {
    use std::io::Write;
    use vitals_core::{cpu, machine, memory, network, process, sampler::Sampler, sensors, sys};

    fn step(name: &str) {
        eprintln!("→ {name}");
        let _ = std::io::stderr().flush();
    }

    pub fn run() {
        step("sysctl perflevels");
        eprintln!("  nperflevels={:?} p={:?} e={:?}", sys::sysctl_u64("hw.nperflevels"),
            sys::sysctl_u64("hw.perflevel0.logicalcpu"), sys::sysctl_u64("hw.perflevel1.logicalcpu"));
        step("machine");
        eprintln!("  model={} chip={} uptime={}", machine::hw_model(), machine::chip(), machine::uptime_seconds());
        step("cpu ticks");
        eprintln!("  cores={:?}", cpu::read_ticks().map(|t| t.len()));
        step("memory");
        eprintln!("  {:?}", memory::read());
        step("network");
        eprintln!("  {:?}", network::read_counters());
        step("volume");
        eprintln!("  {:?}", sensors::volume_space());
        step("disk io");
        let mut s = sensors::Sensors::new_without_thermal();
        eprintln!("  {:?}", s.disk_io());
        step("battery");
        eprintln!("  {:?}", s.battery());
        step("vm check");
        eprintln!("  hv_vmm_present={:?}", sys::sysctl_value::<i32>("kern.hv_vmm_present"));
        step("thermal client");
        let t = sensors::Sensors::new();
        step("thermal read");
        eprintln!("  {:?}", t.all_temperatures());
        step("processes");
        let mut p = process::ProcessSampler::new();
        eprintln!("  {} rows", p.sample(&|b| b.to_string()).len());
        step("full sampler");
        let mut sampler = Sampler::new();
        let snap = sampler.sample(true, &|b| b.to_string());
        eprintln!("  ok: {} processes, temp {:?}", snap.processes.len(), snap.cpu.temperature);
        step("done");
    }
}
