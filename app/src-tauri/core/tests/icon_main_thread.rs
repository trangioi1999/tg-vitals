//! AppKit icon rendering must run on the main thread (as the app does), so this test
//! uses its own `main` instead of libtest's worker threads.
fn main() {
    #[cfg(target_os = "macos")]
    {
        let png = vitals_core::icon::png_for_path("/System/Library/CoreServices/Finder.app", 64).expect("icon");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(png.len() > 500);
        println!("icon ok: {} bytes", png.len());
    }
}
