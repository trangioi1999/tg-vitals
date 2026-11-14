# Gỡ app (App uninstaller) — design

Ngày: 2026-11-14 · Trạng thái: đã duyệt (người dùng: "cứ học hỏi theo Mole")

## Vì sao

Trang Dọn dẹp chỉ trả lời "rác ở đâu". Người dùng muốn biết **app nào đang chiếm nhiều dung lượng** và
**gỡ hẳn app đó cùng dữ liệu nó để lại**, giống `mo uninstall` của [Mole](https://github.com/tw93/mole)
(GPL-3: chỉ học ý tưởng, không chép code). Ví dụ thật: GlobalProtect bị xóa app nhưng còn LaunchAgent /
LaunchDaemon chạy lỗi liên tục và cache thuộc root — phải dọn tay.

## Mục tiêu

- Mục sidebar mới **"Gỡ app"** (nhóm Hệ thống, dưới "Dọn dẹp").
- Danh sách app trong `/Applications` và `~/Applications`: icon, tên, phiên bản, dung lượng app,
  dung lượng dữ liệu sót lại, lần dùng gần nhất, đang chạy hay không. Mặc định sắp theo tổng dung lượng;
  đổi được sang "lâu không dùng". Có ô tìm kiếm.
- Chọn một app → bảng chi tiết: bản thân app + từng mục dữ liệu sót lại (đường dẫn, dung lượng),
  tick sẵn tất cả → **Gỡ** → xác nhận một lần → mọi thứ vào **Thùng rác**.
- App thuộc quyền root (bộ Office, Teams, Goodnotes… 13/31 app trên máy hiện tại) gỡ được qua hộp thoại
  mật khẩu admin chuẩn của macOS (người dùng chọn phương án A).

## Không làm (v1)

- Không gỡ app hệ thống của Apple (`com.apple.*`, mọi thứ trong `/System`), không gỡ chính tg-vitals.
- Không gỡ system extension / kernel extension (cần tắt SIP).
- Không chạy `brew uninstall`; app cài bằng Homebrew cask chỉ hiện ghi chú gợi ý lệnh.
- Không quét "dữ liệu mồ côi" của app đã gỡ từ trước (để mục riêng trong PLAN).

## Không được xảy ra (anti-goals)

- Xóa dữ liệu dùng chung với app khác còn cài (vd. `Group Containers/UBF8T346G9.Office` khi còn Excel,
  `Application Support/JetBrains` khi còn IDE JetBrains khác).
- Xóa vĩnh viễn bất cứ thứ gì, hoặc chạm vào path không nằm trong lần quét gần nhất.
- Gỡ app đang chạy.

## Ràng buộc

- Sửa `CLAUDE.md`: nới quy tắc "không đụng `/Applications`" **chỉ cho tính năng Gỡ app**, chỉ app trong
  lần liệt kê gần nhất, chỉ đưa vào Thùng rác. Quy tắc "chỉ xóa qua `care::trasher`" giữ nguyên: thêm
  hàm mới trong `trasher` thay vì gọi API khác ở chỗ khác.
- Universal (Apple silicon + Intel); logic thuần trong `vitals-core` có test chạy được trên Linux.
- Không thêm dependency: AppKit/Foundation qua `objc2` đã có.

## Các cách đã cân nhắc

| | Cách | Đánh đổi |
|---|---|---|
| **A (chọn)** | Gỡ cả app thuộc root qua hộp thoại mật khẩu của macOS | Gỡ được hết, kể cả bộ Office (~12 GB); có thao tác cần quyền admin |
| B | Chỉ gỡ app thuộc quyền người dùng | An toàn hơn nhưng 13/31 app (lớn nhất) phải gỡ bằng Finder |

Cách cấp quyền cho A (quyết định của người làm, xem dưới): `NSWorkspace.recycleURLs` thay vì
`osascript … with administrator privileges` + `mv`, vì nó vẫn là "đưa vào Thùng rác" của hệ thống,
tự hỏi mật khẩu khi cần, không chạy shell và không phải tự escape đường dẫn.

## Thiết kế

### Backend (`vitals-core`, module mới `care::apps`)

- `list_apps(roots) -> Vec<AppInfo>`: đọc `Contents/Info.plist` (bundle id, tên hiển thị, phiên bản),
  chủ sở hữu (uid), dung lượng bundle (`dir_size`), lần dùng gần nhất (`kMDItemLastUsedDate` qua
  Spotlight; không có thì `None`), đang chạy (`NSRunningApplication`), cài bằng Homebrew (đường dẫn
  `/opt/homebrew/Caskroom/<token>` hoặc `/usr/local/Caskroom` trỏ tới app). Bỏ `com.apple.*` và tg-vitals.
- `find_leftovers(app, installed: &[AppInfo]) -> Vec<CleanupItem>` (hàm thuần trên danh sách tên thư
  mục, có test):
  - Khớp **chính xác bundle id** hoặc **bundle id + "."** (extension của chính app, vd.
    `com.microsoft.Excel.widgetextension`) trong:
    `~/Library/{Application Support, Application Scripts, Caches, Containers, HTTPStorages, WebKit,
    Saved Application State (<id>.savedState), Preferences (<id>.plist, ByHost/<id>.*.plist),
    LaunchAgents (<id>*.plist), Logs}` (danh sách theo Mole).
  - Khớp **theo tên app** chỉ ở `Application Support` và `Logs`, và chỉ khi không app nào khác đang cài
    có cùng tên đó hoặc cùng tiền tố nhà phát hành (vd. `JetBrains` bị loại khi còn IDE JetBrains khác).
  - `Group Containers`: chỉ khi tên là `<TEAMID>.<bundle id>` hoặc thư mục có tiền tố team chỉ app này
    dùng (không app nào còn cài cùng team id). Team id lấy từ chữ ký (`codesign`-free: đọc
    `Contents/_CodeSignature` không đủ → dùng `SecStaticCodeCopySigningInformation` của Security.framework;
    nếu không lấy được thì **không** nhận Group Container nào).
  - Mức hệ thống (thuộc root, cần mật khẩu): `/Library/{LaunchAgents, LaunchDaemons}/<id>*.plist`,
    `/Library/PrivilegedHelperTools/<id>*`, `/Library/{Application Support, Caches, Logs}/<tên|id>`,
    `/Library/Preferences/<id>.plist` — cùng luật khớp như trên. Sau khi gỡ: `pkgutil --forget` các
    receipt có id bắt đầu bằng bundle id (như khi dọn GlobalProtect), trong cùng lần xin quyền.
    Ghi chú cho người dùng: dịch vụ nền đã nạp sẽ dừng hẳn sau khi khởi động lại máy.
  - Bỏ mục không đo/đọc được (thiếu Full Disk Access với `Containers`): hiện "không đo được", không tick.
- `trasher::recycle_with_auth(paths)`: gọi `NSWorkspace.recycleURLs` (hỏi mật khẩu khi có item thuộc
  root), trả `TrashReport`. Chỉ nhận path có trong lần liệt kê/tìm leftovers gần nhất (giống
  `scan_cleanup` → `trash_paths`), và `PathGuard` mở rộng có kiểm soát: cho `/Applications/<x>.app`,
  `~/Applications/<x>.app`, và các path mức hệ thống ở trên khi đến từ `find_leftovers`.

### Lệnh Tauri

- `list_apps()` → danh sách (chạy nền, có thể vài giây vì tính dung lượng; stream tiến trình nếu > 2s,
  dùng lại kiểu terminal của Dọn dẹp ở dạng gọn).
- `app_leftovers(bundle_path)` → các mục sót lại của một app.
- `quit_app_by_bundle(bundle_id)` → thoát app đang chạy (`NSRunningApplication.terminate`).
- `uninstall_app(paths)` → `recycle_with_auth`, sau đó làm mới danh sách.

### UI (`app/src/pages/Uninstall.tsx`)

- Bố cục giống trang Tiến trình: bảng bên trái (icon dùng lại scheme `appicon://`, tên, dung lượng app,
  dữ liệu, lần dùng), bảng chi tiết bên phải.
- Chi tiết: hero (icon, tên, phiên bản, tổng dung lượng sẽ giải phóng), danh sách app + leftovers có
  checkbox như trang Dọn dẹp, badge "cần mật khẩu admin" cho item thuộc root, badge "Homebrew" kèm lệnh
  gợi ý, nút **Thoát app** nếu đang chạy (nút Gỡ bị khóa cho tới khi thoát).
- Xác nhận: nút đổi thành "Xác nhận gỡ <tên> · <dung lượng>" (giống Dọn dẹp). Kết quả: số mục vào Thùng
  rác, mục thất bại kèm lý do.
- Dữ liệu mẫu (`mock.ts`) cho chế độ trình duyệt.

### Kiểm thử

- Thuần (Linux được): khớp leftovers theo bundle id / extension / tên / team, guard dùng chung
  (Office, JetBrains), loại `com.apple.*`, PathGuard cho path app.
- macOS: `list_apps` đọc được Info.plist thật; `recycle_with_auth` thử trên một `.app` giả trong thư mục
  tạm thuộc quyền người dùng (không bao giờ thử trên app thật trong test).

## Rủi ro cần kiểm tra trước

- `NSWorkspace.recycleURLs` có tự hỏi mật khẩu cho item thuộc root không: thử bằng một spike nhỏ
  **trước** khi làm phần còn lại. Nếu không, phương án dự phòng là
  `osascript "do shell script … with administrator privileges"` chuyển vào `~/.Trash` (đã dùng thành công
  khi dọn GlobalProtect), đường dẫn truyền qua tham số đã escape, vẫn đặt trong `care::trasher`.

## Để lại cho người làm

- Ngưỡng hiển thị (ẩn leftovers < 1 KB?), cách sắp xếp phụ, chữ trên UI.
- Có cache danh sách app giữa các lần mở trang hay không.
