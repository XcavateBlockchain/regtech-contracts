#![allow(dead_code)]

pub use solana_signer::Signer;

use {
    anchor_lang::{
        solana_program::{instruction::Instruction, pubkey::Pubkey, system_program},
        AccountDeserialize, AccountSerialize, InstructionData, ToAccountMetas,
    },
    litesvm::{
        types::{FailedTransactionMetadata, TransactionMetadata},
        LiteSVM,
    },
    regtech::{
        constants::{MPL_CORE_KEY_COLLECTION_V1, MPL_CORE_PROGRAM_ID},
        state::{Attempt, Config, Module, Partner},
    },
    solana_account::Account,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_program::hash::hash,
    solana_transaction::versioned::VersionedTransaction,
};

pub const PROGRAM_BYTES: &[u8] = include_bytes!("../../../../target/deploy/regtech.so");

pub fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let _ = svm.add_program(regtech::ID, PROGRAM_BYTES);
    // LiteSVM starts with unix_timestamp at 0, which makes the program's
    // `attempt.last_attempt_at > 0` cooldown guard think nothing has been
    // submitted yet even when something has. Set a realistic epoch time so
    // tests hit the same path production does.
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp = 1_700_000_000;
    svm.set_sysvar(&clock);
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    (svm, admin)
}

pub fn fund(svm: &mut LiteSVM, pubkey: &Pubkey, lamports: u64) {
    svm.airdrop(pubkey, lamports).unwrap();
}

pub fn warp_unix_seconds(svm: &mut LiteSVM, delta_seconds: i64) {
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp = clock.unix_timestamp.saturating_add(delta_seconds);
    svm.set_sysvar(&clock);
}

/// Force a fresh blockhash by advancing the slot. Use this between otherwise-
/// identical transactions in a test to avoid LiteSVM's AlreadyProcessed guard.
pub fn advance_blockhash(svm: &mut LiteSVM) {
    let mut clock = svm.get_sysvar::<solana_clock::Clock>();
    clock.slot = clock.slot.saturating_add(1);
    clock.unix_timestamp = clock.unix_timestamp.saturating_add(1);
    svm.set_sysvar(&clock);
    svm.expire_blockhash();
}

// ----- Error assertions -----
//
// Anchor offsets user-defined errors by 6000, so RegtechError variant N shows
// up as InstructionError::Custom(6000 + N) on a failed tx. We match on the
// Debug output to avoid chasing which crate owns TransactionError and
// InstructionError across the solana 3.x split.

pub const ANCHOR_ERROR_OFFSET: u32 = 6000;

pub fn expect_error_code(
    res: Result<TransactionMetadata, FailedTransactionMetadata>,
    expected_code: u32,
) {
    let failed = res.expect_err("transaction was expected to fail");
    let err_str = format!("{:?}", failed.err);
    assert!(
        err_str.contains(&format!("Custom({})", expected_code)),
        "expected Custom({expected_code}) in error, got:\n{err_str}",
    );
}

pub fn expect_regtech_error(
    res: Result<TransactionMetadata, FailedTransactionMetadata>,
    expected: regtech::error::RegtechError,
) {
    let code = expected as u32 + ANCHOR_ERROR_OFFSET;
    expect_error_code(res, code);
}

// ----- State-mutation helpers for testing defensive constraints -----
//
// There's no set_paused or deactivate_partner instruction yet, but the
// program's `!config.paused`, `partner.active`, and `module.active` checks
// should still fire when those flags flip. We flip them directly via
// LiteSVM so the defensive code gets exercised before the admin ixs land.

pub fn set_config_paused(svm: &mut LiteSVM, paused: bool) {
    let key = config_pda();
    let account = svm.get_account(&key).unwrap();
    let mut config = Config::try_deserialize(&mut &account.data[..]).unwrap();
    config.paused = paused;
    let mut data = Vec::new();
    config.try_serialize(&mut data).unwrap();
    let mut new_account = account;
    new_account.data = data;
    svm.set_account(key, new_account).unwrap();
}

pub fn set_partner_active(svm: &mut LiteSVM, partner_id: &[u8; 16], active: bool) {
    let key = partner_pda(partner_id);
    let account = svm.get_account(&key).unwrap();
    let mut partner = Partner::try_deserialize(&mut &account.data[..]).unwrap();
    partner.active = active;
    let mut data = Vec::new();
    partner.try_serialize(&mut data).unwrap();
    let mut new_account = account;
    new_account.data = data;
    svm.set_account(key, new_account).unwrap();
}

