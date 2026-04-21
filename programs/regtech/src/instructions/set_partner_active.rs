use anchor_lang::prelude::*;

use crate::constants::{CONFIG_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Config, Partner};

#[derive(Accounts)]
pub struct SetPartnerActive<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ RegtechError::NotAuthorized,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
    )]
    pub partner: Account<'info, Partner>,
}

pub(crate) fn handle_set_partner_active(
    ctx: Context<SetPartnerActive>,
    active: bool,
    reason_code: u8,
) -> Result<()> {
    let actor = ctx.accounts.admin.key();
    let partner = &mut ctx.accounts.partner;
    let was_active = partner.active;
    partner.active = active;

    emit!(PartnerActiveChanged {
        actor,
        partner_id: partner.partner_id,
        was_active,
        now_active: active,
        reason_code,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct PartnerActiveChanged {
    pub actor: Pubkey,
    pub partner_id: [u8; 16],
    pub was_active: bool,
    pub now_active: bool,
    pub reason_code: u8,
    pub timestamp: i64,
}
