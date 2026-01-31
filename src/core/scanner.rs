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
        
        // Filter for SPL Token accounts (165 bytes) owned by the operator's keypair
        let filters = vec![
            RpcFilterType::DataSize(165),
            RpcFilterType::Memcmp(Memcmp::new(
                32, // Offset for owner in SPL Token account
                MemcmpEncodedBytes::Base58(owner.to_string()),
            )),
        ];

        let accounts = self.client.get_program_accounts_with_config(
            &token_program_id,
            solana_client::rpc_config::RpcProgramAccountsConfig {
                filters: Some(filters),
                account_config: solana_client::rpc_config::RpcAccountInfoConfig {
                    encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                    ..Default::default()
                },
                ..Default::default()
            },
        )?;

        let mut reclaimable = Vec::new();
        for (pubkey, account) in accounts {
            if crate::core::safety::is_safe_to_reclaim(&pubkey, &account, whitelist) {
                reclaimable.push((pubkey, account));
            }
        }

        Ok(reclaimable)
    }
}
