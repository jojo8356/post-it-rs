use std::path::{Path, PathBuf};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use directories::ProjectDirs;

/// Main configuration structure matching the cahier des charges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub trigger: TriggerConfig,
    #[serde(default)]
    pub title: TitleConfig,
    #[serde(default)]
    pub body: BodyConfig,
    #[serde(default)]
    pub overlay: OverlayConfig,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub fonts: FontsConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralConfig {
    /// Content selection mode: "fixed" | "sequential" | "random" | "file" | "json"
    #[serde(default = "default_content_mode")]
    pub content_mode: ContentMode,
    /// Path to content file (for "file" mode)
    #[serde(default)]
    pub content_file: PathBuf,
    /// Path to JSON post-its file (for "json" mode)
    #[serde(default)]
    pub json_file: PathBuf,
    /// Category filter for JSON mode (empty = all categories)
    #[serde(default)]
    pub json_category: String,
    /// Persistent index for sequential mode (managed automatically)
    #[serde(default)]
    pub sequential_index: usize,
}

fn default_content_mode() -> ContentMode {
    ContentMode::Random
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentMode {
    Fixed,
    Sequential,
    Random,
    File,
    Json,
}

impl Default for ContentMode {
    fn default() -> Self {
        ContentMode::Random
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    /// Show post-it on system boot/login
    #[serde(default = "default_true")]
    pub on_boot: bool,
    /// Show post-it on wake from sleep/hibernation
    #[serde(default = "default_true")]
    pub on_wake: bool,
    /// Delay after trigger before showing (ms) - lets desktop settle
    /// Delay after trigger before showing (ms) - lets desktop settle
    #[serde(default = "default_delay_ms_u64")]
    pub delay_ms: u64,
}

fn default_true() -> bool {
    true
}

fn default_delay_ms_u64() -> u64 {
    800
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            on_boot: true,
            on_wake: true,
            delay_ms: 800,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleConfig {
    /// Enable/disable title tab
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Title text (empty = hidden but space reserved if enabled)
    #[serde(default)]
    pub text: String,
    /// Custom font path (.ttf/.otf) - empty = system default
    #[serde(default)]
    pub font_path: PathBuf,
    #[serde(default = "default_font_size_title")]
    pub font_size: u16,
    #[serde(default = "default_font_weight_bold")]
    pub font_weight: FontWeight,
    #[serde(default = "default_color_white")]
    pub color: Color,
    #[serde(default = "default_title_bg_color")]
    pub bg_color: Color,
    /// Optional background image path
    #[serde(default)]
    pub bg_image: PathBuf,
    #[serde(default = "default_padding_title")]
    pub padding: u16,
    #[serde(default = "default_border_radius_title")]
    pub border_radius: u16,
    #[serde(default = "default_alignment_center")]
    pub alignment: TextAlignment,
}

fn default_font_size_title() -> u16 { 14 }
fn default_font_weight_bold() -> FontWeight { FontWeight::Bold }
fn default_color_white() -> Color { Color::hex("#FFFFFF") }
fn default_title_bg_color() -> Color { Color::hex("#2D6BFF") }
fn default_padding_title() -> u16 { 10 }
fn default_border_radius_title() -> u16 { 8 }
fn default_alignment_center() -> TextAlignment { TextAlignment::Center }

impl Default for TitleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            text: "📌  Post-it du jour".to_string(),
            font_path: PathBuf::new(),
            font_size: 14,
            font_weight: FontWeight::Bold,
            color: Color::hex("#FFFFFF"),
            bg_color: Color::hex("#2D6BFF"),
            bg_image: PathBuf::new(),
            padding: 10,
            border_radius: 8,
            alignment: TextAlignment::Center,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyConfig {
    #[serde(default)]
    pub font_path: PathBuf,
    #[serde(default = "default_font_size_body")]
    pub font_size: u16,
    #[serde(default)]
    pub font_weight: FontWeight,
    #[serde(default = "default_body_text_color")]
    pub color: Color,
    #[serde(default = "default_body_bg_color")]
    pub bg_color: Color,
    #[serde(default)]
    pub bg_image: PathBuf,
    #[serde(default = "default_padding_body")]
    pub padding: u16,
    #[serde(default = "default_border_radius_body")]
    pub border_radius: u16,
    #[serde(default = "default_alignment_left")]
    pub alignment: TextAlignment,
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    #[serde(default = "default_max_width")]
    pub max_width: u16,
    #[serde(default = "default_true")]
    pub shadow: bool,
}

fn default_font_size_body() -> u16 { 13 }
fn default_body_text_color() -> Color { Color::hex("#E0E0E0") }
fn default_body_bg_color() -> Color { Color::hex("#1E1E1E") }
fn default_padding_body() -> u16 { 16 }
fn default_border_radius_body() -> u16 { 10 }
fn default_alignment_left() -> TextAlignment { TextAlignment::Left }
fn default_line_height() -> f32 { 1.5 }
fn default_max_width() -> u16 { 420 }

impl Default for BodyConfig {
    fn default() -> Self {
        Self {
            font_path: PathBuf::new(),
            font_size: 13,
            font_weight: FontWeight::Normal,
            color: Color::hex("#E0E0E0"),
            bg_color: Color::hex("#1E1E1E"),
            bg_image: PathBuf::new(),
            padding: 16,
            border_radius: 10,
            alignment: TextAlignment::Left,
            line_height: 1.5,
            max_width: 420,
            shadow: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_overlay_color")]
    pub color: Color,
    /// Opacity 0.0 - 1.0 (intensity percentage)
    #[serde(default = "default_overlay_opacity")]
    pub opacity: f32,
    /// Click-through: mouse events pass through to windows below
    #[serde(default)]
    pub click_through: bool,
    /// Blur background behind overlay (OS support dependent)
    #[serde(default)]
    pub blur_background: bool,
    /// Animation duration in milliseconds
    #[serde(default = "default_animation_ms_u64")]
    pub animation_ms: u64,
}

fn default_overlay_color() -> Color { Color::hex("#000000") }
fn default_overlay_opacity() -> f32 { 0.40 }
fn default_animation_ms_u64() -> u64 { 250 }

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            color: Color::hex("#000000"),
            opacity: 0.40,
            click_through: false,
            blur_background: false,
            animation_ms: 250,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Position: "center" | "top-center" | "bottom-center" | "top-left" | "top-right" | "bottom-left" | "bottom-right" | "custom"
    #[serde(default = "default_position_center")]
    pub position: WindowPosition,
    /// Custom X position (pixels or percent if 0-100)
    #[serde(default)]
    pub custom_x: i32,
    /// Custom Y position (pixels or percent if 0-100)
    #[serde(default)]
    pub custom_y: i32,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    /// Don't steal keyboard focus
    #[serde(default)]
    pub accept_focus: bool,
    #[serde(default)]
    pub show_in_taskbar: bool,
    #[serde(default)]
    pub resizeable: bool,
    /// Close on click anywhere (otherwise need close button)
    #[serde(default)]
    pub close_on_click: bool,
    /// Auto-close after ms (0 = never)
    #[serde(default)]
    pub auto_close_ms: u64,
    /// Maximum window width for positioning calculations
    #[serde(default = "default_max_width_window")]
    pub max_width: u16,
}

fn default_position_center() -> WindowPosition { WindowPosition::Center }
fn default_max_width_window() -> u16 { 800 }

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            position: WindowPosition::Center,
            custom_x: 0,
            custom_y: 0,
            always_on_top: true,
            accept_focus: false,
            show_in_taskbar: false,
            resizeable: false,
            close_on_click: false,
            auto_close_ms: 0,
            max_width: 800,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WindowPosition {
    Center,
    TopCenter,
    BottomCenter,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Custom,
}

impl Default for WindowPosition {
    fn default() -> Self {
        WindowPosition::Center
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontsConfig {
    /// Custom fonts directory (relative to exe or absolute)
    #[serde(default = "default_fonts_dir")]
    pub custom_fonts_dir: PathBuf,
}

fn default_fonts_dir() -> PathBuf {
    PathBuf::from("fonts")
}

impl Default for FontsConfig {
    fn default() -> Self {
        Self {
            custom_fonts_dir: PathBuf::from("fonts"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// Log level: "trace" | "debug" | "info" | "warn" | "error"
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Log file path (empty = stdout/stderr only)
    #[serde(default)]
    pub log_file: PathBuf,
    /// Monitor index: 0 = primary, -1 = cursor monitor
    #[serde(default)]
    pub monitor_index: i32,
    /// Global hotkey to toggle show/hide (e.g. "Ctrl+Alt+P")
    #[serde(default)]
    pub toggle_hotkey: String,
}

fn default_log_level() -> String { "info".to_string() }

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_file: PathBuf::new(),
            monitor_index: 0,
            toggle_hotkey: String::new(),
        }
    }
}

/// Color representation with hex parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub fn hex(s: &str) -> Self {
        let s = s.trim_start_matches('#');
        let (r, g, b) = match s.len() {
            3 => {
                let r = u8::from_str_radix(&s[0..1].repeat(2), 16).unwrap_or(0);
                let g = u8::from_str_radix(&s[1..2].repeat(2), 16).unwrap_or(0);
                let b = u8::from_str_radix(&s[2..3].repeat(2), 16).unwrap_or(0);
                (r, g, b)
            }
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
                (r, g, b)
            }
            8 => {
                let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
                let a = u8::from_str_radix(&s[6..8], 16).unwrap_or(255);
                return Self { r, g, b, a };
            }
            _ => (0, 0, 0),
        };
        Self { r, g, b, a: 255 }
    }
    pub fn to_egui(self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }
    pub fn to_array(self) -> [f32; 4] {
        [self.r as f32 / 255.0, self.g as f32 / 255.0, self.b as f32 / 255.0, self.a as f32 / 255.0]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::rgb(0, 0, 0)
    }
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex = format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b);
        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Color::hex(&s))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl Default for FontWeight {
    fn default() -> Self {
        FontWeight::Normal
    }
}

impl FontWeight {
    /// Convert to CSS-style font weight (100-900)
    /// Not directly usable in egui 0.29 (no font weight support yet)
    pub fn to_css_weight(&self) -> u16 {
        match self {
            FontWeight::Thin => 100,
            FontWeight::ExtraLight => 200,
            FontWeight::Light => 300,
            FontWeight::Normal => 400,
            FontWeight::Medium => 500,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800,
            FontWeight::Black => 900,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

impl Default for TextAlignment {
    fn default() -> Self {
        TextAlignment::Left
    }
}

impl TextAlignment {
    pub fn to_egui(self) -> egui::Align {
        match self {
            TextAlignment::Left => egui::Align::LEFT,
            TextAlignment::Center => egui::Align::Center,
            TextAlignment::Right => egui::Align::RIGHT,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Self {
        Self {
            general: GeneralConfig::default(),
            trigger: TriggerConfig::default(),
            title: TitleConfig::default(),
            body: BodyConfig::default(),
            overlay: OverlayConfig::default(),
            window: WindowConfig::default(),
            fonts: FontsConfig::default(),
            advanced: AdvancedConfig::default(),
        }
    }

    /// Load config from file, with fallback to defaults
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let mut config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML config: {}", path.display()))?;
        config.resolve_paths(path);
        Ok(config)
    }

    /// Resolve relative paths relative to config file directory
    fn resolve_paths(&mut self, config_path: &Path) {
        let base_dir = config_path.parent().unwrap_or(Path::new("."));
        let resolve = |p: &mut PathBuf| {
            if p.is_relative() && !p.as_os_str().is_empty() {
                *p = base_dir.join(&*p);
            }
        };
        resolve(&mut self.general.content_file);
        resolve(&mut self.title.font_path);
        resolve(&mut self.title.bg_image);
        resolve(&mut self.body.font_path);
        resolve(&mut self.body.bg_image);
        resolve(&mut self.fonts.custom_fonts_dir);
    }

    /// Get default config file path for current platform
    pub fn default_config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "postit-rs", "postit-rs")
            .map(|dirs| dirs.config_dir().join("postit.conf.toml"))
    }

    /// Save config to file (for sequential index persistence)
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }
}

/// Content item (title + body)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostItContent {
    pub title: Option<String>,
    pub body: String,
}

impl PostItContent {
    pub fn from_line(line: &str) -> Self {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return Self { title: None, body: String::new() };
        }
        // Format: [title]|[body]  — title optional
        if let Some(pipe_pos) = line.find('|') {
            let title_part = line[..pipe_pos].trim();
            let body_part = line[pipe_pos + 1..].trim();
            Self {
                title: if title_part.is_empty() { None } else { Some(title_part.to_string()) },
                body: body_part.to_string(),
            }
        } else {
            Self { title: None, body: line.to_string() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_color_hex_parsing() {
        assert_eq!(Color::hex("#FF0000"), Color::rgb(255, 0, 0));
        assert_eq!(Color::hex("#00FF00"), Color::rgb(0, 255, 0));
        assert_eq!(Color::hex("#0000FF"), Color::rgb(0, 0, 255));
        assert_eq!(Color::hex("#ABC"), Color::rgb(0xAA, 0xBB, 0xCC));
        assert_eq!(Color::hex("#AABBCC"), Color::rgb(0xAA, 0xBB, 0xCC));
        assert_eq!(Color::hex("#AABBCCDD"), Color::rgba(0xAA, 0xBB, 0xCC, 0xDD));
    }

    #[test]
    fn test_content_parsing() {
        let c = PostItContent::from_line("Title|Body text");
        assert_eq!(c.title, Some("Title".to_string()));
        assert_eq!(c.body, "Body text");

        let c = PostItContent::from_line("|Body only");
        assert_eq!(c.title, None);
        assert_eq!(c.body, "Body only");

        let c = PostItContent::from_line("No pipe");
        assert_eq!(c.title, None);
        assert_eq!(c.body, "No pipe");
    }

    #[test]
    fn test_config_default() {
        let config = Config::new();
        assert_eq!(config.general.content_mode, ContentMode::Random);
        assert!(config.trigger.on_boot);
        assert!(config.trigger.on_wake);
        assert!(config.title.enabled);
        assert!(config.overlay.enabled);
        assert!(config.window.always_on_top);
    }
}