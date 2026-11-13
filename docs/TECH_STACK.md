> **Cập nhật 11/2026:** app đã chuyển sang **Tauri 2 (Rust + React)**. Bảng *nguồn dữ liệu* bên dưới vẫn đúng —
> backend Rust (`app/src-tauri/core`) gọi đúng các API đó qua `libc`, `core-foundation` và `objc2`. Phần "Tổng quan" là của bản Swift cũ.

# Tech stack

## Tổng quan

| Hạng mục | Lựa chọn | Lý do |
|---|---|---|
| Ngôn ngữ | **Swift** (toolchain 6, language mode 5) | Native, gọi thẳng C API của macOS (Mach, libproc, IOKit, sysctl) không cần bridge |
| UI | **SwiftUI** + `MenuBarExtra` | Menu bar + cửa sổ chính chỉ vài dòng; nhẹ hơn hẳn Electron/Tauri |
| Biểu đồ | **Swift Charts** | Có sẵn trong macOS 13+, đủ cho line/area realtime |
| State | **Observation** (`@Observable`) | Gọn hơn `ObservableObject`, chỉ re-render view đang đọc giá trị đổi |
| Concurrency | `async/await`, `Task`, actor cho sampler nặng | Lấy mẫu không chặn UI |
| Build | **Swift Package Manager** | Không cần `.xcodeproj`, dễ diff, build bằng CLI hoặc mở bằng Xcode |
| Đóng gói | `scripts/bundle.sh` → `.app` | Ký ad-hoc (`codesign -s -`), đủ cho máy cá nhân |
| Test | XCTest / Swift Testing | Test phần logic thuần, không phụ thuộc máy |
| Lưu trữ | JSON (settings, lịch sử dọn) → SQLite khi cần log số liệu | Bắt đầu đơn giản |
| Target | macOS 14+, Apple silicon | Máy cá nhân, không cần hỗ trợ Intel |

**Không dùng:** App Sandbox (cần đọc toàn hệ thống), App Store, dependency bên ngoài ở giai đoạn đầu.

## Nguồn dữ liệu cho từng chỉ số

| Chỉ số | Nguồn | Độ khó | Ghi chú |
|---|---|---|---|
| CPU tổng / từng core | `host_processor_info(PROCESSOR_CPU_LOAD_INFO)` | Dễ | Đã có trong `CPUSampler` |
| P-core / E-core | `sysctl hw.perflevel0.logicalcpu`, `hw.perflevel1.logicalcpu` | Dễ | perflevel0 = P-core, xếp trước trong danh sách core |
| RAM | `host_statistics64(HOST_VM_INFO64)` | Dễ | Đã có trong `MemorySampler` |
| Memory pressure | `sysctl kern.memorystatus_vm_pressure_level` | Dễ | 1 bình thường · 2 cảnh báo · 4 nghiêm trọng |
| Swap | `sysctl vm.swapusage` | Dễ | |
| Mạng tổng | `getifaddrs` → `if_data` | Dễ | Bộ đếm 32-bit, tràn ở 4GB → M3 chuyển sang `NET_RT_IFLIST2` |
| Mạng theo app, connection | `nettop -P -L 1 -x` / `nettop -m tcp` | Trung bình | Parse output CSV; không cần root |
| Process | `libproc`: `proc_listallpids`, `proc_pidinfo(PROC_PIDTASKINFO)`, `proc_pidpath` | Dễ | Thời gian CPU ở đơn vị Mach → đổi bằng `mach_timebase_info` |
| RAM process (chuẩn AM) | `proc_pid_rusage` → `ri_phys_footprint` | Dễ | |
| Disk I/O theo app | `proc_pid_rusage` → `ri_diskio_bytesread/written` | Dễ | |
| Disk I/O toàn máy | IOKit `IOBlockStorageDriver` → `Statistics` | Trung bình | |
| Volume | `FileManager.mountedVolumeURLs` + `volumeAvailableCapacityForImportantUsageKey` | Dễ | |
| Icon app | `NSRunningApplication(processIdentifier:).icon`, `NSWorkspace.icon(forFile:)` | Dễ | |
| Pin | IOKit `AppleSmartBattery` (`CycleCount`, `MaxCapacity`, `DesignCapacity`) | Trung bình | |
| Năng lượng theo app, công suất | `powermetrics --samplers tasks,cpu_power,gpu_power -f plist` | Khó | **Cần root** — xem bên dưới |
| GPU % | IOKit `IOAccelerator` → `PerformanceStatistics["Device Utilization %"]` | Trung bình | |
| Nhiệt độ | SMC (`AppleSMC`) / IOHID sensors | Khó | Tham khảo exelban/stats (MIT) |
| Giữ máy thức | `pmset -g assertions` | Dễ | |

### Về `powermetrics` (cần root)

Phương án cho máy cá nhân, chọn một:

1. **Sudoers không hỏi mật khẩu cho riêng lệnh này** (đơn giản nhất):
   ```
   # sudo visudo -f /etc/sudoers.d/tg-vitals
   <tên-user> ALL=(root) NOPASSWD: /usr/bin/powermetrics
   ```
   App gọi `sudo -n /usr/bin/powermetrics ...`.
2. **LaunchDaemon** chạy `powermetrics` liên tục, ghi plist ra file tạm, app đọc file.

## Phần dọn rác (Care)

| Hạng mục | Cách làm |
|---|---|
| Tính dung lượng | `FileManager.enumerator` + `totalFileAllocatedSizeKey` (khớp Finder) |
| Xóa | **Chỉ** `FileManager.trashItem(at:resultingItemURL:)` |
| Cache tool dev | Ưu tiên lệnh chính chủ khi có: `brew cleanup`, `npm cache clean --force`, `pnpm store prune`, `docker system prune` |
| Quyền | Full Disk Access cấp tay một lần; kiểm tra bằng cách thử đọc `~/Library/Mail` |

## Công cụ dev

- Xcode 16+ hoặc Swift 6 toolchain
- `swift-format` (đi kèm toolchain) — `swift format -i -r Sources Tests`
- GitHub Actions `macos-latest` cho CI (M0)
- Tham khảo mã nguồn mở: **exelban/stats** (menu bar, SMC), **Mission Center**, **btop**
