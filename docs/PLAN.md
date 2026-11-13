> **Đổi hướng (11/2026):** chuyển sang **Tauri 2 (Rust + React)**, UI theo phong cách Pulse.
> Frontend nằm ở `app/` (đã xong, đang chạy dữ liệu mẫu). Bản Swift cũ giữ lại để tham khảo cho tới khi backend Rust thay thế.
>
> **Việc tiếp theo:**
> - [x] Backend Rust (`app/src-tauri/core`): CPU, RAM, disk, mạng, process, pin — gọi thẳng Mach/libproc/sysctl/IOKit
>   (bỏ `sysinfo` vì không cho `phys_footprint`, user/system CPU, bộ đếm mạng 64-bit). Lệnh `snapshot` / `kill_process` / `reveal_path`
> - [x] Dọn rác bằng Rust: `scan_cleanup`, `trash_paths` (chỉ `trashItemAtURL`, PathGuard, chỉ xóa được path có trong lần quét gần nhất)
> - [x] **Menu bar (tray):** item trên thanh menu hiện CPU / RAM / mạng / nhiệt độ, bấm vào mở popup nhỏ kiểu Pulse
> - [x] Icon app thật cho từng process (scheme `appicon://`, render bằng NSWorkspace, có cache)
> - [x] CI build `tg-vitals.app` (ký ad-hoc, không cần Apple Developer) để tải về dùng — artifact mỗi lần chạy, Release khi push tag `v*`
> - [x] Build **universal** (Apple silicon + Intel) trên CI, kiểm tra phần Intel chạy qua Rosetta
> - [x] Nhiệt độ trên Mac Intel: đọc SMC (`iokit::Smc`, key `TC1C…` / `TCXC` / `TC0D` / `TC0P`) — chưa thử trên máy Intel thật
> - [ ] Chạy thử trên máy thật: kiểm tra thứ tự core P/E, tên cảm biến nhiệt trên chip của bạn, số công suất (W)
> - [ ] Xóa bản Swift cũ (`Sources/`, `Tests/`, `Package.swift`) sau khi bản Tauri chạy ổn
> - [ ] Docker (`docker system df` / `prune`), Xcode simulator (`xcrun simctl delete unavailable`), app đã gỡ
> - [ ] Mạng theo app / số connection (`nettop`), năng lượng theo app (`powermetrics`, cần root), GPU %

# Kế hoạch phát triển tg-vitals

Tổng thời gian ước tính: **~3–4 tuần** làm buổi tối/cuối tuần (có AI hỗ trợ).
Mỗi milestone kết thúc bằng một bản chạy được, dùng ngay hằng ngày.

Ký hiệu: `[x]` xong · `[ ]` chưa làm · ⚠️ phần khó / rủi ro

---

## M0 — Khung dự án (½ ngày) ✅ khởi tạo sẵn

- [x] Swift package `TGVitals`, target macOS 14
- [x] Cấu trúc thư mục `App / Core / Features / Shared`
- [x] Script đóng gói `.app` (`scripts/bundle.sh`, `LSUIElement` ẩn khỏi Dock)
- [x] Test cho phần logic thuần (ring buffer, tính % CPU, format byte)
- [ ] Build thành công lần đầu trên máy, sửa lỗi compile nếu có
- [x] Thêm app icon (`app/src-tauri/icons`, nguồn SVG trong `icons/source`)
- [x] GitHub Actions: `swift build` + `swift test` trên macOS

## M1 — Monitor cơ bản (2–3 ngày)

Mục tiêu: thay được Activity Monitor cho nhu cầu xem nhanh.

- [x] `CPUSampler`: tổng + từng core qua `host_processor_info` (code mẫu có sẵn)
- [x] `MemorySampler`: app / wired / compressed qua `host_statistics64`
- [x] `NetworkSampler`: tổng byte vào/ra qua `getifaddrs`
- [x] `ProcessSampler`: danh sách process qua `libproc`, CPU% + RAM
- [x] `SystemMonitor`: timer 1s, giữ lịch sử 60 điểm
- [x] Menu bar: CPU% + biểu đồ mini
- [x] Cửa sổ chính dạng sidebar: Tổng quan · Process · Dọn dẹp
- [x] Thêm trang riêng: CPU · Memory · Network (UI Tauri)
- [ ] Biểu đồ realtime bằng Swift Charts cho từng trang
- [x] Processes: sort theo cột, ô search
- [x] Icon app cho từng process (Tauri: `NSWorkspace.iconForFile`)
- [x] Quit / Force quit (`kill(pid, SIGTERM/SIGKILL)`, hỏi xác nhận khi force quit)
- [ ] Đo chính app: mục tiêu < 2% một core khi mở cửa sổ, < 0.5% khi chỉ có menu bar

## M2 — Care: dọn rác an toàn (2–4 ngày)

Mục tiêu: dọn được vài chục GB rác dev mà không bao giờ mất dữ liệu.

