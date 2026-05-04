use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke_signed, system_instruction};

use crate::constants::{
    ATTEMPT_SEED, CONFIG_SEED, CREDENTIAL_SEED, ENROLLMENT_SEED, MODULE_SEED, PARTNER_SEED,
};
use crate::error::RegtechError;
use crate::state::{Attempt, Config, Credential, Enrollment, Module, Partner};

#[derive(Accounts)]
pub struct ClaimCredential<'info> {
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

    #[account(
        seeds = [MODULE_SEED, &partner.partner_id, &module.module_id_hash],
        bump = module.bump,
        constraint = module.active @ RegtechError::ModuleInactive,
        constraint = module.partner_id == partner.partner_id @ RegtechError::NotAuthorized,
    )]
    pub module: Account<'info, Module>,

    // Enrollment has to still be live when the credential is claimed. If
    // partner_admin revoked, the PDA is closed and the loader bounces with
    // AccountNotInitialized. Passing the quiz on its own isn't enough, the
    // grant of access has to still be in force.
    #[account(
        seeds = [
            ENROLLMENT_SEED,
            enrollment.user.as_ref(),
            &partner.partner_id,
            &module.module_id_hash,
        ],
        bump = enrollment.bump,
    )]
    pub enrollment: Account<'info, Enrollment>,

    // Attempt PDA for the same user+partner+module. The explicit
    // attempt.user == enrollment.user check stops a partner from claiming
    // for user A while pointing at user B's passing attempt.
    #[account(
        seeds = [
            ATTEMPT_SEED,
            attempt.user.as_ref(),
            &partner.partner_id,
            &module.module_id_hash,
        ],
        bump = attempt.bump,
        constraint = attempt.user == enrollment.user @ RegtechError::NotAuthorized,
        constraint = attempt.passed @ RegtechError::AttemptNotPassed,
    )]
    pub attempt: Account<'info, Attempt>,

    /// CHECK: Created via CPI in the handler. PDA seeds verified by Anchor.
    #[account(
        mut,
        seeds = [
            CREDENTIAL_SEED,
            enrollment.user.as_ref(),
            &partner.partner_id,
            &module.module_id_hash,
        ],
        bump,
    )]
    pub credential: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handle_claim_credential(
    ctx: Context<ClaimCredential>,
    metadata_uri: String,
) -> Result<()> {
    require!(
        metadata_uri.len() <= crate::constants::MAX_URI_LEN,
        RegtechError::StringTooLong
    );

    let now = Clock::get()?.unix_timestamp;
    let partner_id = ctx.accounts.partner.partner_id;
    let module = &ctx.accounts.module;
    let enrollment = &ctx.accounts.enrollment;
    let attempt = &ctx.accounts.attempt;
    let issued_by = ctx.accounts.partner_admin.key();
    let credential_bump = ctx.bumps.credential;

    let rent = Rent::get()?;
    let space = 8 + Credential::INIT_SPACE;
    let lamports = rent.minimum_balance(space);
    let partner_own_rent = rent.minimum_balance(8 + Partner::INIT_SPACE);

    let partner_info = ctx.accounts.partner.to_account_info();
    let credential_info = ctx.accounts.credential.to_account_info();
    let user_key = enrollment.user;
    let deficit = lamports.saturating_sub(credential_info.lamports());

    let vault_available = partner_info
        .lamports()
        .checked_sub(partner_own_rent)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    require!(vault_available >= deficit, RegtechError::VaultInsufficient);

    let credential_seeds: &[&[u8]] = &[
        CREDENTIAL_SEED,
        user_key.as_ref(),
        &partner_id,
        &module.module_id_hash,
        &[credential_bump],
    ];

    invoke_signed(
        &system_instruction::allocate(&credential_info.key(), space as u64),
        std::slice::from_ref(&credential_info),
        &[credential_seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(&credential_info.key(), &crate::ID),
        std::slice::from_ref(&credential_info),
        &[credential_seeds],
    )?;

    if deficit > 0 {
        **partner_info.try_borrow_mut_lamports()? = partner_info
            .lamports()
            .checked_sub(deficit)
            .ok_or(error!(RegtechError::ArithmeticOverflow))?;
        **credential_info.try_borrow_mut_lamports()? = credential_info
            .lamports()
            .checked_add(deficit)
            .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    }

    // Snapshot the expiry from the module as it stands right now. If the
    // partner changes the module's expiry policy later, already-issued
    // credentials keep the deadline they were stamped with.
    let expires_at = match module.expires_in_seconds {
        Some(secs) => Some(
            now.checked_add(secs)
                .ok_or(error!(RegtechError::ArithmeticOverflow))?,
        ),
        None => None,
    };

    let state = Credential {
        user: enrollment.user,
        partner_id,
        module_id_hash: module.module_id_hash,
        score_bps: attempt.last_score_bps,
        issued_at: now,
        issued_by,
        expires_at,
        revoked_at: None,
        credential_asset: None,
        metadata_uri,
        bump: credential_bump,
    };
    let mut buf = Vec::with_capacity(space);
    state.try_serialize(&mut buf)?;
    let mut data = credential_info.try_borrow_mut_data()?;
    data[..buf.len()].copy_from_slice(&buf);
    drop(data);

    emit!(CredentialIssued {
        actor: issued_by,
        user: state.user,
        partner_id,
        module_id_hash: state.module_id_hash,
        score_bps: state.score_bps,
        issued_at: state.issued_at,
        expires_at: state.expires_at,
    });

    Ok(())
}

#[event]
pub struct CredentialIssued {
    pub actor: Pubkey,
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub score_bps: u16,
    pub issued_at: i64,
    pub expires_at: Option<i64>,
}
