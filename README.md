# tg-vitals

> Chỉ số sống của chiếc Mac: theo dõi hệ thống realtime + dọn rác an toàn.
> Anh em với **tg-studio**. Dùng cá nhân, chỉ chạy trên macOS (Apple silicon).

## Tính năng

**Theo dõi (Vitals)**
- Icon menu bar hiện CPU / RAM / mạng / nhiệt độ, bấm vào xem chi tiết 60 giây gần nhất
- CPU từng core, tách P-core và E-core, user/system
- RAM: app, wired, compressed, cached, memory pressure, swap
- Mạng: tốc độ lên/xuống theo interface và theo app, danh sách connection
- Disk: tốc độ đọc/ghi, các volume, app ghi nhiều nhất
- Năng lượng & nhiệt: pin, công suất, năng lượng theo app, cảm biến nhiệt
- Danh sách process: sort, search, nhóm theo app, quit / force quit, xem cây parent–children

**Dọn dẹp (Care)**
- Quét rồi mới xóa: luôn hiện danh sách + dung lượng, bạn tick chọn
- Mọi thứ bị xóa đều vào **Thùng rác**, không xóa vĩnh viễn
- Mục tiêu: cache user, log cũ, Xcode DerivedData/Archives/Simulators, `node_modules` của project bỏ lâu, cache Homebrew/npm/pnpm/yarn/pip, Docker, file lớn, sót lại của app đã gỡ

## Yêu cầu

- macOS 14 Sonoma trở lên, Apple silicon
- Xcode 16+ (hoặc Swift 6 Command Line Tools)

## Chạy thử

```bash
swift build
swift run TGVitals          # chạy nhanh khi dev (sẽ có icon ở Dock)
swift test                  # chạy test
./scripts/bundle.sh         # đóng gói thành build/tg-vitals.app (ẩn khỏi Dock)
open build/tg-vitals.app
```

Mở bằng Xcode: `open Package.swift`.

Một số tính năng cần quyền:
- **Full Disk Access** (System Settings → Privacy & Security) để phần Care quét được hết thư mục.
- **sudo** cho `powermetrics` (năng lượng theo app, GPU) — xem `docs/TECH_STACK.md`.

## Tài liệu

| File | Nội dung |
|---|---|
| [`docs/PLAN.md`](docs/PLAN.md) | Lộ trình theo milestone, checklist công việc |
| [`docs/TECH_STACK.md`](docs/TECH_STACK.md) | Công nghệ, nguồn dữ liệu cho từng chỉ số |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Cấu trúc thư mục, luồng dữ liệu, quy ước |
| [`CLAUDE.md`](CLAUDE.md) | Hướng dẫn cho AI agent khi làm việc trong repo |
