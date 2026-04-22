mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

#[test]
fn happy_path_writes_expected_enrollment() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = Keypair::new();

    send_ok(
        &mut svm,
        ix_enroll_user(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            0,
        ),
        &[&partner_admin],
    );

    let e = read_enrollment(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert_eq!(e.user, user.pubkey());
    assert_eq!(e.partner_id, partner_id);
    assert_eq!(e.module_id_hash, module_id_hash);
    assert_eq!(e.enrolled_by, partner_admin.pubkey());
    assert_eq!(e.reason_code, 0);
    assert!(e.enrolled_at > 0);
}

#[test]
fn non_partner_admin_caller_rejected() {
    let ModuleFixture {
        mut svm,
        partner_id,
        module_id_hash,
        admin,
        ..
    } = register_module_fixture();

    // The super-admin doesn't get to enroll users either. Only the
    // partner_admin on the Partner record can do it.
    let user = Keypair::new();

    let res = send(
        &mut svm,
        ix_enroll_user(admin.pubkey(), user.pubkey(), partner_id, module_id_hash, 0),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn imposter_rejected() {
    let ModuleFixture {
        mut svm,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let imposter = Keypair::new();
    fund(&mut svm, &imposter.pubkey(), 1_000_000_000);
    let user = Keypair::new();

    let res = send(
        &mut svm,
        ix_enroll_user(
            imposter.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            0,
        ),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn duplicate_enrollment_rejected() {
    // We use `init` rather than `init_if_needed` so a duplicate enroll
    // on the same (user, partner, module) fails loudly instead of
    // silently overwriting. Revoke-then-reenroll is the refresh path.
    let ModuleFixture {
        mut svm,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = Keypair::new();
    send_ok(
        &mut svm,
        ix_enroll_user(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            0,
        ),
        &[&partner_admin],
    );

    let res = send(
        &mut svm,
        ix_enroll_user(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            0,
        ),
        &[&partner_admin],
    );
    assert!(res.is_err(), "second enroll for same user/module fails init");
}

#[test]
fn enroll_rejected_when_partner_inactive() {
    let ModuleFixture {
        mut svm,
        admin,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    send_ok(
        &mut svm,
        ix_set_partner_active(admin.pubkey(), partner_id, false, 0),
        &[&admin],
    );

    let user = Keypair::new();
    let res = send(
        &mut svm,
        ix_enroll_user(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            0,
        ),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::PartnerInactive);
}

#[test]
fn enroll_rejected_when_module_inactive() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    send_ok(
        &mut svm,
        ix_set_module_active(partner_admin.pubkey(), partner_id, module_id_hash, false, 0),
        &[&partner_admin],
    );

    let user = Keypair::new();
    let res = send(
        &mut svm,
        ix_enroll_user(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            0,
        ),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::ModuleInactive);
}

#[test]
fn revoke_happy_path_returns_rent() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = Keypair::new();

    let balance_before = svm.get_account(&partner_admin.pubkey()).unwrap().lamports;

    send_ok(
        &mut svm,
        ix_enroll_user(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            0,
        ),
        &[&partner_admin],
    );

    let balance_after_enroll = svm.get_account(&partner_admin.pubkey()).unwrap().lamports;
    assert!(
        balance_after_enroll < balance_before,
        "partner_admin paid rent + fee for enrollment"
    );

    send_ok(
        &mut svm,
        ix_revoke_enrollment(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            0,
        ),
        &[&partner_admin],
    );

    let balance_after_revoke = svm.get_account(&partner_admin.pubkey()).unwrap().lamports;
    assert!(
        balance_after_revoke > balance_after_enroll,
        "partner_admin got rent back on revoke (minus a second tx fee)"
    );

    // Enrollment account is closed.
    assert!(
        svm.get_account(&enrollment_pda(&user.pubkey(), &partner_id, &module_id_hash))
            .map(|a| a.data.is_empty())
            .unwrap_or(true),
        "revoked enrollment account should be closed"
    );
}

#[test]
fn revoke_rejected_for_non_partner_admin() {
    let ModuleFixture {
        mut svm,
        admin,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = Keypair::new();
    send_ok(
        &mut svm,
        ix_enroll_user(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            0,
        ),
        &[&partner_admin],
    );

    let res = send(
        &mut svm,
        ix_revoke_enrollment(
            admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            0,
        ),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn partner_b_cannot_revoke_partner_a_enrollment() {
    let PlatformFixture { mut svm, admin } = init_platform();

    // Set up Partner A with a module and a user enrolled.
    let partner_a_id = [0xAAu8; 16];
    let partner_admin_a = Keypair::new();
    let attestor_a = Keypair::new();
    let collection_a = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin_a.pubkey(), 2_000_000_000);
    install_collection(&mut svm, collection_a, partner_pda(&partner_a_id));
    send_ok(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_a_id,
            collection_a,
            "Partner A".to_string(),
            attestor_a.pubkey(),
            partner_admin_a.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );

    let code_a = "module-a".to_string();
    let module_a_hash = code_hash(&code_a);
    send_ok(
        &mut svm,
        ix_register_module(
            partner_admin_a.pubkey(),
            partner_a_id,
            module_a_hash,
            code_a,
            "uri".to_string(),
            None,
            None,
            None,
        ),
        &[&partner_admin_a],
    );

    let user = enrolled_user(&mut svm, &partner_admin_a, partner_a_id, module_a_hash);

    // Set up Partner B under a different admin key.
    let partner_b_id = [0xBBu8; 16];
    let partner_admin_b = Keypair::new();
    let attestor_b = Keypair::new();
    let collection_b = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin_b.pubkey(), 2_000_000_000);
    install_collection(&mut svm, collection_b, partner_pda(&partner_b_id));
    send_ok(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_b_id,
            collection_b,
            "Partner B".to_string(),
            attestor_b.pubkey(),
            partner_admin_b.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );

    // Partner B's admin calls revoke with B's Partner account in the
    // partner slot, but the enrollment PDA for this user+module is
    // derived under partner A. Seeds don't line up, tx fails.
    let res = send(
        &mut svm,
        ix_revoke_enrollment(
            partner_admin_b.pubkey(),
            user.pubkey(),
            partner_b_id,
            module_a_hash,
            0,
        ),
        &[&partner_admin_b],
    );
    assert!(res.is_err(), "partner B should not be able to revoke A's enrollment");

    // Partner A's enrollment should still be there, untouched.
    let _still_there =
        read_enrollment(&svm, &user.pubkey(), &partner_a_id, &module_a_hash);
}

#[test]
fn re_enroll_after_revoke_works() {
    // Revoke then enroll again. Two separate audit events, which is
    // how partners refresh enrollment metadata or change reason codes.
    let ModuleFixture {
        mut svm,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = Keypair::new();

    send_ok(
        &mut svm,
        ix_enroll_user(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            1,
        ),
        &[&partner_admin],
    );

    send_ok(
        &mut svm,
        ix_revoke_enrollment(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            2,
        ),
        &[&partner_admin],
    );

    advance_blockhash(&mut svm);

    send_ok(
        &mut svm,
        ix_enroll_user(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            3,
        ),
        &[&partner_admin],
    );

    let e = read_enrollment(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert_eq!(e.reason_code, 3, "re-enrollment carries the new reason code");
}
