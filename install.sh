#!/bin/bash

# Kora Reclaim Bot Installer
# Installs koralreef from source (fallback to pre-built binaries)
# Usage: curl -sSL https://raw.githubusercontent.com/nathfavour/koralReef/master/install.sh | bash

set -e

# Ensure HOME is set
if [ -z "$HOME" ]; then
    echo "Error: HOME environment variable is not set."
    exit 1
fi

REPO="nathfavour/koralReef"
REPO_URL="https://github.com/$REPO.git"
BINARY_NAME="koralreef"
PROJECT_NAME="koralreef"
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

# Detect Platform
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64) ARCH_NAME="amd64" ;;
    aarch64|arm64) ARCH_NAME="arm64" ;;
    *) ARCH_NAME="unknown" ;;
esac

install_from_prebuilt() {
    echo -e "${YELLOW}Attempting to install from pre-built binary...${NC}"
    
    # Try to find the latest tag, fallback to 'latest'
    LATEST_TAG=$(curl -sSL "https://api.github.com/repos/$REPO/releases/latest" | grep -oE '"tag_name": *"[^"]+"' | head -n 1 | cut -d'"' -f4 || echo "latest")
    
    if [ -z "$LATEST_TAG" ]; then
        LATEST_TAG="latest"
    fi

    REMOTE_BINARY="koralreef-${OS}-${ARCH_NAME}"
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/$REMOTE_BINARY"
    
    echo -e "${BLUE}Downloading $REMOTE_BINARY ($LATEST_TAG)...${NC}"
    if ! curl -L "$DOWNLOAD_URL" -o "$BINARY_NAME"; then
        echo -e "${YELLOW}Failed to download from $LATEST_TAG. Trying 'latest' tag...${NC}"
        LATEST_TAG="latest"
        DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/$REMOTE_BINARY"
        if ! curl -L "$DOWNLOAD_URL" -o "$BINARY_NAME"; then
            echo -e "${RED}Error: Failed to download pre-built binary.${NC}"
            return 1
        fi
    fi
    
    chmod +x "$BINARY_NAME"
    mkdir -p "$INSTALL_DIR"
    mv "$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    return 0
}

# Try to build from source first
CAN_BUILD=true
if ! command -v cargo &> /dev/null || ! command -v git &> /dev/null; then
    CAN_BUILD=false
    echo -e "${YELLOW}Development tools (cargo/git) not found. Skipping source build.${NC}"
fi

SUCCESS=false

if [ "$CAN_BUILD" = true ]; then
    echo -e "${BLUE}Building from source...${NC}"
    mkdir -p "$INSTALL_DIR"

    if [ -f "Cargo.toml" ] && grep -q "name = \"$PROJECT_NAME\"" "Cargo.toml"; then
        echo -e "${BLUE}Detected local repository. Building from current directory...${NC}"
        BUILD_DIR="."
        IS_LOCAL=true
    else
        BUILD_DIR=$(mktemp -d)
        echo -e "${BLUE}Cloning repository to temporary directory...${NC}"
        if git clone "$REPO_URL" "$BUILD_DIR"; then
            IS_LOCAL=false
        else
            echo -e "${YELLOW}Git clone failed.${NC}"
            CAN_BUILD=false
        fi
    fi

    if [ "$CAN_BUILD" = true ]; then
        cd "$BUILD_DIR"
        if cargo build --release; then
            echo -e "${BLUE}Installing binary to $INSTALL_DIR/$BINARY_NAME...${NC}"
            cp "target/release/$PROJECT_NAME" "$INSTALL_DIR/$BINARY_NAME"
            chmod +x "$INSTALL_DIR/$BINARY_NAME"
            SUCCESS=true
        else
            echo -e "${YELLOW}Source build failed.${NC}"
            SUCCESS=false
        fi

        if [ "$IS_LOCAL" = false ]; then
            cd - > /dev/null
            rm -rf "$BUILD_DIR"
        fi
    fi
fi

# Fallback to pre-built
if [ "$SUCCESS" = false ]; then
    if install_from_prebuilt; then
        SUCCESS=true
    else
        echo -e "${RED}Error: Both source build and pre-built installation failed.${NC}"
        exit 1
    fi
fi

# Setup data directory and initial config
mkdir -p "$DATA_DIR"
if [ ! -f "$DATA_DIR/config.toml" ]; then
    echo -e "${YELLOW}No existing configuration found. Creating default config at $DATA_DIR/config.toml...${NC}"
    if [ -f "config.toml.example" ]; then
        cp config.toml.example "$DATA_DIR/config.toml"
    else
        curl -sSL "https://raw.githubusercontent.com/$REPO/master/config.toml.example" -o "$DATA_DIR/config.toml"
    fi
    echo -e "${YELLOW}Please edit $DATA_DIR/config.toml with your settings before starting.${NC}"
fi

# Systemd Service (Linux only)
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo -e "${BLUE}Setting up systemd user service...${NC}"
    SERVICE_DIR="$HOME/.config/systemd/user"
    mkdir -p "$SERVICE_DIR"
    
    cat <<EOF > "$SERVICE_DIR/koralreef.service"
[Unit]
Description=Kora Reclaim Bot (koralreef)
After=network.target

[Service]
ExecStart=$INSTALL_DIR/$BINARY_NAME --config $DATA_DIR/config.toml --demo-only
Restart=always
RestartSec=10
WorkingDirectory=$DATA_DIR
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
EOF

    systemctl --user daemon-reload
    echo -e "${GREEN}Systemd user service created at $SERVICE_DIR/koralreef.service${NC}"
    echo -e "${BLUE}To start the bot: ${NC}systemctl --user start koralreef"
    echo -e "${BLUE}To check status:  ${NC}systemctl --user status koralreef"
    echo -e "${BLUE}To enable on boot: ${NC}systemctl --user enable koralreef"
    echo -e "${YELLOW}Note: Run 'loginctl enable-linger $USER' to allow the service to run without an active session.${NC}"
fi

# Check if PATH includes INSTALL_DIR
if [[ ":$PATH:" != ":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}Warning: $INSTALL_DIR is not in your PATH.${NC}"
    echo -e "You can add it by adding 'export PATH=\"$HOME/.local/bin:$PATH\"' to your .bashrc or .zshrc."
fi

echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}      Installation Successful!        ${NC}"
echo -e "${GREEN}======================================${NC}"
echo -e "Configuration: $DATA_DIR/config.toml"
echo -e "Binary:        $INSTALL_DIR/$BINARY_NAME"