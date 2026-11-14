//! Rebuildable build artifacts inside projects (`target`, `node_modules`, `.next`, `.venv`, …).
//! Read-only. A folder only counts when its parent is a real project of the matching kind
//! (e.g. `target` next to `Cargo.toml`), it holds no Git-tracked file and no nested repository.
use super::scanner::{dir_size, owned_by_user};
use super::CleanupItem;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Artifact folder name → project files, any of which must sit next to it.
/// Pairing by kind keeps generic names like `build` / `dist` from matching unrelated folders.
pub const ARTIFACTS: &[(&str, &[&str])] = &[
    ("node_modules", &["package.json"]),
    ("target", &["Cargo.toml", "pom.xml"]),
    (".next", &["package.json"]),
    (".nuxt", &["package.json"]),
    (".output", &["package.json"]),
    (".svelte-kit", &["package.json"]),
    (".angular", &["angular.json", "package.json"]),
    (".turbo", &["package.json", "turbo.json"]),
    (".parcel-cache", &["package.json"]),
    ("dist", &["package.json"]),
    ("build", &["package.json", "build.gradle", "build.gradle.kts", "pubspec.yaml"]),
    (".gradle", &["build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts"]),
    (".dart_tool", &["pubspec.yaml"]),
    (".venv", &["pyproject.toml", "requirements.txt", "setup.py", "Pipfile"]),
    ("venv", &["pyproject.toml", "requirements.txt", "setup.py", "Pipfile"]),
    (".tox", &["tox.ini", "pyproject.toml", "setup.py"]),
    (".pytest_cache", &["pyproject.toml", "requirements.txt", "setup.py", "pytest.ini"]),
    (".mypy_cache", &["pyproject.toml", "requirements.txt", "setup.py", "mypy.ini"]),
    (".ruff_cache", &["pyproject.toml", "requirements.txt", "setup.py", "ruff.toml"]),
    ("Pods", &["Podfile"]),
    (".build", &["Package.swift"]),
    (".expo", &["app.json", "package.json"]),
    (".zig-cache", &["build.zig"]),
    ("zig-out", &["build.zig"]),
];

/// Folders never walked into while looking for projects.
const SKIP_DIRS: &[&str] = &["Library", "Applications", "Pictures", "Music", "Movies", "Photos Library.photoslibrary"];

/// Project files that must sit next to `name` for it to count as an artifact, if it is one.
pub fn artifact_markers(name: &str) -> Option<&'static [&'static str]> {
    ARTIFACTS.iter().find(|(n, _)| *n == name).map(|(_, markers)| *markers)
}

/// Every build artifact under `roots` (walked `max_depth` levels deep), biggest first.
/// `modified` on each item is when its *project* was last edited, so the UI can say
/// "sửa lần cuối 12 ngày trước".
pub fn find_artifacts(roots: &[PathBuf], max_depth: usize) -> Vec<CleanupItem> {
    find_artifacts_with(roots, max_depth, &mut |_| {})
}

/// Same as `find_artifacts`, calling `on_found` for each artifact as soon as it is sized.
pub fn find_artifacts_with(roots: &[PathBuf], max_depth: usize, on_found: &mut dyn FnMut(&CleanupItem)) -> Vec<CleanupItem> {
    let mut found = Vec::new();
    for root in roots {
        walk(root, root, 0, max_depth, &mut found, on_found);
    }
    found.sort_by_key(|i| std::cmp::Reverse(i.size));
    found
}

fn walk(root: &Path, dir: &Path, depth: usize, max_depth: usize, found: &mut Vec<CleanupItem>, on_found: &mut dyn FnMut(&CleanupItem)) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        // file_type() does not follow symlinks.
        let Ok(kind) = entry.file_type() else { continue };
        if !kind.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if let Some(markers) = artifact_markers(&name) {
            if markers.iter().any(|m| dir.join(m).is_file()) && owned_by_user(&path) && is_disposable(&path) {
                let size = dir_size(&path);
                if size > 0 {
                    let item = artifact_item(root, dir, &path, size);
                    on_found(&item);
                    found.push(item);
                }
            }
            // Never descend into an artifact (nested node_modules are part of it).
            continue;
        }
        if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) || name.ends_with(".app") {
            continue;
        }
        subdirs.push(path);
    }
    for sub in subdirs {
        walk(root, &sub, depth + 1, max_depth, found, on_found);
    }
}

/// No nested repository and nothing in it tracked by Git: removing it loses no source.
fn is_disposable(artifact: &Path) -> bool {
    !contains_repo(artifact, 3) && !has_tracked_files(artifact)
}

/// A `.git` within `depth` levels, e.g. a package cloned into `node_modules/@scope/pkg`.
fn contains_repo(dir: &Path, depth: usize) -> bool {
    if dir.join(".git").exists() {
        return true;
    }
    if depth == 0 {
        return false;
    }
    let Ok(entries) = fs::read_dir(dir) else { return false };
    entries.flatten().any(|e| e.file_type().is_ok_and(|t| t.is_dir()) && contains_repo(&e.path(), depth - 1))
}

