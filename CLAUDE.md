# CLAUDE.md — hướng dẫn cho AI agent

tg-vitals: app macOS cá nhân (Tauri 2: Rust + React) theo dõi hệ thống + dọn rác.
Đọc `docs/PLAN.md` để biết đang ở milestone nào, `docs/ARCHITECTURE.md` cho cấu trúc.
Bản Swift cũ (`Sources/`, `Package.swift`) chỉ để tham khảo, không phát triển thêm.

## Lệnh (chạy trong `app/`)

- Cài: `npm ci`
- Chạy dev: `npm run tauri dev`
- Xem UI trên trình duyệt (dữ liệu mẫu): `npm run dev`
- Build frontend: `npm run build`
- Test Rust: `cd src-tauri && cargo test -p vitals-core` (test máy thật: thêm `-- --include-ignored`, chỉ trên macOS)
- Đóng gói: `../scripts/build-app.sh`

## Quy tắc

- Chỉ macOS 14+, Apple silicon. Không cần code cho Intel/Linux/Windows (chỉ cần `vitals-core` compile được trên Linux để chạy test logic).
- `vitals-core` không có code UI. Logic tính toán để ở hàm thuần và có test.
- **Xóa file: chỉ qua `care::trasher` (dùng `trashItemAtURL`). Cấm `remove_file`/`remove_dir_all`, cấm `rm -rf`.**
- Không đụng đường dẫn hệ thống (`/System`, `/Library`, `/usr`, `/bin`, `/sbin`, `/Applications`).
- Không thêm dependency ngoài nếu Apple framework làm được.
- Đổi struct trong `model.rs` thì sửa `app/src/api/types.ts` (và `mock.ts`) cho khớp.
- Sau khi làm xong một mục trong `docs/PLAN.md`, đánh dấu `[x]`.
- Commit message tiếng Anh, dạng `feat(care): ...`, `fix(cpu): ...`.
