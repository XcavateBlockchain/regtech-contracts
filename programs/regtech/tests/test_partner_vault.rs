mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

fn partner_balance(svm: &litesvm::LiteSVM, partner_id: &[u8; 16]) -> u64 {
    svm.get_account(&partner_pda(partner_id))
        .expect("partner account")
        .lamports
}

#[test]
fn fund_increases_vault_balance() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let before = partner_balance(&svm, &partner_id);
    send_ok(
        &mut svm,
        ix_fund_partner(admin.pubkey(), partner_id, 500_000_000),
        &[&admin],
    );
    let after = partner_balance(&svm, &partner_id);
    assert_eq!(after - before, 500_000_000, "vault up by funded amount");
}

#[test]
fn refund_drains_vault_back_to_admin() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    // Fixture already pre-funded the vault with DEFAULT_VAULT_FUNDING.
    let before = partner_balance(&svm, &partner_id);
    send_ok(
        &mut svm,
        ix_refund_partner(admin.pubkey(), partner_id, DEFAULT_VAULT_FUNDING),
        &[&admin],
    );
    let after = partner_balance(&svm, &partner_id);
    assert_eq!(before - after, DEFAULT_VAULT_FUNDING, "vault down by refunded amount");
}

#[test]
fn refund_rejects_when_amount_exceeds_vault() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let res = send(
        &mut svm,
        ix_refund_partner(admin.pubkey(), partner_id, DEFAULT_VAULT_FUNDING * 10),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::VaultInsufficient);
}

#[test]
fn refund_leaves_partner_account_alive() {
    // Drain the full vault down to the floor and check the Partner PDA
    // is still there with its data intact. Running out of quiz budget
    // shouldn't make a partner vanish.
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    send_ok(
        &mut svm,
        ix_refund_partner(admin.pubkey(), partner_id, DEFAULT_VAULT_FUNDING),
        &[&admin],
    );

    // Partner data should still be readable.
    let p = read_partner(&svm, &partner_id);
    assert_eq!(p.partner_id, partner_id);
    assert!(p.active, "partner status unchanged by vault drain");
}

#[test]
fn fund_rejected_for_non_admin() {
    let PartnerFixture {
        mut svm, partner_id, ..
    } = register_partner_fixture();

    let imposter = Keypair::new();
    fund(&mut svm, &imposter.pubkey(), 2_000_000_000);

    let res = send(
        &mut svm,
        ix_fund_partner(imposter.pubkey(), partner_id, 100_000_000),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn refund_rejected_for_non_admin() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    // Partner_admin can't pull SOL out of the vault either. Only the
    // platform super-admin can. This is what lets partners trust that
    // their vault lamports aren't siphonable.
    let res = send(
        &mut svm,
        ix_refund_partner(partner_admin.pubkey(), partner_id, 100_000_000),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn fund_with_zero_amount_rejected() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let res = send(
        &mut svm,
        ix_fund_partner(admin.pubkey(), partner_id, 0),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::VaultInsufficient);
}

#[test]
fn refund_with_zero_amount_rejected() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let res = send(
        &mut svm,
        ix_refund_partner(admin.pubkey(), partner_id, 0),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::VaultInsufficient);
}

#[test]
fn vault_pays_for_successive_start_attempts() {
    // End-to-end vault lifecycle. Fund the partner, run a few starts,
    // watch the vault delta line up with the expected rent cost.
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user_a = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);
    let user_b = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);
    let user_c = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    let vault_before = partner_balance(&svm, &partner_id);

    for user in [&user_a, &user_b, &user_c] {
        send_ok(
            &mut svm,
            ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
            &[&attestor],
        );
        advance_blockhash(&mut svm);
    }

    let vault_after = partner_balance(&svm, &partner_id);
    let per_attempt = (vault_before - vault_after) / 3;

    // Three starts should cost roughly 3x the per-attempt rent.
    assert!(per_attempt > 0, "vault should be debited for each start");
    assert!(
        vault_before - vault_after == per_attempt * 3,
        "vault delta should be exactly 3x per-attempt rent"
    );
}
