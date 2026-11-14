# Gỡ app (App uninstaller) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A "Gỡ app" page that lists installed apps by size and moves an app plus its leftovers to the Trash, including root-owned apps via the macOS admin prompt.

**Architecture:** New `vitals-core` module `care::apps` (inventory + pure leftover matcher), a second Trash entry point `trasher::recycle_with_auth`, four Tauri commands that remember the last listing (only those paths may be trashed), and a React page `Uninstall.tsx` styled like Cleanup/Processes.

**Tech Stack:** Rust (objc2 / objc2-app-kit / objc2-foundation already in deps, `std::process::Command` for Apple CLI tools), Tauri 2, React + TypeScript.

**Spec:** `docs/superpowers/specs/2026-11-14-app-uninstaller-design.md`

## Global Constraints

- macOS 14+, universal build: code compiles for `aarch64-apple-darwin` and `x86_64-apple-darwin`; `vitals-core` compiles on Linux (macOS-only code behind `#[cfg(target_os = "macos")]`).
- Never delete: only Trash, only inside `care::trasher`. No `remove_file` / `remove_dir_all` / `rm -rf`.
- Only paths from the latest `list_apps` / `app_leftovers` result may be trashed.
- Never offer `com.apple.*` apps, anything under `/System`, or tg-vitals itself (`com.trangioi.tgvitals`).
- No new crates. Apple CLI tools allowed: `/usr/bin/mdls`, `/usr/bin/codesign`, `/usr/sbin/pkgutil`, `/usr/bin/osascript`.
- `model.rs`-style structs that cross to the UI must be mirrored in `app/src/api/types.ts` and `mock.ts`.
- UI copy in Vietnamese; commit messages in English (`feat(apps): …`).

## Review Focus

1. Shared data of a vendor family (Office `UBF8T346G9.Office`, `Application Support/JetBrains`) while another app of that family stays installed → must not be offered. Test in Task 2.
2. App running at uninstall time → Gỡ disabled, "Thoát app" shown; backend refuses too. Test in Task 3 (`uninstall refuses running`).
3. App path with spaces / quotes (`Microsoft Word.app`, `Bob's App.app`) through the admin fallback → no shell injection, path intact. Test in Task 4.
4. `~/Library/Containers/<id>` unreadable without Full Disk Access → listed as "không đo được", unticked, never blocks the rest. Test in Task 2 (size `None`).
5. User cancels the admin password prompt → nothing moved, report says "Đã huỷ", app still listed. Test in Task 4.

---

### Task 1: Spike — does `NSWorkspace.recycleURLs` prompt for root-owned items?

**Files:** throwaway `app/src-tauri/core/examples/recycle_spike.rs` (deleted at the end of the task, not committed).

- [ ] **Step 1:** Create a root-owned test bundle: `osascript -e 'do shell script "mkdir -p /Applications/TgvSpike.app/Contents && chown -R root:wheel /Applications/TgvSpike.app" with administrator privileges'`.
- [ ] **Step 2:** Example calls `NSWorkspace::sharedWorkspace().recycleURLs_completionHandler([url], block)` on the main thread with a run loop until the handler fires; prints the error if any.
- [ ] **Step 3:** Run `cargo run -p vitals-core --example recycle_spike`. Expected: either a macOS auth prompt then success (→ Task 4 uses NSWorkspace) or an error (→ Task 4 uses the `osascript` fallback). Write the outcome into the spec's "Rủi ro" section.
- [ ] **Step 4:** Remove the spike bundle (to Trash via Finder) and the example file; commit only the spec note: `docs(apps): record recycleURLs spike result`.

### Task 2: Pure leftover matcher (Codex)

**Files:**
- Create: `app/src-tauri/core/src/care/apps.rs` (pure part, no `cfg`)
- Modify: `app/src-tauri/core/src/care/mod.rs` (add `pub mod apps;`)

**Interfaces:**
- Produces:
  ```rust
  pub struct AppIdentity { pub bundle_id: String, pub name: String, pub team_id: Option<String> }
  pub enum Location { ById, ByIdOrName, GroupContainers, PlistById, SavedState, ByHost, LaunchPlist }
  pub fn matches_leftover(app: &AppIdentity, others: &[AppIdentity], location: Location, entry: &str) -> bool
  pub struct LeftoverRoot { pub dir: PathBuf, pub location: Location, pub needs_admin: bool }
  pub fn leftover_roots(home: &Path) -> Vec<LeftoverRoot>
  #[derive(Serialize)] #[serde(rename_all = "camelCase")]
  pub struct AppLeftover { pub path: String, pub name: String, pub size: Option<u64>, pub needs_admin: bool }
  pub fn find_leftovers(app: &AppIdentity, others: &[AppIdentity], roots: &[LeftoverRoot]) -> Vec<AppLeftover>
  ```
