//! Short human strings for the menu bar title (the UI formats its own numbers).

const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];

/// Compact byte rate for the menu bar, e.g. `1.2M`, `340K`, `0K`.
pub fn compact_bytes(value: f64) -> String {
    let mut v = value.max(0.0);
    let mut unit = 0;
    while v >= 1000.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        return "0K".into();
    }
    if v < 10.0 {
        format!("{v:.1}{}", UNITS[unit])
    } else {
        format!("{v:.0}{}", UNITS[unit])
    }
}

/// How worrying a menu bar value is; the tray colors values by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Load {
    Normal,
    High,
    Critical,
}

impl Load {
    fn from(value: f64, high: f64, critical: f64) -> Self {
        if value >= critical {
            Load::Critical
        } else if value >= high {
            Load::High
        } else {
            Load::Normal
        }
    }

    /// CPU or RAM usage, 0..1.
    pub fn cpu(fraction: f64) -> Self {
        Self::from(fraction, 0.60, 0.85)
    }

    pub fn memory(fraction: f64) -> Self {
        Self::from(fraction, 0.75, 0.90)
    }

    /// CPU temperature, °C.
    pub fn temperature(celsius: f64) -> Self {
        Self::from(celsius, 70.0, 85.0)
    }
}

/// Percent-encodes a filesystem path for use in a URL path (keeps `/`).
pub fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Inverse of [`encode_path`]. Invalid escapes are kept literally.
pub fn decode_path(encoded: &str) -> String {
    let bytes = encoded.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_levels() {
        assert_eq!(Load::cpu(0.23), Load::Normal);
        assert_eq!(Load::cpu(0.60), Load::High);
        assert_eq!(Load::cpu(0.90), Load::Critical);
        assert_eq!(Load::memory(0.61), Load::Normal);
        assert_eq!(Load::memory(0.80), Load::High);
        assert_eq!(Load::memory(0.95), Load::Critical);
        assert_eq!(Load::temperature(44.0), Load::Normal);
        assert_eq!(Load::temperature(75.0), Load::High);
        assert_eq!(Load::temperature(92.0), Load::Critical);
    }

    #[test]
    fn compact() {
        assert_eq!(compact_bytes(0.0), "0K");
        assert_eq!(compact_bytes(512.0), "0K");
        assert_eq!(compact_bytes(1536.0), "1.5K");
        assert_eq!(compact_bytes(340.0 * 1024.0), "340K");
        assert_eq!(compact_bytes(1.2 * 1024.0 * 1024.0), "1.2M");
        assert_eq!(compact_bytes(-5.0), "0K");
    }

    #[test]
    fn path_round_trip() {
        let p = "/Applications/Google Chrome.app";
        assert_eq!(encode_path(p), "/Applications/Google%20Chrome.app");
        assert_eq!(decode_path(&encode_path(p)), p);
        let vi = "/Users/a/Ứng dụng.app";
        assert_eq!(decode_path(&encode_path(vi)), vi);
        assert_eq!(decode_path("/a%2"), "/a%2");
        assert_eq!(decode_path("/a%zz"), "/a%zz");
        assert_eq!(decode_path("/%Ứ"), "/%Ứ");
    }
}