pub fn set_module_active(
    svm: &mut LiteSVM,
    partner_id: &[u8; 16],
    module_id_hash: &[u8; 32],
    active: bool,
) {
    let key = module_pda(partner_id, module_id_hash);
    let account = svm.get_account(&key).unwrap();
    let mut module = Module::try_deserialize(&mut &account.data[..]).unwrap();
    module.active = active;
    let mut data = Vec::new();
    module.try_serialize(&mut data).unwrap();
    let mut new_account = account;
    new_account.data = data;
    svm.set_account(key, new_account).unwrap();
}

// ----- PDA derivation (mirrors program seeds) -----

pub fn config_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"config"], &regtech::ID).0
}

pub fn partner_pda(partner_id: &[u8; 16]) -> Pubkey {
    Pubkey::find_program_address(&[b"partner", partner_id], &regtech::ID).0
}

pub fn module_pda(partner_id: &[u8; 16], module_id_hash: &[u8; 32]) -> Pubkey {
    Pubkey::find_program_address(&[b"module", partner_id, module_id_hash], &regtech::ID).0
}

pub fn attempt_pda(
    user: &Pubkey,
    partner_id: &[u8; 16],
    module_id_hash: &[u8; 32],
) -> Pubkey {
    Pubkey::find_program_address(
        &[b"attempt", user.as_ref(), partner_id, module_id_hash],
        &regtech::ID,
    )
    .0
}

pub fn code_hash(module_code: &str) -> [u8; 32] {
    hash(module_code.as_bytes()).to_bytes()
}

// ----- Instruction builders -----

