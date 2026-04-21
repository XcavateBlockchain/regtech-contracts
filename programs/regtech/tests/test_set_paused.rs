mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

#[test]
fn admin_can_pause_and_unpause() {
    let PlatformFixture { mut svm, admin } = init_platform();

    send_ok(&mut svm, ix_set_paused(admin.pubkey(), true, 0), &[&admin]);
    assert!(read_config(&svm).paused);

    send_ok(&mut svm, ix_set_paused(admin.pubkey(), false, 0), &[&admin]);
    assert!(!read_config(&svm).paused);
}

#[test]
fn non_admin_caller_rejected() {
    let PlatformFixture { mut svm, admin: _real_admin } = init_platform();
    let imposter = Keypair::new();
    fund(&mut svm, &imposter.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_set_paused(imposter.pubkey(), true, 0),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn pause_blocks_register_partner() {
    let PlatformFixture { mut svm, admin } = init_platform();
    send_ok(&mut svm, ix_set_paused(admin.pubkey(), true, 0), &[&admin]);

    let partner_id = [7u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 1_000_000_000);
    install_collection(&mut svm, collection, partner_pda(&partner_id));

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
fn unpause_restores_normal_operation() {
    let PlatformFixture { mut svm, admin } = init_platform();
    send_ok(&mut svm, ix_set_paused(admin.pubkey(), true, 0), &[&admin]);
    send_ok(&mut svm, ix_set_paused(admin.pubkey(), false, 0), &[&admin]);

    let partner_id = [7u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 1_000_000_000);
    install_collection(&mut svm, collection, partner_pda(&partner_id));

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
}

