//! Small `sysctlbyname` helpers.
use std::ffi::CString;

/// Reads a fixed-size value. Returns None if the name is unknown or the size differs.
pub fn sysctl_value<T: Copy>(name: &str) -> Option<T> {
    let name = CString::new(name).ok()?;
    let mut value = std::mem::MaybeUninit::<T>::zeroed();
    let mut len = std::mem::size_of::<T>();
    // SAFETY: buffer is exactly `len` bytes; we only accept a full-size answer.
    let r = unsafe { libc::sysctlbyname(name.as_ptr(), value.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0) };
    (r == 0 && len == std::mem::size_of::<T>()).then(|| unsafe { value.assume_init() })
}

/// Integer sysctls come as 4 or 8 bytes depending on the name.
pub fn sysctl_u64(name: &str) -> Option<u64> {
    let bytes = sysctl_bytes(name)?;
    match bytes.len() {
        4 => Some(u32::from_ne_bytes(bytes[..4].try_into().ok()?) as u64),
        8 => Some(u64::from_ne_bytes(bytes[..8].try_into().ok()?)),
        _ => None,
    }
}

pub fn sysctl_string(name: &str) -> Option<String> {
    let bytes = sysctl_bytes(name)?;
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    Some(String::from_utf8_lossy(&bytes[..end]).trim().to_string())
}

pub fn sysctl_bytes(name: &str) -> Option<Vec<u8>> {
    let cname = CString::new(name).ok()?;
    let mut len = 0usize;
    // SAFETY: two-step sysctl, buffer sized from the first call.
    unsafe {
        if libc::sysctlbyname(cname.as_ptr(), std::ptr::null_mut(), &mut len, std::ptr::null_mut(), 0) != 0 || len == 0 {
            return None;
        }
        let mut buf = vec![0u8; len];
        if libc::sysctlbyname(cname.as_ptr(), buf.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0) != 0 {
            return None;
        }
        buf.truncate(len);
        Some(buf)
    }
}
