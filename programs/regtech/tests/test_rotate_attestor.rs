mod common;

use {
    anchor_lang::solana_program::pubkey::Pubkey,
    common::*,
    regtech::error::RegtechError,
    solana_keypair::Keypair,
};

#[test]
fn partner_admin_can_rotate_attestor() {
    let PartnerFixture {
        mut svm,
        partner_id,
        partner_admin,
        attestor: _old,
        ..
    } = register_partner_fixture();

    let new_attestor = Keypair::new();
    fund(&mut svm, &new_attestor.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_rotate_attestor(partner_admin.pubkey(), partner_id, new_attestor.pubkey()),
        &[&partner_admin],
    );

    assert_eq!(read_partner(&svm, &partner_id).attestor, new_attestor.pubkey());
}

#[test]
fn old_attestor_cannot_submit_after_rotation() {
    let ModuleFixture {
        mut svm,
        partner_id,
        partner_admin,
        module_id_hash,
        attestor: old_attestor,
        ..
    } = register_module_fixture();

    let new_attestor = Keypair::new();
    fund(&mut svm, &new_attestor.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_rotate_attestor(partner_admin.pubkey(), partner_id, new_attestor.pubkey()),
        &[&partner_admin],
    );

    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);
    send_ok(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );

    // Old attestor's signature no longer matches the stored partner.attestor.
    let res = send(
        &mut svm,
        ix_submit_attempt(
            old_attestor.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            9_000,
        ),
        &[&old_attestor],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn new_attestor_can_submit_after_rotation() {
    let ModuleFixture {
        mut svm,
        partner_id,
        partner_admin,
        module_id_hash,
        ..
    } = register_module_fixture();

    let new_attestor = Keypair::new();
    fund(&mut svm, &new_attestor.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_rotate_attestor(partner_admin.pubkey(), partner_id, new_attestor.pubkey()),
        &[&partner_admin],
    );

    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);
    send_ok(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );

    send_ok(
        &mut svm,
        ix_submit_attempt(
            new_attestor.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            9_000,
        ),
        &[&new_attestor],
    );

    let a = read_attempt(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert!(a.passed);
}

#[test]
fn non_partner_admin_caller_rejected() {
    let PartnerFixture {
        mut svm,
        partner_id,
        admin,
        ..
    } = register_partner_fixture();

    // Even the super-admin can't rotate a partner's attestor. That's the
    // partner_admin's call alone.
    let new_attestor = Keypair::new();
    let res = send(
        &mut svm,
        ix_rotate_attestor(admin.pubkey(), partner_id, new_attestor.pubkey()),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn rotating_to_zero_pubkey_rejected() {
    let PartnerFixture {
        mut svm,
        partner_id,
        partner_admin,
        ..
    } = register_partner_fixture();

    let res = send(
        &mut svm,
        ix_rotate_attestor(partner_admin.pubkey(), partner_id, Pubkey::default()),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::InvalidPubkey);
}
