//! Portable, read-only matching and discovery of an app's leftover data.
use super::scanner;
use serde::Serialize;
use std::{fs, path::{Path, PathBuf}};

#[derive(Clone, Debug, PartialEq)]
pub struct AppIdentity {
    pub bundle_id: String,
    pub name: String,
    pub team_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    ById,
    ByIdOrName,
    GroupContainers,
    PlistById,
    SavedState,
    ByHost,
    LaunchPlist,
}

/// `others` contains the apps that will remain installed.
pub fn matches_leftover(app: &AppIdentity, others: &[AppIdentity], location: Location, entry: &str) -> bool {
    let id = app.bundle_id.to_lowercase();
    // Another installed copy with the same bundle id shares all of this data.
    if others.iter().any(|o| o.bundle_id.to_lowercase() == id) {
        return false;
    }
    let entry = entry.to_lowercase();
    // An installed app whose id extends ours (com.x.Editor.Pro vs com.x.Editor) owns its own entries.
    let claimed = others.iter().any(|o| {
        let o = o.bundle_id.to_lowercase();
        o.starts_with(&format!("{id}.")) && (entry == o || entry.starts_with(&format!("{o}.")))
    });
    if claimed {
        return false;
    }
    let by_id = entry == id || entry.starts_with(&format!("{id}."));
    match location {
        Location::ById => by_id,
        Location::ByIdOrName => by_id || (entry == app.name.to_lowercase()
            && !others.iter().any(|other| shares_name_or_vendor(other, &entry))),
        Location::GroupContainers => {
            let Some(team) = app.team_id.as_ref() else { return false };
            let team = team.to_lowercase();
            // A team-wide container is ours only when every other app's team is known and different.
            entry == format!("{team}.{id}")
                || (entry.starts_with(&format!("{team}."))
                    && others.iter().all(|other| other.team_id.as_ref()
                        .is_some_and(|other_team| other_team.to_lowercase() != team)))
        }
        Location::PlistById => entry == format!("{id}.plist"),
        Location::SavedState => entry == format!("{id}.savedstate"),
        Location::ByHost => entry.starts_with(&format!("{id}.")) && entry.ends_with(".plist"),
        // `com.foo.plist` or `com.foo.<helper>.plist`, never `com.foobar.plist` (another app).
        Location::LaunchPlist => entry == format!("{id}.plist") || (entry.starts_with(&format!("{id}.")) && entry.ends_with(".plist")),
    }
}

fn shares_name_or_vendor(other: &AppIdentity, entry: &str) -> bool {
    let name = other.name.to_lowercase();
    let id = other.bundle_id.to_lowercase();
    name == entry
        || name.strip_prefix(entry).is_some_and(|suffix| {
            suffix.starts_with(|c: char| !c.is_alphanumeric())
        })
        // Reverse-DNS bundle identifiers put the vendor after the domain suffix.
        || id.split('.').nth(1) == Some(entry)
}

pub struct LeftoverRoot {
    pub dir: PathBuf,
    pub location: Location,
    pub needs_admin: bool,
}