- Rules (from the spec): `ById` = entry equals `bundle_id` or starts with `bundle_id + "."`; `ByIdOrName` additionally equals `name` when no other app has the same name or a name/bundle-id vendor prefix that matches the entry (`JetBrains` vs `com.jetbrains.*`); `GroupContainers` = `"{team}.{bundle_id}"` or starts with `"{team}."` only when no other app has that team id, and never when `team_id` is `None`; `PlistById` = `"{bundle_id}.plist"`; `SavedState` = `"{bundle_id}.savedState"`; `ByHost` = starts with `"{bundle_id}."` and ends with `.plist`; `LaunchPlist` = starts with `bundle_id` and ends with `.plist`. Comparisons case-insensitive.
- `leftover_roots(home)`: `~/Library/{Application Support→ByIdOrName, Application Scripts→ById, Caches→ById, Containers→ById, HTTPStorages→ById, WebKit→ById, Saved Application State→SavedState, Preferences→PlistById, Preferences/ByHost→ByHost, LaunchAgents→LaunchPlist, Logs→ByIdOrName, Group Containers→GroupContainers}` (needs_admin false) and `/Library/{LaunchAgents, LaunchDaemons→LaunchPlist, PrivilegedHelperTools→ById, Application Support→ByIdOrName, Caches→ById, Logs→ByIdOrName, Preferences→PlistById}` (needs_admin true).
- `find_leftovers` sizes with `scanner::dir_size`; `size: None` when the entry cannot be read (permission), sorted biggest first, `None` last.

- [ ] **Step 1: Failing tests** in `apps.rs`:
  - `matches_by_id_and_own_extensions`: Excel matches `com.microsoft.Excel`, `com.microsoft.Excel.widgetextension`; not `com.microsoft.ExcelX`.
  - `name_match_only_when_unique`: WebStorm (`com.jetbrains.WebStorm`) with other `com.jetbrains.IntelliJ` installed → `JetBrains` false; alone → `JetBrains`? false (name is `WebStorm`), `WebStorm` true.
  - `group_containers_need_exclusive_team`: Word (team `UBF8T346G9`) with Excel same team → `UBF8T346G9.Office` false, `UBF8T346G9.com.microsoft.Word` true; Word alone → `UBF8T346G9.Office` true; `team_id: None` → always false.
  - `plist_and_saved_state_and_launch_plists`: `com.foo.plist` ✓, `com.foo.savedState` ✓, ByHost `com.foo.ABC-123.plist` ✓, LaunchPlist `com.foo.helper.plist` ✓, `com.foobar.plist` ✗ for PlistById.
  - `find_leftovers_walks_roots` (temp dirs): creates matching + non-matching entries in two roots (one `needs_admin`), asserts paths, `needs_admin` flags, sort order.
  - `unreadable_entry_has_no_size` (unix, chmod 000 dir): `size == None`, still listed.
- [ ] **Step 2:** `cd app/src-tauri && cargo test -p vitals-core apps` → FAIL (not defined).
- [ ] **Step 3:** Implement the interfaces above.
- [ ] **Step 4:** `cargo test -p vitals-core` → all pass; `cargo check -p vitals-core --target x86_64-apple-darwin` passes.
- [ ] **Step 5:** Commit `feat(apps): pure leftover matcher for the app uninstaller`.

### Task 3: App inventory (macOS)

**Files:** Modify `app/src-tauri/core/src/care/apps.rs` (macOS section).

**Interfaces:**
- Consumes: `AppIdentity` (Task 2), `scanner::dir_size`, `scanner::owned_by_user`.
- Produces:
  ```rust
  #[derive(Serialize, Clone)] #[serde(rename_all = "camelCase")]
  pub struct AppInfo { pub path: String, pub name: String, pub bundle_id: String, pub version: String,
    pub size: u64, pub owned_by_user: bool, pub last_used: Option<u64>, pub running: bool,
    pub brew_cask: Option<String>, pub team_id: Option<String> }
  impl AppInfo { pub fn identity(&self) -> AppIdentity }
  pub fn list_apps(roots: &[PathBuf]) -> Vec<AppInfo>          // biggest first
  pub fn list_apps_with(roots: &[PathBuf], on_app: &mut dyn FnMut(&str)) -> Vec<AppInfo> // calls on_app(path) before reading each app
  pub fn is_offerable(bundle_id: &str) -> bool                 // pure: !com.apple.*, != com.trangioi.tgvitals, non-empty
  pub fn parse_team_id(codesign_stderr: &str) -> Option<String> // pure: "TeamIdentifier=XXXX", "not set" → None
  ```
