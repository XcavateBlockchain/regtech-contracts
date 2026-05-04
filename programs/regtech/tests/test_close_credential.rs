mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

const PASSING_SCORE: u16 = 8_500;

fn revoked_credential_scenario() -> (litesvm::LiteSVM, Keypair, Keypair, [u8; 16], [u8; 32]) {
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

    send_ok(
        &mut svm,
        ix_claim_credential(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            "ipfs://test".to_string(),
        ),
        &[&partner_admin],
    );

    send_ok(
        &mut svm,
        ix_revoke_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&partner_admin],
    );

    (svm, user, partner_admin, partner_id, module_id_hash)
}

#[test]
fn happy_path_returns_rent_to_vault() {
    let (mut svm, user, partner_admin, partner_id, module_id_hash) =
        revoked_credential_scenario();

    let vault_before = vault_available(&svm, &partner_id);

    send_ok(
        &mut svm,
        ix_close_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&partner_admin],
    );

    let vault_after = vault_available(&svm, &partner_id);
    assert!(vault_after > vault_before, "vault should recover credential rent");

    assert!(
        svm.get_account(&credential_pda(&user.pubkey(), &partner_id, &module_id_hash)).is_none(),
        "credential PDA should be gone",
    );
}

#[test]
fn close_without_revoke_rejected() {
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

    send_ok(
        &mut svm,
        ix_claim_credential(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            "ipfs://test".to_string(),
        ),
        &[&partner_admin],
    );

    let res = send(
        &mut svm,
        ix_close_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::NotRevoked);
}

#[test]
fn non_partner_admin_rejected() {
    let (mut svm, user, _, partner_id, module_id_hash) = revoked_credential_scenario();

    let imposter = Keypair::new();
    fund(&mut svm, &imposter.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_close_credential(imposter.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}