pub fn ix_initialize_config(
    admin: Pubkey,
    threshold_bps: u16,
    cooldown_secs: i64,
) -> Instruction {
    Instruction {
        program_id: regtech::ID,
        accounts: regtech::accounts::InitializeConfig {
            admin,
            config: config_pda(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: regtech::instruction::InitializeConfig {
            default_pass_threshold_bps: threshold_bps,
            default_cooldown_seconds: cooldown_secs,
        }
        .data(),
    }
}

pub fn ix_register_partner(
    admin: Pubkey,
    partner_id: [u8; 16],
    collection: Pubkey,
    name: String,
    attestor: Pubkey,
    partner_admin: Pubkey,
    threshold_override: Option<u16>,
    cooldown_override: Option<i64>,
) -> Instruction {
    Instruction {
        program_id: regtech::ID,
        accounts: regtech::accounts::RegisterPartner {
            admin,
            config: config_pda(),
            partner: partner_pda(&partner_id),
            collection,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: regtech::instruction::RegisterPartner {
            partner_id,
            name,
            attestor,
            partner_admin,
            pass_threshold_bps_override: threshold_override,
            cooldown_seconds_override: cooldown_override,
        }
        .data(),
    }
}

pub fn ix_register_module(
    partner_admin: Pubkey,
    partner_id: [u8; 16],
    module_id_hash: [u8; 32],
    module_code: String,
    metadata_uri: String,
    threshold_override: Option<u16>,
    cooldown_override: Option<i64>,
    expires_in_seconds: Option<i64>,
) -> Instruction {
    Instruction {
        program_id: regtech::ID,
        accounts: regtech::accounts::RegisterModule {
            partner_admin,
            config: config_pda(),
            partner: partner_pda(&partner_id),
            module: module_pda(&partner_id, &module_id_hash),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: regtech::instruction::RegisterModule {
            module_id_hash,
            module_code,
            metadata_uri,
            pass_threshold_bps_override: threshold_override,
            cooldown_seconds_override: cooldown_override,
            expires_in_seconds,
        }
        .data(),
    }
}

pub fn ix_start_attempt(
    user: Pubkey,
    partner_id: [u8; 16],
    module_id_hash: [u8; 32],
) -> Instruction {
    Instruction {
        program_id: regtech::ID,
        accounts: regtech::accounts::StartAttempt {
            user,
            config: config_pda(),
            partner: partner_pda(&partner_id),
            module: module_pda(&partner_id, &module_id_hash),
            attempt: attempt_pda(&user, &partner_id, &module_id_hash),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: regtech::instruction::StartAttempt {}.data(),
    }
}

pub fn ix_set_paused(admin: Pubkey, paused: bool, reason_code: u8) -> Instruction {
    Instruction {
        program_id: regtech::ID,
        accounts: regtech::accounts::SetPaused {
            admin,
            config: config_pda(),
        }
        .to_account_metas(None),
        data: regtech::instruction::SetPaused { paused, reason_code }.data(),
    }
}

pub fn ix_set_partner_active(
    admin: Pubkey,
    partner_id: [u8; 16],
    active: bool,
    reason_code: u8,
) -> Instruction {
    Instruction {
        program_id: regtech::ID,
        accounts: regtech::accounts::SetPartnerActive {
            admin,
            config: config_pda(),
            partner: partner_pda(&partner_id),
        }
        .to_account_metas(None),
        data: regtech::instruction::SetPartnerActive { active, reason_code }.data(),
    }
}

pub fn ix_set_module_active(
    partner_admin: Pubkey,
    partner_id: [u8; 16],
    module_id_hash: [u8; 32],
    active: bool,
    reason_code: u8,
) -> Instruction {
    Instruction {
        program_id: regtech::ID,
        accounts: regtech::accounts::SetModuleActive {
            partner_admin,
            partner: partner_pda(&partner_id),
            module: module_pda(&partner_id, &module_id_hash),
        }
        .to_account_metas(None),
        data: regtech::instruction::SetModuleActive { active, reason_code }.data(),
    }
}

pub fn ix_propose_admin_update(current_admin: Pubkey, candidate: Pubkey) -> Instruction {
    Instruction {
        program_id: regtech::ID,
        accounts: regtech::accounts::ProposeAdminUpdate {
            admin: current_admin,
            config: config_pda(),
        }
        .to_account_metas(None),
        data: regtech::instruction::ProposeAdminUpdate { candidate }.data(),
    }
}

pub fn ix_accept_admin_update(new_admin: Pubkey) -> Instruction {
    Instruction {
        program_id: regtech::ID,
        accounts: regtech::accounts::AcceptAdminUpdate {
            new_admin,
            config: config_pda(),
        }
        .to_account_metas(None),
        data: regtech::instruction::AcceptAdminUpdate {}.data(),
    }
}

pub fn ix_rotate_attestor(
    partner_admin: Pubkey,
    partner_id: [u8; 16],
    new_attestor: Pubkey,
) -> Instruction {
    Instruction {
        program_id: regtech::ID,
        accounts: regtech::accounts::RotateAttestor {
            partner_admin,
            partner: partner_pda(&partner_id),
        }
        .to_account_metas(None),
        data: regtech::instruction::RotateAttestor { new_attestor }.data(),
    }
}

pub fn ix_submit_attempt(
    attestor: Pubkey,
    user: Pubkey,
    partner_id: [u8; 16],
    module_id_hash: [u8; 32],
    score_bps: u16,
) -> Instruction {
    Instruction {
        program_id: regtech::ID,
        accounts: regtech::accounts::SubmitAttempt {
            attestor,
            user,
            config: config_pda(),
            partner: partner_pda(&partner_id),
            module: module_pda(&partner_id, &module_id_hash),
            attempt: attempt_pda(&user, &partner_id, &module_id_hash),
        }
        .to_account_metas(None),
        data: regtech::instruction::SubmitAttempt { score_bps }.data(),
    }
}

// ----- Transaction helpers -----

pub fn send(
    svm: &mut LiteSVM,
    ix: Instruction,
    signers: &[&Keypair],
) -> Result<TransactionMetadata, FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let payer = signers[0];
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    svm.send_transaction(tx)
}

pub fn send_ok(svm: &mut LiteSVM, ix: Instruction, signers: &[&Keypair]) {
    send(svm, ix, signers).expect("transaction should succeed");
}

// ----- Synthetic mpl-core Collection -----

pub fn make_collection_data(update_authority: Pubkey) -> Vec<u8> {
    make_collection_data_with_key(update_authority, MPL_CORE_KEY_COLLECTION_V1)
}

pub fn make_collection_data_with_key(update_authority: Pubkey, key_byte: u8) -> Vec<u8> {
    let mut data = Vec::with_capacity(1 + 32 + 64);
    data.push(key_byte);
    data.extend_from_slice(update_authority.as_ref());
    data.extend_from_slice(&[0u8; 64]);
    data
}

pub fn install_collection(svm: &mut LiteSVM, key: Pubkey, update_authority: Pubkey) {
    install_collection_custom(svm, key, update_authority, MPL_CORE_KEY_COLLECTION_V1, MPL_CORE_PROGRAM_ID);
}

pub fn install_collection_custom(
    svm: &mut LiteSVM,
    key: Pubkey,
    update_authority: Pubkey,
    key_byte: u8,
    owner: Pubkey,
) {
    let data = make_collection_data_with_key(update_authority, key_byte);
    let account = Account {
        lamports: 10_000_000,
        data,
        owner,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(key, account).unwrap();
}

pub fn install_truncated_collection(svm: &mut LiteSVM, key: Pubkey) {
    let account = Account {
        lamports: 10_000_000,
        data: vec![MPL_CORE_KEY_COLLECTION_V1, 0, 0, 0],
        owner: MPL_CORE_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(key, account).unwrap();
}

// ----- State readers -----

pub fn read_config(svm: &LiteSVM) -> Config {
    let account = svm.get_account(&config_pda()).expect("config account exists");
    Config::try_deserialize(&mut &account.data[..]).expect("decode config")
}

pub fn read_partner(svm: &LiteSVM, partner_id: &[u8; 16]) -> Partner {
    let account = svm
        .get_account(&partner_pda(partner_id))
        .expect("partner account exists");
    Partner::try_deserialize(&mut &account.data[..]).expect("decode partner")
}

pub fn read_module(svm: &LiteSVM, partner_id: &[u8; 16], module_id_hash: &[u8; 32]) -> Module {
    let account = svm
        .get_account(&module_pda(partner_id, module_id_hash))
        .expect("module account exists");
    Module::try_deserialize(&mut &account.data[..]).expect("decode module")
}

pub fn read_attempt(
    svm: &LiteSVM,
    user: &Pubkey,
    partner_id: &[u8; 16],
    module_id_hash: &[u8; 32],
) -> Attempt {
    let account = svm
        .get_account(&attempt_pda(user, partner_id, module_id_hash))
        .expect("attempt account exists");
    Attempt::try_deserialize(&mut &account.data[..]).expect("decode attempt")
}

// ----- Fixture builders (stack common setup) -----

pub struct PlatformFixture {
    pub svm: LiteSVM,
    pub admin: Keypair,
}

pub fn init_platform() -> PlatformFixture {
    let (mut svm, admin) = setup();
    send_ok(
        &mut svm,
        ix_initialize_config(admin.pubkey(), 7_000, 86_400),
        &[&admin],
    );
    PlatformFixture { svm, admin }
}

pub struct PartnerFixture {
    pub svm: LiteSVM,
    pub admin: Keypair,
    pub partner_id: [u8; 16],
    pub partner_admin: Keypair,
    pub attestor: Keypair,
    pub collection: Pubkey,
}

pub fn register_partner_fixture() -> PartnerFixture {
    let PlatformFixture { mut svm, admin } = init_platform();
    let partner_id = [7u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 5_000_000_000);
    fund(&mut svm, &attestor.pubkey(), 5_000_000_000);

    install_collection(&mut svm, collection, partner_pda(&partner_id));

    send_ok(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme Regtech".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );

    PartnerFixture {
        svm,
        admin,
        partner_id,
        partner_admin,
        attestor,
        collection,
    }
}

pub struct ModuleFixture {
    pub svm: LiteSVM,
    pub admin: Keypair,
    pub partner_id: [u8; 16],
    pub partner_admin: Keypair,
    pub attestor: Keypair,
    pub collection: Pubkey,
    pub module_code: String,
    pub module_id_hash: [u8; 32],
}

pub fn register_module_fixture() -> ModuleFixture {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        partner_admin,
        attestor,
        collection,
    } = register_partner_fixture();

    let module_code = "mifid-appr-v1".to_string();
    let module_id_hash = code_hash(&module_code);

    send_ok(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            module_code.clone(),
            "ipfs://QmTestMetadata".to_string(),
            None,
            None,
            Some(31_536_000),
        ),
        &[&partner_admin],
    );

    ModuleFixture {
        svm,
        admin,
        partner_id,
        partner_admin,
        attestor,
        collection,
        module_code,
        module_id_hash,
    }
}
