//! Process list via libproc. CPU% needs two samples. Memory is `phys_footprint`
//! (what Activity Monitor shows) when macOS lets us read it, else resident size.
// libc marks Mach calls deprecated in favour of the `mach2` crate; they are stable system APIs.
#![allow(deprecated)]
use crate::model::ProcessKind;

/// CPU used between two samples, as percent of one core (can exceed 100).
pub fn cpu_percent(previous_ns: Option<u64>, current_ns: u64, elapsed_ns: u64) -> f64 {
    match previous_ns {
        Some(prev) if elapsed_ns > 0 && current_ns >= prev => (current_ns - prev) as f64 / elapsed_ns as f64 * 100.0,
        _ => 0.0,
    }
}

/// Outermost `.app` bundle containing the executable, so helper processes
/// (e.g. "Google Chrome Helper") get their parent app's icon.
pub fn bundle_path(exe: &str) -> Option<&str> {
    let idx = exe.find(".app/")?;
    Some(&exe[..idx + 4])
}

const SYSTEM_PREFIXES: &[&str] = &["/System/", "/usr/libexec/", "/usr/sbin/", "/sbin/", "/Library/Apple/"];

pub fn classify(uid: u32, my_uid: u32, exe: &str) -> ProcessKind {
    if uid != my_uid {
        ProcessKind::System
    } else if bundle_path(exe).is_some() {
        ProcessKind::App
    } else if SYSTEM_PREFIXES.iter().any(|p| exe.starts_with(p)) {
        ProcessKind::System
    } else {
        ProcessKind::Mine
    }
}

/// Executable file name (full length, unlike `proc_name` which truncates), else the kernel name.
pub fn display_name(exe: &str, fallback: &str) -> String {
    exe.rsplit('/').next().filter(|n| !n.is_empty()).unwrap_or(fallback).to_string()
}

/// Mach absolute time → nanoseconds (on Apple silicon 1 tick = 125/3 ns).
pub fn mach_to_ns(ticks: u64, numer: u32, denom: u32) -> u64 {
    if denom == 0 {
        return ticks;
    }
    (ticks as u128 * numer as u128 / denom as u128) as u64
}

