#!/usr/bin/env bash
# compile-prod.sh - Production build script for PostItRs
# Creates AppImage and .deb packages for Linux distribution

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

# Configuration
APP_NAME="postit-rs"
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
ARCH=$(uname -m)
BUILD_DIR="$PROJECT_DIR/target/release"
OUTPUT_DIR="$PROJECT_DIR/dist"

# Default options
CREATE_APPIMAGE=true
CREATE_DEB=true
SIGN=false
CLEAN=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-appimage)
            CREATE_APPIMAGE=false
            shift
            ;;
        --no-deb)
            CREATE_DEB=false
            shift
            ;;
        --sign)
            SIGN=true
            shift
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --version)
            VERSION="$2"
            shift 2
            ;;
        --help|-h)
            cat <<EOF
Usage: ./compile-prod.sh [OPTIONS]

Production build script for PostItRs - creates AppImage and .deb packages

OPTIONS:
    --no-appimage      Skip AppImage creation
    --no-deb           Skip .deb package creation
    --sign             Sign packages (requires GPG key configured)
    --clean            Clean build artifacts before building
    --version VERSION  Override version (default: from Cargo.toml)
    --help, -h         Show this help message

REQUIREMENTS:
    - cargo (Rust toolchain)
    - appimagetool (for AppImage)
    - dpkg-deb (for .deb)
    - desktop-file-validate (for .desktop validation)
    - fakeroot (for .deb building)

EXAMPLES:
    ./compile-prod.sh                    # Build both AppImage and .deb
    ./compile-prod.sh --no-deb           # Build only AppImage
    ./compile-prod.sh --clean            # Clean and rebuild everything
    ./compile-prod.sh --version 1.0.0    # Build with specific version

OUTPUT:
    Packages will be placed in ./dist/
EOF
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Clean if requested
if [[ "$CLEAN" == true ]]; then
    echo -e "${YELLOW}Cleaning previous build artifacts...${NC}"
    cargo clean
    rm -rf "$OUTPUT_DIR"
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Check for required tools
check_tool() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}Missing required tool: $1${NC}"
        echo -e "${YELLOW}Install with: $2${NC}"
        return 1
    fi
    return 0
}

TOOLS_OK=true
if [[ "$CREATE_APPIMAGE" == true ]]; then
    check_tool "appimagetool" "wget https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage -O /usr/local/bin/appimagetool && chmod +x /usr/local/bin/appimagetool" || TOOLS_OK=false
fi
if [[ "$CREATE_DEB" == true ]]; then
    check_tool "dpkg-deb" "apt-get install dpkg-dev" || TOOLS_OK=false
    check_tool "fakeroot" "apt-get install fakeroot" || TOOLS_OK=false
    check_tool "desktop-file-validate" "apt-get install desktop-file-utils" || TOOLS_OK=false
fi

if [[ "$TOOLS_OK" == false ]]; then
    exit 1
fi

# Build release binary
echo -e "${BLUE}Building release binary (version $VERSION)...${NC}"
if ! cargo build --release; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

BINARY="$BUILD_DIR/$APP_NAME"
if [[ ! -f "$BINARY" ]]; then
    echo -e "${RED}Binary not found at $BINARY${NC}"
    exit 1
fi

# Strip binary for smaller size
echo -e "${BLUE}Stripping binary...${NC}"
strip "$BINARY" 2>/dev/null || true

# Get binary size
BINARY_SIZE=$(du -h "$BINARY" | cut -f1)
echo -e "${GREEN}Binary size: $BINARY_SIZE${NC}"

# ============================================================
# Create AppImage
# ============================================================
if [[ "$CREATE_APPIMAGE" == true ]]; then
    echo -e "${BLUE}Creating AppImage...${NC}"
    
    APPDIR="$OUTPUT_DIR/$APP_NAME.AppDir"
    rm -rf "$APPDIR"
    mkdir -p "$APPDIR/usr/bin"
    mkdir -p "$APPDIR/usr/share/applications"
    mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
    mkdir -p "$APPDIR/usr/share/metainfo"
    
    # Copy binary
    cp "$BINARY" "$APPDIR/usr/bin/$APP_NAME"
    
    # Create .desktop file
    cat > "$APPDIR/usr/share/applications/$APP_NAME.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=PostItRs
GenericName=Post-it Notes
Comment=Customizable post-it that appears on boot/wake
Exec=$APP_NAME
Icon=$APP_NAME
Terminal=false
Categories=Utility;Office;
StartupNotify=false
Keywords=note;post-it;memo;reminder;
EOF
    
    # Validate .desktop file
    desktop-file-validate "$APPDIR/usr/share/applications/$APP_NAME.desktop"
    
    # Create AppRun script
    cat > "$APPDIR/AppRun" <<'EOF'
#!/usr/bin/env bash
# AppRun entry point for AppImage
HERE="$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")"
export PATH="$HERE/usr/bin:$PATH"
export LD_LIBRARY_PATH="$HERE/usr/lib:$LD_LIBRARY_PATH"

