//! Overlay mode: transparent always-on-top click-through window on Windows.
//!
//! ## Responsibility
//! Manage overlay window properties for the TFT Synapse UI, including
//! transparency and click-through behaviour.
//!
//! ## Guarantees
//! - `apply_overlay` never panics
//! - On non-Windows platforms, `apply_overlay` is a no-op returning `Ok(())`

/// Configuration for the overlay window.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayConfig {
    /// Window opacity: 0.0 = invisible, 1.0 = fully opaque.
    pub opacity: f32,
    /// If true, mouse clicks pass through to TFT underneath.
    pub click_through: bool,
    /// If true, window stays above all other windows.
    pub always_on_top: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            opacity: 0.85,
            click_through: false,
            always_on_top: true,
        }
    }
}

impl OverlayConfig {
    /// Flip the click-through flag.
    pub fn toggle_click_through(&mut self) {
        self.click_through = !self.click_through;
    }

    /// Set opacity, clamping to [0.1, 1.0].
    pub fn set_opacity(&mut self, v: f32) {
        self.opacity = v.clamp(0.1, 1.0);
    }
}

/// Apply overlay window properties.
///
/// On Windows: sets `WS_EX_LAYERED` for transparency and optionally
/// `WS_EX_TRANSPARENT` for click-through.
/// On other platforms: no-op, always returns `Ok(())`.
///
/// # Returns
/// - `Ok(())` on success or on non-Windows platforms
/// - `Err(String)` if a Windows API call fails
///
/// # Panics
/// This function never panics.
pub fn apply_overlay(config: &OverlayConfig) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        apply_overlay_windows(config)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = config;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn apply_overlay_windows(config: &OverlayConfig) -> Result<(), String> {
    use windows::Win32::Foundation::COLORREF;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW,
        GWL_EXSTYLE, LWA_ALPHA, WINDOW_EX_STYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        // HWND is a pointer-sized integer; a null handle means no foreground window.
        if hwnd.0 as usize == 0 {
            return Err("could not get foreground window".to_string());
        }

        let alpha = (config.opacity * 255.0) as u8;
        let mut ex_style =
            WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32);
        ex_style |= WS_EX_LAYERED;
        if config.click_through {
            ex_style |= WS_EX_TRANSPARENT;
        } else {
            ex_style &= !WS_EX_TRANSPARENT;
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style.0 as isize);
        SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_config_default_values() {
        let c = OverlayConfig::default();
        assert!((c.opacity - 0.85).abs() < f32::EPSILON);
        assert!(!c.click_through);
        assert!(c.always_on_top);
    }

    #[test]
    fn test_toggle_click_through_flips_false_to_true() {
        let mut c = OverlayConfig::default();
        assert!(!c.click_through);
        c.toggle_click_through();
        assert!(c.click_through);
    }

    #[test]
    fn test_toggle_click_through_flips_back_to_false() {
        let mut c = OverlayConfig::default();
        c.toggle_click_through();
        c.toggle_click_through();
        assert!(!c.click_through);
    }

    #[test]
    fn test_set_opacity_clamps_below_minimum() {
        let mut c = OverlayConfig::default();
        c.set_opacity(0.0);
        assert!((c.opacity - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_set_opacity_clamps_above_maximum() {
        let mut c = OverlayConfig::default();
        c.set_opacity(2.0);
        assert!((c.opacity - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_set_opacity_accepts_valid_midrange() {
        let mut c = OverlayConfig::default();
        c.set_opacity(0.5);
        assert!((c.opacity - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_apply_overlay_no_op_on_non_windows() {
        // On any platform where the Windows branch is not compiled the
        // function should succeed without side-effects.
        #[cfg(not(target_os = "windows"))]
        {
            let c = OverlayConfig::default();
            assert!(apply_overlay(&c).is_ok());
        }
        // On Windows we cannot test without a real window handle; the
        // unit tests for the config logic above cover the logic path.
        #[cfg(target_os = "windows")]
        {
            // At minimum, calling with default config does not panic.
            // It may return Ok or Err depending on whether there is a
            // foreground window — either is acceptable here.
            let c = OverlayConfig::default();
            let _ = apply_overlay(&c);
        }
    }

    #[test]
    fn test_multiple_toggles_preserve_other_fields() {
        let mut c = OverlayConfig::default();
        c.set_opacity(0.6);
        c.toggle_click_through();
        c.toggle_click_through();
        c.toggle_click_through();
        assert!(c.click_through);
        assert!((c.opacity - 0.6).abs() < f32::EPSILON);
        assert!(c.always_on_top);
    }
}