#[cfg(target_os = "macos")]
pub use mac::ProcessSampler;

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use crate::model::ProcessRow;
    use std::collections::HashMap;
    use std::ffi::CStr;
    use std::time::Instant;

    pub struct ProcessSampler {
        previous: HashMap<i32, u64>,
        previous_at: Option<Instant>,
        numer: u32,
        denom: u32,
        my_uid: u32,
        users: HashMap<u32, String>,
    }

    impl Default for ProcessSampler {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ProcessSampler {
        pub fn new() -> Self {
            let mut tb = libc::mach_timebase_info { numer: 0, denom: 0 };
            // SAFETY: valid out-pointer.
            unsafe { libc::mach_timebase_info(&mut tb) };
            Self {
                previous: HashMap::new(),
                previous_at: None,
                numer: tb.numer,
                denom: tb.denom,
                my_uid: unsafe { libc::getuid() },
                users: HashMap::new(),
            }
        }

        /// `icon_url` turns a bundle path into whatever URL the UI should load.
        pub fn sample(&mut self, icon_url: &dyn Fn(&str) -> String) -> Vec<ProcessRow> {
            let now = Instant::now();
            let elapsed = self.previous_at.map(|t| now.duration_since(t).as_nanos() as u64).unwrap_or(0);
            let mut next = HashMap::with_capacity(self.previous.len());
            let mut rows = Vec::with_capacity(self.previous.len());

            for pid in all_pids() {
                let Some(bsd) = bsd_info(pid) else { continue };
                let exe = path_of(pid);
                let kernel_name = cstr(&bsd.pbi_name).filter(|n| !n.is_empty()).or_else(|| cstr(&bsd.pbi_comm)).unwrap_or_default();
                let name = if pid == 0 { "kernel_task".to_string() } else { display_name(&exe, &kernel_name) };

                let task = task_info(pid);
                let usage = rusage(pid);
                let cpu_ns = task.map(|t| mach_to_ns(t.pti_total_user + t.pti_total_system, self.numer, self.denom));
                if let Some(ns) = cpu_ns {
                    next.insert(pid, ns);
                }
                let cpu = cpu_ns.map(|ns| cpu_percent(self.previous.get(&pid).copied(), ns, elapsed)).unwrap_or(0.0);
                let memory = usage.map(|u| u.ri_phys_footprint).or(task.map(|t| t.pti_resident_size)).unwrap_or(0);

                let uid = bsd.pbi_uid;
                let user = self.user_name(uid);
                rows.push(ProcessRow {
                    pid,
                    name,
                    cpu,
                    memory,
                    threads: task.map(|t| t.pti_threadnum.max(0) as u32).unwrap_or(0),
                    user,
                    kind: classify(uid, self.my_uid, &exe),
                    icon: bundle_path(&exe).map(icon_url),
                    cpu_time: cpu_ns.unwrap_or(0) as f64 / 1e9,
                    disk_read: usage.map(|u| u.ri_diskio_bytesread).unwrap_or(0),
                    disk_written: usage.map(|u| u.ri_diskio_byteswritten).unwrap_or(0),
                    started_at: bsd.pbi_start_tvsec,
                    path: exe,
                });
            }

            self.previous = next;
            self.previous_at = Some(now);
            rows
        }

        fn user_name(&mut self, uid: u32) -> String {
            self.users.entry(uid).or_insert_with(|| user_name(uid)).clone()
        }
    }

    fn all_pids() -> Vec<i32> {
        // SAFETY: first call with null returns the count; buffer gets slack for new processes.
        unsafe {
            let n = libc::proc_listallpids(std::ptr::null_mut(), 0);
            if n <= 0 {
                return Vec::new();
            }
            let mut pids = vec![0i32; n as usize + 64];
            let bytes = (pids.len() * std::mem::size_of::<i32>()) as i32;
            let count = libc::proc_listallpids(pids.as_mut_ptr().cast(), bytes);
            if count <= 0 {
                return Vec::new();
            }
            pids.truncate(count as usize);
            pids
        }
    }

    fn bsd_info(pid: i32) -> Option<libc::proc_bsdinfo> {
        pidinfo(pid, libc::PROC_PIDTBSDINFO)
    }

    fn task_info(pid: i32) -> Option<libc::proc_taskinfo> {
        pidinfo(pid, libc::PROC_PIDTASKINFO)
    }

    fn pidinfo<T>(pid: i32, flavor: i32) -> Option<T> {
        let size = std::mem::size_of::<T>() as i32;
        let mut value = std::mem::MaybeUninit::<T>::zeroed();
        // SAFETY: buffer is exactly `size` bytes; accept only a full answer.
        let r = unsafe { libc::proc_pidinfo(pid, flavor, 0, value.as_mut_ptr().cast(), size) };
        (r == size).then(|| unsafe { value.assume_init() })
    }

    fn rusage(pid: i32) -> Option<libc::rusage_info_v4> {
        let mut info = std::mem::MaybeUninit::<libc::rusage_info_v4>::zeroed();
        // SAFETY: proc_pid_rusage writes a rusage_info_v4 for flavor V4.
        let r = unsafe { libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V4, info.as_mut_ptr().cast()) };
        (r == 0).then(|| unsafe { info.assume_init() })
    }

    fn path_of(pid: i32) -> String {
        let mut buf = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
        // SAFETY: buffer size passed matches allocation.
        let n = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
        if n <= 0 {
            return String::new();
        }
        String::from_utf8_lossy(&buf[..n as usize]).into_owned()
    }

    fn cstr(chars: &[libc::c_char]) -> Option<String> {
        let bytes: Vec<u8> = chars.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn user_name(uid: u32) -> String {
        let mut pwd = std::mem::MaybeUninit::<libc::passwd>::zeroed();
        let mut buf = vec![0 as libc::c_char; 4096];
        let mut result: *mut libc::passwd = std::ptr::null_mut();
        // SAFETY: getpwuid_r with caller-owned buffers.
        unsafe {
            if libc::getpwuid_r(uid, pwd.as_mut_ptr(), buf.as_mut_ptr(), buf.len(), &mut result) == 0 && !result.is_null() {
                if let Ok(name) = CStr::from_ptr((*result).pw_name).to_str() {
                    return name.to_string();
                }
            }
        }
        uid.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_percent_of_one_core() {
        assert_eq!(cpu_percent(Some(0), 500_000_000, 1_000_000_000), 50.0);
        assert_eq!(cpu_percent(Some(0), 2_000_000_000, 1_000_000_000), 200.0);
        assert_eq!(cpu_percent(None, 5, 10), 0.0);
        assert_eq!(cpu_percent(Some(10), 5, 10), 0.0, "pid reused → no negative");
        assert_eq!(cpu_percent(Some(0), 5, 0), 0.0);
    }

    #[test]
    fn finds_outermost_bundle() {
        assert_eq!(bundle_path("/Applications/Slack.app/Contents/MacOS/Slack"), Some("/Applications/Slack.app"));
        assert_eq!(
            bundle_path("/Applications/Google Chrome.app/Contents/Frameworks/X.framework/Helpers/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper"),
            Some("/Applications/Google Chrome.app")
        );
        assert_eq!(bundle_path("/usr/local/bin/node"), None);
    }

    #[test]
    fn classifies_processes() {
        assert_eq!(classify(0, 501, "/usr/libexec/logd"), ProcessKind::System);
        assert_eq!(classify(501, 501, "/Applications/Slack.app/Contents/MacOS/Slack"), ProcessKind::App);
        assert_eq!(classify(501, 501, "/System/Library/CoreServices/Finder.app/Contents/MacOS/Finder"), ProcessKind::App);
        assert_eq!(classify(501, 501, "/usr/libexec/trustd"), ProcessKind::System);
        assert_eq!(classify(501, 501, "/opt/homebrew/bin/node"), ProcessKind::Mine);
        assert_eq!(classify(501, 501, "/bin/zsh"), ProcessKind::Mine);
    }

    #[test]
    fn names_and_timebase() {
        assert_eq!(display_name("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "Google Chrom"), "Google Chrome");
        assert_eq!(display_name("", "launchd"), "launchd");
        assert_eq!(mach_to_ns(3, 125, 3), 125);
        assert_eq!(mach_to_ns(7, 1, 1), 7);
        assert_eq!(mach_to_ns(7, 1, 0), 7);
    }
}
