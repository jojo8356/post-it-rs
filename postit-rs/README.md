# PostItRs

A customizable post-it note application that appears on system boot and/or wake from sleep. Built with Rust and egui.

## Features

- **Auto-display on boot/wake**: Shows a post-it automatically when you start your computer or wake it from sleep
- **Full graphical customization** via TOML config file:
  - Title tab (like a browser tab) with custom font, colors, background
  - Body text with custom font, colors, alignment, line height
  - Fullscreen overlay/backdrop with configurable color and opacity (intensity)
  - Window position: centered, corners, edges, or custom coordinates
- **Multiple content modes**:
  - Fixed: same message every time
  - Sequential: cycle through a list
  - Random: pick randomly from a list
  - File-based: load from external text file
  - **JSON with checkboxes**: enable/disable individual post-its, categories, weights
- **Custom fonts**: Load `.ttf`/`.otf` fonts from a `fonts/` directory or custom paths
- **Always-on-top**, no focus stealing, click-through overlay option
- **Cross-platform**: Linux (X11/Wayland), Windows, macOS
- **Lightweight**: Single binary, no dependencies, < 10MB
- **Development & production scripts**: `run-dev.sh` and `compile-prod.sh`

## Quick Start

### Installation

```bash
# Build from source
cargo build --release

# The binary will be at target/release/postit-rs
```

### Linux build troubleshooting

Run build commands from the `postit-rs/` directory containing `Cargo.toml`.

If `libdbus-sys` fails with `Package dbus-1 was not found`, install the
D-Bus development files and pkg-config (the runtime D-Bus package alone is
not enough):

```bash
# Debian / Ubuntu
sudo apt update
sudo apt install libdbus-1-dev pkg-config

# Fedora
sudo dnf install dbus-devel pkgconf-pkg-config

# Verify discovery, then retry
pkg-config --modversion dbus-1
cargo build
```

Without administrator access, build the bundled D-Bus source instead:

```bash
cargo build --features vendored-dbus
# Or an optimized build:
cargo build --release --features vendored-dbus
```

This avoids needing `libdbus-1-dev` and D-Bus discovery via pkg-config, but
still requires a working C compiler and linker (for example, `cc` and `ar`).
It does not repair a broken Rust toolchain or replace other GUI dependencies.
Cargo will update `Cargo.lock` as needed when enabling this feature.

An unset `PKG_CONFIG_PATH` is normal for distro-installed libraries. Only set
it manually if D-Bus is installed in a nonstandard prefix; it must point to
the directory containing `dbus-1.pc`.

If rustup reports `failed to install component: 'rust-src', detected conflict`,
this is a separate toolchain installation problem, not an error in `quote` or
`proc-macro2`. Stop other running Cargo/rustup commands before repairing it:

```bash
rustup show active-toolchain
rustup component remove rust-src
rustup component add rust-src
cargo build
```

Use the same toolchain for both component commands (add `--toolchain <name>`
if needed). This project does not itself require `rust-src` for a normal host
build. If component repair still fails, try a clean, separately installed
stable toolchain without deleting the existing installation:

```bash
rustup toolchain install stable --profile minimal
cargo +stable build
```

If `stable` is itself the damaged installation, reinstalling it may be needed;
check `rustup show` and any toolchain overrides first. Avoid manually deleting
files inside the toolchain or running rustup with `sudo`.

### Configuration

Copy the example config and customize:

```bash
# Generate default config
postit-rs --print-default-config > ~/.config/postit-rs/postit.conf.toml

# Edit the config
nano ~/.config/postit-rs/postit.conf.toml
```

### Enable Autostart

```bash
# Linux/macOS
postit-rs --autostart

# Windows (run as administrator for HKLM, or user for HKCU)
postit-rs --autostart
```

### Run Once (for testing)

```bash
postit-rs --once
```

### Test Mode (yellow post-it, black text, 20% black overlay)

```bash
postit-rs --test --once
```

## Configuration Reference

The config file is TOML format with these sections:

### `[general]` - Content selection
```toml
content_mode = "random"  # "fixed" | "sequential" | "random" | "file" | "json"
content_file = "postits.txt"  # Used when content_mode = "file"
json_file = "posts.json"      # Used when content_mode = "json"
json_category = ""            # Filter by category (empty = all)
```

### `[trigger]` - When to show
```toml
on_boot = true      # Show on system boot/login
on_wake = true      # Show on wake from sleep
delay_ms = 800      # Delay after trigger (lets desktop settle)
```

### `[title]` - Title tab (like browser tab)
```toml
enabled = true
text = "📌 Post-it du jour"
font_path = ""           # Custom .ttf/.otf path (empty = system font)
font_size = 14
font_weight = "bold"     # thin, light, normal, medium, semibold, bold, extrabold, black
color = "#FFFFFF"
bg_color = "#2D6BFF"
bg_image = ""            # Optional background image
padding = 10
border_radius = 8
alignment = "center"     # left, center, right
```

### `[body]` - Main content area
```toml
font_path = ""
font_size = 13
font_weight = "normal"
color = "#E0E0E0"
bg_color = "#1E1E1E"
bg_image = ""
padding = 16
border_radius = 10
alignment = "left"
line_height = 1.5
max_width = 420
shadow = true
```

### `[overlay]` - Fullscreen backdrop
```toml
enabled = true
color = "#000000"
opacity = 0.40           # 0.0 - 1.0 (intensity percentage)
click_through = false    # Mouse clicks pass through to windows below
blur_background = false  # Blur desktop behind (OS support varies)
animation_ms = 250       # Fade in/out duration
```

