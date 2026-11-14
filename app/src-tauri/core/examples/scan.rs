//! Real read-only cleanup scan with timing. Run: cargo run --release -p vitals-core --example scan
use std::time::Instant;
use vitals_core::care::scanner::Scanner;

fn main() {
    let home = std::env::var_os("HOME").map(Into::into).expect("HOME");
    let started = Instant::now();
    let mut step = Instant::now();
    let groups = Scanner::new(home).scan_all_with(&mut |e| {
        if let vitals_core::care::ScanEvent::StepDone { title, bytes, items } = e {
            eprintln!("{:>6.2}s  {title:<28} {:>8.2} GB  {items} mục", step.elapsed().as_secs_f64(), bytes as f64 / 1e9);
            step = Instant::now();
        }
    });
    eprintln!("total {:.1}s", started.elapsed().as_secs_f64());
    if let Some(g) = groups.iter().find(|g| g.id == "project-artifacts") {
        for i in g.items.iter().take(15) {
            eprintln!("  {:>7.2} GB  {}", i.size as f64 / 1e9, i.name);
        }
    }
}
