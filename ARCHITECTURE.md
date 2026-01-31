# Architecture: koralreef

## 1. System Overview
`koralreef` is a concurrent Rust application designed for automated rent reclamation on the Solana blockchain. It operates as a persistent daemon with two primary asynchronous tasks coordinated via shared state.

### Core Components
- **Scanner Task:** Periodically polls Solana RPC for SPL Token accounts owned by the operator that have zero token balance but contain SOL (rent).
- **Bot Task:** Provides a Telegram-based Command Line Interface (CLI) for real-time monitoring, manual triggers, and log retrieval.
- **Shared State:** A thread-safe `Arc<Mutex<AppState>>` structure used to synchronize metrics and control signals between tasks.

## 2. Technical Workflow

### Data Flow
1. **Discovery:** Scanner utilizes `getProgramAccounts` with filters (DataSize: 165, Memcmp: Owner Pubkey).
2. **Verification:** Accounts are cross-referenced against a user-defined whitelist and validated for rent-exempt status.
3. **Execution:** Reclaimer batches up to 20 `CloseAccount` instructions into single transactions to optimize blockspace.
4. **Reporting:** Results are persisted to an encrypted SQLite database and pushed to the Telegram interface.

## 3. Security Architecture
- **At-Rest Encryption:** Sensitive data (Solana keypairs, Telegram tokens) is stored in a SQLite database encrypted with AES-256-GCM.
- **Treasury Separation:** Reclaimed funds are automatically transferred to a configured treasury address, minimizing the balance held by the "hot" operational wallet.
- **Access Control:** The Telegram interface is restricted to a whitelist of authorized User IDs.

## 4. Module Map
- `src/core/`: Solana blockchain interaction logic (Scanning, Transaction construction).
- `src/bot/`: Telegram REPL and command handling.
- `src/storage.rs`: Encrypted persistence layer (SQLite + AES-256-GCM).
- `src/state.rs`: In-memory synchronization primitives.
- `src/config.rs`: TOML and CLI argument parsing.

## 5. Technology Stack
- **Runtime:** `tokio` (Async/Non-blocking I/O)
- **Blockchain:** `solana-client`, `solana-sdk`
- **Interface:** `teloxide` (Telegram Bot Framework)
- **Database:** `rusqlite` with `aes-gcm` for field-level encryption.