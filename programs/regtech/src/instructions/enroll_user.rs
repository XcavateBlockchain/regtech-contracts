use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke_signed, system_instruction};

use crate::constants::{ENROLLMENT_SEED, MODULE_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Enrollment, Module, Partner};

#[derive(Accounts)]
pub struct EnrollUser<'info> {
    pub partner_admin: Signer<'info>,

    /// CHECK: only used as a seed for the Enrollment PDA and recorded on it.
    /// Not required to sign. The partner_admin is the authorizing party here.
    pub user: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
        has_one = partner_admin @ RegtechError::NotAuthorized,
        constraint = partner.active @ RegtechError::PartnerInactive,
    )]
    pub partner: Account<'info, Partner>,

    #[account(
        seeds = [MODULE_SEED, &partner.partner_id, &module.module_id_hash],
        bump = module.bump,
        constraint = module.active @ RegtechError::ModuleInactive,
        constraint = module.partner_id == partner.partner_id @ RegtechError::NotAuthorized,
    )]
    pub module: Account<'info, Module>,

    /// CHECK: Created via CPI in the handler. PDA seeds verified by Anchor.
    #[account(
        mut,
        seeds = [
            ENROLLMENT_SEED,
            user.key().as_ref(),
            &partner.partner_id,
            &module.module_id_hash,
        ],
        bump,
    )]
    pub enrollment: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handle_enroll_user(
    ctx: Context<EnrollUser>,
    reason_code: u8,
) -> Result<()> {
    let clock = Clock::get()?;
    let partner_id = ctx.accounts.partner.partner_id;
    let module_id_hash = ctx.accounts.module.module_id_hash;
    let user_key = ctx.accounts.user.key();
    let enrolled_by = ctx.accounts.partner_admin.key();
    let enrollment_bump = ctx.bumps.enrollment;

    let rent = Rent::get()?;
    let space = 8 + Enrollment::INIT_SPACE;
    let lamports = rent.minimum_balance(space);
    let partner_own_rent = rent.minimum_balance(8 + Partner::INIT_SPACE);

    let partner_info = ctx.accounts.partner.to_account_info();
    let enrollment_info = ctx.accounts.enrollment.to_account_info();
    let deficit = lamports.saturating_sub(enrollment_info.lamports());

    let vault_available = partner_info
        .lamports()
        .checked_sub(partner_own_rent)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    require!(vault_available >= deficit, RegtechError::VaultInsufficient);

    let enrollment_seeds: &[&[u8]] = &[
        ENROLLMENT_SEED,
        user_key.as_ref(),
        &partner_id,
        &module_id_hash,
        &[enrollment_bump],
    ];

    invoke_signed(
        &system_instruction::allocate(&enrollment_info.key(), space as u64),
        std::slice::from_ref(&enrollment_info),
        &[enrollment_seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(&enrollment_info.key(), &crate::ID),
        std::slice::from_ref(&enrollment_info),
        &[enrollment_seeds],
    )?;

    if deficit > 0 {
        **partner_info.try_borrow_mut_lamports()? = partner_info
            .lamports()
            .checked_sub(deficit)
            .ok_or(error!(RegtechError::ArithmeticOverflow))?;
        **enrollment_info.try_borrow_mut_lamports()? = enrollment_info
            .lamports()
            .checked_add(deficit)
            .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    }

    let state = Enrollment {
        user: user_key,
        partner_id,
        module_id_hash,
        enrolled_at: clock.unix_timestamp,
        enrolled_by,
        reason_code,
        bump: enrollment_bump,
    };
    let mut buf = Vec::with_capacity(space);
    state.try_serialize(&mut buf)?;
    let mut data = enrollment_info.try_borrow_mut_data()?;
    data[..buf.len()].copy_from_slice(&buf);
    drop(data);

    emit!(UserEnrolled {
        actor: enrolled_by,
        user: user_key,
        partner_id,
        module_id_hash,
        reason_code,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct UserEnrolled {
    pub actor: Pubkey,
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub reason_code: u8,
    pub timestamp: i64,
}
