# Kiến trúc

App là **Tauri 2**: backend Rust gọi thẳng API của macOS, frontend React (UI kiểu Pulse).
Bản Swift cũ (`Sources/`, `Tests/`, `Package.swift`) chỉ còn để tham khảo, sẽ xóa khi bản Tauri chạy ổn.

## Cấu trúc thư mục

```
tg-vitals/
├── app/                              # App Tauri
│   ├── src/                          # Frontend React + TypeScript
│   │   ├── api/                      # types.ts (khớp model Rust), index.ts (invoke / mock), mock.ts
│   │   ├── useVitals.ts              # Poll `snapshot` mỗi 1s, giữ lịch sử 150 điểm; dừng khi cửa sổ ẩn
│   │   ├── App.tsx                   # Cửa sổ chính: sidebar + các trang
│   │   ├── TrayPanel.tsx             # Popup dưới icon menu bar (index.html?view=tray)
│   │   ├── pages/                    # Overview · Processes · Detail (CPU/RAM/Disk/Mạng/Năng lượng) · Cleanup
│   │   └── components/               # DotChart, Donut, Sidebar, ui
│   └── src-tauri/                    # Workspace Rust
│       ├── core/                     # crate `vitals-core` — KHÔNG có code UI, test chạy được trên Linux
│       │   └── src/
│       │       ├── model.rs          # Snapshot (serde camelCase, khớp types.ts)
│       │       ├── cpu.rs            # host_processor_info, delta tick, P-core trước
│       │       ├── memory.rs         # host_statistics64, swap, pressure
│       │       ├── network.rs        # sysctl NET_RT_IFLIST2 (64-bit)
│       │       ├── process.rs        # libproc: taskinfo, bsdinfo, rusage (phys_footprint, disk I/O)
│       │       ├── sensors.rs        # IOKit: pin, disk I/O, nhiệt độ (IOHID), statfs
│       │       ├── iokit.rs          # FFI IOKit/IOHID tối thiểu + RAII
│       │       ├── icon.rs           # NSWorkspace → PNG 64px
│       │       ├── sampler.rs        # Gộp tất cả thành một Snapshot
│       │       └── care/
│       │           ├── targets.rs    # Danh mục chỗ chứa rác
│       │           ├── scanner.rs    # Chỉ đọc, tính dung lượng (allocated, khớp Finder)
│       │           ├── path_guard.rs # Luật đường dẫn nào được phép xóa
│       │           └── trasher.rs    # Nơi DUY NHẤT được xóa (vào Thùng rác qua trashItemAtURL)
│       ├── src/                      # crate app `tg-vitals`
│       │   ├── lib.rs                # Builder: cửa sổ, scheme appicon://, sự kiện
│       │   ├── state.rs              # AppState + thread lấy mẫu 1s
│       │   ├── commands.rs           # snapshot, kill_process, reveal_path, scan_cleanup, trash_paths…
│       │   └── tray.rs               # Item menu bar (chữ + icon template) + popup
│       ├── icons/                    # Icon app (nguồn SVG trong icons/source)
│       └── tauri.conf.json           # Cửa sổ `main` + `tray`, bundle .app ký ad-hoc
├── scripts/build-app.sh              # Đóng gói tg-vitals.app
└── docs/
```

## Luồng dữ liệu

```
 Thread "sampler" (1s, state.rs)
      │  Sampler::sample(with_processes)
      │    cpu · memory · disk · network · battery · temperature
      │    + process list (chỉ khi có cửa sổ đang xem trong 3s gần nhất)
      ├─► tray::update_title  → chữ trên menu bar "23% 61% ↓1.2M 47°"
      └─► AppState.latest (Mutex<Snapshot>)
                │
      invoke("snapshot") mỗi 1s từ useVitals (cửa sổ chính hoặc popup, chỉ khi đang hiện)
                ▼
      React: lịch sử 150 điểm → DotChart, bảng process, inspector
```

Icon process: `ProcessRow.icon = "appicon://localhost/Applications/Slack.app"` →
handler trong `lib.rs` render PNG trên main thread lần đầu, sau đó lấy từ cache.

Care chạy độc lập, chỉ khi người dùng bấm:

```
 Bấm "Quét" ─► scan_cleanup (thread nền) ─► Scanner::scan_all ─► [CleanupGroup] (nhớ lại danh sách path)
                                                                     │ tick chọn
                                                        Bấm "Dọn" ──┘ (xác nhận lần 2)
                                                                     ▼
                         trash_paths ─► chỉ path có trong lần quét + PathGuard ─► trashItemAtURL ─► Thùng rác
```

## Quy ước

- Logic tính toán (delta CPU, phân loại RAM, lọc interface, PathGuard…) là hàm thuần có test trong `vitals-core`.
- Code gọi API macOS nằm sau `#[cfg(target_os = "macos")]`; test máy thật (sampler, icon, Thùng rác) chạy trên CI macOS.
- Struct trong `model.rs` / `care/mod.rs` và `app/src/api/types.ts` phải khớp nhau.
- Mọi thao tác xóa đi qua `care::trasher` — không file nào khác được gọi `remove_file`/`remove_dir_all`.
- Lấy mẫu process chỉ khi có cửa sổ đang hiện.

## Đóng gói

`./scripts/build-app.sh` (hoặc `cd app && npm run tauri build -- --target universal-apple-darwin --bundles app`) tạo
`app/src-tauri/target/universal-apple-darwin/release/bundle/macos/tg-vitals.app` (universal: arm64 + x86_64), ký ad-hoc (`signingIdentity: "-"`).
CI làm việc này trên mỗi lần chạy và đính kèm vào Release khi push tag `v*`.
