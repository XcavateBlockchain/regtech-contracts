use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke_signed, system_instruction};
use solana_program::hash::hash;

use crate::constants::{
    BPS_DENOMINATOR, CONFIG_SEED, MAX_MODULE_CODE_LEN, MAX_URI_LEN, MODULE_SEED, PARTNER_SEED,
};
use crate::error::RegtechError;
use crate::state::{Config, Module, Partner};

#[derive(Accounts)]
#[instruction(module_id_hash: [u8; 32])]
pub struct RegisterModule<'info> {
    pub partner_admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ RegtechError::Paused,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
        has_one = partner_admin @ RegtechError::NotAuthorized,
        constraint = partner.active @ RegtechError::PartnerInactive,
    )]
    pub partner: Account<'info, Partner>,

    /// CHECK: Created via CPI in the handler. PDA seeds verified by Anchor.
    #[account(
        mut,
        seeds = [MODULE_SEED, &partner.partner_id, &module_id_hash],
        bump,
    )]
    pub module: UncheckedAccount<'info>,

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
    let module_bump = ctx.bumps.module;

    let rent = Rent::get()?;
    let space = 8 + Module::INIT_SPACE;
    let lamports = rent.minimum_balance(space);
    let partner_own_rent = rent.minimum_balance(8 + Partner::INIT_SPACE);

    let partner_info = ctx.accounts.partner.to_account_info();
    let module_info = ctx.accounts.module.to_account_info();

    require!(
        module_info.data_is_empty(),
        RegtechError::AlreadyInitialized
    );

    let deficit = lamports.saturating_sub(module_info.lamports());

    let vault_available = partner_info
        .lamports()
        .checked_sub(partner_own_rent)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    require!(vault_available >= deficit, RegtechError::VaultInsufficient);

    let module_seeds: &[&[u8]] = &[
        MODULE_SEED,
        &partner_id,
        &module_id_hash,
        &[module_bump],
    ];

    invoke_signed(
        &system_instruction::allocate(&module_info.key(), space as u64),
        std::slice::from_ref(&module_info),
        &[module_seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(&module_info.key(), &crate::ID),
        std::slice::from_ref(&module_info),
        &[module_seeds],
    )?;

    if deficit > 0 {
        **partner_info.try_borrow_mut_lamports()? = partner_info
            .lamports()
            .checked_sub(deficit)
            .ok_or(error!(RegtechError::ArithmeticOverflow))?;
        **module_info.try_borrow_mut_lamports()? = module_info
            .lamports()
            .checked_add(deficit)
            .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    }

    let state = Module {
        partner_id,
        module_id_hash,
        module_code: module_code.clone(),
        metadata_uri,
        pass_threshold_bps,
        cooldown_seconds,
        expires_in_seconds,
        active: true,
        created_at: clock.unix_timestamp,
        bump: module_bump,
    };
    let mut buf = Vec::with_capacity(space);
    state.try_serialize(&mut buf)?;
    let mut data = module_info.try_borrow_mut_data()?;
    data[..buf.len()].copy_from_slice(&buf);
    drop(data);

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
