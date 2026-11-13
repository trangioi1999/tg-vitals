//! Memory breakdown matching Activity Monitor, plus swap and pressure.
// libc marks Mach calls deprecated in favour of the `mach2` crate; they are stable system APIs.
#![allow(deprecated)]
use crate::model::Pressure;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VmPages {
    pub wired: u64,
    pub purgeable: u64,
    pub compressor: u64,
    pub external: u64,
    pub internal: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Breakdown {
    pub app: u64,
    pub wired: u64,
    pub compressed: u64,
    pub cached: u64,
}

/// App = anonymous pages that can't be purged; Cached = file-backed + purgeable.
pub fn breakdown(p: VmPages, page_size: u64) -> Breakdown {
    Breakdown {
        app: p.internal.saturating_sub(p.purgeable) * page_size,
        wired: p.wired * page_size,
        compressed: p.compressor * page_size,
        cached: (p.external + p.purgeable) * page_size,
    }
}

/// `kern.memorystatus_vm_pressure_level`: 1 normal, 2 warning, 4 critical.
pub fn pressure_from_level(level: i32) -> Pressure {
    match level {
        4 => Pressure::Critical,
        2 => Pressure::Warning,
        _ => Pressure::Normal,
    }
}

#[cfg(target_os = "macos")]
pub fn read() -> Option<crate::model::MemoryInfo> {
    use crate::sys;
    use libc::{host_statistics64, mach_host_self, vm_statistics64, HOST_VM_INFO64, HOST_VM_INFO64_COUNT, KERN_SUCCESS};

    // SAFETY: zeroed POD struct; count tells the kernel how much space we have.
    let stats = unsafe {
        let mut stats: vm_statistics64 = std::mem::zeroed();
        let mut count = HOST_VM_INFO64_COUNT;
        let r = host_statistics64(mach_host_self(), HOST_VM_INFO64, &mut stats as *mut _ as *mut i32, &mut count);
        if r != KERN_SUCCESS {
            return None;
        }
        stats
    };
    let page_size = sys::sysctl_u64("vm.pagesize").filter(|&p| p > 0).unwrap_or(16_384);
    let b = breakdown(
        VmPages {
            wired: stats.wire_count as u64,
            purgeable: stats.purgeable_count as u64,
            compressor: stats.compressor_page_count as u64,
            external: stats.external_page_count as u64,
            internal: stats.internal_page_count as u64,
        },
        page_size,
    );
    Some(crate::model::MemoryInfo {
        total: sys::sysctl_u64("hw.memsize").unwrap_or(0),
        app: b.app,
        wired: b.wired,
        compressed: b.compressed,
        cached: b.cached,
        swap_used: sys::sysctl_value::<libc::xsw_usage>("vm.swapusage").map(|s| s.xsu_used).unwrap_or(0),
        pressure: pressure_from_level(sys::sysctl_value::<i32>("kern.memorystatus_vm_pressure_level").unwrap_or(1)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakdown_like_activity_monitor() {
        let b = breakdown(VmPages { wired: 10, purgeable: 5, compressor: 3, external: 20, internal: 105 }, 16_384);
        assert_eq!(b.app, 100 * 16_384);
        assert_eq!(b.wired, 10 * 16_384);
        assert_eq!(b.compressed, 3 * 16_384);
        assert_eq!(b.cached, 25 * 16_384);
    }

    #[test]
    fn purgeable_larger_than_internal_does_not_underflow() {
        assert_eq!(breakdown(VmPages { purgeable: 9, internal: 3, ..Default::default() }, 1).app, 0);
    }

    #[test]
    fn pressure_levels() {
        assert_eq!(pressure_from_level(1), Pressure::Normal);
        assert_eq!(pressure_from_level(2), Pressure::Warning);
        assert_eq!(pressure_from_level(4), Pressure::Critical);
        assert_eq!(pressure_from_level(0), Pressure::Normal);
    }
}
