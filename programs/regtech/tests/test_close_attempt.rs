mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

const PASSING_SCORE: u16 = 8_500;

#[test]
fn happy_path_returns_rent_to_vault() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = passed_user(
        &mut svm,
        &partner_admin,
        &attestor,
        partner_id,
        module_id_hash,
        PASSING_SCORE,
    );

    let vault_before = vault_available(&svm, &partner_id);

    send_ok(
        &mut svm,
        ix_close_attempt(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&partner_admin],
    );

    let vault_after = vault_available(&svm, &partner_id);
    assert!(vault_after > vault_before, "vault should recover attempt rent");

    assert!(
        svm.get_account(&attempt_pda(&user.pubkey(), &partner_id, &module_id_hash)).is_none(),
        "attempt PDA should be gone",
    );
}

#[test]
fn close_then_re_enroll_and_start_succeeds() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = passed_user(
        &mut svm,
        &partner_admin,
        &attestor,
        partner_id,
        module_id_hash,
        PASSING_SCORE,
    );

    // Revoke enrollment (closes Enrollment PDA).
    send_ok(
        &mut svm,
        ix_revoke_enrollment(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&partner_admin],
    );

    // Close the old Attempt PDA.
    send_ok(
        &mut svm,
        ix_close_attempt(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&partner_admin],
    );

    // Re-enroll and start a fresh attempt.
    advance_blockhash(&mut svm);
    send_ok(
        &mut svm,
        ix_enroll_user(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&partner_admin],
    );
    advance_blockhash(&mut svm);
    send_ok(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );

    let a = read_attempt(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert_eq!(a.attempt_count, 0, "fresh attempt starts at zero");
    assert!(!a.passed, "fresh attempt is not passed");
}

#[test]
fn non_partner_admin_rejected() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);
    send_ok(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );

    let imposter = Keypair::new();
    fund(&mut svm, &imposter.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_close_attempt(imposter.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}
