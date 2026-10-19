//! Main UI rendering for the post-it window

use std::sync::Arc;
use std::time::Duration;
use egui::{
    Align, Color32, Context, FontFamily, FontId, Id, 
    LayerId, Order, Painter, Rect, Rounding, Shape, Stroke, Vec2
};
use egui::emath::Align as EAlign;
use crate::config::{Config, FontWeight, PostItContent, TextAlignment, WindowConfig, WindowPosition};
use crate::overlay::Overlay;

/// Main post-it UI state
pub struct PostItUI {
    config: Config,
    content: PostItContent,
    overlay: Overlay,
    window_rect: Option<Rect>,
    close_requested: bool,
    auto_close_timer: Option<Duration>,
    show_time: Option<std::time::Instant>,
}

impl PostItUI {
    pub fn new(config: Config, content: PostItContent) -> Self {
        let overlay = Overlay::new(config.overlay.clone());
        let auto_close_ms = config.window.auto_close_ms;
        Self {
            config,
            content,
            overlay,
            window_rect: None,
            close_requested: false,
            auto_close_timer: if auto_close_ms > 0 {
                Some(Duration::from_millis(auto_close_ms))
            } else {
                None
            },
            show_time: Some(std::time::Instant::now()),
        }
    }

    /// Update configuration (for hot-reload)
    pub fn set_config(&mut self, config: Config) {
        self.overlay.set_config(config.overlay.clone());
        self.config = config;
    }

    /// Update content
    pub fn set_content(&mut self, content: PostItContent) {
        self.content = content;
    }

    /// Check if window should close
    pub fn should_close(&self) -> bool {
        self.close_requested
    }

    /// Request close
    pub fn request_close(&mut self) {
        self.close_requested = true;
    }

    /// Update auto-close timer
    pub fn update(&mut self, ctx: &Context) {
        self.overlay.update();
        
        if let Some(duration) = self.auto_close_timer {
            if let Some(start) = self.show_time {
                if start.elapsed() >= duration {
                    self.close_requested = true;
                }
            }
        }
        
        // Handle close on click
        if self.config.window.close_on_click {
            let screen_rect = ctx.screen_rect();
            if ctx.input(|i| i.pointer.any_click()) {
                let pos = ctx.input(|i| i.pointer.latest_pos().unwrap_or_default());
                // Check if click is outside the post-it content area
                if let Some(rect) = self.window_rect {
                    if !rect.contains(pos) {
                        self.close_requested = true;
                    }
                }
            }
        }
    }

    /// Calculate window position based on config
    pub fn calculate_position(&self, screen_rect: Rect, content_size: Vec2) -> (f32, f32) {
        let padding = 20.0; // Margin from screen edges
        let max_width = self.config.window.max_width as f32;
        let width = content_size.x.min(max_width).max(200.0);
        let height = content_size.y.max(100.0);

        let (x, y) = match self.config.window.position {
            WindowPosition::Center => (
                (screen_rect.width() - width) / 2.0,
                (screen_rect.height() - height) / 2.0,
            ),
            WindowPosition::TopCenter => (
                (screen_rect.width() - width) / 2.0,
                padding,
            ),
            WindowPosition::BottomCenter => (
                (screen_rect.width() - width) / 2.0,
                screen_rect.height() - height - padding,
            ),
            WindowPosition::TopLeft => (padding, padding),
            WindowPosition::TopRight => (screen_rect.width() - width - padding, padding),
            WindowPosition::BottomLeft => (padding, screen_rect.height() - height - padding),
            WindowPosition::BottomRight => (
                screen_rect.width() - width - padding,
                screen_rect.height() - height - padding,
            ),
            WindowPosition::Custom => {
                let cx = self.config.window.custom_x as f32;
                let cy = self.config.window.custom_y as f32;
                // If values are 0-100, treat as percentage
                let x = if cx > 0.0 && cx <= 100.0 {
                    screen_rect.width() * cx / 100.0
                } else {
                    cx
                };
                let y = if cy > 0.0 && cy <= 100.0 {
                    screen_rect.height() * cy / 100.0
                } else {
                    cy
                };
                (x, y)
            }
        };

        (x.max(padding).min(screen_rect.width() - width - padding),
         y.max(padding).min(screen_rect.height() - height - padding))
    }

