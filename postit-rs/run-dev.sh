#!/usr/bin/env bash
# run-dev.sh - Development runner for PostItRs
# Usage: ./run-dev.sh [options]

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

# Default options
MODE="debug"
CONFIG_FILE=""
TEST_MODE=false
ONCE=false
LOG_LEVEL="debug"
CARGO_ARGS=()

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            MODE="release"
            shift
            ;;
        --config)
            CONFIG_FILE="$2"
            shift 2
            ;;
        --test)
            TEST_MODE=true
            shift
            ;;
        --once)
            ONCE=true
            shift
            ;;
        --log-level)
            LOG_LEVEL="$2"
            shift 2
            ;;
        --help|-h)
            cat <<EOF
Usage: ./run-dev.sh [OPTIONS]

Development runner for PostItRs

OPTIONS:
    --release          Build and run in release mode (optimized)
    --config FILE      Use specific config file
    --test             Run with test config (yellow post-it, black text, 20% overlay)
    --once             Show post-it once and exit (for testing)
    --log-level LEVEL  Set log level (trace, debug, info, warn, error)
    --help, -h         Show this help message

EXAMPLES:
    ./run-dev.sh                    # Run in debug mode with default config
    ./run-dev.sh --test --once      # Show test post-it once
    ./run-dev.sh --release --once   # Run optimized version once
    ./run-dev.sh --config my.toml   # Use custom config file

EOF
            exit 0
            ;;
        *)
            CARGO_ARGS+=("$1")
            shift
            ;;
    esac
done

# Build command
BUILD_CMD="cargo build"
if [[ "$MODE" == "release" ]]; then
    BUILD_CMD="cargo build --release"
fi

echo -e "${BLUE}Building PostItRs ($MODE mode)...${NC}"
if ! $BUILD_CMD "${CARGO_ARGS[@]}"; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

# Determine binary path
if [[ "$MODE" == "release" ]]; then
    BINARY="./target/release/postit-rs"
else
    BINARY="./target/debug/postit-rs"
fi

# Build run arguments
RUN_ARGS=()
if [[ -n "$CONFIG_FILE" ]]; then
    RUN_ARGS+=(--config "$CONFIG_FILE")
fi
if [[ "$TEST_MODE" == true ]]; then
    RUN_ARGS+=(--test)
fi
if [[ "$ONCE" == true ]]; then
    RUN_ARGS+=(--once)
fi
RUN_ARGS+=(--log-level "$LOG_LEVEL")

echo -e "${GREEN}Running PostItRs...${NC}"
echo -e "${YELLOW}Command: $BINARY ${RUN_ARGS[*]}${NC}"
echo ""

# Run with RUST_LOG for additional logging
RUST_LOG="$LOG_LEVEL" "$BINARY" "${RUN_ARGS[@]}"