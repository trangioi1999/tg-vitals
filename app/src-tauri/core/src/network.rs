//! Total network counters across physical interfaces via `sysctl(NET_RT_IFLIST2)`
//! (64-bit `if_data64`, no wrap at 4 GB like `getifaddrs`).

/// Loopback, VPN tunnels, AirDrop and bridges would double-count traffic.
pub fn counts_interface(name: &str) -> bool {
    const SKIP: &[&str] = &["lo", "utun", "awdl", "llw", "bridge", "gif", "stf", "anpi", "ipsec", "ap"];
    !SKIP.iter().any(|p| name.starts_with(p))
}

/// Bytes per second between two counter readings. A counter going backwards
/// (interface disappeared) reports 0 for that tick instead of a bogus spike.
pub fn rate(old: u64, new: u64, seconds: f64) -> f64 {
    if seconds <= 0.0 || new < old {
        return 0.0;
    }
    (new - old) as f64 / seconds
}

#[cfg(target_os = "macos")]
pub fn read_counters() -> Option<(u64, u64)> {
    use libc::{c_int, if_msghdr2, sysctl, CTL_NET, NET_RT_IFLIST2, PF_ROUTE, RTM_IFINFO2};
    let mut mib: [c_int; 6] = [CTL_NET, PF_ROUTE, 0, 0, NET_RT_IFLIST2, 0];
    let mut len: usize = 0;
    // SAFETY: standard two-step sysctl; buffer sized from the first call (+ slack).
    unsafe {
        if sysctl(mib.as_mut_ptr(), 6, std::ptr::null_mut(), &mut len, std::ptr::null_mut(), 0) != 0 {
            return None;
        }
        len += 4096;
        let mut buf = vec![0u8; len];
        if sysctl(mib.as_mut_ptr(), 6, buf.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0) != 0 {
            return None;
        }
        let (mut bytes_in, mut bytes_out) = (0u64, 0u64);
        let mut offset = 0usize;
        let mut name = [0 as libc::c_char; libc::IF_NAMESIZE];
        while offset + std::mem::size_of::<libc::if_msghdr>() <= len {
            let header = buf.as_ptr().add(offset).cast::<libc::if_msghdr>().read_unaligned();
            let msg_len = header.ifm_msglen as usize;
            if msg_len == 0 {
                break;
            }
            if header.ifm_type as c_int == RTM_IFINFO2 && offset + std::mem::size_of::<if_msghdr2>() <= len {
                let m = buf.as_ptr().add(offset).cast::<if_msghdr2>().read_unaligned();
                if !libc::if_indextoname(m.ifm_index as u32, name.as_mut_ptr()).is_null() {
                    let n = std::ffi::CStr::from_ptr(name.as_ptr()).to_string_lossy();
                    if counts_interface(&n) {
                        bytes_in += m.ifm_data.ifi_ibytes;
                        bytes_out += m.ifm_data.ifi_obytes;
                    }
                }
            }
            offset += msg_len;
        }
        Some((bytes_in, bytes_out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_virtual_interfaces() {
        assert!(counts_interface("en0"));
        assert!(counts_interface("en7"));
        for n in ["lo0", "utun3", "awdl0", "llw0", "bridge0", "gif0", "stf0", "anpi1", "ap1"] {
            assert!(!counts_interface(n), "{n}");
        }
    }

    #[test]
    fn rates() {
        assert_eq!(rate(100, 300, 2.0), 100.0);
        assert_eq!(rate(300, 100, 1.0), 0.0);
        assert_eq!(rate(1, 2, 0.0), 0.0);
    }
}
