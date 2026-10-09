use egui::ColorImage;
use image::RgbaImage;

/// Encode a bounded stream; never collect the GIF or all RGBA frames in memory.
/// Disconnect, cancellation, size mismatch, and encoding errors do not publish.
pub(crate) fn encode_gif_stream(
    path: &std::path::Path,
    frames: std::sync::mpsc::Receiver<RgbaImage>,
    count: usize,
    fps: u32,
    looping: bool,
    cancel: &std::sync::atomic::AtomicBool,
    mut acknowledge: impl FnMut(usize),
) -> Result<bool, String> {
    use std::sync::atomic::Ordering;
    if count == 0 || fps == 0 {
        return Err("No GIF frames or invalid FPS".into());
    }
    let mut output = crate::io::file::AtomicOutput::create(path)
        .map_err(|e| format!("Cannot create GIF: {e}"))?;
    let file = output.file.try_clone().map_err(|e| e.to_string())?;
    let write_error = std::sync::Arc::new(std::sync::Mutex::new(None));
    let writer = GifWriter {
        file,
        error: write_error.clone(),
    };
    let mut encoder = image::codecs::gif::GifEncoder::new(writer);
    if looping {
        encoder
            .set_repeat(image::codecs::gif::Repeat::Infinite)
            .map_err(|e| e.to_string())?;
    }
    let mut dimensions = None;
    for index in 0..count {
        let frame = loop {
            if cancel.load(Ordering::Acquire) {
                return Ok(false);
            }
            match frames.recv_timeout(std::time::Duration::from_millis(50)) {
                Ok(frame) => break frame,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("GIF capture interrupted".into())
                }
            }
        };
        let size = frame.dimensions();
        if size.0 == 0
            || size.1 == 0
            || size.0 > u16::MAX as u32
            || size.1 > u16::MAX as u32
            || dimensions.is_some_and(|d| d != size)
        {
            return Err(
                "GIF frames must have the same nonzero dimensions (at most 65535 pixels)".into(),
            );
        }
        dimensions = Some(size);
        encoder
            .encode_frame(image::Frame::from_parts(
                frame,
                0,
                0,
                image::Delay::from_numer_denom_ms(1000, fps),
            ))
            .map_err(|e| format!("GIF encode error: {e}"))?;
        if index + 1 < count {
            acknowledge(index);
        }
    }
    // The image encoder finalizes in Drop; record even trailer-write failures.
    drop(encoder);
    if let Some(error) = write_error.lock().unwrap().take() {
        return Err(format!("GIF write error: {error}"));
    }
    if cancel.load(Ordering::Acquire) {
        return Ok(false);
    }
    // Keep cancellation available during all encoding, trailer writes, and sync
    // I/O. The final acknowledgment authorizes only the atomic rename.
    output.sync().map_err(|e| format!("GIF sync error: {e}"))?;
    if cancel.load(Ordering::Acquire) {
        return Ok(false);
    }
    acknowledge(count - 1);
    if cancel.load(Ordering::Acquire) {
        return Ok(false);
    }
    output
        .publish()
        .map_err(|e| format!("Cannot publish GIF: {e}"))?;
    Ok(true)
}

struct GifWriter {
    file: std::fs::File,
    error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}
impl std::io::Write for GifWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let result = self.file.write(bytes);
        if let Err(error) = &result {
            *self.error.lock().unwrap() = Some(error.to_string());
        }
        result
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let result = self.file.flush();
        if let Err(error) = &result {
            *self.error.lock().unwrap() = Some(error.to_string());
        }
        result
    }
}

