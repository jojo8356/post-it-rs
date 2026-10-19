//! PostItRs - A customizable post-it that appears on boot/wake
//! 
//! Features:
//! - Shows on system boot and/or wake from sleep
//! - Fully configurable via TOML config file
//! - Custom fonts, colors, backgrounds, opacity
//! - Fullscreen overlay with adjustable intensity
//! - Always-on-top, centered, no focus stealing
//! - Multiple content modes: fixed, sequential, random, file-based

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::{Context, Result};
use clap::Parser;
use eframe::egui;
use egui::{Color32, Rounding, Stroke};
use tracing::{info, error};
use tracing_subscriber::{fmt, EnvFilter};

use postit_rs::config::{Config, Color, ContentMode, FontWeight, PostItContent, WindowPosition};
use postit_rs::content::ContentProvider;
use postit_rs::fonts::FontManager;
use postit_rs::system::{create_listener, SystemEvent, set_autostart, AutostartAction};
use postit_rs::ui::PostItUI;

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(name = "postit-rs", version, about = "Post-it on boot/wake")]
struct Args {
    /// Path to config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Run once and exit (for testing)
    #[arg(short, long)]
    once: bool,

    /// Show test post-it (yellow with black text, 20% black overlay)
    #[arg(long)]
    test: bool,

    /// Show specific content (for testing)
    #[arg(long, value_name = "TEXT")]
    content: Option<String>,

    /// Enable autostart on login
    #[arg(long)]
    autostart: bool,

    /// Disable autostart on login
    #[arg(long)]
    no_autostart: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Print default config and exit
    #[arg(long)]
    print_default_config: bool,

    /// Validate config file and exit
    #[arg(long)]
    validate: bool,
}

/// Application state shared between event loop and UI
struct AppState {
    config: Arc<Mutex<Config>>,
    content_provider: Arc<Mutex<ContentProvider>>,
    ui: Arc<Mutex<Option<PostItUI>>>,
    show_postit: Arc<Mutex<bool>>,
    pending_content: Arc<Mutex<Option<PostItContent>>>,
    system_listener: Arc<Mutex<Option<Box<dyn postit_rs::system::SystemListener>>>>,
}

impl AppState {
    fn new(config: Config) -> Self {
        let config_arc = Arc::new(Mutex::new(config));
        let content_provider = Arc::new(Mutex::new(ContentProvider::new(config_arc.clone())));
        
        // Initial content load
        {
            let mut provider = content_provider.lock().unwrap();
            provider.reload().ok();
        }

        Self {
            config: config_arc,
            content_provider,
            ui: Arc::new(Mutex::new(None)),
            show_postit: Arc::new(Mutex::new(false)),
            pending_content: Arc::new(Mutex::new(None)),
            system_listener: Arc::new(Mutex::new(None)),
        }
    }

    /// Trigger post-it display
    fn trigger_show(&self) {
        let mut pending = self.pending_content.lock().unwrap();
        let mut provider = self.content_provider.lock().unwrap();
        let content = provider.next();
        *pending = Some(content);
        
        let mut show = self.show_postit.lock().unwrap();
        *show = true;
    }

    /// Handle system event
    fn handle_system_event(&self, event: SystemEvent) {
        info!("System event: {:?}", event);
        
        let config = self.config.lock().unwrap();
        let should_trigger = match event {
            SystemEvent::Boot => config.trigger.on_boot,
            SystemEvent::Wake => config.trigger.on_wake,
        };
        let delay = config.trigger.delay_ms;
        drop(config);

        if should_trigger {
            // Delay to let desktop settle
            let show_postit = self.show_postit.clone();
            let pending_content = self.pending_content.clone();
            let content_provider = self.content_provider.clone();
            
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(delay));
                let mut provider = content_provider.lock().unwrap();
                let content = provider.next();
                *pending_content.lock().unwrap() = Some(content);
                *show_postit.lock().unwrap() = true;
            });
        }
    }
}

