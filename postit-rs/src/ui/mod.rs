//! Main UI rendering for the post-it window

use std::sync::Arc;
use std::time::Duration;
use egui::{
    Color32, Context, FontFamily, FontId, Id,
    LayerId, Order, Painter, Rect, Rounding, Shape, Stroke, Vec2
};
use egui::emath::Align as EAlign;
use crate::config::{Config, PostItContent, WindowPosition};
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
    settings_open: bool,
    settings_toml: String,
    settings_status: String,
    save_path: Option<std::path::PathBuf>,
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
            settings_open: false,
            settings_toml: String::new(),
            settings_status: String::new(),
            save_path: Config::default_config_path(),
        }
    }

    /// Set the file used by the settings window (including CLI --config paths).
    pub fn set_save_path(&mut self, path: Option<std::path::PathBuf>) {
        self.save_path = path;
    }

    /// Update configuration (for hot-reload)
    pub fn set_config(&mut self, config: Config) {
        self.overlay.set_config(config.overlay.clone());
        self.auto_close_timer = (config.window.auto_close_ms > 0)
            .then(|| Duration::from_millis(config.window.auto_close_ms));
        self.show_time = Some(std::time::Instant::now());
        self.config = config;
    }

    /// Current preview configuration, shared with the application event handler.
    pub fn config(&self) -> &Config {
        &self.config
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

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.settings_open {
                self.settings_open = false;
            } else {
                self.close_requested = true;
            }
        }
        if self.settings_open {
            // Editing should not dismiss the card or expire its auto-close timer.
            self.show_time = Some(std::time::Instant::now());
            return;
        }

        if let Some(duration) = self.auto_close_timer {
            if let Some(start) = self.show_time {
                if start.elapsed() >= duration {
                    self.close_requested = true;
                }
            }
        }
        
        // Handle close on click
        if self.config.window.close_on_click {
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
        let layer_id = LayerId::new(Order::Middle, Id::new("postit_window"));
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
        if !self.config.window.close_on_click && !self.settings_open {
            self.draw_close_button(ctx, &painter, window_rect);
        }
        self.draw_settings_button(ctx, &painter, window_rect);
        self.settings_window(ctx);
    }

    fn setup_fonts(&self, _ctx: &Context) {
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
        // Estimate wrapped height
        let available_width = max_width - padding;
        let wrapped = ctx.fonts(|f| f.layout(
            self.content.body.clone(),
            font_id,
            self.config.body.color.to_egui(),
            available_width,
        ));
        
        let body_height = wrapped.size().y + padding;
        let total_height = title_height + body_height + 50.0; // Margin and settings footer

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
        };

        painter.galley(text_pos, galley, text_color);
        painter.line_segment(
            [egui::pos2(tab_rect.left() + 12.0, tab_rect.bottom()),
             egui::pos2(tab_rect.right() - 12.0, tab_rect.bottom())],
            Stroke::new(1.0_f32, Color32::from_black_alpha(55)),
        );

        // Return body rect (below title tab)
        Rect::from_min_max(
            egui::pos2(window_rect.min.x, tab_rect.max.y + 2.0),
            window_rect.max - Vec2::new(10.0, 42.0),
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

    fn draw_close_button(&mut self, ctx: &Context, painter: &Painter, window_rect: Rect) {
        let btn_size = 20.0;
        let btn_rect = Rect::from_min_size(
            window_rect.right_top() + Vec2::new(-btn_size - 5.0, 5.0),
            Vec2::splat(btn_size),
        );

        let hover = ctx.input(|i| btn_rect.contains(i.pointer.hover_pos().unwrap_or_default()));
        let clicked = ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary) && btn_rect.contains(i.pointer.hover_pos().unwrap_or_default()));
        
        if clicked {
            self.close_requested = true;
        }

        let bg_color = if hover {
            Color32::from_rgb(185, 28, 28)
        } else {
            Color32::from_rgb(220, 38, 38)
        };

        painter.rect_filled(btn_rect, Rounding::same(4.0), bg_color);
        
        // X symbol
        let cross_color = Color32::WHITE;
        let cross_stroke = Stroke::new(2.0_f32, cross_color);
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

    fn draw_settings_button(&mut self, ctx: &Context, painter: &Painter, rect: Rect) {
        let button = Rect::from_min_size(
            rect.right_bottom() - Vec2::new(32.0, 32.0), Vec2::splat(26.0),
        );
        let hovered = ctx.input(|i| i.pointer.hover_pos().is_some_and(|p| button.contains(p)));
        if hovered && !self.settings_open {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            painter.rect_filled(button, Rounding::same(5.0), Color32::from_black_alpha(25));
            if ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary)) {
                self.settings_open = true;
                self.settings_toml = toml::to_string_pretty(&self.config).unwrap_or_default();
                self.settings_status.clear();
            }
        }
        // Vector gear: independent of installed fonts and emoji support.
        let center = button.center();
        let color = self.config.body.color.to_egui();
        let stroke = Stroke::new(1.8_f32, color);
        painter.circle_stroke(center, 6.0, stroke);
        painter.circle_stroke(center, 2.2, stroke);
        for tooth in 0..8 {
            let angle = tooth as f32 * std::f32::consts::TAU / 8.0;
            let direction = Vec2::new(angle.cos(), angle.sin());
            painter.line_segment([center + direction * 6.0, center + direction * 9.0], stroke);
        }
    }

    fn settings_window(&mut self, ctx: &Context) {
        if !self.settings_open { return; }
        let mut open = true;
        let mut edited = self.config.clone();
        let mut apply_toml = false;
        let mut save = false;
        egui::Window::new("Paramètres du post-it")
            .id(Id::new("postit_settings"))
            .open(&mut open)
            .order(Order::Foreground)
            .default_width(460.0)
            .default_pos(ctx.screen_rect().center() - Vec2::new(230.0, 220.0))
            .frame(egui::Frame::window(&ctx.style()).fill(Color32::from_rgb(30, 33, 40)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(500.0).show(ui, |ui| {
                    ui.label("Aperçu en direct · Échap ferme les paramètres");
                    ui.separator();
                    ui.checkbox(&mut edited.title.enabled, "Afficher le titre");
                    ui.horizontal(|ui| {
                        ui.label("Titre");
                        ui.text_edit_singleline(&mut edited.title.text);
                    });
                    ui.label("Texte du post-it (aperçu uniquement)");
                    ui.text_edit_multiline(&mut self.content.body);
                    ui.add(egui::Slider::new(&mut edited.title.font_size, 8..=64).text("Taille du titre"));
                    ui.add(egui::Slider::new(&mut edited.body.font_size, 8..=64).text("Taille du texte"));
                    ui.add(egui::Slider::new(&mut edited.body.max_width, 200..=1000).text("Largeur"));
                    ui.add(egui::Slider::new(&mut edited.body.padding, 0..=60).text("Marge intérieure"));
                    ui.add(egui::Slider::new(&mut edited.body.border_radius, 0..=40).text("Arrondi"));
                    ui.checkbox(&mut edited.body.shadow, "Ombre");
                    Self::color_control(ui, "Fond du titre", &mut edited.title.bg_color);
                    Self::color_control(ui, "Texte du titre", &mut edited.title.color);
                    Self::color_control(ui, "Fond du post-it", &mut edited.body.bg_color);
                    Self::color_control(ui, "Texte", &mut edited.body.color);
                    ui.separator();
                    ui.checkbox(&mut edited.overlay.enabled, "Calque derrière le post-it");
                    ui.add(egui::Slider::new(&mut edited.overlay.opacity, 0.0..=1.0).text("Opacité du calque"));
                    Self::color_control(ui, "Couleur du calque", &mut edited.overlay.color);
                    ui.checkbox(&mut edited.trigger.on_boot, "Afficher au démarrage");
                    ui.checkbox(&mut edited.trigger.on_wake, "Afficher au réveil");
                    ui.separator();
                    ui.collapsing("Configuration complète (TOML)", |ui| {
                        ui.label("Les modifications TOML sont appliquées avec le bouton ci-dessous.");
                        ui.add(egui::TextEdit::multiline(&mut self.settings_toml)
                            .font(egui::TextStyle::Monospace).desired_rows(12).desired_width(f32::INFINITY));
                        apply_toml = ui.button("Appliquer le TOML").clicked();
                    });
                    if let Some(path) = &self.save_path {
                        ui.label(format!("Fichier : {}", path.display()));
                        save = ui.button("Enregistrer la configuration").clicked();
                    } else {
                        ui.label("Aucun chemin de configuration disponible.");
                    }
                    ui.label("Le texte de démonstration n’est pas enregistré dans la configuration.");
                    if !self.settings_status.is_empty() { ui.label(&self.settings_status); }
                });
            });
        self.settings_open = open;
        if apply_toml {
            match toml::from_str::<Config>(&self.settings_toml) {
                Ok(config) => edited = config,
                Err(error) => self.settings_status = format!("TOML invalide : {error}"),
            }
        }
        // Avoid restarting the fade and auto-close timer on every frame.
        if toml::to_string(&edited).ok() != toml::to_string(&self.config).ok() {
            self.set_config(edited);
            self.settings_toml = toml::to_string_pretty(&self.config).unwrap_or_default();
            self.settings_status = "Aperçu mis à jour. Enregistrez pour conserver ces réglages.".into();
        }
        if save {
            if let Some(path) = &self.save_path {
                let result = (|| -> anyhow::Result<()> {
                    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                        std::fs::create_dir_all(parent)?;
                    }
                    self.config.save_to_file(path)
                })();
                self.settings_status = match result {
                    Ok(()) => "Configuration enregistrée. Certains réglages nécessitent un redémarrage.".into(),
                    Err(error) => format!("Échec de l’enregistrement : {error:#}"),
                };
            }
        }
    }

    fn color_control(ui: &mut egui::Ui, label: &str, color: &mut crate::config::Color) {
        ui.horizontal(|ui| {
            ui.label(label);
            let mut rgba = [color.r, color.g, color.b, color.a];
            if ui.color_edit_button_srgba_unmultiplied(&mut rgba).changed() {
                *color = crate::config::Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3]);
            }
        });
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
    fn test_settings_render_and_preserve_save_path() {
        let mut postit = PostItUI::new(Config::new(), PostItContent::default());
        let path = std::path::PathBuf::from("custom/settings.toml");
        postit.set_save_path(Some(path.clone()));
        postit.settings_open = true;
        postit.settings_toml = toml::to_string_pretty(postit.config()).unwrap();
        let ctx = Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| postit.settings_window(ctx));
        assert!(!output.shapes.is_empty());
        assert!(postit.settings_open);
        assert_eq!(postit.save_path, Some(path));
        assert!(!postit.should_close());
    }

    #[test]
    fn test_settings_apply_updates_timer_and_overlay() {
        let mut postit = PostItUI::new(Config::new(), PostItContent::default());
        let mut edited = postit.config().clone();
        edited.window.auto_close_ms = 1500;
        edited.overlay.enabled = false;
        edited.title.text = "Mon titre".into();
        postit.set_config(edited);
        assert_eq!(postit.auto_close_timer, Some(Duration::from_millis(1500)));
        assert!(!postit.overlay.is_enabled());
        assert_eq!(postit.config().title.text, "Mon titre");
    }

    #[test]
    fn test_escape_closes_settings_before_card() {
        let mut postit = PostItUI::new(Config::new(), PostItContent::default());
        postit.settings_open = true;
        let ctx = Context::default();
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| postit.update(ctx));
        assert!(!postit.settings_open);
        assert!(!postit.should_close());
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