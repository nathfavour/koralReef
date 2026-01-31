# Kora Reclaim Bot (`koralReef`)

High-performance Rust binary for reclaiming rent from idle Solana accounts.

## Features
- **Concurrent Polling:** Background task monitors Solana blockchain for reclaimable accounts.
- **ChatOps Interface:** Telegram bot for manual triggers and status reporting.
- **Secure Storage:** Encrypted SQLite database for sensitive configuration and keypairs.
- **Systemd Integration:** Easy deployment as a persistent daemon.

## Installation

### Quick Install (Linux/macOS)
You can install the Kora Reclaim Bot directly using the following command:

```bash
curl -sSL https://raw.githubusercontent.com/nathfavour/koralReef/main/install.sh | bash
```

This script will:
1. Install Rust (if not already present).
2. Build the project from source.
3. Install the binary to `~/.local/bin/koralReef`.
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
systemctl --user start koralReef

# Enable it to start on boot
systemctl --user enable koralReef

# Check logs
journalctl --user -u koralReef -f
```

### Running Manually
```bash
koralReef --config ~/.koralReef/config.toml
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
  koralReef --import-key path/to/your/keypair.json
  ```
  Once imported, the bot no longer needs the plaintext JSON file.

## License
MIT

## License
MIT
