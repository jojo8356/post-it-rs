//! Fullscreen overlay (colored backdrop behind the post-it)

use egui::{Color32, Context, LayerId, Order, Rect, Shape};
use std::time::{Duration, Instant};
use crate::config::OverlayConfig;

/// Overlay renderer for fullscreen colored backdrop
pub struct Overlay {
    config: OverlayConfig,
    animation_start: Option<Instant>,
    animation_duration: Duration,
    visible: bool,
    target_opacity: f32,
    current_opacity: f32,
}

impl Overlay {
    pub fn new(config: OverlayConfig) -> Self {
        let enabled = config.enabled;
        let opacity = if enabled { config.opacity } else { 0.0 };
        let animation_ms = config.animation_ms;
        Self {
            config,
            animation_start: None,
            animation_duration: Duration::from_millis(animation_ms),
            visible: enabled,
            target_opacity: opacity,
            current_opacity: opacity,
        }
    }

    /// Update configuration (for hot-reload)
    pub fn set_config(&mut self, config: OverlayConfig) {
        let was_enabled = self.config.enabled;
        let enabled = config.enabled;
        let animation_ms = config.animation_ms;
        self.config = config;
        self.animation_duration = Duration::from_millis(animation_ms);
        
        if enabled != was_enabled {
            self.set_visible(enabled);
        } else {
            self.target_opacity = if enabled { self.config.opacity } else { 0.0 };
            self.start_animation();
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.visible = visible;
            self.target_opacity = if visible { self.config.opacity } else { 0.0 };
            self.start_animation();
        }
    }

    fn start_animation(&mut self) {
        self.animation_start = Some(Instant::now());
    }

    /// Update animation state
    pub fn update(&mut self) {
        if let Some(start) = self.animation_start {
            let elapsed = start.elapsed();
            if elapsed >= self.animation_duration {
                self.current_opacity = self.target_opacity;
                self.animation_start = None;
            } else {
                let t = elapsed.as_secs_f32() / self.animation_duration.as_secs_f32();
                // Smooth easing
                let eased = Self::ease_out_cubic(t);
                self.current_opacity = self.current_opacity + (self.target_opacity - self.current_opacity) * eased;
            }
        }
    }

    fn ease_out_cubic(t: f32) -> f32 {
        1.0 - (1.0 - t).powi(3)
    }

    /// Paint the overlay on the full screen
    pub fn paint(&self, ctx: &Context, screen_rect: Rect) {
        if !self.visible && self.current_opacity <= 0.0 {
            return;
        }

        if self.current_opacity <= 0.0 {
            return;
        }

        let color = self.config.color.to_egui();
        let alpha = (color.a() as f32 / 255.0) * self.current_opacity;
        let final_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (alpha * 255.0) as u8);

        let painter = ctx.layer_painter(LayerId::new(Order::Background, egui::Id::new("postit_overlay")));
        
        // Full screen rectangle
        painter.add(Shape::rect_filled(
            screen_rect,
            0.0,
            final_color,
        ));

        // If click-through is enabled, we need to handle input differently
        // This is handled at the window level (WS_EX_TRANSPARENT on Windows, etc.)
    }

    /// Check if overlay is currently animating
    pub fn is_animating(&self) -> bool {
        self.animation_start.is_some()
    }

    /// Get current opacity (0.0 - 1.0)
    pub fn current_opacity(&self) -> f32 {
        self.current_opacity
    }

    /// Check if overlay is enabled in config
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Blur effect configuration (platform-specific implementation needed)
#[derive(Debug, Clone, Copy)]
pub struct BlurConfig {
    pub enabled: bool,
    pub radius: f32,
}

impl Default for BlurConfig {
    fn default() -> Self {
        Self { enabled: false, radius: 20.0 }
    }
}

/// Platform-specific window effects (blur, click-through, etc.)
pub mod platform {
    use anyhow::Result;
    use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

    /// Apply click-through to window (mouse events pass through)
    pub fn set_click_through(window: &dyn HasRawWindowHandle, enable: bool) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::*;
            use windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
            use windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
            use windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT;
            use windows::Win32::Foundation::BOOL;

