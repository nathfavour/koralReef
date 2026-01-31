use solana_sdk::pubkey::Pubkey;
use solana_sdk::account::Account;

pub fn is_safe_to_reclaim(pubkey: &Pubkey, account: &Account, whitelist: &[String]) -> bool {
    if whitelist.contains(&pubkey.to_string()) {
        return false;
    }

    // Ensure it's a Token account
    if account.owner != spl_token::id() {
        return false;
    }

    // Ensure it's empty
    if account.data.len() == 165 {
        let amount_bytes = &account.data[64..72];
        let amount = u64::from_le_bytes(amount_bytes.try_into().unwrap());
        if amount != 0 {
            return false;
        }
    } else {
        return false;
    }

    true
}