/// True when Git tracks any file under `path` (or when Git cannot tell us, to stay safe).
/// Not inside a repository → nothing is tracked.
fn has_tracked_files(path: &Path) -> bool {
    let Some(parent) = path.parent() else { return true };
    let Some(name) = path.file_name() else { return true };
    let inside = Command::new("git").arg("-C").arg(parent).args(["rev-parse", "--is-inside-work-tree"]).output();
    match inside {
        Ok(out) if out.status.success() => {}
        Ok(_) => return false, // not a repository
        Err(_) => return true, // git missing: cannot prove it is untracked
    }
    match Command::new("git").arg("-C").arg(parent).args(["ls-files", "-z", "--"]).arg(name).output() {
        Ok(out) if out.status.success() => !out.stdout.is_empty(),
        _ => true,
    }
}

fn artifact_item(root: &Path, project: &Path, path: &Path, size: u64) -> CleanupItem {
    // "vibe-code/tg-studio/src-tauri/target": readable without the full path.
    let name = path.strip_prefix(root).unwrap_or(path).to_string_lossy().into_owned();
    CleanupItem {
        path: path.to_string_lossy().into_owned(),
        name,
        size,
        modified: project_modified(project).and_then(|m| m.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()),
    }
}

/// Newest modification among the project's direct children, ignoring artifact folders and
/// `.git` (a build or fetch touches those without the user editing anything). The folder's own
/// time is only a fallback: creating `node_modules` bumps it.
pub fn project_modified(dir: &Path) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if artifact_markers(&name.to_string_lossy()).is_some() || name == ".git" {
                continue;
            }
            if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
                newest = Some(newest.map_or(m, |n| n.max(m)));
            }
        }
    }
    newest.or_else(|| fs::symlink_metadata(dir).and_then(|m| m.modified()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("tgv-proj-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![1u8; bytes]).unwrap();
    }

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git").arg("-C").arg(dir).args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .args(args).status().unwrap().success();
        assert!(ok, "git {args:?}");
    }

    #[test]
    fn pairs_artifacts_with_their_project_kind() {
        let root = tmp();
        write(&root.join("rusty/Cargo.toml"), 10);
        write(&root.join("rusty/target/debug/app"), 20_000);
        write(&root.join("web/package.json"), 10);
        write(&root.join("web/node_modules/react/index.js"), 20_000);
        write(&root.join("web/node_modules/a/node_modules/b/i.js"), 20_000);
        write(&root.join("web/.next/cache/x"), 20_000);
        // `target` / `build` without the matching project file are someone's own folders.
        write(&root.join("notes/target/plan.md"), 20_000);
        write(&root.join("photos/build/raw.dng"), 20_000);

        let found = find_artifacts(std::slice::from_ref(&root), 6);
        let mut names: Vec<_> = found.iter().map(|i| i.name.as_str()).collect();
        names.sort();
        assert_eq!(names, ["rusty/target", "web/.next", "web/node_modules"]);
        assert!(found.iter().all(|i| i.modified.is_some()));
    }

    #[test]
    fn finds_artifacts_in_nested_packages() {
        let root = tmp();
        write(&root.join("mono/package.json"), 10);
        write(&root.join("mono/app/src-tauri/Cargo.toml"), 10);
        write(&root.join("mono/app/src-tauri/target/release/x"), 20_000);
        let mut announced = Vec::new();
        let found = find_artifacts_with(std::slice::from_ref(&root), 6, &mut |i| announced.push(i.name.clone()));
        assert_eq!(announced, ["mono/app/src-tauri/target"]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "mono/app/src-tauri/target");
    }

    #[test]
    fn respects_depth_and_skips_hidden_and_library() {
        let root = tmp();
        write(&root.join("a/b/c/d/e/f/g/package.json"), 10);
        write(&root.join("a/b/c/d/e/f/g/node_modules/x.js"), 20_000);
        write(&root.join(".hidden/package.json"), 10);
        write(&root.join(".hidden/node_modules/x.js"), 20_000);
        write(&root.join("Library/p/package.json"), 10);
        write(&root.join("Library/p/node_modules/x.js"), 20_000);
        assert!(find_artifacts(std::slice::from_ref(&root), 4).is_empty());
    }

    #[test]
    fn keeps_git_tracked_and_nested_repo_artifacts() {
        let root = tmp();
        let repo = root.join("repo");
        write(&repo.join("package.json"), 10);
        write(&repo.join("dist/committed.js"), 20_000); // tracked on purpose
        write(&repo.join("node_modules/x.js"), 20_000); // untracked
        write(&repo.join("pkg/package.json"), 10);
        write(&repo.join("pkg/node_modules/vendored/.git/HEAD"), 20_000); // nested repo
        git(&repo, &["init", "-q"]);
        git(&repo, &["add", "package.json", "dist/committed.js", "pkg/package.json"]);
        git(&repo, &["commit", "-qm", "init"]);

        let found = find_artifacts(std::slice::from_ref(&root), 6);
        let names: Vec<_> = found.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["repo/node_modules"]);
    }

    #[test]
    fn project_age_ignores_artifacts() {
        let root = tmp();
        write(&root.join("p/package.json"), 10);
        let before = project_modified(&root.join("p")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        write(&root.join("p/node_modules/x.js"), 10);
        // Touching only node_modules must not count as editing the project.
        let after = project_modified(&root.join("p")).unwrap();
        assert!(after.duration_since(before).unwrap_or_default().as_millis() < 1000);
    }
}
