mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

const PASSING_SCORE: u16 = 8_500;

fn credential_scenario() -> (litesvm::LiteSVM, Keypair, Keypair, Keypair, [u8; 16], [u8; 32]) {
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

    (svm, user, partner_admin, attestor, partner_id, module_id_hash)
}

#[test]
fn happy_path_sets_revoked_at() {
    let (mut svm, user, partner_admin, _, partner_id, module_id_hash) = credential_scenario();

    send_ok(
        &mut svm,
        ix_revoke_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&partner_admin],
    );

    let c = read_credential(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert!(c.revoked_at.is_some(), "revoked_at should be set");
}

#[test]
fn double_revoke_rejected() {
    let (mut svm, user, partner_admin, _, partner_id, module_id_hash) = credential_scenario();

    send_ok(
        &mut svm,
        ix_revoke_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&partner_admin],
    );

    advance_blockhash(&mut svm);

    let res = send(
        &mut svm,
        ix_revoke_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::AlreadyRevoked);
}

#[test]
fn non_partner_admin_rejected() {
    let (mut svm, user, _, _, partner_id, module_id_hash) = credential_scenario();

    let imposter = Keypair::new();
    fund(&mut svm, &imposter.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_revoke_credential(imposter.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}
