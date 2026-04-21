mod common;

use {
    anchor_lang::solana_program::{pubkey::Pubkey, system_program},
    common::*,
    regtech::error::RegtechError,
    solana_keypair::Keypair,
};

fn scenario() -> (litesvm::LiteSVM, Keypair, Keypair, Keypair, Pubkey, [u8; 16]) {
    let PlatformFixture { mut svm, admin } = init_platform();
    let partner_id = [9u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 1_000_000_000);
    install_collection(&mut svm, collection, partner_pda(&partner_id));
    (svm, admin, partner_admin, attestor, collection, partner_id)
}

#[test]
fn happy_path_writes_expected_partner() {
    let (mut svm, admin, partner_admin, attestor, collection, partner_id) = scenario();

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

    let p = read_partner(&svm, &partner_id);
    assert_eq!(p.partner_id, partner_id);
    assert_eq!(p.name, "Acme Regtech");
    assert_eq!(p.credential_collection, collection);
    assert_eq!(p.attestor, attestor.pubkey());
    assert_eq!(p.partner_admin, partner_admin.pubkey());
    assert_eq!(p.pass_threshold_bps, 7_000, "should snapshot config default");
    assert_eq!(p.cooldown_seconds, 86_400, "should snapshot config default");
    assert!(p.active);
}

#[test]
fn explicit_overrides_win_over_config_defaults() {
    let (mut svm, admin, partner_admin, attestor, collection, partner_id) = scenario();

    send_ok(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            Some(8_500),
            Some(3_600),
        ),
        &[&admin],
    );

    let p = read_partner(&svm, &partner_id);
    assert_eq!(p.pass_threshold_bps, 8_500);
    assert_eq!(p.cooldown_seconds, 3_600);
}

#[test]
fn non_admin_caller_rejected() {
    let (mut svm, _admin, partner_admin, attestor, collection, partner_id) = scenario();
    let imposter = Keypair::new();
    fund(&mut svm, &imposter.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_register_partner(
            imposter.pubkey(),
            partner_id,
            collection,
            "Evil".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn name_too_long_rejected() {
    let (mut svm, admin, partner_admin, attestor, collection, partner_id) = scenario();
    let long_name = "a".repeat(65);

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            long_name,
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::StringTooLong);
}

#[test]
fn name_at_exactly_sixty_four_chars_accepted() {
    let (mut svm, admin, partner_admin, attestor, collection, partner_id) = scenario();
    let name = "a".repeat(64);

    send_ok(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            name.clone(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );

    let p = read_partner(&svm, &partner_id);
    assert_eq!(p.name, name);
}

#[test]
fn threshold_override_over_bounds_rejected() {
    let (mut svm, admin, partner_admin, attestor, collection, partner_id) = scenario();

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            Some(10_001),
            None,
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::InvalidThreshold);
}

#[test]
fn negative_cooldown_override_rejected() {
    let (mut svm, admin, partner_admin, attestor, collection, partner_id) = scenario();

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            Some(-1),
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::InvalidCooldown);
}

#[test]
fn collection_not_owned_by_mpl_core_rejected() {
    let PlatformFixture { mut svm, admin } = init_platform();
    let partner_id = [9u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 1_000_000_000);

    // Plant a fake "collection" owned by the system program instead of mpl-core.
    install_collection_custom(
        &mut svm,
        collection,
        partner_pda(&partner_id),
        regtech::constants::MPL_CORE_KEY_COLLECTION_V1,
        system_program::ID,
    );

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::CollectionNotOwnedByMplCore);
}

#[test]
fn collection_key_byte_group_v1_rejected() {
    let PlatformFixture { mut svm, admin } = init_platform();
    let partner_id = [9u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 1_000_000_000);

    install_collection_custom(
        &mut svm,
        collection,
        partner_pda(&partner_id),
        6, // GroupV1
        regtech::constants::MPL_CORE_PROGRAM_ID,
    );

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::CollectionWrongType);
}

#[test]
fn collection_key_byte_asset_v1_rejected() {
    let PlatformFixture { mut svm, admin } = init_platform();
    let partner_id = [9u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 1_000_000_000);

    install_collection_custom(
        &mut svm,
        collection,
        partner_pda(&partner_id),
        1, // AssetV1
        regtech::constants::MPL_CORE_PROGRAM_ID,
    );

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::CollectionWrongType);
}

#[test]
fn collection_data_truncated_rejected() {
    let PlatformFixture { mut svm, admin } = init_platform();
    let partner_id = [9u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 1_000_000_000);

    install_truncated_collection(&mut svm, collection);

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::CollectionWrongType);
}

#[test]
fn collection_update_authority_mismatch_rejected() {
    let PlatformFixture { mut svm, admin } = init_platform();
    let partner_id = [9u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 1_000_000_000);

    // update_authority on the collection points at a random pubkey, not the
    // Partner PDA we're about to create. The authority check should catch it.
    install_collection(&mut svm, collection, Pubkey::new_unique());

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::CollectionAuthorityMismatch);
}

#[test]
fn duplicate_partner_id_rejected() {
    let (mut svm, admin, partner_admin, attestor, collection, partner_id) = scenario();

    send_ok(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme 2".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    // Duplicate init fails at the system program layer, not through our
    // error enum, so we only check the tx fails.
    assert!(res.is_err(), "duplicate partner_id should fail init constraint");
}

#[test]
fn collection_owned_by_random_program_rejected() {
    // The owner check needs to reject any non-mpl-core owner, not just the
    // system program. An attacker could deploy their own program and forge
    // CollectionV1-looking bytes under it; this test makes sure that doesn't
    // slip past the check.
    let PlatformFixture { mut svm, admin } = init_platform();
    let partner_id = [9u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 1_000_000_000);

    install_collection_custom(
        &mut svm,
        collection,
        partner_pda(&partner_id),
        regtech::constants::MPL_CORE_KEY_COLLECTION_V1,
        Pubkey::new_unique(), // hypothetical malicious program
    );

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::CollectionNotOwnedByMplCore);
}

#[test]
fn rejects_when_paused() {
    let (mut svm, admin, partner_admin, attestor, collection, partner_id) = scenario();
    set_config_paused(&mut svm, true);

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::Paused);
}

#[test]
fn collection_exactly_thirty_two_bytes_rejected() {
    // One byte short of what we need. The bounds check runs before the
    // discriminator check, so this proves the guard fires right at the
    // boundary.
    let PlatformFixture { mut svm, admin } = init_platform();
    let partner_id = [9u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 1_000_000_000);

    let account = solana_account::Account {
        lamports: 10_000_000,
        data: vec![0u8; 32],
        owner: regtech::constants::MPL_CORE_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(collection, account).unwrap();

    let res = send(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Acme".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::CollectionWrongType);
}
