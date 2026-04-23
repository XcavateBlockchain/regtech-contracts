use anchor_lang::prelude::*;

#[error_code]
pub enum RegtechError {
    #[msg("Invalid threshold (must be 0-10000 bps)")]
    InvalidThreshold,
    #[msg("Invalid cooldown (must be non-negative)")]
    InvalidCooldown,
    #[msg("Invalid expiry (must be positive)")]
    InvalidExpiry,
    #[msg("Invalid score (must be 0-10000 bps)")]
    InvalidScore,
    #[msg("Program is paused")]
    Paused,
    #[msg("Caller is not authorized")]
    NotAuthorized,
    #[msg("Partner is inactive")]
    PartnerInactive,
    #[msg("Module is inactive")]
    ModuleInactive,
    #[msg("String exceeds maximum length")]
    StringTooLong,
    #[msg("Module id hash does not match sha256(module_code)")]
    ModuleHashMismatch,
    #[msg("Collection account is not owned by mpl-core")]
    CollectionNotOwnedByMplCore,
    #[msg("Account is not a mpl-core CollectionV1 or is too short")]
    CollectionWrongType,
    #[msg("Collection update authority does not match Partner PDA")]
    CollectionAuthorityMismatch,
    #[msg("Cooldown period has not elapsed")]
    CooldownNotElapsed,
    #[msg("User has already passed this module")]
    AlreadyPassed,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("Pubkey cannot be the default (zero) pubkey")]
    InvalidPubkey,
    #[msg("No admin rotation is pending")]
    NoPendingAdmin,
    #[msg("Signer does not match the pending admin proposal")]
    PendingAdminMismatch,
    #[msg("Attempt has not passed")]
    AttemptNotPassed,
    #[msg("Partner vault has insufficient lamports")]
    VaultInsufficient,
}