- Sources: `Contents/Info.plist` via `NSDictionary::dictionaryWithContentsOfFile` (`CFBundleIdentifier`, `CFBundleDisplayName`→`CFBundleName`→file stem, `CFBundleShortVersionString`); last used `mdls -raw -name kMDItemLastUsedDate` (`(null)` → None, parse `YYYY-MM-DD HH:MM:SS +0000` to unix seconds); team via `codesign -dv --verbose=2 <app>` stderr (plan decision: replaces the spec's Security.framework call — same data, no FFI, an Apple tool already on every Mac); running via `NSWorkspace.runningApplications` bundle ids; brew cask = an entry in `/opt/homebrew/Caskroom` or `/usr/local/Caskroom` whose name matches the app stem lower-cased with spaces→`-`.
- Roots: `/Applications`, `~/Applications` (one level, `.app` only, no symlinks).

- [ ] **Step 1: Failing tests:** `offerable_filters_apple_and_self` (`com.apple.Safari` ✗, `com.trangioi.tgvitals` ✗, `""` ✗, `com.microsoft.Word` ✓); `parses_team_id` (sample stderr with `TeamIdentifier=UBF8T346G9` → Some, `TeamIdentifier=not set` → None); `#[ignore] reads_real_applications` prints `list_apps` and asserts every entry has a non-empty bundle id and size > 0.
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `cargo test -p vitals-core` pass; `cargo test -p vitals-core reads_real_applications -- --include-ignored --nocapture` lists WebStorm, Microsoft Word, etc. with sizes; x86_64 check passes.
- [ ] **Step 5:** Commit `feat(apps): list installed apps with size, last use and owner`.

### Task 4: Trash with admin rights + guarded app paths

**Files:**
- Modify: `app/src-tauri/core/src/care/trasher.rs`, `app/src-tauri/core/src/care/path_guard.rs`
- Modify: `CLAUDE.md` (rule text), `docs/PLAN.md` (new item)

**Interfaces:**
- Produces:
  ```rust
  // path_guard.rs
  pub fn is_allowed_app_path(path: &Path, home: &Path) -> bool
  // trasher.rs
  pub fn recycle_with_auth(paths: &[String], allowed: &HashMap<String, u64>, home: &Path) -> TrashReport
  pub fn shell_quote(s: &str) -> String   // pure, single-quote escaping for the osascript fallback
  pub fn forget_receipts(bundle_id: &str) // pkgutil --forget for receipts starting with bundle_id (admin batch)
  ```
- `is_allowed_app_path`: true for `/Applications/<x>.app`, `~/Applications/<x>.app` (exactly one level, canonical, not a symlink), and for `/Library/{LaunchAgents,LaunchDaemons,PrivilegedHelperTools,Application Support,Caches,Logs,Preferences}/<entry>` (exactly one level); everything else delegates to `is_allowed`. Never `/System`, never `/Library/<root>` itself.
- `recycle_with_auth`: same `allowed` contract as `trash`; user-owned items go through `move_to_trash`; the rest in one batch via the Task 1 mechanism. Cancelled prompt → each item fails with reason `"Đã huỷ"`.
- `CLAUDE.md` rule becomes: "Không đụng đường dẫn hệ thống … **ngoại trừ** tính năng Gỡ app: chỉ app trong lần liệt kê gần nhất và leftovers khớp bundle id, chỉ vào Thùng rác qua `care::trasher::recycle_with_auth`."

- [ ] **Step 1: Failing tests:** `app_paths_allowed_one_level_only` (`/Applications/Foo.app` ✓, `/Applications/Foo.app/Contents` ✗, `/Applications` ✗, `/System/Applications/Mail.app` ✗, `/Library/LaunchDaemons/com.foo.plist` ✓, `/Library/LaunchDaemons` ✗); `shell_quote_handles_quotes_and_spaces` (`Bob's App.app` → `'Bob'\''s App.app'`); `recycle_refuses_unlisted_and_running` (path not in `allowed` → failed "Không có trong kết quả quét gần nhất"); `recycle_moves_user_owned_fake_app` (temp `~/Applications/Fake.app` under a temp home → trashed 1).
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement; update `CLAUDE.md` and add to `docs/PLAN.md`: `- [x] Gỡ app: …` under M2.
- [ ] **Step 4:** `cargo test -p vitals-core` pass. Manual: `Fake.app` owned by root in `/Applications` (create as in Task 1) → admin prompt → lands in Trash; cancel prompt → reported "Đã huỷ".
- [ ] **Step 5:** Commit `feat(apps): trash apps and system leftovers with admin rights`.

### Task 5: Tauri commands

**Files:** Modify `app/src-tauri/src/commands.rs`, `app/src-tauri/src/state.rs`, `app/src-tauri/src/lib.rs` (register).

**Interfaces:**
- Consumes: `list_apps`, `find_leftovers`, `leftover_roots`, `recycle_with_auth`, `forget_receipts`.
- Produces (frontend names in `api/index.ts`):
  - `list_apps(on_event: Channel<ScanEvent>) -> Vec<AppInfo>` → `api.listApps(onEvent)`; emits `ScanEvent::Path { path }` per app (reuses the Cleanup event type); stores each app path→size in `AppState.app_paths`.
  - `app_leftovers(bundle_path: String) -> Vec<AppLeftover>` → `api.appLeftovers(path)`; adds leftover paths→size (0 when `None`) to `AppState.app_paths`.
  - `quit_app(bundle_id: String)` → `api.quitApp(id)` (`NSRunningApplication.terminate`; rename the existing tray `quit_app` command to `quit_self` and its frontend `api.quit` stays pointed at it).
  - `uninstall_app(bundle_path: String, paths: Vec<String>) -> TrashReport` → `api.uninstallApp(path, paths)`; refuses with every item failed `"App đang chạy, hãy thoát trước"` when the app is running; on success calls `forget_receipts`; clears `app_paths`.
- [ ] **Step 1:** Implement; `cargo check` (app) passes with no warnings.
- [ ] **Step 2:** Commit `feat(apps): tauri commands for listing and uninstalling apps`.

### Task 6: "Gỡ app" page

**Files:**
- Create: `app/src/pages/Uninstall.tsx`
- Modify: `app/src/components/Sidebar.tsx` (item `{ id: "uninstall", label: "Gỡ app", icon: PackageX, color: ACCENT.gpu }` after "Dọn dẹp"), `app/src/App.tsx` (TITLES `"Gỡ app"`, subtitle `"Gỡ app cùng dữ liệu nó để lại · mọi thứ vào Thùng rác"`, route, hide live-status like cleanup), `app/src/api/{index,types,mock}.ts`, `app/src/styles.css`.

**Interfaces:**
- Consumes: Task 5 API; `fileBytes`, `ago` (move `ago` from `Cleanup.tsx` to `format.ts`), `IconTile`, `appicon://localhost<encoded path>` for icons.
- Loading: while `listApps` runs, show the hero with "Đang đọc app…" + the moving striped bar and the latest path from the events (no full terminal).
- Layout: left table (icon, name + version, app size, data size once loaded, last used via `ago`, "đang chạy" pill), search box + sort toggle (`Dung lượng` | `Lâu không dùng`); right inspector for the selected app: hero (icon, name, total to free), checklist (app bundle + leftovers; `needsAdmin` → tag "cần mật khẩu admin"; `size === null` → "không đo được", unticked), Homebrew tag with `brew uninstall --cask <token>` hint, button `Thoát app` when running (Gỡ disabled), `Gỡ` → `Xác nhận gỡ <name> · <size>` → result line (trashed count / failures with reasons). Reuse `.check`, `.group`, `.tag`, `.hero` styles.
- Mock: 6 apps incl. a root-owned Office app with leftovers needing admin, a running app, a Homebrew app, one leftover with `size: null`.
- [ ] **Step 1:** Implement; `npm run build` passes.
- [ ] **Step 2:** Browser preview (`npx vite --port 1421`, `?page=uninstall`): screenshot list, inspector of the Office mock (admin tags), running app (Gỡ disabled), confirm state. Fix anything misaligned.
- [ ] **Step 3:** Commit `feat(apps): "Gỡ app" page`.

### Task 7: End-to-end on the real Mac

- [ ] **Step 1:** `npm run tauri dev`; open Gỡ app; verify the list matches `/Applications` (WebStorm 3.1 GB top, Microsoft apps flagged admin).
- [ ] **Step 2:** Uninstall a disposable user-owned app installed for the test (e.g. a small Homebrew cask chosen with the user) → app + leftovers in Trash, list refreshes.
- [ ] **Step 3:** Full checks: `cargo test -p vitals-core`, `cargo check` app + x86_64, `npm run build`. Push, open PR.
