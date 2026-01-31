# Kora Reclaim Bot (`kora-reclaim-rs`) Architecture

## 1. High-Level Overview

`kora-reclaim-rs` is a high-performance, concurrent Rust binary designed to run as a persistent daemon on Kora Operator infrastructure. It employs a **"ChatOps" architecture**, allowing operators to interact with the background reclamation service via a Telegram interface without requiring a web frontend or exposing public ports.

The system is designed as a **Single-Process, Multi-Threaded Engine**:
1.  **The Sentinel (Thread A):** A background infinite loop that polls the Solana blockchain, identifies idle rent-locked accounts, and safely reclaims SOL.
2.  **The Interface (Thread B):** A Telegram Bot listener that handles incoming commands (`/status`, `/sweep`) and pushes notifications.
3.  **Shared State:** An atomic, thread-safe memory structure that synchronizes metrics between the scanning engine and the user interface.

### System Diagram

```mermaid
graph TD
    User((Operator)) -->|Telegram Commands| BotThread
    Config[Config.toml] -->|Load Settings| Main
    Keypair[Wallet Keypair] -->|Sign Tx| CoreEngine

    subgraph "Rust Binary (kora-reclaim-rs)"
        Main[Main Entry Point]
        
        subgraph "Async Runtime (Tokio)"
            BotThread[Telegram Listener Task]
            SentinelThread[Sentinel Loop Task]
            
            SharedState[Arc Mutex AppState]
        end
        
        BotThread <-->|Read/Write Metrics| SharedState
        SentinelThread -->|Update Metrics| SharedState
        BotThread -->|Trigger Manual Sweep| SentinelThread
    end

    subgraph "External World"
        SolanaRPC[Solana Mainnet RPC]
        Kora[Kora Protocol Accounts]
    end

    SentinelThread <-->|getProgramAccounts| SolanaRPC
    SentinelThread -->|close_account Tx| Kora

2. Directory Structure
The project follows a modular "domain-driven" layout to separate Solana logic from Bot interaction logic.
kora-reclaim-rs/
├── Cargo.toml            # Dependencies (solana-client, teloxide, tokio)
├── config.toml.example   # Template for operators
├── src/
│   ├── main.rs           # CLI Entry point, Tokio runtime init, thread spawning
│   ├── config.rs         # Configuration structs (Serde/TOML) & Arg parsing
│   ├── state.rs          # Shared AppState definitions (Metrics, Status)
│   │
│   ├── bot/              # TELEGRAM INTERFACE MODULE
│   │   ├── mod.rs        # Teloxide REPL handler
│   │   └── commands.rs   # Enum for commands (/start, /stats, /log)
│   │
│   └── core/             # SOLANA LOGIC MODULE
│       ├── mod.rs
│       ├── scanner.rs    # RPC Polling & Account Filtering logic
│       ├── reclaimer.rs  # Transaction construction & Signing
│       └── safety.rs     # "Cool-down" & Whitelist verification
│
└── tests/                # Integration tests for dry-run logic

3. Module Responsibilities
A. src/main.rs (The Orchestrator)
 * Role: Initializes the application.
 * Key Responsibilities:
   * Parses CLI arguments (using clap) to determine mode (start, dry-run, config).
   * Loads the Config struct from disk.
   * Initializes the Arc<Mutex<AppState>>.
   * Spawns two concurrent tokio tasks:
     * The monitor_loop (Sentinel).
     * The bot_listener (Interface).
   * Handles graceful shutdown (Ctrl+C).
B. src/core/scanner.rs (The Eye)
 * Role: Identifies reclaimable accounts.
 * Logic:
   * Connects to Solana RPC.
   * Uses getProgramAccounts with specific filters:
     * Program ID: Tokenkeg... (SPL Token).
     * Data Size: 165 bytes.
     * Owner: Matches the Operator's Configured Keypair.
   * Crucial Filter: Checks account.lamports > 0 AND token_amount.amount == 0.
C. src/core/reclaimer.rs (The Hand)
 * Role: Executes the cleanup.
 * Logic:
   * Receives a list of Pubkeys from the Scanner.
   * Constructs a close_account instruction for each.
   * Batching: Bundles up to ~20 instructions per transaction to save on blockspace/fees.
   * Signing: Signs with the local Keypair (hot wallet).
   * Destination: Hardcoded to send reclaimed rent to the treasury_address defined in config.
D. src/bot/mod.rs (The Voice)
 * Role: Interactive command center.
 * Logic:
   * Authentication: Ignores messages from User_IDs not in the whitelist.
   * /stats: Locks AppState and reports total SOL reclaimed and uptime.
   * /sweep: Sets a force_run flag in AppState that the Sentinel checks immediately.
4. Data Flow & Logic Cycles
The Automated Cycle (Default)
 * Sleep: Sentinel sleeps for scan_interval (default: 6 hours).
 * Wake & Scan: Queries RPC for candidate accounts.
 * Safety Check: * Is the account on the whitelist? (Skip)
   * Is the account "dust" (balance < minimum rent)? (Skip)
 * Dry Run Check: If config is dry_run = true, just log the potential profit.
 * Execute: Build and send Transaction.
 * Update State: Increment total_reclaimed in SharedState.
 * Notify: Push a message via the Telegram Bot: "♻️ Reclaimed 0.04 SOL from 20 accounts."
The Manual "ChatOps" Cycle
 * Operator sends /sweep to Bot.
 * Bot validates User ID.
 * Bot triggers the Scanner function immediately (bypassing the timer).
 * Bot replies with a "Processing..." message.
 * Upon completion, Bot edits the message with the final report.
5. Security & Safety Mechanisms
 * Treasury Isolation: The bot can be configured to send reclaimed funds to a cold wallet address, meaning the hot wallet running the bot never holds the profit.
 * Whitelist Protection: config.toml supports a list of accounts that must never be closed, preventing accidental deletion of critical operational accounts.
 * Rent Exemption Check: The logic strictly verifies that the account balance matches the rent-exempt minimum for its size before attempting closure, preventing errors on non-rent-paying accounts.
 * RPC Rate Limiting: The scanner implements exponential backoff to avoid being IP-banned by public RPC nodes.
6. Technology Stack
 * Language: Rust (2021 Edition)
 * Async Runtime: tokio (v1.0+)
 * Solana Interaction: solana-client, solana-sdk
 * Telegram API: teloxide
 * Config Management: confy or toml
 * CLI Interface: clap (derive feature)
<!-- end list -->