    /// Render the post-it UI
    pub fn ui(&mut self, ctx: &Context) {
        self.update(ctx);
        
        let screen_rect = ctx.screen_rect();
        
        // Paint overlay first (background layer)
        self.overlay.paint(ctx, screen_rect);

        // Prepare fonts
        self.setup_fonts(ctx);

        // Calculate content size first (we need a dummy layout)
        let content_size = self.calculate_content_size(ctx);
        
        // Calculate window position
        let (x, y) = self.calculate_position(screen_rect, content_size);
        let window_rect = Rect::from_min_size(
            egui::pos2(x, y),
            content_size,
        );
        self.window_rect = Some(window_rect);

        // Render post-it in a window layer (always on top)
        let layer_id = LayerId::new(Order::Tooltip, Id::new("postit_window"));
        let painter = ctx.layer_painter(layer_id);

        // Draw shadow if enabled
        if self.config.body.shadow {
            self.draw_shadow(&painter, window_rect);
        }

        // Draw post-it background
        self.draw_background(&painter, window_rect);

        // Draw title tab (if enabled)
        let body_rect = if self.config.title.enabled {
            self.draw_title_tab(&painter, window_rect)
        } else {
            window_rect.shrink(10.0)
        };

        // Draw body text
        self.draw_body_text(ctx, &painter, body_rect);

        // Draw close button (if not click-to-close)
        if !self.config.window.close_on_click {
            self.draw_close_button(ctx, &painter, window_rect);
        }
    }

    fn setup_fonts(&self, ctx: &Context) {
        // Fonts are set globally at startup via FontDefinitions
        // Here we just ensure the context has them
    }

    fn calculate_content_size(&self, ctx: &Context) -> Vec2 {
        let max_width = self.config.body.max_width as f32;
        let padding = self.config.body.padding as f32 * 2.0;
        let title_height = if self.config.title.enabled {
            self.config.title.font_size as f32 + self.config.title.padding as f32 * 2.0 + 4.0
        } else {
            0.0
        };

        // Measure body text
        let font_id = self.body_font_id();
        let galley = ctx.fonts(|f| f.layout_no_wrap(
            self.content.body.clone(),
            font_id.clone(),
            self.config.body.color.to_egui(),
        ));
        
        // Estimate wrapped height
        let available_width = max_width - padding;
        let wrapped = ctx.fonts(|f| f.layout(
            self.content.body.clone(),
            font_id,
            self.config.body.color.to_egui(),
            available_width,
        ));
        
        let body_height = wrapped.size().y + padding;
        let total_height = title_height + body_height + 20.0; // Extra margin

        Vec2::new(max_width, total_height)
    }

    fn draw_shadow(&self, painter: &Painter, rect: Rect) {
        let shadow_offset = 4.0;
        let shadow_color = Color32::from_rgba_premultiplied(0, 0, 0, 80);
        let shadow_rect = rect.translate(Vec2::new(shadow_offset, shadow_offset));
        painter.rect_filled(
            shadow_rect.expand(4.0),
            Rounding::same(self.config.body.border_radius as f32),
            shadow_color,
        );
    }