/// eframe App implementation
struct PostItApp {
    state: Arc<AppState>,
    last_frame_time: std::time::Instant,
}

impl PostItApp {
    fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            last_frame_time: std::time::Instant::now(),
        }
    }
}

impl eframe::App for PostItApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Only the overlay should tint the desktop, not eframe's default clear.
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if we should show the post-it
        let mut show = self.state.show_postit.lock().unwrap();
        if *show {
            // Create or update UI
            let mut ui_lock = self.state.ui.lock().unwrap();
            let pending = self.state.pending_content.lock().unwrap().take();
            
            if let Some(content) = pending {
                let config = self.state.config.lock().unwrap().clone();
                *ui_lock = Some(PostItUI::new(config, content));
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            }
            
            *show = false;
        }

        // Update and render UI if active
        let mut ui_lock = self.state.ui.lock().unwrap();
        if let Some(ui) = ui_lock.as_mut() {
            // Native fullscreen may bypass desktop compositing and make a
            // transparent surface opaque. Use a borderless monitor-sized window.
            if self.state.config.lock().unwrap().overlay.enabled {
                let (monitor_size, inner_rect) = ctx.input(|i| {
                    (i.viewport().monitor_size, i.viewport().inner_rect)
                });
                if let Some(size) = monitor_size {
                    if inner_rect.is_none_or(|rect| (rect.size() - size).length() > 1.0) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::Pos2::ZERO));
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                    }
                }
            }
            ui.ui(ctx);
            
            if ui.should_close() {
                *ui_lock = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                // Request repaint for animations
                ctx.request_repaint_after(Duration::from_millis(16));
            }
        }

        // Handle config hot-reload (check every 5 seconds)
        let now = std::time::Instant::now();
        if now.duration_since(self.last_frame_time) > Duration::from_secs(5) {
            self.last_frame_time = now;
            // Could add config file watching here
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        info!("Application exiting");
        if let Ok(mut listener) = self.state.system_listener.lock() {
            if let Some(l) = listener.as_mut() {
                let _ = l.stop();
            }
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args.log_level);

    // Handle special commands
    if args.print_default_config {
        print_default_config();
        return Ok(());
    }

    if args.autostart {
        return set_autostart(AutostartAction::Enable).context("Failed to enable autostart");
    }

    if args.no_autostart {
        return set_autostart(AutostartAction::Disable).context("Failed to disable autostart");
    }

    // Load configuration (or use test config if --test flag)
    let config = if args.test {
        create_test_config()
    } else {
        load_config(args.config.as_deref())?
    };

    if args.validate {
        println!("✓ Config is valid");
        return Ok(());
    }

    // Initialize font manager
    let mut font_manager = FontManager::new(config.fonts.custom_fonts_dir.clone());
    font_manager.load_custom_fonts().ok(); // Non-fatal

    // Build font definitions for egui
    let font_defs = font_manager.build_font_definitions(&config);

    // Create app state
    let state = Arc::new(AppState::new(config.clone()));

    // One-shot demos do not need a background system listener.
    if !args.once {
        let mut listener = create_listener();
        let (tx, rx) = std::sync::mpsc::channel::<SystemEvent>();
        listener.start(tx)?;
        *state.system_listener.lock().unwrap() = Some(listener);

        let state_for_events = state.clone();
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                state_for_events.handle_system_event(event);
            }
        });
    }

    let show_immediately = args.once || args.test || args.content.is_some();
    if show_immediately {
        state.trigger_show();
        if let Some(body) = args.content {
            *state.pending_content.lock().unwrap() = Some(PostItContent {
                title: None,
                body,
            });
        }
    }

    // Configure eframe
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(config.window.resizeable)
            .with_visible(show_immediately) // Map demos before requesting fullscreen
            .with_taskbar(!config.window.show_in_taskbar),
        ..Default::default()
    };

    // Run the app
    let result = eframe::run_native(
        "PostItRs",
        native_options,
        Box::new(move |cc| -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
            // Set up custom fonts
            cc.egui_ctx.set_fonts(font_defs);
            
            // Configure visual style
            setup_style(&cc.egui_ctx, &config);
            
            // Apply window settings (no focus, click-through, etc.)
            apply_window_settings(&cc.egui_ctx, &config);
            
            Ok(Box::new(PostItApp::new(state)))
        }),
    );
    
    if let Err(e) = result {
        error!("Failed to run eframe application: {}", e);
        return Err(anyhow::anyhow!("eframe error: {}", e));
    }

    Ok(())
}

