use anchor_lang::prelude::*;

use crate::constants::{MODULE_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Module, Partner};

// Deliberately NOT gated on !config.paused. Partner admins should be able to
// deactivate a broken or compromised module even while the program is paused
// globally. See the same comment on set_partner_active.
#[derive(Accounts)]
pub struct SetModuleActive<'info> {
    #[account(mut)]
    pub partner_admin: Signer<'info>,

    #[account(
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
        has_one = partner_admin @ RegtechError::NotAuthorized,
    )]
    pub partner: Account<'info, Partner>,

    #[account(
        mut,
        seeds = [MODULE_SEED, &partner.partner_id, &module.module_id_hash],
        bump = module.bump,
        constraint = module.partner_id == partner.partner_id @ RegtechError::NotAuthorized,
    )]
    pub module: Account<'info, Module>,
}

pub(crate) fn handle_set_module_active(
    ctx: Context<SetModuleActive>,
    active: bool,
    reason_code: u8,
) -> Result<()> {
    let actor = ctx.accounts.partner_admin.key();
    let module = &mut ctx.accounts.module;
    let was_active = module.active;
    module.active = active;

    emit!(ModuleActiveChanged {
        actor,
        partner_id: module.partner_id,
        module_id_hash: module.module_id_hash,
        was_active,
        now_active: active,
        reason_code,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct ModuleActiveChanged {
    pub actor: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub was_active: bool,
    pub now_active: bool,
    pub reason_code: u8,
    pub timestamp: i64,
}
