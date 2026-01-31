# Kora Reclaim Bot (`koralreef`)

High-performance Rust binary for reclaiming rent from idle Solana accounts.

## Features
- **Concurrent Polling:** Background task monitors Solana blockchain for reclaimable accounts.
- **ChatOps Interface:** Telegram bot for manual triggers and status reporting.
- **Secure Storage:** Encrypted SQLite database for sensitive configuration and keypairs.
- **Systemd Integration:** Easy deployment as a persistent daemon.

### Installation

#### 1. Via Cargo (Easiest)
If you have Rust and Cargo installed:
```bash
cargo install koralreef
```

#### 2. Quick Install (Linux/macOS)
You can install the Kora Reclaim Bot directly using the following command:

```bash
curl -sSL https://raw.githubusercontent.com/nathfavour/koralReef/master/install.sh | bash
```

This script will:
1. Install Rust (if not already present).
2. Build the project from source.
3. Install the binary to `~/.local/bin/koralreef`.
4. Set up a default configuration in `~/.koralReef/config.toml`.
5. Create a systemd user service (on Linux).

**Note:** Ensure `~/.local/bin` is in your `PATH`. You can add it by adding this to your `.bashrc` or `.zshrc`:
```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Manual Installation
1. Clone the repository:
   ```bash
   git clone https://github.com/nathfavour/koralReef.git
   cd koralReef
   ```
2. Build and install:
   ```bash
   make install
   ```

## Self-Deployment

`koralreef` is designed to be a lightweight, self-hosted worker. We highly encourage operators to deploy their own instances on personal infrastructure to maintain full control over their keys and reclaimed funds.

### Deployment Options

#### 1. VPS (Recommended for Reliability)
The simplest way to deploy on a Linux VPS (Ubuntu, Debian, etc.):

```bash
# Run the universal installer
curl -sSL https://raw.githubusercontent.com/nathfavour/koralReef/master/install.sh | bash

# Configure your bot
nano ~/.koralReef/config.toml

# Start the service
systemctl --user enable --now koralreef
```

#### 2. Docker (Cloud Agnostic)
Ideal for AWS (ECS), Azure (Container Instances), or DigitalOcean:

```bash
cd deployment
# Edit the config.toml in the deployment folder
docker-compose up -d
```

#### 3. Manual Cloud Deployment (AWS/Azure/GCP)
- **AWS EC2 / Azure VM:** Follow the VPS instructions. Use a `t3.micro` or similar instance (it's very lightweight).
- **Security Note:** Ensure your VPS firewall allows outgoing connections to Solana RPC and Telegram API. No incoming ports need to be opened.

### Why Self-Deploy?
- **Key Sovereignty:** Your Solana private keys never leave your infrastructure.
- **Custom Whitelisting:** Easily manage which accounts are "safe" according to your specific needs.
- **Zero Fees:** Reclaim 100% of the rent directly to your treasury without middleman fees.

---

## Configuration
Edit `~/.koralReef/config.toml` to configure your Solana RPC, Telegram Bot Token, and Treasury address.

```toml
[solana]
rpc_url = "https://api.mainnet-beta.solana.com"
keypair_path = "path/to/your/keypair.json"
treasury_address = "YourTreasuryAddressHere"

[telegram]
bot_token = "YourBotTokenHere"
authorized_user_ids = [12345678]
```

## Usage

### Running as a Service (Recommended)
If you used the install script, you can manage the bot using `systemctl`:

```bash
# Start the bot
systemctl --user start koralreef

# Enable it to start on boot
systemctl --user enable koralreef

# Check logs
journalctl --user -u koralreef -f
```

### Running Manually
```bash
koralreef --config ~/.koralReef/config.toml
```

### Commands
Interact with the bot via Telegram using these commands:
- `/start` - Initialize connection and register admin.
- `/status` - Get current reclamation metrics and health.
- `/sweep` - Force an immediate scan and reclamation cycle.
- `/log` - View the last 10 events from the history.

## Security
- **Encrypted Storage:** All sensitive data (keys, tokens) is stored in an AES-256-GCM encrypted SQLite database at `~/.koralReef/koral.db`.
- **Keypair Management:** You can securely import your Solana keypair directly into the encrypted database:
  ```bash
  koralreef --import-key path/to/your/keypair.json
  ```
  Once imported, the bot no longer needs the plaintext JSON file.

## License
MIT