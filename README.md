# tg-vitals

> Chỉ số sống của chiếc Mac: theo dõi hệ thống realtime + dọn rác an toàn.
> Anh em với **tg-studio**. Dùng cá nhân, chỉ chạy trên macOS (Apple silicon). Viết bằng Tauri 2 (Rust + React).

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
- Để build: Node 22, Rust (rustup), Xcode Command Line Tools

## Cài bản build sẵn

Tải `tg-vitals-macos-arm64.zip` ở tab **Actions** (artifact) hoặc **Releases**, giải nén, kéo vào `/Applications`.
App chỉ ký ad-hoc (không có Apple Developer ID) nên lần đầu cần gỡ cờ quarantine:

```bash
xattr -dr com.apple.quarantine /Applications/tg-vitals.app
```

## Chạy thử

```bash
cd app
npm ci
npm run tauri dev           # chạy app khi dev
npm run dev                 # chỉ xem UI trên trình duyệt, dữ liệu mẫu
cd src-tauri && cargo test -p vitals-core   # test logic
../../scripts/build-app.sh  # đóng gói tg-vitals.app
```

Đóng cửa sổ chính thì app vẫn chạy trên thanh menu; thoát hẳn bằng menu chuột phải trên icon hoặc nút ⏻ trong popup.

Một số tính năng cần quyền:
- **Full Disk Access** (System Settings → Privacy & Security) để phần Dọn dẹp quét được hết thư mục.
- Process của user khác (root) có thể thiếu số RAM / disk I/O vì macOS không cho app thường đọc.

## Tài liệu

| File | Nội dung |
|---|---|
| [`docs/PLAN.md`](docs/PLAN.md) | Lộ trình theo milestone, checklist công việc |
| [`docs/TECH_STACK.md`](docs/TECH_STACK.md) | Công nghệ, nguồn dữ liệu cho từng chỉ số |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Cấu trúc thư mục, luồng dữ liệu, quy ước |
| [`CLAUDE.md`](CLAUDE.md) | Hướng dẫn cho AI agent khi làm việc trong repo |
| [`AGENTS.md`](AGENTS.md) | Như trên, cho Codex |