# Run the application
exec "$HERE/usr/bin/postit-rs" "$@"
EOF
    chmod +x "$APPDIR/AppRun"
    
    # Create icon (placeholder - replace with actual icon)
    # For now, create a simple SVG icon
    cat > "$APPDIR/usr/share/icons/hicolor/256x256/apps/$APP_NAME.svg" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<svg width="256" height="256" viewBox="0 0 256 256" xmlns="http://www.w3.org/2000/svg">
  <rect x="32" y="32" width="192" height="192" rx="16" fill="#F5DE4C"/>
  <rect x="32" y="32" width="192" height="48" rx="16" fill="#2D6BFF"/>
  <text x="128" y="64" font-family="sans-serif" font-size="24" font-weight="bold" fill="white" text-anchor="middle">📌</text>
  <text x="128" y="160" font-family="sans-serif" font-size="20" fill="#333" text-anchor="middle">PostItRs</text>
</svg>
EOF
    
    # Copy icon to root for AppImage
    cp "$APPDIR/usr/share/icons/hicolor/256x256/apps/$APP_NAME.svg" "$APPDIR/$APP_NAME.svg"
    
    # Create AppStream metainfo
    cat > "$APPDIR/usr/share/metainfo/$APP_NAME.metainfo.xml" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>$APP_NAME</id>
  <name>PostItRs</name>
  <summary>Customizable post-it that appears on boot/wake</summary>
  <description>
    <p>A lightweight, customizable post-it note application that automatically appears when you start your computer or wake it from sleep.</p>
    <p>Features:</p>
    <ul>
      <li>Shows on system boot and/or wake from sleep</li>
      <li>Fully configurable via TOML config file</li>
      <li>Custom fonts, colors, backgrounds, opacity</li>
      <li>Fullscreen overlay with adjustable intensity</li>
      <li>Always-on-top, centered, no focus stealing</li>
      <li>Multiple content modes: fixed, sequential, random, file-based, JSON with checkboxes</li>
    </ul>
  </description>
  <url type="homepage">https://github.com/yourusername/postit-rs</url>
  <url type="bugtracker">https://github.com/yourusername/postit-rs/issues</url>
  <license>MIT OR Apache-2.0</license>
  <releases>
    <release version="$VERSION" date="$(date +%Y-%m-%d)"/>
  </release>
  <categories>
    <category>Utility</category>
    <category>Office</category>
  </categories>
  <keywords>
    <keyword>note</keyword>
    <keyword>post-it</keyword>
    <keyword>memo</keyword>
    <keyword>reminder</keyword>
  </keywords>
</component>
EOF
    
    # Build AppImage
    APPIMAGE_OUTPUT="$OUTPUT_DIR/${APP_NAME}-${VERSION}-${ARCH}.AppImage"
    echo -e "${YELLOW}Running appimagetool...${NC}"
    ARCH="$ARCH" appimagetool "$APPDIR" "$APPIMAGE_OUTPUT"
    
    if [[ -f "$APPIMAGE_OUTPUT" ]]; then
        APPIMAGE_SIZE=$(du -h "$APPIMAGE_OUTPUT" | cut -f1)
        echo -e "${GREEN}AppImage created: $APPIMAGE_OUTPUT ($APPIMAGE_SIZE)${NC}"
        
        if [[ "$SIGN" == true ]]; then
            echo -e "${BLUE}Signing AppImage...${NC}"
            gpg --detach-sign "$APPIMAGE_OUTPUT"
        fi
    else
        echo -e "${RED}AppImage creation failed!${NC}"
        exit 1
    fi
    
    # Cleanup AppDir
    rm -rf "$APPDIR"
fi

# ============================================================
# Create .deb package
# ============================================================
if [[ "$CREATE_DEB" == true ]]; then
    echo -e "${BLUE}Creating .deb package...${NC}"
    
    DEB_DIR="$OUTPUT_DIR/${APP_NAME}_${VERSION}-1_${ARCH}"
    rm -rf "$DEB_DIR"
    mkdir -p "$DEB_DIR/DEBIAN"
    mkdir -p "$DEB_DIR/usr/bin"
    mkdir -p "$DEB_DIR/usr/share/applications"
    mkdir -p "$DEB_DIR/usr/share/icons/hicolor/256x256/apps"
    mkdir -p "$DEB_DIR/usr/share/doc/$APP_NAME"
    mkdir -p "$DEB_DIR/etc/xdg/autostart"
    
    # Copy binary
    cp "$BINARY" "$DEB_DIR/usr/bin/$APP_NAME"
    
    # Copy icon
    cp "$PROJECT_DIR/assets/icon.svg" "$DEB_DIR/usr/share/icons/hicolor/256x256/apps/$APP_NAME.svg" 2>/dev/null || \
        cp "$OUTPUT_DIR/${APP_NAME}.AppDir/usr/share/icons/hicolor/256x256/apps/$APP_NAME.svg" "$DEB_DIR/usr/share/icons/hicolor/256x256/apps/$APP_NAME.svg" 2>/dev/null || true
    
    # Create .desktop file
    cat > "$DEB_DIR/usr/share/applications/$APP_NAME.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=PostItRs