            if let RawWindowHandle::Win32(handle) = window.raw_window_handle() {
                let hwnd = HWND(handle.hwnd as *mut _);
                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                
                let new_style = if enable {
                    ex_style | WS_EX_LAYERED.0 as isize | WS_EX_TRANSPARENT.0 as isize
                } else {
                    ex_style & !(WS_EX_TRANSPARENT.0 as isize)
                };
                
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
                
                // For layered window with transparency
                if enable {
                    SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
                }
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            // On Linux/X11, use _NET_WM_WINDOW_TYPE_DOCK or override_redirect
            // For Wayland, this is compositor-dependent
            // Implementation would use gtk-layer-shell or similar
        }
        
        #[cfg(target_os = "macos")]
        {
            // On macOS, use NSWindow.setIgnoresMouseEvents
        }
        
        Ok(())
    }

    /// Apply background blur (platform-specific)
    pub fn set_blur(window: &dyn HasRawWindowHandle, enable: bool, radius: f32) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            // Windows 11: DWM blur behind
            // Windows 10: Acrylic blur (requires undocumented APIs)
        }
        
        #[cfg(target_os = "linux")]
        {
            // KDE: KWindowEffects
            // GNOME: Mutter/GTK4 blur
            // wlroots: layer-shell with blur protocol
        }
        
        #[cfg(target_os = "macos")]
        {
            // NSVisualEffectView
        }
        
        Ok(())
    }

    /// Set window to always-on-top
    pub fn set_always_on_top(window: &dyn HasRawWindowHandle, enable: bool) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Foundation::{HWND, HWND_TOPMOST, HWND_NOTOPMOST};
            use windows::Win32::UI::WindowsAndMessaging::*;
            
            if let RawWindowHandle::Win32(handle) = window.raw_window_handle() {
                let hwnd = HWND(handle.hwnd as *mut _);
                let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
                let insert_after = if enable { HWND_TOPMOST } else { HWND_NOTOPMOST };
                SetWindowPos(hwnd, insert_after, 0, 0, 0, 0, flags);
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            // X11: _NET_WM_STATE_ABOVE
            // Wayland: layer-shell with layer=overlay
        }
        
        #[cfg(target_os = "macos")]
        {
            // NSWindow.level = .floating or .popUpMenu
        }
        
        Ok(())
    }

    /// Set window to not accept focus
    pub fn set_no_focus(window: &dyn HasRawWindowHandle, enable: bool) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::*;
            use windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
            use windows::Win32::UI::WindowsAndMessaging::WS_EX_NOACTIVATE;
            
            if let RawWindowHandle::Win32(handle) = window.raw_window_handle() {
                let hwnd = HWND(handle.hwnd as *mut _);
                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let new_style = if enable {
                    ex_style | WS_EX_NOACTIVATE.0 as isize
                } else {
                    ex_style & !(WS_EX_NOACTIVATE.0 as isize)
                };
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            // X11: _NET_WM_STATE_SKIP_TASKBAR | _NET_WM_STATE_SKIP_PAGER
            // Wayland: layer-shell with keyboard_interactivity=none
        }
        
        #[cfg(target_os = "macos")]
        {
            // NSWindow.collectionBehavior = .canJoinAllSpaces | .stationary | .ignoresCycle
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OverlayConfig;

    #[test]
    fn test_overlay_creation() {
        let config = OverlayConfig::default();
        let overlay = Overlay::new(config);
        assert!(overlay.is_enabled());
        assert_eq!(overlay.current_opacity(), 0.40);
    }

    #[test]
    fn test_overlay_visibility_toggle() {
        let config = OverlayConfig::default();
        let mut overlay = Overlay::new(config);
        
        overlay.set_visible(false);
        assert!(!overlay.is_animating() || overlay.current_opacity() < 0.40);
        
        overlay.set_visible(true);
        assert!(overlay.is_animating() || overlay.current_opacity() > 0.0);
    }

    #[test]
    fn test_easing() {
        assert_eq!(Overlay::ease_out_cubic(0.0), 0.0);
        assert_eq!(Overlay::ease_out_cubic(1.0), 1.0);
        assert!(Overlay::ease_out_cubic(0.5) > 0.5); // Ease out is faster at end
    }
}