/// Load configuration from file or defaults
fn load_config(config_path: Option<&std::path::Path>) -> Result<Config> {
    if let Some(path) = config_path {
        info!("Loading config from: {}", path.display());
        Config::load_from_file(path)
    } else if let Some(default_path) = Config::default_config_path() {
        if default_path.exists() {
            info!("Loading config from default location: {}", default_path.display());
            Config::load_from_file(&default_path)
        } else {
            info!("No config file found, using defaults");
            Ok(Config::new())
        }
    } else {
        info!("No config directory available, using defaults");
        Ok(Config::new())
    }
}

/// Initialize logging
fn init_logging(level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));
    
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();
}

/// Print default configuration to stdout
fn print_default_config() {
    let config = Config::new();
    let toml = toml::to_string_pretty(&config).unwrap();
    println!("{}", toml);
}

/// Set up egui visual style from config
fn setup_style(ctx: &egui::Context, config: &Config) {
    let mut style = (*ctx.style()).clone();
    
    // Transparent background for the viewport
    style.visuals.window_fill = Color32::TRANSPARENT;
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.extreme_bg_color = Color32::TRANSPARENT;
    
    // No window frame
    style.visuals.window_stroke = Stroke::NONE;
    style.visuals.window_rounding = Rounding::same(config.body.border_radius as f32);
    
    ctx.set_style(style);
}

/// Apply platform-specific window settings
fn apply_window_settings(ctx: &egui::Context, config: &Config) {
    // These would be applied via viewport commands or platform-specific code
    // For now, we set viewport properties
    
    if !config.window.accept_focus {
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus); // Don't steal focus
    }
    
    // Click-through for overlay is handled in overlay::platform
}

/// Create test configuration: yellow post-it, black text, 20% black overlay
fn create_test_config() -> Config {
    let mut config = Config::new();
    
    // Test content
    config.general.content_mode = ContentMode::Fixed;
    
    // Yellow post-it style
    config.title.enabled = true;
    config.title.text = "Test".to_string();
    config.title.bg_color = Color::hex("#F5DE4C"); // Yellow
    config.title.color = Color::hex("#000000");    // Black text
    config.title.font_size = 14;
    config.title.font_weight = FontWeight::Bold;
    
    config.body.bg_color = Color::hex("#F5DE4C"); // Yellow
    config.body.color = Color::hex("#000000");    // Black text
    config.body.font_size = 14;
    config.body.font_weight = FontWeight::Normal;
    config.body.padding = 20;
    config.body.border_radius = 8;
    
    // Black overlay at 20%
    config.overlay.enabled = true;
    config.overlay.color = Color::hex("#000000");
    config.overlay.opacity = 0.20;
    config.overlay.animation_ms = 100;
    
    // Window settings
    config.window.position = WindowPosition::Center;
    config.window.always_on_top = true;
    config.window.accept_focus = false;
    config.window.show_in_taskbar = false;
    config.window.auto_close_ms = 0;
    
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        let args = Args::try_parse_from(["postit-rs", "--once"]).unwrap();
        assert!(args.once);
        assert!(!args.autostart);
    }

    #[test]
    fn test_default_config_load() {
        let config = Config::new();
        assert_eq!(config.general.content_mode, ContentMode::Random);
    }
}