/// Builds paths only; it does not access the user's or system's Library.
pub fn leftover_roots(home: &Path) -> Vec<LeftoverRoot> {
    use Location::*;
    let user = [
        ("Application Support", ByIdOrName), ("Application Scripts", ById),
        ("Caches", ById), ("Containers", ById), ("HTTPStorages", ById), ("WebKit", ById),
        ("Saved Application State", SavedState), ("Preferences", PlistById),
        ("Preferences/ByHost", ByHost), ("LaunchAgents", LaunchPlist),
        ("Logs", ByIdOrName), ("Group Containers", GroupContainers),
    ];
    let system = [
        ("LaunchAgents", LaunchPlist), ("LaunchDaemons", LaunchPlist),
        ("PrivilegedHelperTools", ById), ("Application Support", ByIdOrName),
        ("Caches", ById), ("Logs", ByIdOrName), ("Preferences", PlistById),
    ];
    user.into_iter().map(|(name, location)| LeftoverRoot {
        dir: home.join("Library").join(name), location, needs_admin: false,
    }).chain(system.into_iter().map(|(name, location)| LeftoverRoot {
        dir: Path::new("/Library").join(name), location, needs_admin: true,
    })).collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppLeftover {
    pub path: String,
    pub name: String,
    pub size: Option<u64>,
    pub needs_admin: bool,
}

/// Scan immediate children, retaining matching entries even when sizing is denied.
pub fn find_leftovers(app: &AppIdentity, others: &[AppIdentity], roots: &[LeftoverRoot]) -> Vec<AppLeftover> {
    let mut found = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(&root.dir) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !matches_leftover(app, others, root.location, &name) {
                continue;
            }
            let path = entry.path();
            // dir_size intentionally hides I/O errors. Probe the entry first so
            // permission-denied containers aren't reported as empty directories.
            let readable = fs::symlink_metadata(&path).and_then(|metadata| {
                if metadata.is_dir() {
                    fs::read_dir(&path).map(|_| ())
                } else if metadata.is_file() {
                    fs::File::open(&path).map(|_| ())
                } else {
                    Ok(()) // Size the link itself, never its target.
                }
            }).is_ok();
            found.push(AppLeftover {
                path: path.to_string_lossy().into_owned(),
                name,
                size: readable.then(|| scanner::dir_size(&path)),
                needs_admin: root.needs_admin,
            });
        }
    }
    found.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
    found
}

