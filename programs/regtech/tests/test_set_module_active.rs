mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

#[test]
fn partner_admin_can_deactivate_and_reactivate() {
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
    assert!(!read_module(&svm, &partner_id, &module_id_hash).active);

    send_ok(
        &mut svm,
        ix_set_module_active(partner_admin.pubkey(), partner_id, module_id_hash, true, 0),
        &[&partner_admin],
    );
    assert!(read_module(&svm, &partner_id, &module_id_hash).active);
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

    // The super-admin can't flip a module's active state either. Only the
    // partner_admin registered on the Partner has that authority.
    let res = send(
        &mut svm,
        ix_set_module_active(admin.pubkey(), partner_id, module_id_hash, false, 0),
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

    let res = send(
        &mut svm,
        ix_set_module_active(imposter.pubkey(), partner_id, module_id_hash, false, 0),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn deactivation_blocks_start_attempt() {
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
    fund(&mut svm, &user.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );
    expect_regtech_error(res, RegtechError::ModuleInactive);
}

#[test]
fn deactivation_blocks_submit_attempt() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    // Start an attempt while the module is still active.
    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);
    send_ok(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );

    // Partner admin pulls the module.
    send_ok(
        &mut svm,
        ix_set_module_active(partner_admin.pubkey(), partner_id, module_id_hash, false, 0),
        &[&partner_admin],
    );

    // Score submission is now blocked even though the attempt exists.
    let res = send(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 9_000),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::ModuleInactive);
}

#[test]
fn one_module_deactivation_doesnt_affect_others() {
    // Prove module deactivation is scoped to the specific module.
    // Register a second module and check it still works after the first one
    // is deactivated.
    let ModuleFixture {
        mut svm,
        partner_admin,
        partner_id,
        module_id_hash: module_a_hash,
        ..
    } = register_module_fixture();

    let code_b = "module-b".to_string();
    let module_b_hash = code_hash(&code_b);
    send_ok(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_b_hash,
            code_b,
            "uri-b".to_string(),
            None,
            None,
            None,
        ),
        &[&partner_admin],
    );

    send_ok(
        &mut svm,
        ix_set_module_active(partner_admin.pubkey(), partner_id, module_a_hash, false, 0),
        &[&partner_admin],
    );

    // Module B is untouched.
    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);
    send_ok(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_b_hash),
        &[&user],
    );
}
