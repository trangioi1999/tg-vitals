//! CPU load per core via `host_processor_info`. Needs two samples to produce usage.
// libc marks Mach calls deprecated in favour of the `mach2` crate; they are stable system APIs.
#![allow(deprecated)]

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuTicks {
    pub user: u64,
    pub system: u64,
    pub idle: u64,
    pub nice: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CpuUsage {
    pub user: f64,
    pub system: f64,
    /// Kernel order (on Apple silicon: efficiency cores first).
    pub per_core: Vec<f64>,
}

/// Counters are 32-bit in the kernel and may wrap; treat a decrease as a wrap.
pub fn tick_delta(old: u64, new: u64) -> u64 {
    if new >= old {
        new - old
    } else {
        (u32::MAX as u64 - old) + new + 1
    }
}

pub fn usage(old: &[CpuTicks], new: &[CpuTicks]) -> CpuUsage {
    let (mut user_sum, mut system_sum, mut total_sum) = (0.0, 0.0, 0.0);
    let per_core = old
        .iter()
        .zip(new)
        .map(|(o, n)| {
            let user = (tick_delta(o.user, n.user) + tick_delta(o.nice, n.nice)) as f64;
            let system = tick_delta(o.system, n.system) as f64;
            let idle = tick_delta(o.idle, n.idle) as f64;
            let total = user + system + idle;
            user_sum += user;
            system_sum += system;
            total_sum += total;
            if total > 0.0 { (user + system) / total } else { 0.0 }
        })
        .collect();
    if total_sum == 0.0 {
        return CpuUsage { user: 0.0, system: 0.0, per_core };
    }
    CpuUsage { user: user_sum / total_sum, system: system_sum / total_sum, per_core }
}

/// The kernel numbers efficiency cores first on Apple silicon; the UI wants
/// performance cores first.
pub fn performance_first(per_core: &[f64], e_cores: usize) -> Vec<f64> {
    if e_cores == 0 || e_cores >= per_core.len() {
        return per_core.to_vec();
    }
    per_core[e_cores..].iter().chain(&per_core[..e_cores]).copied().collect()
}

#[cfg(target_os = "macos")]
pub fn read_ticks() -> Option<Vec<CpuTicks>> {
    use libc::{
        host_processor_info, mach_host_self, natural_t, processor_info_array_t, vm_deallocate,
        CPU_STATE_IDLE, CPU_STATE_MAX, CPU_STATE_NICE, CPU_STATE_SYSTEM, CPU_STATE_USER,
        KERN_SUCCESS, PROCESSOR_CPU_LOAD_INFO,
    };
    let mut cpu_count: natural_t = 0;
    let mut info: processor_info_array_t = std::ptr::null_mut();
    let mut info_count = 0;
    // SAFETY: out-pointers are valid; the returned buffer is freed below.
    unsafe {
        let r = host_processor_info(mach_host_self(), PROCESSOR_CPU_LOAD_INFO, &mut cpu_count, &mut info, &mut info_count);
        if r != KERN_SUCCESS || info.is_null() {
            return None;
        }
        let stride = CPU_STATE_MAX as usize;
        let data = std::slice::from_raw_parts(info, info_count as usize);
        let tick = |core: usize, state: i32| data[core * stride + state as usize] as u32 as u64;
        let ticks = (0..cpu_count as usize)
            .filter(|c| (c + 1) * stride <= data.len())
            .map(|c| CpuTicks {
                user: tick(c, CPU_STATE_USER),
                system: tick(c, CPU_STATE_SYSTEM),
                idle: tick(c, CPU_STATE_IDLE),
                nice: tick(c, CPU_STATE_NICE),
            })
            .collect();
        #[allow(deprecated)]
        vm_deallocate(libc::mach_task_self(), info as usize, info_count as usize * std::mem::size_of::<i32>());
        Some(ticks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(user: u64, system: u64, idle: u64) -> CpuTicks {
        CpuTicks { user, system, idle, nice: 0 }
    }

    #[test]
    fn computes_user_system_and_per_core() {
        let u = usage(&[t(0, 0, 0), t(0, 0, 0)], &[t(50, 25, 25), t(0, 0, 100)]);
        assert!((u.user - 0.25).abs() < 1e-9);
        assert!((u.system - 0.125).abs() < 1e-9);
        assert_eq!(u.per_core, vec![0.75, 0.0]);
    }

    #[test]
    fn no_time_passed_is_zero() {
        let u = usage(&[t(5, 5, 5)], &[t(5, 5, 5)]);
        assert_eq!((u.user, u.system, u.per_core.clone()), (0.0, 0.0, vec![0.0]));
    }

    #[test]
    fn wraps_32_bit_counters() {
        assert_eq!(tick_delta(u32::MAX as u64 - 1, 3), 5);
        assert_eq!(tick_delta(10, 15), 5);
    }

    #[test]
    fn reorders_performance_cores_first() {
        assert_eq!(performance_first(&[0.1, 0.2, 0.9, 0.8], 2), vec![0.9, 0.8, 0.1, 0.2]);
        assert_eq!(performance_first(&[0.1, 0.2], 0), vec![0.1, 0.2]);
    }
}