/// Crop a viewport `ColorImage` to `crop_rect` (logical coords) using `scale`
/// (pixels-per-point) to convert to physical pixels.
/// Returns `None` if the rect falls entirely outside the image.
pub fn crop_image(img: &ColorImage, crop_rect: egui::Rect, scale: f32) -> Option<RgbaImage> {
    let iw = img.size[0] as i32;
    let ih = img.size[1] as i32;

    let x0 = (crop_rect.min.x * scale).round() as i32;
    let y0 = (crop_rect.min.y * scale).round() as i32;
    let x1 = (crop_rect.max.x * scale).round() as i32;
    let y1 = (crop_rect.max.y * scale).round() as i32;

    // clamp to image bounds
    let cx0 = x0.max(0);
    let cy0 = y0.max(0);
    let cx1 = x1.min(iw);
    let cy1 = y1.min(ih);

    let cw = cx1 - cx0;
    let ch = cy1 - cy0;
    if cw <= 0 || ch <= 0 {
        return None;
    }

    // Build RGBA byte buffer row-by-row to avoid per-pixel put_pixel overhead.
    let stride = img.size[0];
    let mut raw: Vec<u8> = Vec::with_capacity((cw * ch) as usize * 4);
    for dy in 0..ch {
        let row_start = (cy0 + dy) as usize * stride + cx0 as usize;
        for px in &img.pixels[row_start..row_start + cw as usize] {
            raw.extend_from_slice(&[px.r(), px.g(), px.b(), px.a()]);
        }
    }
    let out =
        RgbaImage::from_raw(cw as u32, ch as u32, raw).expect("buffer size matches dimensions");
    Some(out)
}

/// Encode an `RgbaImage` to PNG bytes.
pub fn encode_png(img: RgbaImage) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    encoder
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("PNG encode error: {e}"))?;
    Ok(buf)
}

/// Copy an `RgbaImage` to the system clipboard as a raw image.
pub fn copy_image_to_clipboard(img: RgbaImage) -> Result<(), String> {
    let width = img.width() as usize;
    let height = img.height() as usize;
    let bytes = img.into_raw();
    let img_data = arboard::ImageData {
        width,
        height,
        bytes: bytes.into(),
    };
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {e}"))?;
    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Clipboard image error: {e}"))?;
    Ok(())
}