- [x] Model `CleanupTarget` + danh mục mục tiêu mặc định
- [x] `DirectorySizer` tính dung lượng (allocated size, đúng với Finder)
- [x] `Trasher`: chỉ chuyển vào Thùng rác qua `FileManager.trashItem`
- [x] `PathGuard`: chặn mọi đường dẫn ngoài home và thư mục nhạy cảm (có test)
- [x] Màn hình Care: nút **Quét** → danh sách nhóm theo loại, dung lượng, tick chọn → **Dọn**
- [x] `node_modules` finder (Rust `Scanner::stale_node_modules`, gốc: `~/WebstormProjects`, `~/Projects`, `~/code`…): quét các thư mục gốc cấu hình được (`~/Projects`, `~/code`…), chỉ gợi ý project không sửa > 30 ngày
- [~] Xcode: DerivedData, Archives, iOS DeviceSupport (xong) · simulator không dùng (chưa) (`xcrun simctl delete unavailable`)
- [x] Cache tool dev: Homebrew, npm, pnpm, yarn, pip, Gradle, CocoaPods (quét thư mục cache)
- [ ] Docker: hiện kết quả `docker system df`, dọn bằng `docker system prune` (hỏi xác nhận riêng)
- [x] File lớn: liệt kê file > 500MB trong `~/Downloads`, `~/Desktop`, `~/Movies`
- [ ] App đã gỡ: so `~/Library/Application Support` + `Preferences` với app đang cài
- [ ] Hướng dẫn cấp Full Disk Access khi bị từ chối quyền
- [ ] Lưu lịch sử dọn (ngày, dung lượng giải phóng) ra file JSON

**Quy tắc bất biến** (viết test để giữ):
1. Không bao giờ dùng `removeItem` — chỉ `trashItem`.
2. Không bao giờ đụng `/System`, `/Library`, `/usr`, `/bin`, `/Applications` của hệ thống.
3. Không quét/xóa gì khi người dùng chưa bấm.

## M3 — Monitor nâng cao (4–6 ngày)

- [x] CPU: tách P-core / E-core (`sysctl hw.perflevel0/1.logicalcpu`)
- [x] Memory pressure (`kern.memorystatus_vm_pressure_level`), swap (`vm.swapusage`)
- [ ] Network theo app + từng connection ⚠️ — đọc `nettop -P -L 1 -J bytes_in,bytes_out` hoặc `-m tcp`
- [x] Disk I/O theo hệ thống (IOKit `IOBlockStorageDriver` statistics) + theo app (`proc_pid_rusage` `ri_diskio_*`)
- [x] Bộ đếm mạng 64-bit qua `sysctl NET_RT_IFLIST2` (thay `getifaddrs` vốn tràn ở 4GB)
- [ ] Process: lịch sử của một process, cây parent–children, nhóm theo app (bundle)
- [x] RAM process theo `phys_footprint` (`proc_pid_rusage`) cho khớp Activity Monitor

## M4 — Năng lượng & nhiệt (3–5 ngày) ⚠️

- [x] Pin: health, cycle count, đang sạc (IOKit `AppleSmartBattery`)
- [ ] Công suất toàn máy + năng lượng theo app ⚠️ — `powermetrics` cần root:
  - phương án A: helper chạy bằng `sudo` lúc mở app (đơn giản, dùng cá nhân OK)
  - phương án B: LaunchDaemon cài một lần, ghi số liệu ra socket/file
- [~] Nhiệt độ ⚠️ — đã đọc IOHID (cần kiểm tra trên máy thật), chưa có SMC (tham khảo code mở của *Stats* — exelban/stats, MIT)
- [ ] GPU: % sử dụng (IOKit `IOAccelerator` → `PerformanceStatistics`)
- [ ] Cái gì đang giữ máy không ngủ (`pmset -g assertions`)

## M5 — Hoàn thiện (2–3 ngày)

- [ ] Menu bar: mỗi chỉ số là một item riêng, bật/tắt trong Settings
- [ ] Cảnh báo: thông báo khi một app ăn RAM/CPU vượt ngưỡng trong X phút
- [ ] Ghi log số liệu ra SQLite/CSV để xem lại
- [ ] Mở cùng hệ thống (`SMAppService.mainApp.register()`)
- [ ] Light/Dark, phím tắt toàn cục mở cửa sổ
- [ ] Tối ưu: chỉ lấy mẫu chi tiết khi cửa sổ đang mở

---

## Thứ tự ưu tiên nếu thiếu thời gian

M0 → M1 → M2 là bản "đủ dùng" (~1 tuần). M3–M5 làm dần khi thấy cần.

## Rủi ro chính

| Rủi ro | Cách xử lý |
|---|---|
| Private API của Apple đổi theo bản macOS | Ưu tiên lệnh CLI có sẵn (`nettop`, `powermetrics`) — ổn định hơn private framework |
| `powermetrics` cần root | Chấp nhận nhập mật khẩu một lần, hoặc LaunchDaemon |
| App tự tốn CPU vì lấy mẫu nhiều | Lấy mẫu 1s cho số tổng, 2–5s cho danh sách process; dừng khi ẩn cửa sổ |
| Xóa nhầm dữ liệu | Chỉ `trashItem`, danh sách chặn đường dẫn hệ thống, test bắt buộc |

- [x] Giao diện graphite: Tổng quan CPU/RAM ưu tiên, biểu đồ SVG, dải tài nguyên, bảng tiến trình và lối vào Dọn dẹp; thẻ nổi và điều hướng dùng chung.

- [x] Menu bar: nhãn CPU/RAM/mạng/nhiệt độ có màu native AppKit, số tabular, tooltip chi tiết và giữ thao tác popup.
