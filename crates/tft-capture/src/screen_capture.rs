//! # Screen Capture Reader
//!
//! ## Responsibility
//! Provide a best-effort fallback game-state reader that captures the TFT
//! window via Win32 BitBlt and parses pixel data for HP, gold, and round.
//!
//! ## Guarantees
//! - Never panics; all fallible operations return `Result` or `Option`
//! - On non-Windows platforms, `capture_screen` returns `Err(PlatformNotSupported)`
//! - `poll` always returns `Ok(None)` instead of propagating pixel-parse failures
//!
//! ## NOT Responsible For
//! - Precise OCR / text recognition (pixel brightness heuristics only)
//! - Cross-resolution normalisation beyond the 1080p reference layout

use tft_types::{GameState, RoundInfo, ShopSlot, TftError};
use crate::reader::{GameStateReader, ReaderMode};

/// Screen-capture based fallback reader.
/// Uses Win32 BitBlt on Windows; no-ops gracefully on other platforms.
pub struct ScreenCaptureReader {
    /// Logical capture width (pixels).
    width: u32,
    /// Logical capture height (pixels).
    height: u32,
    /// Whether screen capture is available on this platform.
    enabled: bool,
}

impl ScreenCaptureReader {
    /// Construct a new reader.  Automatically detects whether the platform
    /// supports Win32 screen capture.
    pub fn new() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self { width: 1920, height: 1080, enabled: true }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self { width: 0, height: 0, enabled: false }
        }
    }

    /// Returns `true` if the platform supports screen capture.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Attempt to capture the primary screen into a flat BGRA byte buffer.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)` — flat BGRA pixel buffer of length `width * height * 4`
    /// - `Err(TftError::PlatformNotSupported)` — on non-Windows
    /// - `Err(TftError::Capture)` — on Win32 failure
    fn capture_screen(&self) -> Result<Vec<u8>, TftError> {
        #[cfg(target_os = "windows")]
        {
            self.capture_screen_windows()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(TftError::PlatformNotSupported(
                "screen capture requires Windows".to_string(),
            ))
        }
    }

    /// Win32 BitBlt screen capture implementation.
    #[cfg(target_os = "windows")]
    fn capture_screen_windows(&self) -> Result<Vec<u8>, TftError> {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
            GetDIBits, GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
            DIB_RGB_COLORS, SRCCOPY,
        };

        let w = self.width as i32;
        let h = self.height as i32;

        // SAFETY: all Win32 handles are checked for null and freed in reverse order.
        unsafe {
            let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
            if hdc_screen.is_invalid() {
                return Err(TftError::Capture("GetDC failed".to_string()));
            }

            let hdc_mem = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_invalid() {
                ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
                return Err(TftError::Capture("CreateCompatibleDC failed".to_string()));
            }

            let hbm = CreateCompatibleBitmap(hdc_screen, w, h);
            if hbm.is_invalid() {
                DeleteDC(hdc_mem).ok()
                    .map_err(|_| TftError::Capture("DeleteDC failed".to_string()))?;
                ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
                return Err(TftError::Capture("CreateCompatibleBitmap failed".to_string()));
            }

            let old_obj = SelectObject(hdc_mem, hbm);

            let blt_ok = BitBlt(hdc_mem, 0, 0, w, h, hdc_screen, 0, 0, SRCCOPY);

            // Prepare to read pixels back as 32-bit BGRA
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [Default::default()],
            };

            let pixel_count = (w * h) as usize;
            let mut pixels: Vec<u8> = vec![0u8; pixel_count * 4];

            let scan_result = GetDIBits(
                hdc_mem,
                hbm,
                0,
                h as u32,
                Some(pixels.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            // Cleanup — run regardless of earlier errors
            SelectObject(hdc_mem, old_obj);
            let _ = DeleteObject(hbm);
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

            if blt_ok.is_err() {
                return Err(TftError::Capture("BitBlt failed".to_string()));
            }
            if scan_result == 0 {
                return Err(TftError::Capture("GetDIBits failed".to_string()));
            }

            Ok(pixels)
        }
    }

    /// Parse health percentage (0–100) from a pixel buffer.
    ///
    /// Examines the top-left HP bar region (roughly x: 0–200, y: 0–30 in a
    /// 1920×1080 layout).  A green-dominant pixel population above a threshold
    /// is treated as the filled portion of the bar.
    ///
    /// Returns 0 if the buffer is too small to read the region.
    pub fn parse_hp(pixels: &[u8], width: u32, height: u32) -> u8 {
        if width == 0 || height == 0 || pixels.len() < (width * height * 4) as usize {
            return 0;
        }

        // HP bar region: top-left corner, 200 wide × 30 tall (scaled to image)
        let bar_w = (width / 10).min(200);
        let bar_h = (height / 36).min(30);

        let mut green_px: u32 = 0;
        let mut total_px: u32 = 0;

        for row in 0..bar_h {
            for col in 0..bar_w {
                let idx = ((row * width + col) * 4) as usize;
                if idx + 3 >= pixels.len() {
                    break;
                }
                let b = pixels[idx] as u32;
                let g = pixels[idx + 1] as u32;
                let r = pixels[idx + 2] as u32;
                total_px += 1;
                // Green-dominant pixel = health bar fill
                if g > 100 && g > r.saturating_add(30) && g > b.saturating_add(30) {
                    green_px += 1;
                }
            }
        }

        if total_px == 0 {
            return 0;
        }

        // Scale the fraction to 0–100
        let fraction = (green_px * 100) / total_px;
        fraction.min(100) as u8
    }

    /// Parse gold amount (0–99) from a pixel buffer.
    ///
    /// Examines the bottom HUD region (y: 90–100% height, x: 40–60% width).
    /// Counts bright yellow pixels as a proxy for the gold digit display.
    /// Returns 0 when the buffer is too small or no yellow pixels are found.
    pub fn parse_gold(pixels: &[u8], width: u32, height: u32) -> u8 {
        if width == 0 || height == 0 || pixels.len() < (width * height * 4) as usize {
            return 0;
        }

        // Gold HUD region: bottom-center strip
        let x0 = width * 2 / 5;
        let x1 = width * 3 / 5;
        let y0 = height * 9 / 10;
        let y1 = height;

        let mut yellow_px: u32 = 0;

        for row in y0..y1 {
            for col in x0..x1 {
                let idx = ((row * width + col) * 4) as usize;
                if idx + 3 >= pixels.len() {
                    break;
                }
                let b = pixels[idx] as u32;
                let g = pixels[idx + 1] as u32;
                let r = pixels[idx + 2] as u32;
                // Yellow = high R, high G, low B
                if r > 180 && g > 160 && b < 80 {
                    yellow_px += 1;
                }
            }
        }

        // Heuristic: each gold "digit segment" contributes ~4 bright pixels.
        // Cap at 99 — TFT gold maximum is 99 (with economy augments).
        (yellow_px / 4).min(99) as u8
    }

    /// Approximate round info from the pixel buffer.
    ///
    /// Uses screen brightness in the stage indicator region as a proxy for
    /// stage number.  This is a coarse approximation and returns stage 1/round 1
    /// when nothing useful can be detected.
    fn parse_round(pixels: &[u8], width: u32, height: u32) -> RoundInfo {
        if width == 0 || height == 0 || pixels.len() < (width * height * 4) as usize {
            return RoundInfo { stage: 1, round: 1 };
        }

        // Stage indicator: top-center region
        let x0 = width * 45 / 100;
        let x1 = width * 55 / 100;
        let y0 = 0u32;
        let y1 = height / 20;

        let mut brightness_sum: u64 = 0;
        let mut count: u64 = 0;

        for row in y0..y1 {
            for col in x0..x1 {
                let idx = ((row * width + col) * 4) as usize;
                if idx + 2 >= pixels.len() {
                    break;
                }
                let b = pixels[idx] as u64;
                let g = pixels[idx + 1] as u64;
                let r = pixels[idx + 2] as u64;
                brightness_sum += r + g + b;
                count += 1;
            }
        }

        if count == 0 {
            return RoundInfo { stage: 1, round: 1 };
        }

        let avg = brightness_sum / (count * 3);
        // Map average brightness (0-255) to a stage guess (1-7)
        let stage = ((avg * 7) / 255).clamp(1, 7) as u8;
        RoundInfo { stage, round: 1 }
    }
}