/// Backend gate before uninstalling: the app must come from the latest listing and not be running
/// (checked on the exact bundle path: two copies can share a bundle id).
pub fn check_uninstall(bundle_path: &str, listed: bool, running: &std::collections::HashSet<String>) -> Result<(), &'static str> {
    if !listed {
        return Err("App không có trong danh sách gần nhất");
    }
    if running.contains(bundle_path) {
        return Err("App đang chạy, hãy thoát trước");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

    fn app(id: &str, name: &str, team: Option<&str>) -> AppIdentity {
        AppIdentity { bundle_id: id.into(), name: name.into(), team_id: team.map(str::to_owned) }
    }

    fn temp_dir() -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("tgv-apps-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path // Intentionally retained: tests never delete files.
    }

    fn write(path: &Path, size: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![7; size]).unwrap();
    }

    #[test]
    fn matches_by_id_and_own_extensions() {
        let excel = app("com.microsoft.Excel", "Excel", None);
        for location in [Location::ById, Location::ByIdOrName] {
            for entry in ["com.microsoft.Excel", "COM.MICROSOFT.EXCEL.widgetextension"] {
                assert!(matches_leftover(&excel, &[], location, entry));
            }
            assert!(!matches_leftover(&excel, &[], location, "com.microsoft.ExcelX"));
        }
    }

    #[test]
    fn name_match_only_when_unique() {
        let webstorm = app("com.jetbrains.WebStorm", "WebStorm", None);
        let others = [app("com.jetbrains.IntelliJ", "IntelliJ", None)];
        assert!(!matches_leftover(&webstorm, &others, Location::ByIdOrName, "JetBrains"));
        assert!(!matches_leftover(&webstorm, &[], Location::ByIdOrName, "JetBrains"));
        assert!(matches_leftover(&webstorm, &[], Location::ByIdOrName, "WEBSTORM"));
        assert!(!matches_leftover(&webstorm, &[], Location::ById, "WebStorm"));
        let duplicate = [app("org.other.ide", "webstorm", None)];
        assert!(!matches_leftover(&webstorm, &duplicate, Location::ByIdOrName, "WebStorm"));
        let vendor = app("com.jetbrains.launcher", "JetBrains", None);
        assert!(!matches_leftover(&vendor, &others, Location::ByIdOrName, "JetBrains"));
        let named_vendor = [app("org.other.ide", "JetBrains Toolbox", None)];
        assert!(!matches_leftover(&vendor, &named_vendor, Location::ByIdOrName, "JetBrains"));
        assert!(matches_leftover(&vendor, &[], Location::ByIdOrName, "JetBrains"));
    }

    #[test]
    fn group_containers_need_exclusive_team() {
        let word = app("com.microsoft.Word", "Word", Some("UBF8T346G9"));
        let others = [app("com.microsoft.Excel", "Excel", Some("ubf8t346g9"))];
        assert!(!matches_leftover(&word, &others, Location::GroupContainers, "UBF8T346G9.Office"));
        assert!(matches_leftover(&word, &others, Location::GroupContainers, "ubf8t346g9.COM.MICROSOFT.WORD"));
        assert!(!matches_leftover(&word, &others, Location::GroupContainers, "UBF8T346G9.com.microsoft.Word.extension"));
        assert!(matches_leftover(&word, &[], Location::GroupContainers, "UBF8T346G9.Office"));
        assert!(!matches_leftover(&word, &[], Location::GroupContainers, "UBF8T346G9X.Office"));
        let unknown = app("com.microsoft.Word", "Word", None);
        assert!(!matches_leftover(&unknown, &[], Location::GroupContainers, "UBF8T346G9.com.microsoft.Word"));
    }

    #[test]
    fn plist_and_saved_state_and_launch_plists() {
        let foo = app("com.foo", "Foo", None);
        for (location, yes, no) in [
            (Location::PlistById, "COM.FOO.PLIST", "com.foobar.plist"),
            (Location::SavedState, "com.foo.savedState", "com.foobar.savedState"),
            (Location::ByHost, "com.foo.ABC-123.plist", "com.foobar.ABC.plist"),
            (Location::LaunchPlist, "com.foo.helper.plist", "org.foo.helper.plist"),
        ] {
            assert!(matches_leftover(&foo, &[], location, yes));
            assert!(!matches_leftover(&foo, &[], location, no));
            assert!(!matches_leftover(&foo, &[], location, "com.foo.helper.txt"));
        }
        // A different app whose id merely starts with ours is not ours.
        assert!(!matches_leftover(&foo, &[], Location::LaunchPlist, "com.foobar.plist"));
        assert!(matches_leftover(&foo, &[], Location::LaunchPlist, "com.foo.plist"));
    }

    #[test]
    fn nothing_while_another_copy_with_the_same_id_stays() {
        // e.g. two Antigravity.app bundles with one bundle id: their data is shared.
        let ag = app("com.google.antigravity", "Antigravity", Some("EQHXZ8M8AV"));
        let copy = [app("com.google.antigravity", "Antigravity", Some("EQHXZ8M8AV"))];
        for (location, entry) in [
            (Location::ById, "com.google.antigravity"),
            (Location::ByIdOrName, "Antigravity"),
            (Location::PlistById, "com.google.antigravity.plist"),
            (Location::GroupContainers, "EQHXZ8M8AV.com.google.antigravity"),
            (Location::LaunchPlist, "com.google.antigravity.helper.plist"),
        ] {
            assert!(matches_leftover(&ag, &[], location, entry), "{entry}");
            assert!(!matches_leftover(&ag, &copy, location, entry), "{entry}");
        }
    }

    #[test]
    fn roots_have_expected_locations_and_permissions() {
        let home = temp_dir();
        let roots = leftover_roots(&home);
        let user = [
            ("Application Support", Location::ByIdOrName), ("Application Scripts", Location::ById),
            ("Caches", Location::ById), ("Containers", Location::ById),
            ("HTTPStorages", Location::ById), ("WebKit", Location::ById),
            ("Saved Application State", Location::SavedState), ("Preferences", Location::PlistById),
            ("Preferences/ByHost", Location::ByHost), ("LaunchAgents", Location::LaunchPlist),
            ("Logs", Location::ByIdOrName), ("Group Containers", Location::GroupContainers),
        ];
        let system = [
            ("LaunchAgents", Location::LaunchPlist), ("LaunchDaemons", Location::LaunchPlist),
            ("PrivilegedHelperTools", Location::ById), ("Application Support", Location::ByIdOrName),
            ("Caches", Location::ById), ("Logs", Location::ByIdOrName), ("Preferences", Location::PlistById),
        ];
        assert_eq!(roots.len(), user.len() + system.len());
        for (base, expected, admin) in [(home.join("Library"), user.as_slice(), false), (PathBuf::from("/Library"), system.as_slice(), true)] {
            for (name, location) in expected {
                assert!(roots.iter().any(|r| r.dir == base.join(name) && r.location == *location && r.needs_admin == admin));
            }
        }
    }

    #[test]
    fn find_leftovers_walks_roots() {
        let home = temp_dir();
        let small = home.join("user/com.foo");
        let big = home.join("admin/com.foo.helper.plist");
        write(&small.join("data"), 4_096);
        write(&big, 131_072);
        write(&home.join("user/com.foobar/data"), 200_000);
        write(&home.join("admin/org.other.plist"), 200_000);
        let roots = [
            LeftoverRoot { dir: home.join("user"), location: Location::ById, needs_admin: false },
            LeftoverRoot { dir: home.join("admin"), location: Location::LaunchPlist, needs_admin: true },
            LeftoverRoot { dir: home.join("missing"), location: Location::ById, needs_admin: false },
        ];
        let found = find_leftovers(&app("com.foo", "Foo", None), &[], &roots);
        assert_eq!(found.len(), 2);
        for (item, path, admin) in [(&found[0], big, true), (&found[1], small, false)] {
            assert_eq!(item.path, path.to_string_lossy());
            assert_eq!(item.name, path.file_name().unwrap().to_string_lossy());
            assert_eq!(item.needs_admin, admin);
            assert_eq!(item.size, Some(super::super::scanner::dir_size(&path)));
        }
        assert!(found[0].size > found[1].size);
        let json = serde_json::to_value(&found[0]).unwrap();
        assert_eq!(json["needsAdmin"], true);
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_entry_has_no_size() {
        use std::os::unix::fs::PermissionsExt;
        let home = temp_dir();
        let locked = home.join("com.foo.locked");
        fs::create_dir(&locked).unwrap();
        write(&home.join("com.foo/data"), 4096);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        let denied = fs::read_dir(&locked).is_err();
        let found = find_leftovers(&app("com.foo", "Foo", None), &[], &[LeftoverRoot {
            dir: home, location: Location::ById, needs_admin: false,
        }]);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(found.len(), 2);
        // Root can read chmod 000 directories, so no permission failure exists in that case.
        if denied {
            assert_eq!(found[1].path, locked.to_string_lossy());
            assert_eq!(found[1].size, None);
            assert!(found[0].size.is_some());
        }
    }

    #[test]
    fn never_takes_data_of_an_installed_app_with_a_longer_id() {
        let editor = app("com.example.Editor", "Editor", Some("T1"));
        let pro = [app("com.example.Editor.Pro", "Editor Pro", Some("T1"))];
        for (location, entry) in [
            (Location::ById, "com.example.Editor.Pro"),
            (Location::ById, "com.example.Editor.Pro.helper"),
            (Location::LaunchPlist, "com.example.Editor.Pro.plist"),
            (Location::ByHost, "com.example.Editor.Pro.ABC-1.plist"),
        ] {
            assert!(!matches_leftover(&editor, &pro, location, entry), "{entry}");
        }
        assert!(matches_leftover(&editor, &pro, Location::ById, "com.example.Editor"));
        assert!(matches_leftover(&editor, &pro, Location::ById, "com.example.Editor.widget"));
        // The longer id keeps its own data even while the shorter one stays installed.
        let pro_app = app("com.example.Editor.Pro", "Editor Pro", Some("T1"));
        let editor_stays = [app("com.example.Editor", "Editor", Some("T1"))];
        assert!(matches_leftover(&pro_app, &editor_stays, Location::ById, "com.example.Editor.Pro"));
    }

    #[test]
    fn team_prefix_needs_every_other_team_known() {
        let word = app("com.microsoft.Word", "Word", Some("UBF8T346G9"));
        let unknown = [app("com.x.unsigned", "X", None)];
        assert!(!matches_leftover(&word, &unknown, Location::GroupContainers, "UBF8T346G9.Office"));
        assert!(matches_leftover(&word, &unknown, Location::GroupContainers, "UBF8T346G9.com.microsoft.Word"));
    }

    #[test]
    fn uninstall_check_needs_listed_and_stopped_app() {
        let running: std::collections::HashSet<String> = ["/Applications/B.app".to_string()].into();
        assert_eq!(check_uninstall("/Applications/A.app", true, &running), Ok(()));
        assert_eq!(check_uninstall("/Applications/B.app", true, &running), Err("App đang chạy, hãy thoát trước"));
        assert_eq!(check_uninstall("/Applications/C.app", false, &running), Err("App không có trong danh sách gần nhất"));
    }
}
