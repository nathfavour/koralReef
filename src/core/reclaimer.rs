use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_token::instruction::close_account;
use anyhow::Result;
use log::{info, error};

pub struct Reclaimer {
    client: RpcClient,
    keypair: Keypair,
    treasury: Pubkey,
}

impl Reclaimer {
    pub fn new(rpc_url: &str, keypair: Keypair, treasury: Pubkey) -> Self {
        Self {
            client: RpcClient::new(rpc_url.to_string()),
            keypair,
            treasury,
        }
    }

    pub fn reclaim_accounts(&self, accounts: &[Pubkey], dry_run: bool) -> Result<(u64, u64)> {
        if accounts.is_empty() {
            return Ok((0, 0));
        }

        if dry_run {
            info!("Dry run: would reclaim {} accounts", accounts.len());
            return Ok((0, accounts.len() as u64));
        }

        let total_lamports = 0;
        let mut closed_count = 0;

        // Batch instructions (up to 20 per transaction)
        for chunk in accounts.chunks(20) {
            let mut instructions = Vec::new();
            for pubkey in chunk {
                let ix = close_account(
                    &spl_token::id(),
                    pubkey,
                    &self.treasury,
                    &self.keypair.pubkey(),
                    &[],
                )?;
                instructions.push(ix);
            }

            let recent_blockhash = self.client.get_latest_blockhash()?;
            let tx = Transaction::new_signed_with_payer(
                &instructions,
                Some(&self.keypair.pubkey()),
                &[&self.keypair],
                recent_blockhash,
            );

            match self.client.send_and_confirm_transaction(&tx) {
                Ok(sig) => {
                    info!("Transaction successful: {}", sig);
                    closed_count += chunk.len() as u64;
                    // Note: Calculating exact lamports reclaimed from tx receipt would be better
                    // but for now we assume success means all in chunk were closed.
                }
                Err(e) => {
                    error!("Transaction failed: {}", e);
                }
            }
        }

        Ok((total_lamports, closed_count))
    }
}