impl Default for ScreenCaptureReader {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStateReader for ScreenCaptureReader {
    /// Poll the screen for current game state.
    ///
    /// Returns `Ok(None)` on any parse failure — screen capture is best-effort.
    fn poll(&self) -> Result<Option<GameState>, TftError> {
        if !self.enabled {
            return Ok(None);
        }

        let pixels = match self.capture_screen() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };

        let hp = Self::parse_hp(&pixels, self.width, self.height);
        let gold = Self::parse_gold(&pixels, self.width, self.height);
        let round = Self::parse_round(&pixels, self.width, self.height);

        let state = GameState {
            round,
            board: vec![],
            bench: vec![None; 9],
            shop: (0..5)
                .map(|_| ShopSlot { champion_id: None, cost: 0, locked: false, sold: false })
                .collect(),
            gold,
            hp,
            level: 1,
            xp: 0,
            streak: 0,
            current_augments: vec![],
            augment_choices: None,
            active_traits: vec![],
        };

        Ok(Some(state))
    }

    fn mode(&self) -> ReaderMode {
        ReaderMode::ScreenCapture
    }

    fn is_connected(&self) -> bool {
        self.enabled
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Build a flat BGRA pixel buffer of the given dimensions, all zeroes.
    fn blank_pixels(w: u32, h: u32) -> Vec<u8> {
        vec![0u8; (w * h * 4) as usize]
    }

    /// Paint a rectangle in a pixel buffer with a given (B, G, R, A) colour.
    fn fill_rect(pixels: &mut [u8], w: u32, x0: u32, y0: u32, x1: u32, y1: u32, bgra: [u8; 4]) {
        for row in y0..y1 {
            for col in x0..x1 {
                let idx = ((row * w + col) * 4) as usize;
                if idx + 3 < pixels.len() {
                    pixels[idx] = bgra[0];
                    pixels[idx + 1] = bgra[1];
                    pixels[idx + 2] = bgra[2];
                    pixels[idx + 3] = bgra[3];
                }
            }
        }
    }

    // ── ScreenCaptureReader construction ─────────────────────────────────────

    #[test]
    fn test_new_returns_reader() {
        let r = ScreenCaptureReader::new();
        // On Windows enabled=true, elsewhere false.
        // Either way the struct is valid.
        let _ = r.is_enabled();
    }

    #[test]
    fn test_default_equals_new() {
        let a = ScreenCaptureReader::new();
        let b = ScreenCaptureReader::default();
        assert_eq!(a.enabled, b.enabled);
        assert_eq!(a.width, b.width);
        assert_eq!(a.height, b.height);
    }

    // ── mode() ───────────────────────────────────────────────────────────────

    #[test]
    fn test_mode_is_screen_capture() {
        let r = ScreenCaptureReader::new();
        assert_eq!(r.mode(), ReaderMode::ScreenCapture);
    }

    // ── is_connected() / is_enabled() ────────────────────────────────────────

    #[test]
    fn test_is_connected_matches_enabled() {
        let r = ScreenCaptureReader::new();
        assert_eq!(r.is_connected(), r.is_enabled());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_non_windows_is_not_enabled() {
        let r = ScreenCaptureReader::new();
        assert!(!r.is_enabled());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_is_enabled() {
        let r = ScreenCaptureReader::new();
        assert!(r.is_enabled());
    }

    // ── parse_hp ─────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_hp_empty_buffer_returns_zero() {
        assert_eq!(ScreenCaptureReader::parse_hp(&[], 0, 0), 0);
    }

    #[test]
    fn test_parse_hp_undersized_buffer_returns_zero() {
        // Buffer too small for declared dimensions
        assert_eq!(ScreenCaptureReader::parse_hp(&[0u8; 4], 100, 100), 0);
    }

    #[test]
    fn test_parse_hp_all_black_returns_zero() {
        let w = 100u32;
        let h = 100u32;
        let pixels = blank_pixels(w, h);
        let hp = ScreenCaptureReader::parse_hp(&pixels, w, h);
        assert_eq!(hp, 0);
    }

    #[test]
    fn test_parse_hp_full_green_bar_returns_high_value() {
        let w = 1920u32;
        let h = 1080u32;
        let mut pixels = blank_pixels(w, h);
        // Paint the entire HP bar region bright green (BGRA: 0, 200, 0, 255)
        let bar_w = (w / 10).min(200);
        let bar_h = (h / 36).min(30);
        fill_rect(&mut pixels, w, 0, 0, bar_w, bar_h, [0, 200, 0, 255]);
        let hp = ScreenCaptureReader::parse_hp(&pixels, w, h);
        assert!(hp > 50, "expected hp > 50 for full green bar, got {}", hp);
    }

    #[test]
    fn test_parse_hp_result_in_range() {
        let w = 1920u32;
        let h = 1080u32;
        let mut pixels = blank_pixels(w, h);
        // Half the bar green
        let bar_w = (w / 10).min(200);
        let bar_h = (h / 36).min(30);
        fill_rect(&mut pixels, w, 0, 0, bar_w / 2, bar_h, [0, 200, 0, 255]);
        let hp = ScreenCaptureReader::parse_hp(&pixels, w, h);
        assert!(hp <= 100, "hp must be <= 100, got {}", hp);
    }

    // ── parse_gold ────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_gold_empty_buffer_returns_zero() {
        assert_eq!(ScreenCaptureReader::parse_gold(&[], 0, 0), 0);
    }

    #[test]
    fn test_parse_gold_all_black_returns_zero() {
        let w = 100u32;
        let h = 100u32;
        let pixels = blank_pixels(w, h);
        assert_eq!(ScreenCaptureReader::parse_gold(&pixels, w, h), 0);
    }

    #[test]
    fn test_parse_gold_yellow_pixels_returns_nonzero() {
        let w = 1920u32;
        let h = 1080u32;
        let mut pixels = blank_pixels(w, h);
        // Paint gold-coloured strip in the HUD region (BGRA: 20, 200, 220, 255)
        let x0 = w * 2 / 5;
        let x1 = w * 3 / 5;
        let y0 = h * 9 / 10;
        let y1 = h;
        fill_rect(&mut pixels, w, x0, y0, x1, y1, [20, 200, 220, 255]);
        let gold = ScreenCaptureReader::parse_gold(&pixels, w, h);
        assert!(gold > 0, "expected gold > 0 for yellow HUD pixels, got {}", gold);
    }

    #[test]
    fn test_parse_gold_result_capped_at_99() {
        let w = 1920u32;
        let h = 1080u32;
        let mut pixels = blank_pixels(w, h);
        // Fill the entire image with yellow — should never exceed 99
        fill_rect(&mut pixels, w, 0, 0, w, h, [20, 200, 220, 255]);
        let gold = ScreenCaptureReader::parse_gold(&pixels, w, h);
        assert!(gold <= 99, "gold must be <= 99, got {}", gold);
    }

    // ── non-Windows fallback ──────────────────────────────────────────────────

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_poll_non_windows_returns_ok_none() {
        let r = ScreenCaptureReader::new();
        let result = r.poll();
        assert!(result.is_ok());
        assert!(result.expect("poll failed in test").is_none());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_capture_screen_non_windows_returns_platform_error() {
        let r = ScreenCaptureReader::new();
        let err = r.capture_screen().expect_err("expected error on non-Windows");
        assert!(
            matches!(err, TftError::PlatformNotSupported(_)),
            "unexpected error variant: {:?}",
            err
        );
    }
}
