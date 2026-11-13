//! App icons as small PNGs (NSWorkspace → 64×64 bitmap). Cached by the caller.
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace, NSGraphicsContext, NSWorkspace,
};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};

/// PNG bytes of the Finder icon for `path` (an .app bundle or any file), `px` pixels square.
pub fn png_for_path(path: &str, px: u32) -> Option<Vec<u8>> {
    objc2::rc::autoreleasepool(|_| {
        let image = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path));
        let size = NSSize::new(px as f64, px as f64);
        image.setSize(size);

        // SAFETY: null planes → AppKit allocates the buffer.
        let rep: Retained<NSBitmapImageRep> = unsafe {
            NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
                NSBitmapImageRep::alloc(),
                std::ptr::null_mut(),
                px as isize,
                px as isize,
                8,
                4,
                true,
                false,
                NSDeviceRGBColorSpace,
                0,
                0,
            )
        }?;

        let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep)?;
        NSGraphicsContext::saveGraphicsState_class();
        NSGraphicsContext::setCurrentContext(Some(&context));
        let rect = NSRect::new(NSPoint::new(0.0, 0.0), size);
        image.drawInRect_fromRect_operation_fraction(rect, NSRect::ZERO, NSCompositingOperation::SourceOver, 1.0);
        context.flushGraphics();
        NSGraphicsContext::restoreGraphicsState_class();

        // SAFETY: empty properties dictionary is valid for PNG.
        let data = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }?;
        Some(data.to_vec())
    })
}
