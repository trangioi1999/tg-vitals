//! Machine name, chip, uptime.

/// Best-effort marketing name from `hw.model` ("MacBookPro18,3" → "MacBook Pro").
/// Newer models report "Mac15,6"; the sampler upgrades the name from system_profiler later.
pub fn family_from_model(model: &str) -> String {
    let family: String = model.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    match family.as_str() {
        "MacBookPro" => "MacBook Pro",
        "MacBookAir" => "MacBook Air",
        "MacBook" => "MacBook",
        "Macmini" => "Mac mini",
        "MacPro" => "Mac Pro",
        "iMac" => "iMac",
        "iMacPro" => "iMac Pro",
        _ => "Mac",
    }
    .to_string()
}

/// Pulls `machine_name` out of `system_profiler SPHardwareDataType -json`.
pub fn machine_name_from_profiler(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let name = v["SPHardwareDataType"][0]["machine_name"].as_str()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

#[cfg(target_os = "macos")]
pub fn chip() -> String {
    crate::sys::sysctl_string("machdep.cpu.brand_string").unwrap_or_else(|| "Apple silicon".into())
}

#[cfg(target_os = "macos")]
pub fn hw_model() -> String {
    crate::sys::sysctl_string("hw.model").unwrap_or_default()
}

/// Slow (~0.5 s): call once, off the sampling thread.
#[cfg(target_os = "macos")]
pub fn marketing_name() -> Option<String> {
    let out = std::process::Command::new("/usr/sbin/system_profiler")
        .args(["SPHardwareDataType", "-json"])
        .output()
        .ok()?;
    machine_name_from_profiler(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(target_os = "macos")]
pub fn uptime_seconds() -> u64 {
    let boot = crate::sys::sysctl_value::<libc::timeval>("kern.boottime").map(|t| t.tv_sec as u64).unwrap_or(0);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    now.saturating_sub(boot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families() {
        assert_eq!(family_from_model("MacBookPro18,3"), "MacBook Pro");
        assert_eq!(family_from_model("MacBookAir10,1"), "MacBook Air");
        assert_eq!(family_from_model("Macmini9,1"), "Mac mini");
        assert_eq!(family_from_model("Mac15,6"), "Mac");
        assert_eq!(family_from_model(""), "Mac");
    }

    #[test]
    fn profiler_json() {
        let json = r#"{"SPHardwareDataType":[{"machine_name":"MacBook Pro","chip_type":"Apple M3 Pro"}]}"#;
        assert_eq!(machine_name_from_profiler(json).as_deref(), Some("MacBook Pro"));
        assert_eq!(machine_name_from_profiler("nope"), None);
    }
}