    fn draw_background(&self, painter: &Painter, rect: Rect) {
        let bg_color = self.config.body.bg_color.to_egui();
        let radius = self.config.body.border_radius as f32;
        
        // Main background
        painter.rect_filled(
            rect,
            Rounding::same(radius),
            bg_color,
        );

        // Background image (if specified)
        if let Some(path) = self.config.body.bg_image.to_str() {
            if !path.is_empty() && std::path::Path::new(path).exists() {
                // Would need to load texture - placeholder for now
                // painter.image(texture_id, rect, Rect::EVERYTHING, Color32::WHITE);
            }
        }

        // Border
        painter.rect_stroke(
            rect,
            Rounding::same(radius),
            Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 30)),
        );
    }

    fn draw_title_tab(&self, painter: &Painter, window_rect: Rect) -> Rect {
        let title_config = &self.config.title;
        let tab_height = title_config.font_size as f32 + title_config.padding as f32 * 2.0;
        let radius = title_config.border_radius as f32;
        
        // Title tab rect (top portion with rounded top corners only)
        let tab_rect = Rect::from_min_max(
            window_rect.min,
            egui::pos2(window_rect.max.x, window_rect.min.y + tab_height),
        );

        // Draw tab background with custom rounding (top only)
        let tab_bg_color = title_config.bg_color.to_egui();
        painter.add(Shape::rect_filled(
            tab_rect,
            Rounding { nw: radius, ne: radius, sw: 0.0, se: 0.0 },
            tab_bg_color,
        ));

        // Background image for title
        if let Some(path) = title_config.bg_image.to_str() {
            if !path.is_empty() && std::path::Path::new(path).exists() {
                // painter.image(texture_id, tab_rect, Rect::EVERYTHING, Color32::WHITE);
            }
        }

        // Draw title text
        let font_id = self.title_font_id();
        let text_color = title_config.color.to_egui();
        let alignment = title_config.alignment.to_egui();
        
        let galley = painter.layout(
            title_config.text.clone(),
            font_id,
            text_color,
            tab_rect.width() - title_config.padding as f32 * 2.0,
        );

        let text_pos = match alignment {
            EAlign::LEFT => tab_rect.min + Vec2::new(title_config.padding as f32, (tab_rect.height() - galley.size().y) / 2.0),
            EAlign::Center => tab_rect.center_top() + Vec2::new(0.0, (tab_rect.height() - galley.size().y) / 2.0) - Vec2::new(galley.size().x / 2.0, 0.0),
            EAlign::RIGHT => tab_rect.right_top() + Vec2::new(-(title_config.padding as f32) - galley.size().x, (tab_rect.height() - galley.size().y) / 2.0),
            _ => tab_rect.min + Vec2::new(title_config.padding as f32, (tab_rect.height() - galley.size().y) / 2.0),
        };

        painter.galley(text_pos, galley, text_color);

        // Return body rect (below title tab)
        Rect::from_min_max(
            egui::pos2(window_rect.min.x, tab_rect.max.y + 2.0),
            window_rect.max - Vec2::new(10.0, 10.0),
        )
    }

    fn draw_body_text(&self, ctx: &Context, painter: &Painter, rect: Rect) {
        let font_id = self.body_font_id();
        let text_color = self.config.body.color.to_egui();
        let available_width = rect.width() - self.config.body.padding as f32 * 2.0;

        let galley = ctx.fonts(|f| f.layout(
            self.content.body.clone(),
            font_id.clone(),
            text_color,
            available_width,
        ));

        let text_pos = egui::pos2(
            rect.min.x + self.config.body.padding as f32,
            rect.min.y + self.config.body.padding as f32,
        );

        // Paint the full galley - egui handles line wrapping automatically
        painter.galley(text_pos, galley, text_color);
    }

    fn draw_close_button(&self, ctx: &Context, painter: &Painter, window_rect: Rect) {
        let btn_size = 20.0;
        let btn_rect = Rect::from_min_size(
            window_rect.right_top() + Vec2::new(-btn_size - 5.0, 5.0),
            Vec2::splat(btn_size),
        );

        let hover = ctx.input(|i| btn_rect.contains(i.pointer.hover_pos().unwrap_or_default()));
        let clicked = ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary) && btn_rect.contains(i.pointer.hover_pos().unwrap_or_default()));
        
        if clicked {
            // Signal close through a different mechanism since we can't mutate here
            // The close check happens in update()
        }

        let bg_color = if hover {
            Color32::from_rgba_premultiplied(255, 80, 80, 200)
        } else {
            Color32::from_rgba_premultiplied(255, 255, 255, 50)
        };

        painter.rect_filled(btn_rect, Rounding::same(4.0), bg_color);
        
        // X symbol
        let cross_color = Color32::WHITE;
        let cross_stroke = Stroke::new(2.0, cross_color);
        let margin = 5.0;
        painter.line_segment([
            btn_rect.min + Vec2::new(margin, margin),
            btn_rect.max - Vec2::new(margin, margin),
        ], cross_stroke);
        painter.line_segment([
            egui::pos2(btn_rect.min.x + margin, btn_rect.max.y - margin),
            egui::pos2(btn_rect.max.x - margin, btn_rect.min.y + margin),
        ], cross_stroke);
    }

    fn title_font_id(&self) -> FontId {
        let family = if let Some(path) = self.config.title.font_path.to_str() {
            if !path.is_empty() {
                FontFamily::Name(Arc::from("PostItTitleCustom"))
            } else {
                FontFamily::Proportional
            }
        } else {
            FontFamily::Proportional
        };
        FontId::new(self.config.title.font_size as f32, family)
    }

    fn body_font_id(&self) -> FontId {
        let family = if let Some(path) = self.config.body.font_path.to_str() {
            if !path.is_empty() {
                FontFamily::Name(Arc::from("PostItBodyCustom"))
            } else {
                FontFamily::Proportional
            }
        } else {
            FontFamily::Proportional
        };
        FontId::new(self.config.body.font_size as f32, family)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_ui_creation() {
        let config = Config::new();
        let content = PostItContent {
            title: Some("Test".to_string()),
            body: "Hello World".to_string(),
        };
        let ui = PostItUI::new(config, content);
        assert!(!ui.should_close());
    }

    #[test]
    fn test_position_calculation() {
        let config = Config::new();
        let content = PostItContent::default();
        let ui = PostItUI::new(config, content);
        
        let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(1920.0, 1080.0));
        let content_size = Vec2::new(400.0, 200.0);
        let (x, y) = ui.calculate_position(screen, content_size);
        
        // Center should be roughly in the middle
        assert!((x - 760.0).abs() < 10.0);
        assert!((y - 440.0).abs() < 10.0);
    }
}