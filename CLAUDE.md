# CLAUDE.md — hướng dẫn cho AI agent

tg-vitals: app macOS cá nhân (Swift 6, SwiftUI, SwiftPM) theo dõi hệ thống + dọn rác.
Đọc `docs/PLAN.md` để biết đang ở milestone nào, `docs/ARCHITECTURE.md` cho cấu trúc.

## Lệnh

- Build: `swift build`
- Test: `swift test`
- Chạy: `swift run TGVitals`
- Đóng gói: `./scripts/bundle.sh`

## Quy tắc

- Chỉ macOS 14+, Apple silicon. Không cần code cho Intel/Linux/Windows.
- Sampler không import SwiftUI. Logic tính toán để ở hàm `static` thuần và có test.
- **Xóa file: chỉ qua `Trasher` (dùng `trashItem`). Cấm `removeItem`, cấm `rm -rf`.**
- Không đụng đường dẫn hệ thống (`/System`, `/Library`, `/usr`, `/bin`, `/sbin`, `/Applications`).
- Không thêm dependency ngoài nếu Apple framework làm được.
- Sau khi làm xong một mục trong `docs/PLAN.md`, đánh dấu `[x]`.
- Commit message tiếng Anh, dạng `feat(care): ...`, `fix(cpu): ...`.
