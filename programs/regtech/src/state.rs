use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub pending_admin: Option<Pubkey>,
    pub paused: bool,
    pub default_pass_threshold_bps: u16,
    pub default_cooldown_seconds: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Partner {
    pub partner_id: [u8; 16],
    #[max_len(64)]
    pub name: String,
    pub credential_collection: Pubkey,
    pub attestor: Pubkey,
    pub partner_admin: Pubkey,
    pub pass_threshold_bps: u16,
    pub cooldown_seconds: i64,
    pub active: bool,
    pub created_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Module {
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    #[max_len(64)]
    pub module_code: String,
    #[max_len(256)]
    pub metadata_uri: String,
    pub pass_threshold_bps: u16,
    pub cooldown_seconds: i64,
    pub expires_in_seconds: Option<i64>,
    pub active: bool,
    pub created_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Attempt {
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub last_attempt_at: i64,
    pub last_score_bps: u16,
    pub attempt_count: u32,
    pub passed: bool,
    pub passed_at: Option<i64>,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Enrollment {
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub enrolled_at: i64,
    pub enrolled_by: Pubkey,
    pub reason_code: u8,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Credential {
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub score_bps: u16,
    pub issued_at: i64,
    pub issued_by: Pubkey,
    // Stamped at issuance from the module's current policy. If the module
    // changes its expiry later, credentials already out the door keep the
    // deadline they were issued with.
    pub expires_at: Option<i64>,
    // Filled in by a future revoke_credential ix. None means still live.
    pub revoked_at: Option<i64>,
    // Filled in when the off-chain mpl-core Asset mint gets linked back.
    // Option so claim_credential can land before that ix exists.
    pub credential_asset: Option<Pubkey>,
    pub bump: u8,
}