### `[window]` - Window behavior
```toml
position = "center"      # center, top-center, bottom-center, top-left, top-right, bottom-left, bottom-right, custom
custom_x = 0             # For custom position (pixels or 0-100%)
custom_y = 0
always_on_top = true
accept_focus = false     # Don't steal keyboard focus
show_in_taskbar = false
resizeable = false
close_on_click = false   # Close on any click (vs close button)
auto_close_ms = 0        # Auto-close after ms (0 = never)
max_width = 800          # Max width for positioning
```

### `[fonts]` - Custom fonts directory
```toml
custom_fonts_dir = "fonts"  # Relative to executable or absolute path
```

### `[advanced]` - Advanced options
```toml
log_level = "info"       # trace, debug, info, warn, error
log_file = ""            # Empty = stdout/stderr
monitor_index = 0        # 0 = primary, -1 = cursor monitor
toggle_hotkey = ""       # Global hotkey to show/hide (e.g. "Ctrl+Alt+P")
```

## JSON Post-its with Checkboxes

When using `content_mode = "json"`, create a JSON file with enable/disable checkboxes:

```json
{
  "version": 1,
  "default_category": "",
  "posts": [
    {
      "id": "hydration",
      "title": "💧 Hydratation",
      "body": "Buvez un grand verre d'eau !",
      "enabled": true,
      "category": "santé",
      "weight": 3
    },
    {
      "id": "focus",
      "title": "🎯 Focus",
      "body": "Une seule tâche à la fois. Terminez-la.",
      "enabled": false,
      "category": "productivité",
      "weight": 2
    }
  ]
}
```

Each post-it has:
- **id**: Unique identifier
- **title**: Optional display title (shown in tab)
- **body**: Main content text
- **enabled**: Checkbox - only enabled post-its are selected
- **category**: Optional category for filtering
- **weight**: Selection weight for random mode (higher = more likely)

### Using JSON mode

```toml
[general]
content_mode = "json"
json_file = "posts.json"
json_category = "santé"  # Optional: filter by category
```

## Content File Format (Legacy)

When using `content_mode = "file"`, create a text file with one post-it per line:

```text
# Comments start with #
# Format: [title]|[body]  (title is optional)

Rappel|Buvez de l'eau !
Citation|"Le code est lu plus souvent qu'il n'est écrit." — Guido van Rossum
|Note sans titre
Focus|Une seule chose à la fois.
```

## Development Scripts

### `run-dev.sh` - Development runner

```bash
# Run in debug mode
./run-dev.sh

# Run with test config (yellow post-it)
./run-dev.sh --test --once

# Run with custom config
./run-dev.sh --config my.toml

# Run in release mode
./run-dev.sh --release --once
```

### `compile-prod.sh` - Production package builder

Creates AppImage and .deb packages:

```bash
# Build both AppImage and .deb
./compile-prod.sh

# Build only AppImage
./compile-prod.sh --no-deb

# Clean and rebuild
./compile-prod.sh --clean

# Custom version
./compile-prod.sh --version 1.0.0
```

**Requirements:**
- `cargo` (Rust toolchain)
- `appimagetool` (for AppImage) - download from AppImageKit releases
- `dpkg-deb`, `fakeroot` (for .deb) - `apt-get install dpkg-dev fakeroot`
- `desktop-file-validate` - `apt-get install desktop-file-utils`

**Output:** Packages in `./dist/`

## Theme Examples

### Minimalist Black/White
```toml
[title]
enabled = false

[body]
font_path = "fonts/IBM_Plex_Mono/IBMPlexMono-Regular.ttf"
font_size = 14
color = "#FFFFFF"
bg_color = "#000000"
padding = 20
border_radius = 0

[overlay]
color = "#000000"
opacity = 0.50
```

### Classic Yellow Paper
```toml
[title]
text = "📝 Note"
bg_color = "#D4A843"
color = "#2C1810"
font_path = "fonts/Patrick_Hand/PatrickHand-Regular.ttf"

[body]
font_path = "fonts/Patrick_Hand/PatrickHand-Regular.ttf"
font_size = 16
color = "#2C1810"
bg_color = "#FDF6D9"
bg_image = "textures/paper_yellow.png"

[overlay]
enabled = false
```

### Cyberpunk Neon
```toml
[title]
text = "// SYSTÈME PRÊT //"
bg_color = "#0A0A0F"
color = "#00FFCC"
font_path = "fonts/JetBrains_Mono/JetBrainsMono-Bold.ttf"
border_radius = 0

[body]
font_path = "fonts/JetBrains_Mono/JetBrainsMono-Regular.ttf"
color = "#00FFCC"
bg_color = "#0D0D14"
border_radius = 0

[overlay]
color = "#000000"
opacity = 0.70
blur_background = true
```

### Test Theme (via `--test` flag)
Yellow post-it, black text, 20% black overlay:
```bash
postit-rs --test --once
```

## Platform-Specific Notes

### Linux
- Uses `systemd` user services for boot detection
- Uses `logind` D-Bus for wake detection
- Autostart creates `.desktop` file in `~/.config/autostart/`
- Requires `libdbus-1-dev` and `pkg-config` to build

### Windows
- Uses `WM_POWERBROADCAST` for wake detection
- Uses Registry `Run` key for autostart
- Requires `windows` crate features

### macOS
- Uses `NSWorkspace` notifications for wake detection
- Uses `LaunchAgent` plist for autostart
- Requires `objc2` crates (optional)

## Building

```bash
# Debug build
cargo build

# Release build (optimized, stripped)
cargo build --release

# Cross-compilation (example for Windows from Linux)
cargo build --release --target x86_64-pc-windows-gnu
```

## License

MIT OR Apache-2.0