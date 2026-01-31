use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_client::rpc_filter::{RpcFilterType, Memcmp, MemcmpEncodedBytes};
use solana_sdk::account::Account;
use anyhow::Result;

pub struct Scanner {
    client: RpcClient,
}

impl Scanner {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            client: RpcClient::new(rpc_url.to_string()),
        }
    }

    pub fn find_reclaimable_accounts(&self, owner: &Pubkey, whitelist: &[String]) -> Result<Vec<(Pubkey, Account)>> {
        let token_program_id = spl_token::id();
        
        let filters = vec![
            RpcFilterType::DataSize(165),
            RpcFilterType::Memcmp(Memcmp::new(
                32, 
                MemcmpEncodedBytes::Base58(owner.to_string()),
            )),
        ];

        let mut delay = std::time::Duration::from_millis(500);
        let mut attempts = 0;
        let max_attempts = 5;

        let accounts = loop {
            match self.client.get_program_accounts_with_config(
                &token_program_id,
                solana_client::rpc_config::RpcProgramAccountsConfig {
                    filters: Some(filters.clone()),
                    account_config: solana_client::rpc_config::RpcAccountInfoConfig {
                        encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ) {
                Ok(accounts) => break accounts,
                Err(e) if attempts < max_attempts => {
                    log::warn!("RPC call failed (attempt {}): {}. Retrying in {:?}...", attempts + 1, e, delay);
                    std::thread::sleep(delay);
                    delay *= 2;
                    attempts += 1;
                }
                Err(e) => return Err(anyhow::anyhow!("RPC failed after {} attempts: {}", max_attempts, e)),
            }
        };

        let mut reclaimable = Vec::new();
        for (pubkey, account) in accounts {
            if crate::core::safety::is_safe_to_reclaim(&pubkey, &account, whitelist) {
                reclaimable.push((pubkey, account));
            }
        }

        Ok(reclaimable)
    }
}
