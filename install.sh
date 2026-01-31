#!/bin/bash

# Kora Reclaim Bot Installer
# Installs kora-reclaim-rs from source and sets up a systemd service.
# Usage: curl -sSL https://raw.githubusercontent.com/nathfavour/koralReef/main/install.sh | bash

set -e

# Ensure HOME is set
if [ -z "$HOME" ]; then
    echo "Error: HOME environment variable is not set."
    exit 1
fi

REPO_URL="https://github.com/nathfavour/koralReef.git"
BINARY_NAME="kora-reclaim"
PROJECT_NAME="kora-reclaim-rs"
INSTALL_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.koralReef"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}      Kora Reclaim Bot Installer      ${NC}"
echo -e "${BLUE}======================================${NC}"

# Check for git
if ! command -v git &> /dev/null; then
    echo -e "${RED}Error: git is not installed. Please install git and try again.${NC}"
    exit 1
fi

# Check for Rust/Cargo
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Rust/Cargo not found. Installing Rust toolchain...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}Rust installed successfully.${NC}"
fi

# Create installation directory
mkdir -p "$INSTALL_DIR"

# Check if we are already in the repository
if [ -f "Cargo.toml" ] && grep -q "name = \"$PROJECT_NAME\"" "Cargo.toml"; then
    echo -e "${BLUE}Detected local repository. Building from current directory...${NC}"
    BUILD_DIR="."
    IS_LOCAL=true
else
    # Temporary build directory
    BUILD_DIR=$(mktemp -d)
    echo -e "${BLUE}Cloning repository to temporary directory...${NC}"
    git clone "$REPO_URL" "$BUILD_DIR"
    IS_LOCAL=false
fi

cd "$BUILD_DIR"

# Build the project
echo -e "${BLUE}Building $PROJECT_NAME in release mode... (this may take a few minutes)${NC}"
cargo build --release

# Install binary
echo -e "${BLUE}Installing binary to $INSTALL_DIR/$BINARY_NAME...${NC}"
cp "target/release/$PROJECT_NAME" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Setup data directory and initial config
mkdir -p "$DATA_DIR"
if [ ! -f "$DATA_DIR/config.toml" ]; then
    echo -e "${YELLOW}No existing configuration found. Creating default config at $DATA_DIR/config.toml...${NC}"
    cp config.toml.example "$DATA_DIR/config.toml"
    echo -e "${YELLOW}Please edit $DATA_DIR/config.toml with your settings before starting.${NC}"
fi

# Systemd Service (Linux only)
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo -e "${BLUE}Setting up systemd user service...${NC}"
    SERVICE_DIR="$HOME/.config/systemd/user"
    mkdir -p "$SERVICE_DIR"
    
    cat <<EOF > "$SERVICE_DIR/kora-reclaim.service"
[Unit]
Description=Kora Reclaim Bot
After=network.target

[Service]
ExecStart=$INSTALL_DIR/$BINARY_NAME --config $DATA_DIR/config.toml
Restart=always
RestartSec=10
WorkingDirectory=$DATA_DIR
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
EOF

    systemctl --user daemon-reload
    echo -e "${GREEN}Systemd user service created at $SERVICE_DIR/kora-reclaim.service${NC}"
    echo -e "${BLUE}To start the bot: ${NC}systemctl --user start kora-reclaim"
    echo -e "${BLUE}To check status:  ${NC}systemctl --user status kora-reclaim"
    echo -e "${BLUE}To enable on boot: ${NC}systemctl --user enable kora-reclaim"
    echo -e "${YELLOW}Note: Run 'loginctl enable-linger \$USER' to allow the service to run without an active session.${NC}"
fi

# Cleanup
if [ "$IS_LOCAL" = false ]; then
    cd - > /dev/null
    rm -rf "$BUILD_DIR"
fi

# Check if PATH includes INSTALL_DIR
if [[ ":$PATH:" != ":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}Warning: $INSTALL_DIR is not in your PATH.${NC}"
    echo -e "You can add it by adding 'export PATH="\$HOME/.local/bin:\$PATH"' to your .bashrc or .zshrc."
fi

echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}      Installation Successful!        ${NC}"
echo -e "${GREEN}======================================${NC}"
echo -e "Configuration: $DATA_DIR/config.toml"
echo -e "Binary:        $INSTALL_DIR/$BINARY_NAME"
