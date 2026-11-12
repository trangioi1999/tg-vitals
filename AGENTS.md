# AGENTS.md — hướng dẫn cho Codex và các agent khác

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

## Việc tiếp theo nên giao (theo docs/PLAN.md)

1. M0: chạy `swift build && swift test`, sửa hết lỗi compile/test. Không đổi hành vi.
2. M1: thêm icon app cho từng process trong `ProcessesView` (`NSRunningApplication`).
3. M2: nối `CleanupScanner.staleNodeModules` vào màn Care, thư mục gốc mặc định `~/WebstormProjects`.
4. M3: chuyển `NetworkSampler` sang `sysctl(NET_RT_IFLIST2)` + `if_data64`.

Mỗi việc: một branch, một commit, chạy lại `swift test` trước khi xong, đánh dấu `[x]` trong docs/PLAN.md.
