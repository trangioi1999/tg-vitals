//! Installed apps for the "Gỡ app" page: bundle id, size, owner, last use, running.
//! Read-only. Uninstalling goes through `care::trasher::recycle_with_auth`.
use serde::Serialize;

/// One installed app, as the UI shows it.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub path: String,
    pub name: String,
    pub bundle_id: String,
    pub version: String,
    /// Allocated size of the `.app` bundle.
    pub size: u64,
    /// False → moving it to the Trash needs the admin password (Finder asks).
    pub owned_by_user: bool,
    /// Unix seconds, from Spotlight's `kMDItemLastUsedDate`.
    pub last_used: Option<u64>,
    pub running: bool,
    /// Homebrew cask token when the app was installed with `brew install --cask`.
    pub brew_cask: Option<String>,
    pub team_id: Option<String>,
}

impl AppInfo {
    /// What the leftover matcher needs to know about this app.
    pub fn identity(&self) -> super::apps::AppIdentity {
        super::apps::AppIdentity { bundle_id: self.bundle_id.clone(), name: self.name.clone(), team_id: self.team_id.clone() }
    }
}

/// Apple's own apps and tg-vitals itself are never offered for removal.
pub fn is_offerable(bundle_id: &str) -> bool {
    !bundle_id.is_empty() && !bundle_id.starts_with("com.apple.") && bundle_id != "com.trangioi.tgvitals"
}

/// `TeamIdentifier=XXXX` from `codesign -dv --verbose=2` (it prints to stderr).
pub fn parse_team_id(codesign_output: &str) -> Option<String> {
    codesign_output
        .lines()
        .find_map(|l| l.strip_prefix("TeamIdentifier="))
        .map(str::trim)
        .filter(|t| !t.is_empty() && *t != "not set")
        .map(str::to_owned)
}