GenericName=Post-it Notes
Comment=Customizable post-it that appears on boot/wake
Exec=$APP_NAME
Icon=$APP_NAME
Terminal=false
Categories=Utility;Office;
StartupNotify=false
Keywords=note;post-it;memo;reminder;
EOF
    
    # Create autostart .desktop (disabled by default)
    cat > "$DEB_DIR/etc/xdg/autostart/$APP_NAME.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=PostItRs
Comment=Customizable post-it on boot/wake
Exec=$APP_NAME
Hidden=true
NoDisplay=true
X-GNOME-Autostart-enabled=false
EOF
    
    # Create control file
    INSTALLED_SIZE=$(du -sk "$DEB_DIR/usr" | cut -f1)
    cat > "$DEB_DIR/DEBIAN/control" <<EOF
Package: $APP_NAME
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCH
Installed-Size: $INSTALLED_SIZE
Maintainer: Your Name <you@example.com>
Description: Customizable post-it that appears on boot/wake
 A lightweight, customizable post-it note application that automatically
 appears when you start your computer or wake it from sleep.
 .
 Features:
  * Shows on system boot and/or wake from sleep
  * Fully configurable via TOML config file
  * Custom fonts, colors, backgrounds, opacity
  * Fullscreen overlay with adjustable intensity
  * Always-on-top, centered, no focus stealing
  * Multiple content modes: fixed, sequential, random, file-based, JSON
Homepage: https://github.com/yourusername/postit-rs
License: MIT OR Apache-2.0
EOF
    
    # Create copyright file
    cat > "$DEB_DIR/usr/share/doc/$APP_NAME/copyright" <<EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: PostItRs
Source: https://github.com/yourusername/postit-rs

Files: *
Copyright: 2024 Your Name <you@example.com>
License: MIT OR Apache-2.0

License: MIT
 Permission is hereby granted, free of charge, to any person obtaining a copy
 of this software and associated documentation files (the "Software"), to deal
 in the Software without restriction, including without limitation the rights
 to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 copies of the Software, and to permit persons to whom the Software is
 furnished to do so, subject to the following conditions:
 .
 The above copyright notice and this permission notice shall be included in
 all copies or substantial portions of the Software.
 .
 THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.

License: Apache-2.0
 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at
 .
     http://www.apache.org/licenses/LICENSE-2.0
 .
 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
EOF
    
    # Create changelog
    cat > "$DEB_DIR/usr/share/doc/$APP_NAME/changelog.Debian" <<EOF
$APP_NAME ($VERSION-1) unstable; urgency=medium

  * Initial release

 -- Your Name <you@example.com>  $(date -R)
EOF
    gzip -9 "$DEB_DIR/usr/share/doc/$APP_NAME/changelog.Debian"
    
    # Build .deb
    DEB_OUTPUT="$OUTPUT_DIR/${APP_NAME}_${VERSION}-1_${ARCH}.deb"
    echo -e "${YELLOW}Building .deb package...${NC}"
    fakeroot dpkg-deb --build "$DEB_DIR" "$DEB_OUTPUT"
    
    if [[ -f "$DEB_OUTPUT" ]]; then
        DEB_SIZE=$(du -h "$DEB_OUTPUT" | cut -f1)
        echo -e "${GREEN}.deb package created: $DEB_OUTPUT ($DEB_SIZE)${NC}"
        
        # Verify package
        dpkg-deb -I "$DEB_OUTPUT"
        dpkg-deb -c "$DEB_OUTPUT"
        
        if [[ "$SIGN" == true ]]; then
            echo -e "${BLUE}Signing .deb...${NC}"
            dpkg-sig --sign builder "$DEB_OUTPUT" 2>/dev/null || gpg --detach-sign "$DEB_OUTPUT"
        fi
    else
        echo -e "${RED}.deb creation failed!${NC}"
        exit 1
    fi
    
    # Cleanup
    rm -rf "$DEB_DIR"
fi

# ============================================================
# Summary
# ============================================================
echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Production build complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "Version: $VERSION"
echo -e "Architecture: $ARCH"
echo -e "Output directory: $OUTPUT_DIR"
echo ""

if [[ "$CREATE_APPIMAGE" == true ]]; then
    echo -e "${BLUE}AppImage:${NC} $OUTPUT_DIR/${APP_NAME}-${VERSION}-${ARCH}.AppImage"
fi

if [[ "$CREATE_DEB" == true ]]; then
    echo -e "${BLUE}.deb:${NC} $OUTPUT_DIR/${APP_NAME}_${VERSION}-1_${ARCH}.deb"
fi

echo ""
echo -e "${YELLOW}Installation instructions:${NC}"
if [[ "$CREATE_APPIMAGE" == true ]]; then
    echo -e "  AppImage: chmod +x ${APP_NAME}-${VERSION}-${ARCH}.AppImage && ./${APP_NAME}-${VERSION}-${ARCH}.AppImage"
fi
if [[ "$CREATE_DEB" == true ]]; then
    echo -e "  .deb:     sudo dpkg -i ${APP_NAME}_${VERSION}-1_${ARCH}.deb"
fi