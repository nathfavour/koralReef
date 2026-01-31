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
3. Install the binary to `~/.local/bin/kora-reclaim`.
4. Set up a default configuration in `~/.koralReef/config.toml`.
5. Create a systemd user service (on Linux).

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

### Running the Bot
If you used the install script, you can start the bot with:
```bash
systemctl --user start kora-reclaim
```

To run it manually:
```bash
kora-reclaim --config ~/.koralReef/config.toml
```

### Commands
- `/start` - Initialize connection.
- `/status` - Get current reclamation metrics.
- `/sweep` - Force an immediate scan and reclamation.
- `/log` - View recent activity.

## Security
- The bot uses an encrypted SQLite database stored in `~/.koralReef/koral.db`.
- Keypairs can be imported into the encrypted database using:
  ```bash
  kora-reclaim --import-key path/to/keypair.json
  ```

## License
MIT