/// `mdls -raw` date, e.g. `2026-05-03 02:51:38 +0000` (always UTC) → unix seconds.
pub fn parse_last_used(raw: &str) -> Option<u64> {
    let raw = raw.trim();
    let (date, rest) = raw.split_once(' ')?;
    let time = rest.split_whitespace().next()?;
    let d: Vec<i64> = date.split('-').map(|x| x.parse().ok()).collect::<Option<_>>()?;
    let t: Vec<i64> = time.split(':').map(|x| x.parse().ok()).collect::<Option<_>>()?;
    let ([y, m, day], [hh, mm, ss]) = (d.as_slice(), t.as_slice()) else { return None };
    // Days from civil (Howard Hinnant), proleptic Gregorian.
    let (y, m) = if *m <= 2 { (y - 1, m + 9) } else { (*y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hh * 3600 + mm * 60 + ss).ok()
}

/// Homebrew cask token for an app bundle name: "Visual Studio Code.app" → "visual-studio-code".
pub fn cask_token(app_file_name: &str) -> String {
    app_file_name.trim_end_matches(".app").to_lowercase().replace(' ', "-")
}

#[cfg(target_os = "macos")]
pub use mac::{list_apps, list_apps_with};

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use crate::care::scanner::{dir_size, owned_by_user};
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// Every offerable app directly inside `roots`, biggest first.
    pub fn list_apps(roots: &[PathBuf]) -> Vec<AppInfo> {
        list_apps_with(roots, &mut |_| {})
    }

    /// Same as `list_apps`, calling `on_app(path)` before reading each app.
    pub fn list_apps_with(roots: &[PathBuf], on_app: &mut dyn FnMut(&str)) -> Vec<AppInfo> {
        let running = crate::process::running_bundles();
        let casks = cask_tokens();
        let mut apps = Vec::new();
        for root in roots {
            let Ok(entries) = std::fs::read_dir(root) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                let is_app = path.extension().is_some_and(|e| e == "app");
                // file_type() does not follow symlinks: a symlinked app is not ours to remove.
                if !is_app || !entry.file_type().is_ok_and(|t| t.is_dir()) {
                    continue;
                }
                let path_str = path.to_string_lossy().into_owned();
                on_app(&path_str);
                let Some(info) = read_info_plist(&path) else { continue };
                let bundle_id = info.get("CFBundleIdentifier").and_then(|v| v.as_str()).unwrap_or_default().to_owned();
                if !is_offerable(&bundle_id) {
                    continue;
                }
                let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let name = ["CFBundleDisplayName", "CFBundleName"]
                    .iter()
                    .find_map(|k| info.get(*k).and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
                    .map(str::to_owned)
                    .unwrap_or_else(|| file_name.trim_end_matches(".app").to_owned());
                let token = cask_token(&file_name);
                apps.push(AppInfo {
                    name,
                    version: info.get("CFBundleShortVersionString").and_then(|v| v.as_str()).unwrap_or_default().to_owned(),
                    size: dir_size(&path),
                    owned_by_user: owned_by_user(&path),
                    last_used: last_used(&path),
                    running: running.contains(&path_str),
                    brew_cask: casks.contains(&token).then_some(token),
                    team_id: team_id(&path),
                    bundle_id,
                    path: path_str,
                });
            }
        }
        apps.sort_by_key(|a| std::cmp::Reverse(a.size));
        apps
    }

    /// Info.plist as JSON (`plutil` reads both XML and binary plists).
    fn read_info_plist(app: &Path) -> Option<serde_json::Map<String, serde_json::Value>> {
        let plist = app.join("Contents/Info.plist");
        let out = Command::new("/usr/bin/plutil").args(["-convert", "json", "-o", "-"]).arg(&plist).output().ok()?;
        if !out.status.success() {
            return None;
        }
        // Dates/data make plutil refuse JSON for a few plists; those apps are skipped.
        serde_json::from_slice::<serde_json::Value>(&out.stdout).ok()?.as_object().cloned()
    }

    fn last_used(app: &Path) -> Option<u64> {
        let out = Command::new("/usr/bin/mdls").args(["-raw", "-name", "kMDItemLastUsedDate"]).arg(app).output().ok()?;
        parse_last_used(&String::from_utf8_lossy(&out.stdout))
    }

    fn team_id(app: &Path) -> Option<String> {
        let out = Command::new("/usr/bin/codesign").args(["-dv", "--verbose=2"]).arg(app).output().ok()?;
        parse_team_id(&String::from_utf8_lossy(&out.stderr))
    }

    fn cask_tokens() -> std::collections::HashSet<String> {
        ["/opt/homebrew/Caskroom", "/usr/local/Caskroom"]
            .iter()
            .filter_map(|d| std::fs::read_dir(d).ok())
            .flat_map(|it| it.flatten().map(|e| e.file_name().to_string_lossy().into_owned()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offerable_filters_apple_and_self() {
        assert!(!is_offerable("com.apple.Safari"));
        assert!(!is_offerable("com.trangioi.tgvitals"));
        assert!(!is_offerable(""));
        assert!(is_offerable("com.microsoft.Word"));
    }

    #[test]
    fn parses_team_id() {
        let signed = "Executable=/Applications/Microsoft Word.app/Contents/MacOS/Microsoft Word\nIdentifier=com.microsoft.Word\nTeamIdentifier=UBF8T346G9\nSealed Resources version=2";
        assert_eq!(parse_team_id(signed).as_deref(), Some("UBF8T346G9"));
        assert_eq!(parse_team_id("Identifier=x\nTeamIdentifier=not set\n"), None);
        assert_eq!(parse_team_id("code object is not signed at all"), None);
    }

    #[test]
    fn parses_last_used() {
        assert_eq!(parse_last_used("2026-05-03 02:51:38 +0000"), Some(1_777_776_698));
        assert_eq!(parse_last_used("(null)"), None);
        assert_eq!(parse_last_used(""), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "reads the real /Applications"]
    fn reads_real_applications() {
        let apps = list_apps(&[std::path::PathBuf::from("/Applications")]);
        for a in &apps {
            eprintln!("{:>7.2} GB  {:<28} {:<36} user={} run={} team={:?} cask={:?} used={:?}",
                a.size as f64 / 1e9, a.name, a.bundle_id, a.owned_by_user, a.running, a.team_id, a.brew_cask, a.last_used);
        }
        assert!(!apps.is_empty());
        assert!(apps.iter().all(|a| !a.bundle_id.is_empty() && a.size > 0));
        assert!(apps.iter().all(|a| is_offerable(&a.bundle_id)));
    }

    #[test]
    fn identity_carries_matcher_fields() {
        let a = AppInfo { path: "/Applications/X.app".into(), name: "X".into(), bundle_id: "com.x".into(),
            version: "1".into(), size: 1, owned_by_user: true, last_used: None, running: false,
            brew_cask: None, team_id: Some("T".into()) };
        assert_eq!(a.identity(), crate::care::apps::AppIdentity { bundle_id: "com.x".into(), name: "X".into(), team_id: Some("T".into()) });
    }

    #[test]
    fn cask_token_from_app_name() {
        assert_eq!(cask_token("DBeaver.app"), "dbeaver");
        assert_eq!(cask_token("Visual Studio Code.app"), "visual-studio-code");
    }
}
