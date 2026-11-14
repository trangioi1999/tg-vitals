//! Per-reader cost of one menu-bar-only tick. Run: cargo run --release -p vitals-core --example bench
fn main() {
    #[cfg(target_os = "macos")]
    {
        use std::time::Instant;
        use vitals_core::{cpu, memory, network, sampler::Sampler, sensors};
        let n = 50;
        let time = |name: &str, f: &mut dyn FnMut()| {
            f();
            let t = Instant::now();
            for _ in 0..n { f(); }
            eprintln!("{name:<22} {:>8.2} ms", t.elapsed().as_secs_f64() * 1000.0 / n as f64);
        };
        let mut s = sensors::Sensors::new();
        time("cpu ticks", &mut || { cpu::read_ticks(); });
        time("memory", &mut || { memory::read(); });
        time("network", &mut || { network::read_counters(); });
        time("volume_space (NSURL)", &mut || { sensors::volume_space(); });
        time("disk_io (IOKit)", &mut || { s.disk_io(); });
        time("battery (IOKit)", &mut || { s.battery(); });
        time("temperature (IOHID)", &mut || { s.temperature(); });
        time("Sensors::new", &mut || { sensors::Sensors::new(); });
        time("Sensors::new + 1 read", &mut || { sensors::Sensors::new().temperature(); });
        time("all sensors (debug)", &mut || { s.all_temperatures(); });
        let mut sampler = Sampler::new();
        time("full tick, no procs", &mut || { sampler.sample(false, &|b| b.to_string()); });
        time("full tick, procs", &mut || { sampler.sample(true, &|b| b.to_string()); });
    }
}
