# Kiến trúc

## Cấu trúc thư mục

```
tg-vitals/
├── Package.swift
├── Sources/
│   ├── VitalsCore/               # Thư viện: thu thập số liệu + logic dọn rác. KHÔNG import SwiftUI
│   │   ├── Samplers/             # Mỗi file đọc một loại số liệu
│   │   │   ├── CPUSampler.swift
│   │   │   ├── MemorySampler.swift
│   │   │   ├── NetworkSampler.swift
│   │   │   └── ProcessSampler.swift
│   │   ├── Models/Snapshots.swift    # Struct snapshot thuần, Sendable
│   │   ├── Care/
│   │   │   ├── CleanupTarget.swift   # Danh mục chỗ chứa rác
│   │   │   ├── CleanupScanner.swift  # Chỉ đọc, tính dung lượng
│   │   │   ├── PathGuard.swift       # Luật đường dẫn nào được phép xóa
│   │   │   └── Trasher.swift         # Nơi DUY NHẤT được xóa (vào Thùng rác)
│   │   └── Shared/               # RingBuffer, Format
│   └── TGVitals/                 # App SwiftUI
│       ├── App/
│       │   ├── TGVitalsApp.swift     # @main: MenuBarExtra + cửa sổ chính
│       │   ├── SystemMonitor.swift   # Hẹn giờ 1s, gọi samplers, giữ lịch sử, @Observable
│       │   └── MenuBarView.swift
│       └── Features/
│           ├── MainView.swift        # Sidebar điều hướng
│           ├── Overview/
│           ├── Processes/
│           └── Care/                 # CareModel + CareView
├── Tests/VitalsCoreTests/
├── Resources/Info.plist
├── scripts/bundle.sh
└── docs/
```

## Luồng dữ liệu

```
 Timer 1s (SystemMonitor)
      │
      ├─► CPUSampler.sample()      ─┐
      ├─► MemorySampler.sample()    ├─► Snapshot structs ─► RingBuffer (60 điểm)
      ├─► NetworkSampler.sample()   │                          │
      └─► ProcessSampler.sample()  ─┘   (mỗi 2s)               ▼
                                                    @Observable SystemMonitor
                                                               │
                                     ┌─────────────────────────┼──────────────┐
                                     ▼                         ▼              ▼
                               MenuBarView               OverviewView    ProcessesView
```

Care chạy độc lập, chỉ khi người dùng bấm:

```
 Bấm "Quét" ─► CleanupScanner (Task nền) ─► [ScanResult] ─► CareView (tick chọn)
                                                                  │
                                                     Bấm "Dọn" ───┘
                                                                  ▼
                                               Trasher.trash(urls) ─► Thùng rác
```

## Quy ước

- **Sampler** là class/struct thuần, giữ trạng thái lần đo trước để tính delta; không import SwiftUI.
- **Snapshot** là struct `Sendable`, bất biến, dễ test.
- Logic tính toán (delta CPU, chuyển đơn vị) viết thành hàm `static` thuần để test được trên mọi máy.
- UI chỉ đọc từ `SystemMonitor`, không gọi sampler trực tiếp.
- Mọi thao tác xóa đi qua `Trasher` — không file nào khác được gọi `removeItem`.
- Lấy mẫu chi tiết (process, connection) chỉ khi cửa sổ chính đang hiện.

## Đóng gói

`scripts/bundle.sh` build release, tạo `build/tg-vitals.app` với `Info.plist`
(`LSUIElement = true` để ẩn khỏi Dock), rồi ký ad-hoc. Kéo app vào `/Applications` để dùng.
