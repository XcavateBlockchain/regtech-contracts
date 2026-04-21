use anchor_lang::prelude::*;
use solana_program::hash::hash;

use crate::constants::{
    BPS_DENOMINATOR, CONFIG_SEED, MAX_MODULE_CODE_LEN, MAX_URI_LEN, MODULE_SEED, PARTNER_SEED,
};
use crate::error::RegtechError;
use crate::state::{Config, Module, Partner};

#[derive(Accounts)]
#[instruction(module_id_hash: [u8; 32])]
pub struct RegisterModule<'info> {
    #[account(mut)]
    pub partner_admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ RegtechError::Paused,
    )]
    pub config: Account<'info, Config>,

    #[account(
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
        has_one = partner_admin @ RegtechError::NotAuthorized,
        constraint = partner.active @ RegtechError::PartnerInactive,
    )]
    pub partner: Account<'info, Partner>,

    #[account(
        init,
        payer = partner_admin,
        space = 8 + Module::INIT_SPACE,
        seeds = [MODULE_SEED, &partner.partner_id, &module_id_hash],
        bump,
    )]
    pub module: Account<'info, Module>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handle_register_module(
    ctx: Context<RegisterModule>,
    module_id_hash: [u8; 32],
    module_code: String,
    metadata_uri: String,
    pass_threshold_bps_override: Option<u16>,
    cooldown_seconds_override: Option<i64>,
    expires_in_seconds: Option<i64>,
) -> Result<()> {
    require!(
        module_code.len() <= MAX_MODULE_CODE_LEN,
        RegtechError::StringTooLong
    );
    require!(
        metadata_uri.len() <= MAX_URI_LEN,
        RegtechError::StringTooLong
    );

    let computed_hash = hash(module_code.as_bytes()).to_bytes();
    require!(
        computed_hash == module_id_hash,
        RegtechError::ModuleHashMismatch
    );

    let partner = &ctx.accounts.partner;
    let pass_threshold_bps =
        pass_threshold_bps_override.unwrap_or_else(|| partner.pass_threshold_bps);
    let cooldown_seconds =
        cooldown_seconds_override.unwrap_or_else(|| partner.cooldown_seconds);

    require!(
        pass_threshold_bps <= BPS_DENOMINATOR,
        RegtechError::InvalidThreshold
    );
    require!(cooldown_seconds >= 0, RegtechError::InvalidCooldown);
    if let Some(expires) = expires_in_seconds {
        require!(expires > 0, RegtechError::InvalidExpiry);
    }

    let clock = Clock::get()?;
    let partner_id = partner.partner_id;

    let module = &mut ctx.accounts.module;
    module.partner_id = partner_id;
    module.module_id_hash = module_id_hash;
    module.module_code = module_code.clone();
    module.metadata_uri = metadata_uri;
    module.pass_threshold_bps = pass_threshold_bps;
    module.cooldown_seconds = cooldown_seconds;
    module.expires_in_seconds = expires_in_seconds;
    module.active = true;
    module.created_at = clock.unix_timestamp;
    module.bump = ctx.bumps.module;

    emit!(ModuleRegistered {
        partner_id,
        module_id_hash,
        module_code,
        pass_threshold_bps,
        cooldown_seconds,
        expires_in_seconds,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct ModuleRegistered {
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub module_code: String,
    pub pass_threshold_bps: u16,
    pub cooldown_seconds: i64,
    pub expires_in_seconds: Option<i64>,
    pub timestamp: i64,
}