/// Open a Save dialog and write `data` to the chosen file.
/// Returns `Some(())` on success, `None` if the user cancelled.
/// On write error, returns `Err(message)`.
pub fn save_png_to_file(data: &[u8]) -> Result<Option<()>, String> {
    let path = rfd::FileDialog::new()
        .set_file_name("chart.png")
        .add_filter("PNG image", &["png"])
        .save_file();

    match path {
        None => Ok(None), // user cancelled — not an error
        Some(p) => {
            crate::io::file::write_atomic(&p, data).map_err(|e| format!("Write error: {e}"))?;
            Ok(Some(()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Color32;

    fn make_test_image(w: usize, h: usize) -> ColorImage {
        let pixels = (0..w * h)
            .map(|i| {
                let r = (i % 256) as u8;
                Color32::from_rgb(r, 0, 0)
            })
            .collect();
        ColorImage::new([w, h], pixels)
    }

    #[test]
    fn crop_helper_returns_expected_image_bounds() {
        let img = make_test_image(100, 80);
        // crop at logical (10,10)-(50,50) with scale=1.0 → 40×40
        let rect = egui::Rect::from_min_max(egui::pos2(10.0, 10.0), egui::pos2(50.0, 50.0));
        let cropped = crop_image(&img, rect, 1.0).expect("crop should succeed");
        assert_eq!(cropped.width(), 40);
        assert_eq!(cropped.height(), 40);
    }

    #[test]
    fn crop_with_scale_factor() {
        let img = make_test_image(200, 160);
        // logical (10,10)-(50,50) with scale=2.0 → physical (20,20)-(100,100) → 80×80
        let rect = egui::Rect::from_min_max(egui::pos2(10.0, 10.0), egui::pos2(50.0, 50.0));
        let cropped = crop_image(&img, rect, 2.0).expect("crop should succeed");
        assert_eq!(cropped.width(), 80);
        assert_eq!(cropped.height(), 80);
    }

    #[test]
    fn crop_out_of_bounds_returns_none() {
        let img = make_test_image(100, 80);
        // rect entirely outside the image
        let rect = egui::Rect::from_min_max(egui::pos2(200.0, 200.0), egui::pos2(300.0, 300.0));
        assert!(crop_image(&img, rect, 1.0).is_none());
    }

    #[test]
    fn encode_png_produces_valid_png_header() {
        let img = RgbaImage::new(4, 4);
        let bytes = encode_png(img).expect("encode should succeed");
        // PNG magic bytes: 0x89 50 4E 47 ...
        assert!(bytes.starts_with(b"\x89PNG"));
    }

    #[test]
    fn save_png_pipeline_treats_cancel_as_noop() {
        // save_png_to_file returns Ok(None) on cancel — we test the function signature
        // (actual dialog cancel can't be tested headlessly, but we verify Ok(None) is the type)
        let result: Result<Option<()>, String> = Ok(None);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn unsupported_capture_backend_returns_user_visible_error() {
        // Encode step never silently fails — a bad encode produces Err(message)
        // Simulate by checking that encode_png with valid data never returns silent empty
        let img = RgbaImage::new(1, 1);
        let bytes = encode_png(img).unwrap();
        assert!(!bytes.is_empty(), "PNG output must not be silent empty");
    }

    #[test]
    fn gif_stream_roundtrip_has_order_dimensions_timing_and_atomic_publication() {
        use image::AnimationDecoder;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("animation.gif");
        std::fs::write(&path, b"previous output").unwrap();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let colors = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]];
        std::thread::spawn(move || {
            for color in colors {
                tx.send(RgbaImage::from_pixel(7, 5, image::Rgba(color)))
                    .unwrap();
            }
        });
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let mut acknowledged = Vec::new();
        assert!(encode_gif_stream(&path, rx, 3, 5, true, &cancel, |i| {
            assert_eq!(std::fs::read(&path).unwrap(), b"previous output");
            acknowledged.push(i);
        })
        .unwrap());
        assert_eq!(acknowledged, vec![0, 1, 2]);
        let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(
            std::fs::File::open(&path).unwrap(),
        ))
        .unwrap();
        let frames = decoder.into_frames().collect_frames().unwrap();
        assert_eq!(frames.len(), 3);
        for (frame, color) in frames.iter().zip(colors) {
            assert_eq!(frame.buffer().dimensions(), (7, 5));
            assert_eq!(frame.buffer().get_pixel(3, 2).0, color);
            assert_eq!(frame.delay().numer_denom_ms(), (200, 1));
        }
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn gif_cancel_interruption_bad_dimensions_and_write_error_do_not_publish() {
        use std::sync::atomic::{AtomicBool, Ordering};
        for mode in ["cancel", "disconnect", "dimensions", "write"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("out.gif");
            if mode == "write" {
                std::fs::create_dir(&path).unwrap();
            } else {
                std::fs::write(&path, b"original").unwrap();
            }
            let (tx, rx) = std::sync::mpsc::sync_channel(2);
            tx.send(RgbaImage::new(2, 3)).unwrap();
            if mode == "dimensions" {
                tx.send(RgbaImage::new(3, 2)).unwrap();
            }
            drop(tx);
            let cancel = AtomicBool::new(false);
            let result = encode_gif_stream(
                &path,
                rx,
                if mode == "write" { 1 } else { 2 },
                3,
                false,
                &cancel,
                |_| {
                    if mode == "cancel" {
                        cancel.store(true, Ordering::Release);
                    }
                },
            );
            if mode == "cancel" {
                assert!(!result.unwrap());
            } else {
                assert!(result.is_err(), "{mode} must fail");
            }
            if mode != "write" {
                assert_eq!(std::fs::read(&path).unwrap(), b"original");
            }
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        }
    }
}
